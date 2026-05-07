mod common;

use anchor_lang::prelude::{
    AccountLoader, Context, Interface, InterfaceAccount, Program, Pubkey, Signer, System,
};
use anchor_spl::token_interface::{spl_token_2022, Mint, TokenAccount, TokenInterface};
use bytemuck::Zeroable;
use common::{mint_account_data, token_account_data, zero_copy_account_data, TestAccount};
use limo::{
    handlers::initialize_vault::{handler_initialize_vault, InitializeVault, InitializeVaultBumps},
    state::GlobalConfig,
};

#[test]
fn initialize_vault_handler_accepts_initialized_context() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let mint_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;

    let mut global_config = GlobalConfig::zeroed();
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;

    let mut payer = TestAccount::new(Pubkey::new_unique(), owner).signer().writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();
    let mut pda_authority = TestAccount::new(pda_authority_key, owner);
    let mut mint = TestAccount::new(mint_key, token_program_key)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(mint_key, pda_authority_key, 0))
        .writable();
    let mut fee_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(mint_key, pda_authority_key, 0))
        .writable();
    let mut token_program = TestAccount::new(token_program_key, owner).executable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();

    let payer_info = payer.info();
    let global_config_info = global_config.info();
    let pda_authority_info = pda_authority.info();
    let mint_info = mint.info();
    let vault_info = vault.info();
    let fee_vault_info = fee_vault.info();
    let token_program_info = token_program.info();
    let system_program_info = system_program.info();

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
