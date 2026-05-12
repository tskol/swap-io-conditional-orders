use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{
    dbg_msg,
    state::{GlobalConfig, OraclePoolsState, Order},
    utils::fraction::{Fraction, FractionExtra},
    LimoError,
};

pub(super) fn validate_stop_loss_price(
    global_config: &GlobalConfig,
    order: &Order,
    input_oracle_pool: Option<&AccountLoader<OraclePoolsState>>,
    output_oracle_pool: Option<&AccountLoader<OraclePoolsState>>,
    input_price_update: Option<&PriceUpdateV2>,
    output_price_update: Option<&PriceUpdateV2>,
    input_decimals: u8,
    output_decimals: u8,
    input_to_send_to_taker: u64,
) -> Result<()> {
    let input_oracle_pool_loader = input_oracle_pool.ok_or(LimoError::InvalidAccount)?;
    let output_oracle_pool_loader = output_oracle_pool.ok_or(LimoError::InvalidAccount)?;
    let input_oracle_pool = input_oracle_pool_loader.load()?;
    let output_oracle_pool = output_oracle_pool_loader.load()?;
    let input_price_update = input_price_update.ok_or(LimoError::InvalidAccount)?;
    let output_price_update = output_price_update.ok_or(LimoError::InvalidAccount)?;
    let anchor_clock = Clock::get()?;

    let input_price = input_price_update
        .get_price_no_older_than(
            &anchor_clock,
            global_config.oracle_max_staleness_seconds,
            &input_oracle_pool.oracle_feed_id,
        )
        .map_err(|_| LimoError::InvalidAccount)?;
    let output_price = output_price_update
        .get_price_no_older_than(
            &anchor_clock,
            global_config.oracle_max_staleness_seconds,
            &output_oracle_pool.oracle_feed_id,
        )
        .map_err(|_| LimoError::InvalidAccount)?;

    let numerator1 = u128::from(input_to_send_to_taker) * u128::from(order.expected_output_amount);
    let denominator1 = u128::from(order.initial_input_amount);
    let div_ceil_result1 = (numerator1 + denominator1 - 1) / denominator1;
    let mut expected_output_usd_price = u64::try_from(
        div_ceil_result1
            .checked_mul(u128::from(output_price.price as u64))
            .unwrap()
            .checked_div(10_u128.pow(output_price.exponent.abs().try_into().unwrap()))
            .unwrap(),
    )
    .map_err(|_| dbg_msg!(LimoError::MathOverflow))?;

    let numerator2 = u128::from(input_to_send_to_taker) * u128::from(input_price.price as u64);
    let denominator2 = 10_u128.pow(input_price.exponent.abs().try_into().unwrap());
    let div_ceil_result2 = (numerator2 + denominator2 - 1) / denominator2;
    let mut expected_input_usd_price =
        u64::try_from(div_ceil_result2).map_err(|_| dbg_msg!(LimoError::MathOverflow))?;

    if input_decimals > output_decimals {
        expected_output_usd_price = expected_output_usd_price
            .checked_mul(10_u64.pow((input_decimals - output_decimals).into()))
            .unwrap();
    } else {
        expected_input_usd_price = expected_input_usd_price
            .checked_mul(10_u64.pow((output_decimals - input_decimals).into()))
            .unwrap();
    }

    let sl_max_upward_deviation = (Fraction::from_bps(global_config.sl_max_upward_deviation_bps)
        * Fraction::from(expected_input_usd_price))
    .to_ceil::<u64>();

    if expected_input_usd_price
        > expected_output_usd_price
            .checked_add(sl_max_upward_deviation)
            .unwrap()
    {
        return err!(LimoError::PriceTooHigh);
    }

    Ok(())
}
