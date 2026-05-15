use anchor_lang::prelude::*;

use crate::{
    instruction::{LogUserSwapBalancesEnd, LogUserSwapBalancesStart},
    operations::record_user_swap_balances,
    utils::log_user_swap_balance_introspection,
    UserSwapBalanceDiffs,
};

mod accounts;
mod balances;

pub use accounts::*;
pub use balances::get_balances_checked;

pub fn handler_log_user_swap_balances_start(
    ctx: Context<LogUserSwapBalancesStartContext>,
) -> Result<()> {
    let swap_program_id = ctx.accounts.base_accounts.swap_program_id.key();
    log_user_swap_balance_introspection::ensure_end_ix_match::<LogUserSwapBalancesEnd>(
        &ctx.accounts.sysvar_instructions,
        &swap_program_id,
    )?;

    let balances = get_balances_checked(&ctx.accounts.base_accounts)?;

    let user_swap_balance_state = &mut ctx.accounts.user_swap_balance_state.load_init()?;
    record_user_swap_balances(user_swap_balance_state, balances);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn handler_log_user_swap_balances_end(
    ctx: Context<LogUserSwapBalancesEndContext>,
    simulated_swap_amount_out: u64,
    simulated_ts: u64,
    minimum_amount_out: u64,
    swap_amount_in: u64,
    simulated_amount_out_next_best: u64,
    aggregator: u8,
    next_best_aggregator: u8,
) -> Result<()> {
    let swap_program_id = ctx.accounts.base_accounts.swap_program_id.key();
    log_user_swap_balance_introspection::ensure_start_ix_match::<LogUserSwapBalancesStart>(
        &ctx.accounts.sysvar_instructions,
        &swap_program_id,
    )?;

    let balances = get_balances_checked(&ctx.accounts.base_accounts)?;

    {
        let user_swap_balance_state = &mut ctx.accounts.user_swap_balance_state.load()?;

        emit_cpi!(UserSwapBalanceDiffs {
            user_lamports_before: user_swap_balance_state.user_lamports,
            input_ta_balance_before: user_swap_balance_state.input_ta_balance,
            output_ta_balance_before: user_swap_balance_state.output_ta_balance,
            user_lamports_after: balances.lamports_balance,
            input_ta_balance_after: balances.input_balance,
            output_ta_balance_after: balances.output_balance,
            swap_program: swap_program_id,
            simulated_swap_amount_out,
            simulated_ts,
            minimum_amount_out,
            swap_amount_in,
            simulated_amount_out_next_best,
            aggregator,
            next_best_aggregator,
        });
    }

    ctx.accounts
        .user_swap_balance_state
        .close(ctx.accounts.base_accounts.maker.to_account_info().clone())?;

    Ok(())
}
