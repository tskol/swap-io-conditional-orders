use anchor_lang::prelude::*;

use crate::{
    operations, state::GlobalConfig, utils::constraints::token_2022::validate_token_extensions,
    utils::consts::FULL_BPS, LimoError, OrderType,
};

use super::{
    accounts::CreateOrder,
    types::{CreateOrderArgs, CreateOrderFees},
};

pub(super) fn validate_token_extensions_for_create_order(ctx: &Context<CreateOrder>) -> Result<()> {
    validate_token_extensions(
        &ctx.accounts.input_mint.to_account_info(),
        vec![&ctx.accounts.maker_ata.to_account_info()],
    )?;
    validate_token_extensions(&ctx.accounts.output_mint.to_account_info(), vec![])
}

pub(super) fn validate_order_args_and_calculate_fees(
    ctx: &Context<CreateOrder>,
    args: CreateOrderArgs,
) -> Result<(OrderType, CreateOrderFees)> {
    require!(args.input_amount > 0, LimoError::OrderInputAmountInvalid);
    require!(args.output_amount > 0, LimoError::OrderOutputAmountInvalid);
    require!(
        ctx.accounts.input_mint.key() != ctx.accounts.output_mint.key(),
        LimoError::OrderSameMint
    );

    let parsed_order_type =
        OrderType::try_from(args.order_type).map_err(|_| LimoError::OrderTypeInvalid)?;
    require!(
        parsed_order_type != OrderType::LimitTP && parsed_order_type != OrderType::LimitSL,
        LimoError::OrderTypeInvalid
    );

    let gc_state = ctx.accounts.global_config.load()?;
    validate_tp_sl_args(args, parsed_order_type, &gc_state)?;

    let fees = CreateOrderFees {
        create_order_fee: operations::calculate_fee_amount(
            args.input_amount,
            gc_state.create_order_fee_bps,
        )?,
        lamports: gc_state.ata_creation_cost + gc_state.txn_fee_cost,
    };

    Ok((parsed_order_type, fees))
}

fn validate_tp_sl_args(
    args: CreateOrderArgs,
    parsed_order_type: OrderType,
    global_config: &GlobalConfig,
) -> Result<()> {
    if parsed_order_type == OrderType::Vanilla {
        require!(
            args.tp_output_amount == 0 && args.sl_output_amount == 0,
            LimoError::OrderParametersInvalid
        );
        return Ok(());
    }

    require!(global_config.tp_sl_enabled == 1, LimoError::TPSLNotEnabled);
    require!(
        args.tp_output_amount > 0 || args.sl_output_amount > 0,
        LimoError::OrderParametersInvalid
    );

    let tp_sl_min_distance = args
        .input_amount
        .checked_mul(u64::from(global_config.tp_sl_min_distance_bps))
        .unwrap()
        .checked_div(FULL_BPS)
        .unwrap_or(0);

    if args.tp_output_amount > 0 {
        require!(
            args.tp_output_amount >= args.input_amount.checked_add(tp_sl_min_distance).unwrap(),
            LimoError::TPSLMinDistanceNotMet
        );
    }
    if args.sl_output_amount > 0 {
        require!(
            args.sl_output_amount <= args.input_amount.checked_sub(tp_sl_min_distance).unwrap(),
            LimoError::TPSLMinDistanceNotMet
        );
    }

    Ok(())
}
