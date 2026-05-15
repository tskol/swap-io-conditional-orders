use anchor_lang::prelude::*;

use crate::{
    handlers::{order_events, take_order::permissions::check_permission_and_get_tip},
    operations,
    state::ExecuteOrderEffects,
};

mod accounts;
mod permissions;
mod transfers;
mod validation;

pub use accounts::*;
use transfers::{tip_transfer_and_validation, transfer_output_and_input};
use validation::{validate_child_order_accounts, validate_take_order_token_extensions};

pub fn handler_take_order(
    ctx: Context<ExecuteOrder>,
    input_amount: u64,
    min_output_amount: u64,
    tip_amount_permissionless_taking: u64,
) -> Result<()> {
    validate_take_order_token_extensions(&ctx)?;
    let (_is_order_permissionless, counterparty) = validate_child_order_accounts(&ctx)?;

    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    let tip = check_permission_and_get_tip(
        &ctx,
        &counterparty,
        &global_config.allowed_taker,
        tip_amount_permissionless_taking,
    )?;

    let order = &mut ctx.accounts.order.load_mut()?;
    let clock = Clock::get()?;

    let ExecuteOrderEffects {
        input_to_send_to_taker,
        output_to_send_to_maker,
        output_to_send_to_protocol,
        output_keeper_fee,
    } = operations::take_order(
        global_config,
        order,
        ctx.accounts.parent_order.as_ref(),
        ctx.accounts.brother_order.as_ref(),
        ctx.accounts.input_oracle_pool.as_ref(),
        ctx.accounts.output_oracle_pool.as_ref(),
        ctx.accounts.input_price_update.as_deref(),
        ctx.accounts.output_price_update.as_deref(),
        ctx.accounts.input_mint.decimals,
        ctx.accounts.output_mint.decimals,
        input_amount,
        tip,
        clock.unix_timestamp,
        min_output_amount,
    )?;

    transfer_output_and_input(
        &ctx,
        global_config,
        order.order_type,
        input_to_send_to_taker,
        output_to_send_to_maker,
        output_to_send_to_protocol,
        output_keeper_fee,
    )?;

    tip_transfer_and_validation(&ctx, global_config, tip)?;

    emit_cpi!(order_events::order_display(
        order,
        output_to_send_to_maker,
        tip
    ));

    Ok(())
}
