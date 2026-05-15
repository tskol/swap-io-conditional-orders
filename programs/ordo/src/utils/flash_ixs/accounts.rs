use super::*;

pub fn check_same_accounts(start_ix: &Instruction, end_ix: &Instruction) -> Result<()> {
    if end_ix.accounts.len() != start_ix.accounts.len() {
        msg!("Number of accounts mismatch between start and end ix");
        return err!(OrdoError::FlashIxsAccountMismatch);
    }

    for (idx, (account_start, account_end)) in start_ix
        .accounts
        .iter()
        .zip(end_ix.accounts.iter())
        .enumerate()
    {
        let account_start_pk = &account_start.pubkey;
        let account_end_pk = &account_end.pubkey;
        if account_start_pk != account_end_pk {
            msg!("Some accounts in assert_user_swap_balances tx differ. index: {idx}, start:{account_start_pk}, end:{account_end_pk}",);
            return err!(OrdoError::FlashIxsAccountMismatch);
        }
    }
    Ok(())
}
