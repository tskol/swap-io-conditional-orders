use anchor_lang::prelude::Pubkey;
use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use limo::utils::constraints::{
    get_token_account_checked, is_counterparty_matching, is_wsol, verify_ata,
};
use solana_program::{account_info::AccountInfo, program_option::COption, program_pack::Pack};

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
