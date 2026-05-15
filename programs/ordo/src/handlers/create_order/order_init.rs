use anchor_lang::prelude::*;

use crate::{operations, state::Order, OrderType, OrdoError};

use super::{accounts::CreateOrder, types::CreateOrderArgs};

pub(super) fn initialize_parent_order(
    ctx: &Context<CreateOrder>,
    order: &mut Order,
    args: CreateOrderArgs,
    current_timestamp: i64,
) -> Result<()> {
    operations::create_order(
        order,
        ctx.accounts.global_config.key(),
        ctx.accounts.maker.key(),
        args.input_amount,
        args.output_amount,
        Pubkey::default(),
        ctx.accounts.input_mint.key(),
        ctx.accounts.output_mint.key(),
        ctx.accounts.input_token_program.key(),
        ctx.accounts.output_token_program.key(),
        args.order_type,
        ctx.bumps.input_vault,
        current_timestamp,
        args.active_duration_seconds,
    )
}

pub(super) fn initialize_child_orders(
    ctx: &Context<CreateOrder>,
    order: &mut Order,
    parsed_order_type: OrderType,
    args: CreateOrderArgs,
    current_timestamp: i64,
) -> Result<()> {
    if parsed_order_type != OrderType::LimitParent {
        return Ok(());
    }

    require!(
        args.tp_output_amount > 0 || args.sl_output_amount > 0,
        OrdoError::OrderParametersInvalid
    );
    ctx.accounts
        .output_vault
        .as_ref()
        .ok_or(OrdoError::InvalidAccount)?;

    if args.tp_output_amount > 0 {
        let tp_order_account = ctx
            .accounts
            .tp_order
            .as_ref()
            .ok_or(OrdoError::InvalidAccount)?;
        order.tp_child_order = initialize_child_order(
            ctx,
            ChildOrderSpec {
                account: tp_order_account,
                expected_output_amount: args.tp_output_amount,
                order_type: OrderType::LimitTP,
            },
            args,
            current_timestamp,
        )?;
    }
    if args.sl_output_amount > 0 {
        let sl_order_account = ctx
            .accounts
            .sl_order
            .as_ref()
            .ok_or(OrdoError::InvalidAccount)?;
        order.sl_child_order = initialize_child_order(
            ctx,
            ChildOrderSpec {
                account: sl_order_account,
                expected_output_amount: args.sl_output_amount,
                order_type: OrderType::LimitSL,
            },
            args,
            current_timestamp,
        )?;
    }

    Ok(())
}

struct ChildOrderSpec<'a, 'info> {
    account: &'a AccountLoader<'info, Order>,
    expected_output_amount: u64,
    order_type: OrderType,
}

fn initialize_child_order(
    ctx: &Context<CreateOrder>,
    spec: ChildOrderSpec,
    args: CreateOrderArgs,
    current_timestamp: i64,
) -> Result<Pubkey> {
    let child_order = &mut spec.account.load_init()?;
    operations::create_order(
        child_order,
        ctx.accounts.global_config.key(),
        ctx.accounts.maker.key(),
        args.output_amount,
        spec.expected_output_amount,
        ctx.accounts.order.key(),
        ctx.accounts.output_mint.key(),
        ctx.accounts.input_mint.key(),
        ctx.accounts.output_token_program.key(),
        ctx.accounts.input_token_program.key(),
        spec.order_type as u8,
        ctx.bumps.output_vault,
        current_timestamp,
        args.active_duration_seconds,
    )?;

    Ok(spec.account.key())
}
