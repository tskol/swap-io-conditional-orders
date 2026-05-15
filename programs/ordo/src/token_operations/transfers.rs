use anchor_lang::{
    prelude::{AccountInfo, CpiContext},
    Result,
};
use anchor_spl::token_interface;

#[allow(clippy::too_many_arguments)]
pub fn transfer_from_user_to_token_account<'a>(
    user_token_account: AccountInfo<'a>,
    destination_token_account: AccountInfo<'a>,
    user_authority: AccountInfo<'a>,
    token_mint: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    deposit_amount: u64,
    token_decimals: u8,
) -> Result<()> {
    token_interface::transfer_checked(
        CpiContext::new(
            token_program.clone(),
            transfer_checked_accounts(
                user_token_account,
                destination_token_account,
                user_authority,
                token_mint,
            ),
        ),
        deposit_amount,
        token_decimals,
    )?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_from_vault_to_token_account<'a>(
    user_token_account: AccountInfo<'a>,
    vault_token_account: AccountInfo<'a>,
    pda_authority: AccountInfo<'a>,
    token_mint: AccountInfo<'a>,
    token_program: AccountInfo<'a>,
    authority_signer_seeds: &[&[u8]],
    deposit_amount: u64,
    token_decimals: u8,
) -> Result<()> {
    token_interface::transfer_checked(
        CpiContext::new_with_signer(
            token_program.clone(),
            transfer_checked_accounts(
                vault_token_account,
                user_token_account,
                pda_authority,
                token_mint,
            ),
            &[authority_signer_seeds],
        ),
        deposit_amount,
        token_decimals,
    )?;

    Ok(())
}

fn transfer_checked_accounts<'a>(
    from: AccountInfo<'a>,
    to: AccountInfo<'a>,
    authority: AccountInfo<'a>,
    mint: AccountInfo<'a>,
) -> token_interface::TransferChecked<'a> {
    token_interface::TransferChecked {
        from,
        to,
        authority,
        mint,
    }
}
