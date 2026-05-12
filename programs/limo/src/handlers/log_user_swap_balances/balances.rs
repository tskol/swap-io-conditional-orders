use anchor_lang::prelude::*;

use crate::{utils::constraints::get_token_account_checked, GetBalancesCheckedResult};

use super::accounts::LogUserSwapBalances;

pub fn get_balances_checked(ctx: &LogUserSwapBalances) -> Result<GetBalancesCheckedResult> {
    let lamports_balance = ctx.maker.lamports();

    let input_balance = if ctx.input_ta.data_len() > 0 {
        let input_token_account = get_token_account_checked(
            &ctx.input_ta.to_account_info(),
            &ctx.input_mint.key(),
            &ctx.maker.key(),
        )?;

        input_token_account.amount
    } else {
        0
    };

    let output_balance = if ctx.output_ta.data_len() > 0 {
        let output_token_account = get_token_account_checked(
            &ctx.output_ta.to_account_info(),
            &ctx.output_mint.key(),
            &ctx.maker.key(),
        )?;

        output_token_account.amount
    } else {
        0
    };

    Ok(GetBalancesCheckedResult {
        lamports_balance,
        input_balance,
        output_balance,
    })
}
