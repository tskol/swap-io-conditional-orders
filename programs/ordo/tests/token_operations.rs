mod common;

use anchor_spl::token_interface::spl_token_2022;
use common::{install_noop_syscall_stubs, mint_account_data, token_account_data, TestAccount};
use ordo::token_operations::{
    close_ata_accounts_with_signer_seeds, initialize_intermediary_token_account_with_signer_seeds,
    lamports_transfer_from_authority_to_account, native_transfer_from_authority_to_user,
    native_transfer_from_user_to_account, transfer_from_user_to_token_account,
    transfer_from_vault_to_token_account,
};
use solana_program::pubkey::Pubkey;

#[test]
fn token_operation_transfer_wrappers_invoke_successfully_with_stubs() {
    install_noop_syscall_stubs();

    let owner = ordo::ID;
    let token_program_key = spl_token_2022::ID;
    let user_key = Pubkey::new_unique();
    let pda_key = Pubkey::new_unique();
    let mint_key = Pubkey::new_unique();
    let source_key = Pubkey::new_unique();
    let destination_key = Pubkey::new_unique();
    let system_program_key = anchor_lang::system_program::ID;
    let signer_bump = [254];
    let signer_seeds: &[&[u8]] = &[b"authority", signer_bump.as_ref()];

    let mut user = TestAccount::new(user_key, owner).signer().writable();
    let mut pda = TestAccount::new(pda_key, owner).writable();
    let mut mint = TestAccount::new(mint_key, token_program_key)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut source = TestAccount::new(source_key, token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(mint_key, user_key, 10))
        .writable();
    let mut destination = TestAccount::new(destination_key, token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(mint_key, pda_key, 0))
        .writable();
    let mut token_program = TestAccount::new(token_program_key, owner).executable();
    let mut system_program = TestAccount::new(system_program_key, owner).executable();

    let user_info = user.info();
    let pda_info = pda.info();
    let mint_info = mint.info();
    let source_info = source.info();
    let destination_info = destination.info();
    let token_program_info = token_program.info();
    let system_program_info = system_program.info();

    transfer_from_user_to_token_account(
        source_info.clone(),
        destination_info.clone(),
        user_info.clone(),
        mint_info.clone(),
        token_program_info.clone(),
        1,
        6,
    )
    .unwrap();

    transfer_from_vault_to_token_account(
        source_info.clone(),
        destination_info.clone(),
        pda_info.clone(),
        mint_info.clone(),
        token_program_info.clone(),
        signer_seeds,
        1,
        6,
    )
    .unwrap();

    lamports_transfer_from_authority_to_account(
        user_info.clone(),
        pda_info.clone(),
        system_program_info.clone(),
        signer_seeds,
        1,
    )
    .unwrap();

    native_transfer_from_user_to_account(user_info.clone(), pda_info.clone(), 1).unwrap();

    native_transfer_from_authority_to_user(pda_info.clone(), user_info.clone(), signer_seeds, 1)
        .unwrap();

    close_ata_accounts_with_signer_seeds(
        source_info,
        user_info,
        pda_info,
        token_program_info,
        signer_seeds,
    )
    .unwrap();
}

#[test]
fn initialize_intermediary_token_account_covers_create_and_reallocate_paths() {
    install_noop_syscall_stubs();

    let owner = ordo::ID;
    let authority_key = Pubkey::new_unique();
    let mint_key = Pubkey::new_unique();
    let account_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;
    let rent_key = solana_program::sysvar::rent::ID;
    let signer_bump = [254];
    let account_signer_seeds: &[&[u8]] = &[b"intermediary", signer_bump.as_ref()];
    let authority_signer_seeds: &[&[u8]] = &[b"authority", signer_bump.as_ref()];

    let mut authority = TestAccount::new(authority_key, owner).writable();
    let mut mint = TestAccount::new(mint_key, token_program_key)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut token_program = TestAccount::new(token_program_key, owner).executable();
    let mut rent = TestAccount::new(rent_key, owner);

    let authority_info = authority.info();
    let mint_info = mint.info();
    let token_program_info = token_program.info();
    let rent_info = rent.info();

    let mut empty_intermediary = TestAccount::new(account_key, owner).writable();
    let empty_intermediary_info = empty_intermediary.info();
    initialize_intermediary_token_account_with_signer_seeds(
        empty_intermediary_info,
        mint_info.clone(),
        token_program_info.clone(),
        authority_info.clone(),
        rent_info.clone(),
        account_signer_seeds,
        authority_signer_seeds,
    )
    .unwrap();

    let mut existing_intermediary = TestAccount::new(Pubkey::new_unique(), owner)
        .with_lamports(1)
        .writable();
    let existing_intermediary_info = existing_intermediary.info();
    initialize_intermediary_token_account_with_signer_seeds(
        existing_intermediary_info,
        mint_info,
        token_program_info,
        authority_info,
        rent_info,
        account_signer_seeds,
        authority_signer_seeds,
    )
    .unwrap();
}
