use anchor_lang::{
    prelude::{AccountInfo, AccountLoader, Pubkey, Result},
    Discriminator,
};
use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use bytemuck::{bytes_of, Pod, Zeroable};
use limo::{
    state::{GlobalConfig, Order},
    utils::constraints::{
        create_new_orders_disabled, emergency_mode_disabled, flash_taking_orders_disabled,
        get_token_account_checked, is_counterparty_matching, is_wsol, order_expired,
        taking_orders_disabled, verify_ata,
    },
};
use solana_program::{program_option::COption, program_pack::Pack};

fn zero_copy_account_data<T: Discriminator + Pod>(account: &T) -> Vec<u8> {
    let mut data = vec![0; 8 + std::mem::size_of::<T>()];
    data[..8].copy_from_slice(&T::discriminator());
    data[8..].copy_from_slice(bytes_of(account));
    data
}

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
