use anchor_lang::prelude::*;
use solana_program::{program::invoke, system_instruction};

use crate::token_operations::transfer_from_user_to_token_account;

use super::{accounts::CreateOrder, types::CreateOrderFees};

pub(super) fn fund_order_accounts(
    ctx: &Context<CreateOrder>,
    input_amount: u64,
    fees: CreateOrderFees,
) -> Result<()> {
    transfer_from_user_to_token_account(
        ctx.accounts.maker_ata.to_account_info(),
        ctx.accounts.input_vault.to_account_info(),
        ctx.accounts.maker.to_account_info(),
        ctx.accounts.input_mint.to_account_info(),
        ctx.accounts.input_token_program.to_account_info(),
        input_amount,
        ctx.accounts.input_mint.decimals,
    )?;

    if fees.create_order_fee > 0 {
        transfer_from_user_to_token_account(
            ctx.accounts.maker_ata.to_account_info(),
            ctx.accounts.input_fee_vault.to_account_info(),
            ctx.accounts.maker.to_account_info(),
            ctx.accounts.input_mint.to_account_info(),
            ctx.accounts.input_token_program.to_account_info(),
            fees.create_order_fee,
            ctx.accounts.input_mint.decimals,
        )?;
    }
    if fees.lamports > 0 {
        let maker = ctx.accounts.maker.key();
        let gc = ctx.accounts.global_config.key();
        let ixn = system_instruction::transfer(&maker, &gc, fees.lamports);

        invoke(
            &ixn,
            &[
                ctx.accounts.maker.to_account_info().clone(),
                ctx.accounts.global_config.to_account_info().clone(),
                ctx.accounts.system_program.to_account_info().clone(),
            ],
        )?;
    }

    Ok(())
}
