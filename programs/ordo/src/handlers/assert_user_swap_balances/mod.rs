use anchor_lang::{prelude::*, Discriminator};

use crate::{
    instruction::{AssertUserSwapBalancesEnd, AssertUserSwapBalancesStart},
    operations::{record_user_swap_balances, validate_user_swap_balances},
    utils::assert_user_swap_balance_introspection,
};

mod accounts;
mod balances;
mod cpi_guard;

pub use accounts::*;
use balances::get_user_balances_checked;
use cpi_guard::check_cpi_not_allowed;

pub fn handler_assert_user_swap_balances_start(
    ctx: Context<AssertUserSwapBalancesStartContext>,
) -> Result<()> {
    check_cpi_not_allowed(&ctx.accounts.sysvar_instructions)?;
    assert_user_swap_balance_introspection::ensure_end_ix_match::<AssertUserSwapBalancesEnd>(
        &ctx.accounts.sysvar_instructions,
        &AssertUserSwapBalancesStart::discriminator(),
    )?;

    let balances = get_user_balances_checked(
        &ctx.accounts.maker,
        &ctx.accounts.input_ta,
        &ctx.accounts.output_ta,
    );

    let user_swap_balance_state = &mut ctx.accounts.user_swap_balance_state.load_init()?;
    record_user_swap_balances(user_swap_balance_state, balances);

    Ok(())
}

pub fn handler_assert_user_swap_balances_end(
    ctx: Context<AssertUserSwapBalancesEndContext>,
    max_input_amount_change: u64,
    min_output_amount_change: u64,
) -> Result<()> {
    check_cpi_not_allowed(&ctx.accounts.sysvar_instructions)?;
    assert_user_swap_balance_introspection::ensure_start_ix_match::<AssertUserSwapBalancesStart>(
        &ctx.accounts.sysvar_instructions,
        &AssertUserSwapBalancesEnd::discriminator(),
    )?;

    let balances = get_user_balances_checked(
        &ctx.accounts.maker,
        &ctx.accounts.input_ta,
        &ctx.accounts.output_ta,
    );

    {
        let user_swap_balance_state = &mut ctx.accounts.user_swap_balance_state.load()?;
        validate_user_swap_balances(
            user_swap_balance_state,
            balances,
            max_input_amount_change,
            min_output_amount_change,
        )?;
    }

    Ok(())
}
