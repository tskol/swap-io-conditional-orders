mod common;

use anchor_lang::prelude::{AccountInfo, AccountLoader, Pubkey, Result};
use anchor_spl::token_2022::spl_token_2022::{
    extension::{
        confidential_transfer::{ConfidentialTransferAccount, ConfidentialTransferMint},
        default_account_state::DefaultAccountState,
        transfer_fee::TransferFeeConfig,
        transfer_hook::TransferHook,
        ExtensionType, StateWithExtensionsMut,
    },
    state::{
        Account as Token2022Account, AccountState as Token2022AccountState, Mint as Token2022Mint,
    },
};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id, token,
    token_2022::spl_token_2022,
};
use bytemuck::Zeroable;
use common::{
    install_noop_syscall_stubs, mint_account_data as token_2022_mint_data,
    token_account_data as token_2022_account_data, zero_copy_account_data,
};
use limo::{
    state::{GlobalConfig, Order},
    utils::constraints::{
        check_permission_express_relay_and_get_fees, create_new_orders_disabled,
        emergency_mode_disabled, flash_taking_orders_disabled, get_token_account_checked,
        is_counterparty_matching, is_wsol, order_expired, taking_orders_disabled,
        token_2022::validate_token_extensions, verify_ata,
    },
};
use solana_program::{program_option::COption, program_pack::Pack};

fn run_global_config_guard(
    global_config: GlobalConfig,
    guard: for<'info> fn(&AccountLoader<'info, GlobalConfig>) -> Result<()>,
) -> bool {
    let key = Pubkey::new_unique();
    let owner = limo::ID;
    let mut lamports = 0;
    let mut data = zero_copy_account_data(&global_config);
    let account = AccountInfo::new(
        &key,
        false,
        true,
        &mut lamports,
        &mut data,
        &owner,
        false,
        0,
    );
    let loader = AccountLoader::<GlobalConfig>::try_from(&account).unwrap();

    guard(&loader).is_err()
}

fn run_order_expired_guard(order: Order) -> bool {
    let key = Pubkey::new_unique();
    let owner = limo::ID;
    let mut lamports = 0;
    let mut data = zero_copy_account_data(&order);
    let account = AccountInfo::new(
        &key,
        false,
        true,
        &mut lamports,
        &mut data,
        &owner,
        false,
        0,
    );
    let loader = AccountLoader::<Order>::try_from(&account).unwrap();

    order_expired(&loader).is_err()
}

fn token_account_data(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
    let mut data = vec![0; token::spl_token::state::Account::LEN];
    let token_account = token::spl_token::state::Account {
        mint,
        owner,
        amount,
        delegate: COption::None,
        state: token::spl_token::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    token::spl_token::state::Account::pack(token_account, &mut data).unwrap();
    data
}

fn token_2022_mint_data_with_extensions<F>(
    extension_types: &[ExtensionType],
    configure: F,
) -> Vec<u8>
where
    F: FnOnce(&mut StateWithExtensionsMut<Token2022Mint>),
{
    let mint_size =
        ExtensionType::try_calculate_account_len::<Token2022Mint>(extension_types).unwrap();
    let mut data = vec![0; mint_size];
    {
        let mut state =
            StateWithExtensionsMut::<Token2022Mint>::unpack_uninitialized(&mut data).unwrap();

        for extension_type in extension_types {
            match extension_type {
                ExtensionType::ConfidentialTransferMint => {
                    state
                        .init_extension::<ConfidentialTransferMint>(true)
                        .unwrap();
                }
                ExtensionType::DefaultAccountState => {
                    state.init_extension::<DefaultAccountState>(true).unwrap();
                }
                ExtensionType::TransferFeeConfig => {
                    state.init_extension::<TransferFeeConfig>(true).unwrap();
                }
                ExtensionType::TransferHook => {
                    state.init_extension::<TransferHook>(true).unwrap();
                }
                other => panic!("unsupported test mint extension: {other:?}"),
            }
        }

        configure(&mut state);
        state.base = Token2022Mint {
            mint_authority: COption::None,
            supply: 0,
            decimals: 6,
            is_initialized: true,
            freeze_authority: COption::None,
        };
        state.pack_base();
        state.init_account_type().unwrap();
    }

    data
}

fn token_2022_account_data_with_confidential_extension<F>(
    mint: Pubkey,
    owner: Pubkey,
    configure: F,
) -> Vec<u8>
where
    F: FnOnce(&mut ConfidentialTransferAccount),
{
    let account_size = ExtensionType::try_calculate_account_len::<Token2022Account>(&[
        ExtensionType::ConfidentialTransferAccount,
    ])
    .unwrap();
    let mut data = vec![0; account_size];
    {
        let mut state =
            StateWithExtensionsMut::<Token2022Account>::unpack_uninitialized(&mut data).unwrap();
        state
            .init_extension::<ConfidentialTransferAccount>(true)
            .unwrap();
        configure(
            state
                .get_extension_mut::<ConfidentialTransferAccount>()
                .unwrap(),
        );
        state.base = Token2022Account {
            mint,
            owner,
            amount: 1,
            delegate: COption::None,
            state: Token2022AccountState::Initialized,
            is_native: COption::None,
            delegated_amount: 0,
            close_authority: COption::None,
        };
        state.pack_base();
        state.init_account_type().unwrap();
    }

    data
}

fn validate_token_2022_mint_data_without_token_accounts(
    mint_key: &Pubkey,
    mint_data: &mut [u8],
) -> Result<()> {
    let token_program = spl_token_2022::ID;
    let mut mint_lamports = 0;
    let mint = account_info_with_data(mint_key, &token_program, &mut mint_lamports, mint_data);

    validate_token_extensions(&mint, vec![])
}

fn validate_token_2022_mint_and_account_data(
    mint_key: &Pubkey,
    mint_data: &mut [u8],
    token_data: &mut [u8],
) -> Result<()> {
    let token_program = spl_token_2022::ID;
    let token_account_key = Pubkey::new_unique();
    let mut mint_lamports = 0;
    let mut token_lamports = 0;
    let mint = account_info_with_data(mint_key, &token_program, &mut mint_lamports, mint_data);
    let token_account = account_info_with_data(
        &token_account_key,
        &token_program,
        &mut token_lamports,
        token_data,
    );

    validate_token_extensions(&mint, vec![&token_account])
}

fn account_info_with_data<'a>(
    key: &'a Pubkey,
    owner: &'a Pubkey,
    lamports: &'a mut u64,
    data: &'a mut [u8],
) -> AccountInfo<'a> {
    AccountInfo::new(key, false, false, lamports, data, owner, false, 0)
}

#[test]
fn verify_ata_accepts_expected_associated_token_address() {
    let wallet = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let ata = get_associated_token_address_with_program_id(&wallet, &mint, &token::ID);

    verify_ata(&wallet, &mint, &ata, &token::ID).unwrap();
}

#[test]
fn verify_ata_rejects_unrelated_address() {
    let wallet = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let unrelated_ata = Pubkey::new_unique();

    assert!(verify_ata(&wallet, &mint, &unrelated_ata, &token::ID).is_err());
}

#[test]
fn is_wsol_matches_native_mint_only() {
    assert!(is_wsol(&token::spl_token::native_mint::ID));
    assert!(!is_wsol(&Pubkey::new_unique()));
}

#[test]
fn counterparty_matching_uses_allowed_taker_when_counterparty_is_unset() {
    let allowed_taker = Pubkey::new_unique();
    let blocked_taker = Pubkey::new_unique();

    assert!(is_counterparty_matching(
        &Pubkey::default(),
        &allowed_taker,
        &allowed_taker,
    ));
    assert!(!is_counterparty_matching(
        &Pubkey::default(),
        &allowed_taker,
        &blocked_taker,
    ));
}

#[test]
fn counterparty_matching_prefers_explicit_counterparty() {
    let allowed_taker = Pubkey::new_unique();
    let counterparty = Pubkey::new_unique();

    assert!(is_counterparty_matching(
        &counterparty,
        &allowed_taker,
        &counterparty,
    ));
    assert!(!is_counterparty_matching(
        &counterparty,
        &allowed_taker,
        &allowed_taker,
    ));
}

#[test]
fn global_config_guards_accept_disabled_flags_and_reject_enabled_flags() {
    let mut global_config = GlobalConfig::zeroed();

    assert!(!run_global_config_guard(
        global_config,
        emergency_mode_disabled,
    ));
    global_config.emergency_mode = 1;
    assert!(run_global_config_guard(
        global_config,
        emergency_mode_disabled,
    ));

    global_config = GlobalConfig::zeroed();
    assert!(!run_global_config_guard(
        global_config,
        flash_taking_orders_disabled,
    ));
    global_config.flash_take_order_blocked = 1;
    assert!(run_global_config_guard(
        global_config,
        flash_taking_orders_disabled,
    ));

    global_config = GlobalConfig::zeroed();
    assert!(!run_global_config_guard(
        global_config,
        create_new_orders_disabled,
    ));
    global_config.new_orders_blocked = 1;
    assert!(run_global_config_guard(
        global_config,
        create_new_orders_disabled,
    ));

    global_config = GlobalConfig::zeroed();
    assert!(!run_global_config_guard(
        global_config,
        taking_orders_disabled,
    ));
    global_config.orders_taking_blocked = 1;
    assert!(run_global_config_guard(
        global_config,
        taking_orders_disabled,
    ));
}

#[test]
fn order_expired_accepts_zero_expiry_without_clock_sysvar() {
    let mut order = Order::zeroed();
    order.expiry_timestamp = 0;

    assert!(!run_order_expired_guard(order));
}

#[test]
fn order_expired_compares_nonzero_expiry_with_clock() {
    install_noop_syscall_stubs();

    let mut active_order = Order::zeroed();
    active_order.expiry_timestamp = 101;
    assert!(!run_order_expired_guard(active_order));

    let mut expired_order = Order::zeroed();
    expired_order.expiry_timestamp = 99;
    assert!(run_order_expired_guard(expired_order));
}

#[test]
fn express_relay_permission_check_rejects_wrong_permission_account_before_cpi() {
    let owner = Pubkey::new_unique();
    let order_key = Pubkey::new_unique();
    let wrong_permission_key = Pubkey::new_unique();
    let sysvar_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let config_router_key = Pubkey::new_unique();
    let relay_metadata_key = Pubkey::new_unique();
    let relay_program_key = Pubkey::new_unique();
    let mut sysvar_lamports = 0;
    let mut permission_lamports = 0;
    let mut pda_lamports = 0;
    let mut config_lamports = 0;
    let mut metadata_lamports = 0;
    let mut program_lamports = 0;
    let mut sysvar_data = [];
    let mut permission_data = [];
    let mut pda_data = [];
    let mut config_data = [];
    let mut metadata_data = [];
    let mut program_data = [];

    let sysvar_instructions =
        account_info_with_data(&sysvar_key, &owner, &mut sysvar_lamports, &mut sysvar_data);
    let permission = account_info_with_data(
        &wrong_permission_key,
        &owner,
        &mut permission_lamports,
        &mut permission_data,
    );
    let pda_authority =
        account_info_with_data(&pda_authority_key, &owner, &mut pda_lamports, &mut pda_data);
    let config_router = account_info_with_data(
        &config_router_key,
        &owner,
        &mut config_lamports,
        &mut config_data,
    );
    let relay_metadata = account_info_with_data(
        &relay_metadata_key,
        &owner,
        &mut metadata_lamports,
        &mut metadata_data,
    );
    let relay_program = account_info_with_data(
        &relay_program_key,
        &owner,
        &mut program_lamports,
        &mut program_data,
    );

    assert!(check_permission_express_relay_and_get_fees(
        &sysvar_instructions,
        &permission,
        &pda_authority,
        &config_router,
        &relay_metadata,
        &relay_program,
        order_key,
    )
    .is_err());
}

#[test]
fn validate_token_extensions_accepts_legacy_mint_without_token_accounts() {
    let mint_key = Pubkey::new_unique();
    let token_program = token::ID;
    let mut lamports = 0;
    let mut data = [];
    let mint = account_info_with_data(&mint_key, &token_program, &mut lamports, &mut data);

    validate_token_extensions(&mint, vec![]).unwrap();
}

#[test]
fn validate_token_extensions_accepts_token_2022_mint_without_extensions() {
    let mint_key = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let token_program = spl_token_2022::ID;
    let mut mint_lamports = 0;
    let mut token_lamports = 0;
    let mut mint_data = token_2022_mint_data();
    let mut token_data = token_2022_account_data(mint_key, owner, 1);
    let token_account_key = Pubkey::new_unique();

    let mint = account_info_with_data(
        &mint_key,
        &token_program,
        &mut mint_lamports,
        &mut mint_data,
    );
    let token_account = account_info_with_data(
        &token_account_key,
        &token_program,
        &mut token_lamports,
        &mut token_data,
    );

    validate_token_extensions(&mint, vec![&token_account]).unwrap();
}

#[test]
fn validate_token_extensions_rejects_legacy_token_account_for_token_2022_mint() {
    let mint_key = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mut mint_lamports = 0;
    let mut token_lamports = 0;
    let mut mint_data = token_2022_mint_data();
    let mut token_data = token_account_data(mint_key, owner, 1);
    let token_account_key = Pubkey::new_unique();

    let mint = account_info_with_data(
        &mint_key,
        &spl_token_2022::ID,
        &mut mint_lamports,
        &mut mint_data,
    );
    let token_account = account_info_with_data(
        &token_account_key,
        &token::ID,
        &mut token_lamports,
        &mut token_data,
    );

    assert!(validate_token_extensions(&mint, vec![&token_account]).is_err());
}

#[test]
fn validate_token_extensions_rejects_unsupported_token_2022_mint_extension() {
    let mint_key = Pubkey::new_unique();
    let mut mint_data =
        token_2022_mint_data_with_extensions(&[ExtensionType::DefaultAccountState], |_| {});

    assert!(
        validate_token_2022_mint_data_without_token_accounts(&mint_key, &mut mint_data).is_err()
    );
}

#[test]
fn validate_token_extensions_checks_transfer_fee_and_hook_extensions() {
    let mint_key = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mut token_data = token_2022_account_data(mint_key, owner, 1);
    let mut valid_mint_data = token_2022_mint_data_with_extensions(
        &[
            ExtensionType::TransferFeeConfig,
            ExtensionType::TransferHook,
        ],
        |_| {},
    );
    validate_token_2022_mint_and_account_data(&mint_key, &mut valid_mint_data, &mut token_data)
        .unwrap();

    let mut token_data = token_2022_account_data(mint_key, owner, 1);
    let mut fee_mint_data =
        token_2022_mint_data_with_extensions(&[ExtensionType::TransferFeeConfig], |state| {
            let extension = state.get_extension_mut::<TransferFeeConfig>().unwrap();
            extension.older_transfer_fee.transfer_fee_basis_points = 1.into();
        });
    assert!(validate_token_2022_mint_and_account_data(
        &mint_key,
        &mut fee_mint_data,
        &mut token_data,
    )
    .is_err());

    let mut token_data = token_2022_account_data(mint_key, owner, 1);
    let mut hook_mint_data =
        token_2022_mint_data_with_extensions(&[ExtensionType::TransferHook], |state| {
            let extension = state.get_extension_mut::<TransferHook>().unwrap();
            extension.program_id = Some(Pubkey::new_unique()).try_into().unwrap();
        });
    assert!(validate_token_2022_mint_and_account_data(
        &mint_key,
        &mut hook_mint_data,
        &mut token_data,
    )
    .is_err());
}

#[test]
fn validate_token_extensions_checks_confidential_transfer_extensions() {
    let mint_key = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let mut token_data = token_2022_account_data(mint_key, owner, 1);
    let mut valid_mint_data =
        token_2022_mint_data_with_extensions(&[ExtensionType::ConfidentialTransferMint], |_| {});
    validate_token_2022_mint_and_account_data(&mint_key, &mut valid_mint_data, &mut token_data)
        .unwrap();

    let mut token_data = token_2022_account_data(mint_key, owner, 1);
    let mut auto_approve_mint_data =
        token_2022_mint_data_with_extensions(&[ExtensionType::ConfidentialTransferMint], |state| {
            let extension = state
                .get_extension_mut::<ConfidentialTransferMint>()
                .unwrap();
            extension.auto_approve_new_accounts = true.into();
        });
    assert!(validate_token_2022_mint_and_account_data(
        &mint_key,
        &mut auto_approve_mint_data,
        &mut token_data,
    )
    .is_err());

    let mut valid_mint_data =
        token_2022_mint_data_with_extensions(&[ExtensionType::ConfidentialTransferMint], |_| {});
    let mut credits_token_data =
        token_2022_account_data_with_confidential_extension(mint_key, owner, |extension| {
            extension.allow_confidential_credits = true.into();
        });
    assert!(validate_token_2022_mint_and_account_data(
        &mint_key,
        &mut valid_mint_data,
        &mut credits_token_data,
    )
    .is_err());

    let mut valid_mint_data =
        token_2022_mint_data_with_extensions(&[ExtensionType::ConfidentialTransferMint], |_| {});
    let mut pending_token_data =
        token_2022_account_data_with_confidential_extension(mint_key, owner, |extension| {
            extension.pending_balance_lo.0[0] = 1;
            extension.pending_balance_hi.0[0] = 1;
        });
    assert!(validate_token_2022_mint_and_account_data(
        &mint_key,
        &mut valid_mint_data,
        &mut pending_token_data,
    )
    .is_err());
}

#[test]
fn get_token_account_checked_accepts_initialized_matching_token_account() {
    let key = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let token_program = token::ID;
    let mut lamports = 0;
    let mut data = token_account_data(mint, owner, 42);
    let account = account_info_with_data(&key, &token_program, &mut lamports, &mut data);

    let token_account = get_token_account_checked(&account, &mint, &owner).unwrap();

    assert_eq!(token_account.mint, mint);
    assert_eq!(token_account.owner, owner);
    assert_eq!(token_account.amount, 42);
}

#[test]
fn get_token_account_checked_rejects_empty_or_wrong_owner_accounts() {
    let key = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let wrong_program = Pubkey::new_unique();
    let token_program = token::ID;
    let mut lamports = 0;
    let mut empty_data = [];
    let mut token_data = token_account_data(mint, owner, 1);

    let empty_account =
        account_info_with_data(&key, &token_program, &mut lamports, &mut empty_data);
    assert!(get_token_account_checked(&empty_account, &mint, &owner).is_err());

    let wrong_owner_account =
        account_info_with_data(&key, &wrong_program, &mut lamports, &mut token_data);
    assert!(get_token_account_checked(&wrong_owner_account, &mint, &owner).is_err());
}

#[test]
fn get_token_account_checked_rejects_bad_data_mint_or_authority() {
    let key = Pubkey::new_unique();
    let mint = Pubkey::new_unique();
    let owner = Pubkey::new_unique();
    let token_program = token::ID;
    let mut lamports = 0;
    let mut bad_data = vec![7; token::spl_token::state::Account::LEN];
    let mut token_data = token_account_data(mint, owner, 1);
    let bad_data_account =
        account_info_with_data(&key, &token_program, &mut lamports, &mut bad_data);
    assert!(get_token_account_checked(&bad_data_account, &mint, &owner).is_err());

    let token_account =
        account_info_with_data(&key, &token_program, &mut lamports, &mut token_data);
    assert!(get_token_account_checked(&token_account, &Pubkey::new_unique(), &owner).is_err());
    assert!(get_token_account_checked(&token_account, &mint, &Pubkey::new_unique()).is_err());
}
