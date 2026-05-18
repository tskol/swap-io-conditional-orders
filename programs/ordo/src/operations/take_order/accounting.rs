use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{
    dbg_msg,
    state::{GlobalConfig, OraclePoolsState, Order, OrderStatus, OrderType, TipCalcs},
    utils::fraction::{Fraction, FractionExtra},
    OrdoError,
};

use super::sl_guard::validate_stop_loss_price;

pub(super) fn update_take_order_accounting_and_tips(
    global_config: &mut GlobalConfig,
    order: &mut Order,
    input_to_send_to_taker: u64,
    output_to_send_to_maker: u64,
    tip_amount: u64,
    current_timestamp: i64,
) -> Result<()> {
    let last_updated_timestamp = current_timestamp.try_into().map_err(OrdoError::from)?;
    let number_of_fills = order
        .number_of_fills
        .checked_add(1)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;

    order.remaining_input_amount = order
        .remaining_input_amount
        .checked_sub(input_to_send_to_taker)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;

    order.filled_output_amount = order
        .filled_output_amount
        .checked_add(output_to_send_to_maker)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;

    if order.order_type == OrderType::LimitParent as u8 {
        order.available_child_input_amount = order
            .available_child_input_amount
            .checked_add(output_to_send_to_maker)
            .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;
    }

    let TipCalcs {
        host_tip,
        maker_tip,
    } = tip_calcs(global_config, tip_amount)?;

    global_config.host_tip_amount = global_config
        .host_tip_amount
        .checked_add(host_tip)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;

    order.tip_amount = order
        .tip_amount
        .checked_add(maker_tip)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;

    global_config.total_tip_amount = global_config
        .total_tip_amount
        .checked_add(tip_amount)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;

    order.number_of_fills = number_of_fills;

    if order.remaining_input_amount == 0
        && order.filled_output_amount >= order.expected_output_amount
    {
        order.status = OrderStatus::Filled as u8;
    }
    order.last_updated_timestamp = last_updated_timestamp;
    Ok(())
}

pub(super) fn update_take_child_order_accounting_and_tips(
    global_config: &mut GlobalConfig,
    order: &mut Order,
    parent_order: &mut Order,
    brother_order: Option<&AccountLoader<Order>>,
    input_oracle_pool: Option<&AccountLoader<OraclePoolsState>>,
    output_oracle_pool: Option<&AccountLoader<OraclePoolsState>>,
    input_price_update: Option<&PriceUpdateV2>,
    output_price_update: Option<&PriceUpdateV2>,
    input_decimals: u8,
    output_decimals: u8,
    input_to_send_to_taker: u64,
    output_to_send_to_maker: u64,
    tip_amount: u64,
    current_timestamp: i64,
) -> Result<()> {
    require!(
        input_to_send_to_taker <= parent_order.available_child_input_amount,
        OrdoError::OrderInputAmountTooLarge
    );
    parent_order.available_child_input_amount = parent_order
        .available_child_input_amount
        .checked_sub(input_to_send_to_taker)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;
    if order.order_type == OrderType::LimitSL as u8 {
        validate_stop_loss_price(
            global_config,
            order,
            input_oracle_pool,
            output_oracle_pool,
            input_price_update,
            output_price_update,
            input_decimals,
            output_decimals,
            input_to_send_to_taker,
        )?;
    }

    update_take_order_accounting_and_tips(
        global_config,
        order,
        input_to_send_to_taker,
        output_to_send_to_maker,
        tip_amount,
        current_timestamp,
    )?;

    if parent_order.status == OrderStatus::Filled as u8 && order.status == OrderStatus::Filled as u8
    {
        let brother_key = match order.order_type {
            value if value == OrderType::LimitTP as u8 => parent_order.sl_child_order,
            value if value == OrderType::LimitSL as u8 => parent_order.tp_child_order,
            _ => Pubkey::default(),
        };

        if brother_key != Pubkey::default() {
            if let Some(brother_order) = brother_order {
            let mut brother_order = brother_order.load_mut()?;
            brother_order.status = OrderStatus::Filled as u8;
            }
        }
    }
    Ok(())
}

fn tip_calcs(global_config: &GlobalConfig, tip_amount: u64) -> Result<TipCalcs> {
    let host_tip = (Fraction::from_bps(global_config.host_fee_bps) * Fraction::from(tip_amount))
        .to_ceil::<u64>();

    let maker_tip = tip_amount
        .checked_sub(host_tip)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;

    Ok(TipCalcs {
        host_tip,
        maker_tip,
    })
}

#[cfg(test)]
#[path = "accounting_tests.rs"]
mod tests;
