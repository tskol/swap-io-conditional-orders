mod common;

use anchor_lang::prelude::{
    AccountLoader, Context, Interface, InterfaceAccount, Program, Pubkey, Signer, System,
};
use anchor_spl::token_interface::{spl_token_2022, Mint, TokenAccount, TokenInterface};
use bytemuck::Zeroable;
use common::{mint_account_data, token_account_data, zero_copy_account_data, TestAccount};
use limo::{
    handlers::withdraw_fee::{withdraw_fee, WithdrawFee, WithdrawFeeBumps},
    state::GlobalConfig,
};

#[test]
fn withdraw_fee_handler_rejects_zero_amount_before_cpi() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let admin_key = Pubkey::new_unique();
    let fee_receiver_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let token_mint_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;

    let mut global_config = GlobalConfig::zeroed();
    global_config.admin_authority = admin_key;
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;

    let mut admin = TestAccount::new(admin_key, owner).signer().writable();
    let mut fee_receiver = TestAccount::new(fee_receiver_key, owner).writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();
    let mut pda_authority = TestAccount::new(pda_authority_key, owner);
    let mut token_mint = TestAccount::new(token_mint_key, token_program_key)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut fee_receiver_ata = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(token_mint_key, fee_receiver_key, 0))
        .writable();
    let mut fee_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(token_mint_key, pda_authority_key, 0))
        .writable();
    let mut token_program = TestAccount::new(token_program_key, owner).executable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();

    let admin_info = admin.info();
    let fee_receiver_info = fee_receiver.info();
    let global_config_info = global_config.info();
    let pda_authority_info = pda_authority.info();
    let token_mint_info = token_mint.info();
    let fee_receiver_ata_info = fee_receiver_ata.info();
    let fee_vault_info = fee_vault.info();
    let token_program_info = token_program.info();
    let system_program_info = system_program.info();

    let mut accounts = WithdrawFee {
        admin_authority: Signer::try_from(&admin_info).unwrap(),
        fee_receiver: fee_receiver_info,
        fee_receiver_ata: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&fee_receiver_ata_info).unwrap(),
        ),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_authority_info,
        token_mint: Box::new(InterfaceAccount::<Mint>::try_from(&token_mint_info).unwrap()),
        fee_vault: Box::new(InterfaceAccount::<TokenAccount>::try_from(&fee_vault_info).unwrap()),
        token_program: Interface::<TokenInterface>::try_from(&token_program_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        WithdrawFeeBumps { fee_vault: 254 },
    );

    assert!(withdraw_fee(ctx, 0).is_err());
}
