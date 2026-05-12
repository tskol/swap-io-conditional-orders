use anchor_lang::{err, prelude::*, Key, Result};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token::{self, spl_token},
    token_2022::spl_token_2022,
    token_interface::TokenAccount,
};

use crate::LimoError;

pub fn verify_ata(
    wallet: &Pubkey,
    mint: &Pubkey,
    ata_account_key: &Pubkey,
    token_program_id: &Pubkey,
) -> Result<()> {
    let expected_ata = get_associated_token_address_with_program_id(wallet, mint, token_program_id);

    require_keys_eq!(
        ata_account_key.key(),
        expected_ata,
        LimoError::InvalidAtaAddress
    );

    Ok(())
}

pub fn is_wsol(mint: &Pubkey) -> bool {
    *mint == token::spl_token::native_mint::ID
}

pub fn is_counterparty_matching(
    counterparty: &Pubkey,
    allowed_taker: &Pubkey,
    taker: &Pubkey,
) -> bool {
    counterparty.eq(&Pubkey::default()) && allowed_taker == taker || taker == counterparty
}

pub fn get_token_account_checked(
    account: &AccountInfo,
    expected_mint: &Pubkey,
    expected_owner: &Pubkey,
) -> Result<TokenAccount> {
    if account.data_len() == 0 {
        return err!(LimoError::UninitializedTokenAccount);
    }

    if *account.owner != spl_token::id() && *account.owner != spl_token_2022::id() {
        return err!(LimoError::InvalidTokenAccountOwner);
    }

    let token_account = match TokenAccount::try_deserialize(&mut &account.data.borrow()[..]) {
        Ok(ta) => ta,
        Err(_) => return err!(LimoError::InvalidAccount),
    };

    if token_account.mint != *expected_mint {
        return err!(LimoError::InvalidTokenMint);
    }

    if token_account.owner != *expected_owner {
        return err!(LimoError::InvalidTokenAuthority);
    }

    Ok(token_account)
}
