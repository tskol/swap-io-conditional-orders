use anchor_lang::prelude::Pubkey;
use anchor_spl::{associated_token::get_associated_token_address_with_program_id, token};
use limo::utils::constraints::{is_counterparty_matching, is_wsol, verify_ata};

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
