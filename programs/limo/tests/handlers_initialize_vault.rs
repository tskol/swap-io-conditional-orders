use anchor_lang::{
    prelude::{AccountInfo, AccountLoader, Context, Interface, InterfaceAccount, Program, Pubkey, Signer, System},
    Discriminator,
};
use anchor_spl::token_interface::{spl_token_2022, Mint, TokenAccount, TokenInterface};
use bytemuck::{bytes_of, Pod, Zeroable};
use limo::{
    handlers::initialize_vault::{handler_initialize_vault, InitializeVault, InitializeVaultBumps},
    state::GlobalConfig,
};
use solana_program::{program_option::COption, program_pack::Pack};

fn zero_copy_account_data<T: Discriminator + Pod>(account: &T) -> Vec<u8> {
    let mut data = vec![0; 8 + std::mem::size_of::<T>()];
    data[..8].copy_from_slice(&T::discriminator());
    data[8..].copy_from_slice(bytes_of(account));
    data
}

fn mint_account_data() -> Vec<u8> {
    let mut data = vec![0; spl_token_2022::state::Mint::LEN];
    let mint = spl_token_2022::state::Mint {
        mint_authority: COption::None,
        supply: 0,
        decimals: 6,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    spl_token_2022::state::Mint::pack(mint, &mut data).unwrap();
    data
}

fn token_account_data(mint: Pubkey, owner: Pubkey) -> Vec<u8> {
    let mut data = vec![0; spl_token_2022::state::Account::LEN];
    let token_account = spl_token_2022::state::Account {
        mint,
        owner,
        amount: 0,
        delegate: COption::None,
        state: spl_token_2022::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    spl_token_2022::state::Account::pack(token_account, &mut data).unwrap();
    data
}

#[test]
fn initialize_vault_handler_accepts_initialized_context() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let payer_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let mint_key = Pubkey::new_unique();
    let vault_key = Pubkey::new_unique();
    let fee_vault_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;
    let system_program_key = anchor_lang::system_program::ID;

    let mut payer_lamports = 0;
    let mut payer_data = [];
    let payer_info = AccountInfo::new(
        &payer_key,
        true,
        true,
        &mut payer_lamports,
        &mut payer_data,
        &owner,
        false,
        0,
    );

    let mut global_config = GlobalConfig::zeroed();
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;
    let mut global_config_lamports = 0;
    let mut global_config_data = zero_copy_account_data(&global_config);
    let global_config_info = AccountInfo::new(
        &global_config_key,
        false,
        true,
        &mut global_config_lamports,
        &mut global_config_data,
        &owner,
        false,
        0,
    );

    let mut pda_authority_lamports = 0;
    let mut pda_authority_data = [];
    let pda_authority_info = AccountInfo::new(
        &pda_authority_key,
        false,
        false,
        &mut pda_authority_lamports,
        &mut pda_authority_data,
        &owner,
        false,
        0,
    );

    let mut mint_lamports = 1;
    let mut mint_data = mint_account_data();
    let mint_info = AccountInfo::new(
        &mint_key,
        false,
        false,
        &mut mint_lamports,
        &mut mint_data,
        &token_program_key,
        false,
        0,
    );

    let mut vault_lamports = 1;
    let mut vault_data = token_account_data(mint_key, pda_authority_key);
    let vault_info = AccountInfo::new(
        &vault_key,
        false,
        true,
        &mut vault_lamports,
        &mut vault_data,
        &token_program_key,
        false,
        0,
    );

    let mut fee_vault_lamports = 1;
    let mut fee_vault_data = token_account_data(mint_key, pda_authority_key);
    let fee_vault_info = AccountInfo::new(
        &fee_vault_key,
        false,
        true,
        &mut fee_vault_lamports,
        &mut fee_vault_data,
        &token_program_key,
        false,
        0,
    );

    let mut token_program_lamports = 0;
    let mut token_program_data = [];
    let token_program_info = AccountInfo::new(
        &token_program_key,
        false,
        false,
        &mut token_program_lamports,
        &mut token_program_data,
        &owner,
        true,
        0,
    );

    let mut system_program_lamports = 0;
    let mut system_program_data = [];
    let system_program_info = AccountInfo::new(
        &system_program_key,
        false,
        false,
        &mut system_program_lamports,
        &mut system_program_data,
        &owner,
        true,
        0,
    );

    let mut accounts = InitializeVault {
        payer: Signer::try_from(&payer_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_authority_info,
        mint: Box::new(InterfaceAccount::<Mint>::try_from(&mint_info).unwrap()),
        vault: Box::new(InterfaceAccount::<TokenAccount>::try_from(&vault_info).unwrap()),
        fee_vault: Box::new(InterfaceAccount::<TokenAccount>::try_from(&fee_vault_info).unwrap()),
        token_program: Interface::<TokenInterface>::try_from(&token_program_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        InitializeVaultBumps {
            vault: 253,
            fee_vault: 252,
        },
    );

    handler_initialize_vault(ctx).unwrap();
}
