use anchor_lang::prelude::*;

use crate::handlers::order_events;

mod accounts;
mod funding;
mod order_init;
mod types;
mod validation;

pub use accounts::*;
use funding::fund_order_accounts;
use order_init::{initialize_child_orders, initialize_parent_order};
use types::CreateOrderArgs;
use validation::{
    validate_order_args_and_calculate_fees, validate_token_extensions_for_create_order,
};

pub fn handler_create_order(
    ctx: Context<CreateOrder>,
    input_amount: u64,
    output_amount: u64,
    order_type: u8,
    tp_output_amount: u64,
    sl_output_amount: u64,
    active_duration_seconds: u64,
) -> Result<()> {
    let args = CreateOrderArgs {
        input_amount,
        output_amount,
        order_type,
        tp_output_amount,
        sl_output_amount,
        active_duration_seconds,
    };

    validate_token_extensions_for_create_order(&ctx)?;
    let (parsed_order_type, fees) = validate_order_args_and_calculate_fees(&ctx, args)?;

    let order = &mut ctx.accounts.order.load_init()?;
    let clock = Clock::get()?;
    initialize_parent_order(&ctx, order, args, clock.unix_timestamp)?;
    initialize_child_orders(&ctx, order, parsed_order_type, args, clock.unix_timestamp)?;
    fund_order_accounts(&ctx, args.input_amount, fees)?;

    msg!(
        "Created order {}, input_amount {}, input_mint {}, output_amount {}, output_mint {}",
        ctx.accounts.order.key(),
        input_amount,
        ctx.accounts.input_mint.key(),
        output_amount,
        ctx.accounts.output_mint.key(),
    );

    emit_cpi!(order_events::order_display(order, 0, 0));

    Ok(())
}
