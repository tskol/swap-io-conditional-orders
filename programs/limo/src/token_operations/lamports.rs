use anchor_lang::{prelude::AccountInfo, Result};
use solana_program::{
    instruction::Instruction,
    program::{invoke, invoke_signed},
    system_instruction,
};

#[allow(clippy::too_many_arguments)]
pub fn lamports_transfer_from_authority_to_account<'a>(
    user_account: AccountInfo<'a>,
    authority_account: AccountInfo<'a>,
    system_program: AccountInfo<'a>,
    authority_signer_seeds: &[&[u8]],
    amount_lamports: u64,
) -> Result<()> {
    let ix = transfer_lamports_ix(&authority_account, &user_account, amount_lamports);

    invoke_signed(
        &ix,
        &[
            authority_account.clone(),
            user_account.clone(),
            system_program.clone(),
        ],
        &[authority_signer_seeds],
    )?;

    Ok(())
}

pub fn native_transfer_from_user_to_account<'a>(
    from_account: AccountInfo<'a>,
    to_account: AccountInfo<'a>,
    amount: u64,
) -> Result<()> {
    let transfer_ix = transfer_lamports_ix(&from_account, &to_account, amount);

    invoke(&transfer_ix, &[from_account.clone(), to_account.clone()])?;

    Ok(())
}

pub fn native_transfer_from_authority_to_user<'a>(
    authority: AccountInfo<'a>,
    to_account: AccountInfo<'a>,
    authority_signer_seeds: &[&[u8]],
    amount: u64,
) -> Result<()> {
    let transfer_ix = transfer_lamports_ix(&authority, &to_account, amount);

    invoke_signed(
        &transfer_ix,
        &[authority.clone(), to_account.clone()],
        &[authority_signer_seeds],
    )?;

    Ok(())
}

fn transfer_lamports_ix(
    from_account: &AccountInfo,
    to_account: &AccountInfo,
    amount: u64,
) -> Instruction {
    system_instruction::transfer(from_account.key, to_account.key, amount)
}
