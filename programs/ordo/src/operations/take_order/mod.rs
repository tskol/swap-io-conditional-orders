use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use solana_program::clock;

use crate::{
    state::{GlobalConfig, OraclePoolsState, Order, TakeOrderEffects},
    OrdoError,
};

mod accounting;
mod calculations;
mod sl_guard;

use accounting::{
    update_take_child_order_accounting_and_tips, update_take_order_accounting_and_tips,
};
pub use calculations::take_order_calcs;

pub fn take_order(
    global_config: &mut GlobalConfig,
    order: &mut Order,
    parent_order: Option<&AccountLoader<Order>>,
    brother_order: Option<&AccountLoader<Order>>,
    input_oracle_pool: Option<&AccountLoader<OraclePoolsState>>,
    output_oracle_pool: Option<&AccountLoader<OraclePoolsState>>,
    input_price_update: Option<&PriceUpdateV2>,
    output_price_update: Option<&PriceUpdateV2>,
    input_decimals: u8,
    output_decimals: u8,
    input_amount: u64,
    tip_amount: u64,
    current_timestamp: clock::UnixTimestamp,
    output_amount: u64,
) -> Result<TakeOrderEffects> {
    require!(
        order.flash_ix_lock == 0,
        OrdoError::OrderWithinFlashOperation
    );

    let TakeOrderEffects {
        input_to_send_to_taker,
        output_to_send_to_maker,
        output_to_send_to_protocol,
        output_keeper_fee,
    } = take_order_calcs(order, global_config, input_amount, output_amount)?;

    let is_child_order = order.parent_order != Pubkey::default();

    if is_child_order {
        let parent_order_loader = parent_order.ok_or(OrdoError::InvalidAccount)?;
        let mut parent_order = parent_order_loader.load_mut()?;
        update_take_child_order_accounting_and_tips(
            global_config,
            order,
            &mut parent_order,
            brother_order,
            input_oracle_pool,
            output_oracle_pool,
            input_price_update,
            output_price_update,
            input_decimals,
            output_decimals,
            input_to_send_to_taker,
            output_to_send_to_maker,
            tip_amount,
            current_timestamp,
        )?;
    } else {
        update_take_order_accounting_and_tips(
            global_config,
            order,
            input_to_send_to_taker,
            output_to_send_to_maker,
            tip_amount,
            current_timestamp,
        )?;
    }

    Ok(TakeOrderEffects {
        input_to_send_to_taker,
        output_to_send_to_maker,
        output_to_send_to_protocol,
        output_keeper_fee,
    })
}
