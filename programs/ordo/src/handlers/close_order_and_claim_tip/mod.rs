use anchor_lang::prelude::*;

use crate::{
    global_seeds,
    handlers::{close_order_and_claim_tip::validation::validate_closer, order_events},
    seeds::GLOBAL_AUTH,
    state::OrderType,
};

mod accounts;
mod child_flow;
mod close_flow;
mod fees;
mod validation;
mod vaults;

pub use accounts::*;
use child_flow::close_child_order_and_emit;
use close_flow::close_order_and_claim_tip;
use fees::pay_allowed_taker_close_fee;
use validation::{validate_close_order_token_extensions, validate_close_order_type};
use vaults::return_remaining_order_vaults;

pub fn handler_close_order_and_claim_tip(ctx: Context<ExitOrderAndClaimTip>) -> Result<()> {
    validate_close_order_token_extensions(&ctx)?;

    let global_config_key = ctx.accounts.global_config.key();
    let order = &mut ctx.accounts.order.load_mut()?;
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    let parsed_order_type = validate_close_order_type(order.order_type)?;
    let ts = u64::try_from(Clock::get()?.unix_timestamp).unwrap();
    // 0 means "no expiry" (backward compatibility: old orders had padding here)
    let is_order_expired = order.expiry_timestamp != 0 && order.expiry_timestamp < ts;

    validate_closer(
        ctx.accounts.closer.key(),
        ctx.accounts.maker.key(),
        global_config.allowed_taker,
        is_order_expired,
    )?;

    if parsed_order_type == OrderType::LimitParent {
        close_child_order_and_emit(
            &ctx,
            ctx.accounts.tp_child_order.as_ref(),
            order.tp_child_order,
            global_config,
            ts,
            global_config_key,
        )?;
        close_child_order_and_emit(
            &ctx,
            ctx.accounts.sl_child_order.as_ref(),
            order.sl_child_order,
            global_config,
            ts,
            global_config_key,
        )?;
    }

    close_order_and_claim_tip(
        order,
        global_config,
        ts,
        global_config_key,
        &ctx.accounts.pda_authority.to_account_info(),
        &ctx.accounts.maker.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;

    let gc = global_config_key;
    let seeds: &[&[u8]] = global_seeds!(global_config.pda_authority_bump as u8, &gc);
    pay_allowed_taker_close_fee(&ctx, global_config, order, seeds)?;
    return_remaining_order_vaults(&ctx, order, seeds)?;

    global_config.pda_authority_previous_lamports_balance = ctx.accounts.pda_authority.lamports();
    emit_cpi!(order_events::order_display(order, 0, 0));

    Ok(())
}
