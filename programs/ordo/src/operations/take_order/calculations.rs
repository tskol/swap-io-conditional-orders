use anchor_lang::prelude::*;

use crate::{
    dbg_msg,
    state::{ExecuteOrderEffects, GlobalConfig, Order, OrderStatus, OrderType},
    utils::fraction::{Fraction, FractionExtra},
    OrdoError,
};

pub fn take_order_calcs(
    order: &Order,
    global_config: &GlobalConfig,
    input_amount: u64,
    output_amount: u64,
) -> Result<ExecuteOrderEffects> {
    require!(input_amount > 0, OrdoError::OrderInputAmountInvalid);

    require!(
        order.status == OrderStatus::Active as u8,
        OrdoError::OrderNotActive
    );

    require!(
        input_amount <= order.remaining_input_amount,
        OrdoError::OrderInputAmountTooLarge
    );

    require!(
        order.initial_input_amount > 0,
        OrdoError::OrderInputAmountInvalid
    );

    let input_to_send_to_taker = input_amount;
    let numerator = u128::from(input_to_send_to_taker) * u128::from(order.expected_output_amount);
    let denominator = u128::from(order.initial_input_amount);
    let minimum_output_to_send_to_maker_u128 = (numerator + denominator - 1) / denominator;

    let minimum_output_to_send_to_maker = u64::try_from(minimum_output_to_send_to_maker_u128)
        .map_err(|_| dbg_msg!(OrdoError::MathOverflow))?;

    let fee_pot = output_amount.saturating_sub(minimum_output_to_send_to_maker);

    let mut output_to_send_to_protocol = 0;
    let mut output_keeper_fee = 0;
    let mut output_fee_pot_keeper_fee = 0;
    if order.order_type == OrderType::Vanilla as u8 {
        output_keeper_fee = (Fraction::from_bps(global_config.keeper_take_fee_bps)
            * Fraction::from(output_amount))
        .to_ceil::<u64>();
    } else if order.order_type == OrderType::LimitParent as u8 && fee_pot > 0 {
        output_to_send_to_protocol =
            (Fraction::from_bps(global_config.parent_fill_fee_protocol_bps)
                * Fraction::from(fee_pot))
            .to_ceil::<u64>();
        output_fee_pot_keeper_fee = (Fraction::from_bps(global_config.parent_fill_fee_keeper_bps)
            * Fraction::from(fee_pot))
        .to_ceil::<u64>();
    } else if order.order_type == OrderType::LimitTP as u8
        || order.order_type == OrderType::LimitSL as u8
    {
        output_keeper_fee = (Fraction::from_bps(global_config.keeper_take_fee_bps)
            * Fraction::from(output_amount))
        .to_ceil::<u64>();
        if fee_pot > 0 {
            output_to_send_to_protocol =
                (Fraction::from_bps(global_config.tp_sl_child_fee_protocol_bps)
                    * Fraction::from(fee_pot))
                .to_ceil::<u64>();
            output_fee_pot_keeper_fee +=
                (Fraction::from_bps(global_config.tp_sl_child_fee_keeper_bps)
                    * Fraction::from(fee_pot))
                .to_ceil::<u64>();
        }
    }

    let output_to_send_to_maker = subtract_take_order_output_fees(
        output_amount,
        output_to_send_to_protocol,
        output_fee_pot_keeper_fee,
    )?;

    if output_to_send_to_maker < minimum_output_to_send_to_maker {
        msg!("output_amount: {}", output_amount);
        msg!(
            "minimum_output_to_send_to_maker: {}",
            minimum_output_to_send_to_maker
        );
        return err!(OrdoError::OrderOutputAmountInvalid);
    }

    msg!("input_to_send_to_taker: {}", input_to_send_to_taker);
    msg!("output_to_send_to_maker: {}", output_to_send_to_maker);
    msg!("output_to_send_to_protocol: {}", output_to_send_to_protocol);
    msg!("output_keeper_fee: {}", output_keeper_fee);

    Ok(ExecuteOrderEffects {
        input_to_send_to_taker,
        output_to_send_to_maker,
        output_to_send_to_protocol,
        output_keeper_fee,
    })
}

fn subtract_take_order_output_fees(
    output_amount: u64,
    output_to_send_to_protocol: u64,
    output_fee_pot_keeper_fee: u64,
) -> Result<u64> {
    Ok(output_amount
        .checked_sub(output_to_send_to_protocol)
        .and_then(|output_amount| output_amount.checked_sub(output_fee_pot_keeper_fee))
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?)
}
