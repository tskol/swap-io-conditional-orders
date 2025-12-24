#![allow(clippy::too_many_arguments)]
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};

use anchor_lang::prelude::*;
use solana_program::clock;

use crate::{
    dbg_msg, require_lte,
    state::*,
    utils::{
        consts::UPDATE_GLOBAL_CONFIG_BYTE_SIZE,
        fraction::{Fraction, FractionExtra},
    },
    LimoError,
};

pub fn calculate_fee_amount(amount: u64, fee_bps: u16) -> Result<u64> {
    let fee_amount = (Fraction::from_bps(fee_bps) * Fraction::from(amount))
        .to_ceil::<u64>();
    Ok(fee_amount)
}

pub fn initialize_global_config(
    global_config: &mut GlobalConfig,
    admin_authority: Pubkey,
    pda_authority: Pubkey,
    pda_bump: u64,
    pda_authority_previous_lamports_balance: u64,
) {
    global_config.emergency_mode = 0;
    global_config.pda_authority = pda_authority;
    global_config.pda_authority_bump = pda_bump;
    global_config.admin_authority = admin_authority;
    global_config.admin_authority_cached = admin_authority;
    global_config.total_tip_amount = 0;
    global_config.host_tip_amount = 0;
    global_config.pda_authority_previous_lamports_balance = pda_authority_previous_lamports_balance;
}

pub fn initialize_oracle_pool(
    oracle_pool: &mut OraclePoolsState,
    global_config: Pubkey,
    feed_id: String,
    token_mint: Pubkey,
    bump: u8,
) {
    oracle_pool.global_config = global_config;
    oracle_pool.oracle_feed_id = feed_id;
    oracle_pool.token_mint = token_mint;
    oracle_pool.bump = bump;
}

pub fn update_oracle_pool(
    oracle_pool: &mut OraclePoolsState,
    feed_id: String,
) {
    oracle_pool.oracle_feed_id = feed_id;
}

pub fn create_order(
    order: &mut Order,
    global_config: Pubkey,
    owner: Pubkey,
    input_amount: u64,
    output_amount: u64,
    parent_order: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
    input_mint_program_id: Pubkey,
    output_mint_program_id: Pubkey,
    order_type: u8,
    in_vault_bump: u8,
    current_timestamp: i64,
) -> Result<()> {
    order.global_config = global_config;
    order.initial_input_amount = input_amount;
    order.remaining_input_amount = input_amount;
    order.expected_output_amount = output_amount;
    order.parent_order = parent_order;
    order.available_child_input_amount = 0;
    order.tp_child_order = Pubkey::default();
    order.sl_child_order = Pubkey::default();
    order.number_of_fills = 0;
    order.filled_output_amount = 0;
    order.input_mint = input_mint;
    order.input_mint_program_id = input_mint_program_id;
    order.output_mint = output_mint;
    order.output_mint_program_id = output_mint_program_id;
    order.maker = owner;
    order.status = OrderStatus::Active as u8;
    order.order_type = order_type;
    order.in_vault_bump = in_vault_bump;
    order.last_updated_timestamp = current_timestamp.try_into().expect("Negative timestamp");
    order.counterparty = Pubkey::default();
    order.permissionless = 0;

    Ok(())
}

pub fn update_order(order: &mut Order, mode: UpdateOrderMode, value: &[u8]) -> Result<()> {
    match mode {
        UpdateOrderMode::UpdatePermissionless => {
            require!(value.len() == 1, LimoError::InvalidParameterType);
            msg!("update_order mode={:?}", mode);
            msg!("new={} prev={}", value[0], order.permissionless);
            order.permissionless = value[0];
        }
        UpdateOrderMode::UpdateCounterparty => {
            require!(value.len() == 32, LimoError::InvalidParameterType);
            msg!("update_order mode={:?}", mode);
            msg!("new={:?} prev={}", &value[..32], order.counterparty);
            order.counterparty = Pubkey::new_from_array(
                value[..32]
                    .try_into()
                    .map_err(|_| LimoError::InvalidParameterType)?,
            );
        }
    }
    Ok(())
}

pub fn validate_user_swap_balances(
    start_balance_state: &UserSwapBalancesState,
    end_balance_state: GetBalancesCheckedResult,
    max_input_amount_change: u64,
    min_output_amount_change: u64,
) -> Result<()> {
    require_gte!(
        start_balance_state.input_ta_balance,
        end_balance_state.input_balance,
        LimoError::SwapInputInvalidBalanceChange
    );

    require_lte!(
        start_balance_state.output_ta_balance,
        end_balance_state.output_balance,
        LimoError::SwapOutputInvalidBalanceChange
    );

    require_lte!(
        start_balance_state.input_ta_balance - end_balance_state.input_balance,
        max_input_amount_change,
        LimoError::SwapInputAmountTooLarge
    );
    require_gte!(
        end_balance_state.output_balance - start_balance_state.output_ta_balance,
        min_output_amount_change,
        LimoError::SwapOutputAmountTooSmall
    );
    Ok(())
}

pub fn close_order_and_claim_tip(
    order: &mut Order,
    global_config: &mut GlobalConfig,
    current_timestamp: u64,
) -> Result<()> {
    require!(
        order.status == OrderStatus::Active as u8 || order.status == OrderStatus::Filled as u8,
        LimoError::OrderCanNotBeCanceled
    );

    require!(
        current_timestamp >= order.last_updated_timestamp + global_config.order_close_delay_seconds,
        LimoError::NotEnoughTimePassedSinceLastUpdate
    );

    require!(
        order.flash_ix_lock == 0,
        LimoError::OrderWithinFlashOperation
    );

    order.status = OrderStatus::Cancelled as u8;

    global_config.total_tip_amount -= order.tip_amount;

    Ok(())
}

pub fn withdraw_host_tip(
    global_config: &mut GlobalConfig,
    pda_authority_balance: u64,
) -> Result<u64> {
    require_gte!(
        pda_authority_balance,
        global_config.host_tip_amount,
        LimoError::InvalidHostTipBalance
    );
    let host_tip_amount = global_config.host_tip_amount;
    global_config.total_tip_amount -= host_tip_amount;
    global_config.host_tip_amount = 0;
    Ok(host_tip_amount)
}

// pub fn flash_withdraw_order_input(
//     order: &mut Order,
//     input_amount: u64,
//     output_amount: u64,
// ) -> Result<TakeOrderEffects> {
//     let TakeOrderEffects {
//         input_to_send_to_taker,
//         output_to_send_to_maker,
//     } = take_order_calcs(order, input_amount, output_amount)?;

//     require!(
//         order.flash_ix_lock == 0,
//         LimoError::OrderWithinFlashOperation
//     );

//     order.flash_ix_lock = 1;
//     Ok(TakeOrderEffects {
//         input_to_send_to_taker,
//         output_to_send_to_maker,
//     })
// }

// pub fn flash_pay_order_output(
//     global_config: &mut GlobalConfig,
//     order: &mut Order,
//     input_amount: u64,
//     output_amount: u64,
//     tip_amount: u64,
//     current_timestamp: clock::UnixTimestamp,
// ) -> Result<TakeOrderEffects> {
//     let TakeOrderEffects {
//         input_to_send_to_taker,
//         output_to_send_to_maker,
//     } = take_order_calcs(order, input_amount, output_amount)?;

//     require!(
//         order.flash_ix_lock == 1,
//         LimoError::OrderNotWithinFlashOperation
//     );

//     update_take_order_accounting_and_tips(
//         global_config,
//         order,
//         input_to_send_to_taker,
//         output_to_send_to_maker,
//         tip_amount,
//         current_timestamp,
//     )?;

//     order.flash_ix_lock = 0;
//     Ok(TakeOrderEffects {
//         input_to_send_to_taker,
//         output_to_send_to_maker,
//     })
// }

pub fn take_order_calcs(
    order: &Order,
    global_config: &GlobalConfig,
    input_amount: u64,
    output_amount: u64,
) -> Result<TakeOrderEffects> {
    require!(input_amount > 0, LimoError::OrderInputAmountInvalid);

    require!(
        order.status == OrderStatus::Active as u8,
        LimoError::OrderNotActive
    );

    require!(
        input_amount <= order.remaining_input_amount,
        LimoError::OrderInputAmountTooLarge
    );

    let input_to_send_to_taker = input_amount;
    let minimum_output_to_send_to_maker_u128 = (u128::from(input_to_send_to_taker)
        * u128::from(order.expected_output_amount))
    .div_ceil(u128::from(order.initial_input_amount));

    let minimum_output_to_send_to_maker = u64::try_from(minimum_output_to_send_to_maker_u128)
        .map_err(|_| dbg_msg!(LimoError::MathOverflow))?;

    let fee_pot = output_amount.checked_sub(minimum_output_to_send_to_maker).unwrap_or(0);

    let mut output_to_send_to_protocol = 0;
    let mut output_keeper_fee = 0;
    if fee_pot > 0 {
        if order.order_type == OrderType::LimitParent as u8 {
            output_to_send_to_protocol = (Fraction::from_bps(global_config.parent_fill_fee_protocol_bps) * Fraction::from(fee_pot))
                .to_ceil::<u64>();
            output_keeper_fee = (Fraction::from_bps(global_config.parent_fill_fee_keeper_bps) * Fraction::from(fee_pot))
                .to_ceil::<u64>();
        } else if order.order_type == OrderType::LimitTP as u8 || order.order_type == OrderType::LimitSL as u8 {
            output_to_send_to_protocol = (Fraction::from_bps(global_config.tp_sl_child_fee_protocol_bps) * Fraction::from(fee_pot))
                .to_ceil::<u64>();
            output_keeper_fee = (Fraction::from_bps(global_config.tp_sl_child_fee_keeper_bps) * Fraction::from(fee_pot))
                .to_ceil::<u64>();
        }
    }

    let output_to_send_to_maker = output_amount
        .checked_sub(output_to_send_to_protocol)
        .ok_or(LimoError::MathOverflow)
        .unwrap()
        .checked_sub(output_keeper_fee)
        .ok_or(LimoError::MathOverflow)
        .unwrap();

    if output_to_send_to_maker < minimum_output_to_send_to_maker {
        msg!("output_amount: {}", output_amount);
        msg!(
            "minimum_output_to_send_to_maker: {}",
            minimum_output_to_send_to_maker
        );
        return err!(LimoError::OrderOutputAmountInvalid);
    }

    msg!("input_to_send_to_taker: {}", input_to_send_to_taker);
    msg!("output_to_send_to_maker: {}", output_to_send_to_maker);
    msg!("output_to_send_to_protocol: {}", output_to_send_to_protocol);

    Ok(TakeOrderEffects {
        input_to_send_to_taker,
        output_to_send_to_maker,
        output_to_send_to_protocol,
    })
}

pub fn take_order(
    global_config: &mut GlobalConfig,
    order: &mut Order,
    parent_order: Option<&mut Order>,
    brother_order: Option<&mut Order>,
    input_oracle_pool: Option<&OraclePoolsState>,
    output_oracle_pool: Option<&OraclePoolsState>,
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
        LimoError::OrderWithinFlashOperation
    );

    let TakeOrderEffects {
        input_to_send_to_taker,
        output_to_send_to_maker,
        output_to_send_to_protocol,
    } = take_order_calcs(order, global_config, input_amount, output_amount)?;

    let is_child_order = order.parent_order != Pubkey::default();

    if is_child_order {
        let parent_order = parent_order.ok_or(LimoError::InvalidAccount)?;
        update_take_child_order_accounting_and_tips(
            global_config,
            order,
            parent_order,
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
    })
}

pub fn update_global_config(
    global_config: &mut GlobalConfig,
    mode: UpdateGlobalConfigMode,
    value: &[u8; UPDATE_GLOBAL_CONFIG_BYTE_SIZE],
    ts: u64,
) -> Result<()> {
    match mode {
        UpdateGlobalConfigMode::UpdateEmergencyMode
        | UpdateGlobalConfigMode::UpdateFlashTakeOrderBlocked
        | UpdateGlobalConfigMode::UpdateBlockNewOrders
        | UpdateGlobalConfigMode::UpdateBlockOrderTaking
        | UpdateGlobalConfigMode::UpdateTpSlEnabled => {
            let value = value[0];
            update_global_config_flag(global_config, mode, value, ts)?;
        }
        UpdateGlobalConfigMode::UpdateHostFeeBps
        | UpdateGlobalConfigMode::UpdateCreateOrderFeeBps
        | UpdateGlobalConfigMode::UpdateSlMaxUpwardDeviationBps
        | UpdateGlobalConfigMode::UpdateTpSlMinDistanceBps
        | UpdateGlobalConfigMode::UpdateParentFillFeeKeeperBps
        | UpdateGlobalConfigMode::UpdateParentFillFeeProtocolBps
        | UpdateGlobalConfigMode::UpdateTpSlChildFeeKeeperBps
        | UpdateGlobalConfigMode::UpdateTpSlChildFeeProtocolBps => {
            let value = u16::from_le_bytes(value[0..2].try_into().unwrap());
            update_global_config_bps(global_config, mode, value, ts)?;
        }
        UpdateGlobalConfigMode::UpdateOrderCloseDelaySeconds => {
            let value = u64::from_le_bytes(value[0..8].try_into().unwrap());
            msg!("update_global_config mode={:?} ts={}", mode, ts);
            msg!(
                "new={} prev={}",
                value,
                global_config.order_close_delay_seconds
            );
            global_config.order_close_delay_seconds = value;
        }
        UpdateGlobalConfigMode::UpdateAdminAuthorityCached => {
            let value = Pubkey::new_from_array(value[0..32].try_into().unwrap());
            update_global_config_pubkey(global_config, mode, value, ts)?
        }
        UpdateGlobalConfigMode::UpdateTxnFeeCost => {
            let value = u64::from_le_bytes(value[0..8].try_into().unwrap());
            msg!("update_global_config mode={:?} ts={}", mode, ts);
            msg!("new={} prev={}", value, global_config.txn_fee_cost);
            global_config.txn_fee_cost = value;
        }
        UpdateGlobalConfigMode::UpdateAtaCreationCost => {
            let value = u64::from_le_bytes(value[0..8].try_into().unwrap());
            msg!("update_global_config mode={:?} ts={}", mode, ts);
            msg!("new={} prev={}", value, global_config.ata_creation_cost);
            global_config.ata_creation_cost = value;
        }
        UpdateGlobalConfigMode::UpdateOracleMaxStalenessSeconds => {
            let value = u64::from_le_bytes(value[0..8].try_into().unwrap());
            msg!("update_global_config mode={:?} ts={}", mode, ts);
            msg!("new={} prev={}", value, global_config.oracle_max_staleness_seconds);
            global_config.oracle_max_staleness_seconds = value;
        }
    }
    Ok(())
}

pub fn validate_pda_authority_balance_and_update_accounting(
    global_config: &mut GlobalConfig,
    pda_authority_balance: u64,
    tip: u64,
) -> Result<()> {
    require_gte!(
        pda_authority_balance - global_config.pda_authority_previous_lamports_balance,
        tip,
        LimoError::InvalidTipTransferAmount
    );
    require_gte!(
        pda_authority_balance,
        global_config.total_tip_amount,
        LimoError::InvalidTipBalance
    );

    global_config.pda_authority_previous_lamports_balance = pda_authority_balance;

    Ok(())
}

fn update_take_order_accounting_and_tips(
    global_config: &mut GlobalConfig,
    order: &mut Order,
    input_to_send_to_taker: u64,
    output_to_send_to_maker: u64,
    tip_amount: u64,
    current_timestamp: i64,
) -> Result<()> {
    order.remaining_input_amount = order
        .remaining_input_amount
        .checked_sub(input_to_send_to_taker)
        .ok_or_else(|| dbg_msg!(LimoError::MathOverflow))?;

    order.filled_output_amount = order
        .filled_output_amount
        .checked_add(output_to_send_to_maker)
        .ok_or_else(|| dbg_msg!(LimoError::MathOverflow))?;

    let TipCalcs {
        host_tip,
        maker_tip,
    } = tip_calcs(global_config, tip_amount)?;

    global_config.host_tip_amount = global_config
        .host_tip_amount
        .checked_add(host_tip)
        .ok_or_else(|| dbg_msg!(LimoError::MathOverflow))?;

    order.tip_amount = order
        .tip_amount
        .checked_add(maker_tip)
        .ok_or_else(|| dbg_msg!(LimoError::MathOverflow))?;

    global_config.total_tip_amount = global_config
        .total_tip_amount
        .checked_add(tip_amount)
        .ok_or_else(|| dbg_msg!(LimoError::MathOverflow))?;

    order.number_of_fills += 1;

    if order.remaining_input_amount == 0
        && order.filled_output_amount >= order.expected_output_amount
    {
        order.status = OrderStatus::Filled as u8;
    }
    order.last_updated_timestamp = current_timestamp.try_into().expect("Negative timestamp");
    Ok(())
}

fn update_take_child_order_accounting_and_tips(
    global_config: &mut GlobalConfig,
    order: &mut Order,
    parent_order: &mut Order,
    brother_order: Option<&mut Order>,
    input_oracle_pool: Option<&OraclePoolsState>,
    output_oracle_pool: Option<&OraclePoolsState>,
    input_price_update: Option<&PriceUpdateV2>,
    output_price_update: Option<&PriceUpdateV2>,
    input_decimals: u8,
    output_decimals: u8,
    input_to_send_to_taker: u64,
    output_to_send_to_maker: u64,
    tip_amount: u64,
    current_timestamp: i64,
) -> Result<()> {
    require!(input_to_send_to_taker <= parent_order.available_child_input_amount, LimoError::OrderInputAmountTooLarge);
    parent_order.available_child_input_amount = parent_order
        .available_child_input_amount
        .checked_sub(input_to_send_to_taker)
        .ok_or_else(|| dbg_msg!(LimoError::MathOverflow))?;
    if order.order_type == OrderType::LimitSL as u8 {
        let input_oracle_pool = input_oracle_pool.ok_or(LimoError::InvalidAccount)?;
        let output_oracle_pool = output_oracle_pool.ok_or(LimoError::InvalidAccount)?;
        let input_price_update = input_price_update.ok_or(LimoError::InvalidAccount)?;
        let output_price_update = output_price_update.ok_or(LimoError::InvalidAccount)?;
        let anchor_clock = Clock::get()?;

        let input_feed_id = get_feed_id_from_hex(&input_oracle_pool.oracle_feed_id)
            .map_err(|_| LimoError::InvalidAccount)?;
        let input_price = input_price_update
            .get_price_no_older_than(&anchor_clock, global_config.oracle_max_staleness_seconds, &input_feed_id)
            .map_err(|_| LimoError::InvalidAccount)?;
        let output_feed_id = get_feed_id_from_hex(&output_oracle_pool.oracle_feed_id)
            .map_err(|_| LimoError::InvalidAccount)?;
        let output_price = output_price_update
            .get_price_no_older_than(&anchor_clock, global_config.oracle_max_staleness_seconds, &output_feed_id)
            .map_err(|_| LimoError::InvalidAccount)?;

        let mut expected_output_usd_price = u64::try_from((u128::from(input_to_send_to_taker)
            * u128::from(order.expected_output_amount))
            .div_ceil(u128::from(order.initial_input_amount))
            .checked_mul(u128::from(output_price.price as u64))
            .unwrap()
            .checked_div(10_u128.pow(output_price.exponent.abs().try_into().unwrap()))
            .unwrap())
            .map_err(|_| dbg_msg!(LimoError::MathOverflow))?;

        let mut expected_input_usd_price = u64::try_from((u128::from(input_to_send_to_taker)
            * u128::from(input_price.price as u64))
            .div_ceil(u128::from(10_u128.pow(input_price.exponent.abs().try_into().unwrap()))))
            .map_err(|_| dbg_msg!(LimoError::MathOverflow))?;

        if input_decimals > output_decimals {
            expected_output_usd_price = expected_output_usd_price.checked_mul(10_u64.pow((input_decimals - output_decimals).try_into().unwrap())).unwrap();
        } else {
            expected_input_usd_price = expected_input_usd_price.checked_mul(10_u64.pow((output_decimals - input_decimals).try_into().unwrap())).unwrap();
        }

        let sl_max_upward_deviation = (Fraction::from_bps(global_config.sl_max_upward_deviation_bps) * Fraction::from(expected_output_usd_price))
            .to_ceil::<u64>();

        if expected_input_usd_price > expected_output_usd_price.checked_add(sl_max_upward_deviation).unwrap() {
            return err!(LimoError::PriceTooHigh);
        }
    }

    update_take_order_accounting_and_tips(
        global_config,
        order,
        input_to_send_to_taker,
        output_to_send_to_maker,
        tip_amount,
        current_timestamp
    )?;

    if parent_order.status == OrderStatus::Filled as u8 && parent_order.available_child_input_amount == 0 {
        order.status = OrderStatus::Filled as u8;
        if let Some(brother_order) = brother_order {
            brother_order.status = OrderStatus::Filled as u8;
        }
    }
    Ok(())
}

fn tip_calcs(global_config: &GlobalConfig, tip_amount: u64) -> Result<TipCalcs> {
    let host_tip = (Fraction::from_bps(global_config.host_fee_bps) * Fraction::from(tip_amount))
        .to_ceil::<u64>();

    let maker_tip = tip_amount
        .checked_sub(host_tip)
        .ok_or_else(|| dbg_msg!(LimoError::MathOverflow))?;

    Ok(TipCalcs {
        host_tip,
        maker_tip,
    })
}

fn update_global_config_flag(
    global_config: &mut GlobalConfig,
    mode: UpdateGlobalConfigMode,
    value: u8,
    ts: u64,
) -> Result<()> {
    msg!("update_global_config_flag mode={:?} ts={}", mode, ts);

    if value != 0 && value != 1 {
        return err!(LimoError::InvalidFlag);
    }

    match mode {
        UpdateGlobalConfigMode::UpdateEmergencyMode => {
            msg!("new={} prev={}", value, global_config.emergency_mode,);
            global_config.emergency_mode = value;
        }
        UpdateGlobalConfigMode::UpdateFlashTakeOrderBlocked => {
            msg!(
                "new={} prev={}",
                value,
                global_config.flash_take_order_blocked,
            );
            global_config.flash_take_order_blocked = value;
        }
        UpdateGlobalConfigMode::UpdateBlockNewOrders => {
            msg!("new={} prev={}", value, global_config.new_orders_blocked,);
            global_config.new_orders_blocked = value;
        }
        UpdateGlobalConfigMode::UpdateBlockOrderTaking => {
            msg!("new={} prev={}", value, global_config.orders_taking_blocked,);
            global_config.orders_taking_blocked = value;
        }
        UpdateGlobalConfigMode::UpdateTpSlEnabled => {
            msg!("new={} prev={}", value, global_config.tp_sl_enabled,);
            global_config.tp_sl_enabled = value;
        }
        _ => return Err(LimoError::InvalidConfigOption.into()),
    }

    Ok(())
}

fn update_global_config_bps(
    global_config: &mut GlobalConfig,
    mode: UpdateGlobalConfigMode,
    value: u16,
    ts: u64,
) -> Result<()> {
    msg!("update_global_config_bps mode={:?} ts={}", mode, ts);

    if value > 10000 {
        return err!(LimoError::InvalidBps);
    }

    match mode {
        UpdateGlobalConfigMode::UpdateHostFeeBps => {
            msg!("new={} prev={}", value, global_config.host_fee_bps);
            global_config.host_fee_bps = value;
        }
        UpdateGlobalConfigMode::UpdateCreateOrderFeeBps => {
            msg!("new={} prev={}", value, global_config.create_order_fee_bps);
            global_config.create_order_fee_bps = value;
        }
        UpdateGlobalConfigMode::UpdateSlMaxUpwardDeviationBps => {
            msg!("new={} prev={}", value, global_config.sl_max_upward_deviation_bps);
            global_config.sl_max_upward_deviation_bps = value;
        }
        UpdateGlobalConfigMode::UpdateTpSlMinDistanceBps => {
            msg!("new={} prev={}", value, global_config.tp_sl_min_distance_bps);
            global_config.tp_sl_min_distance_bps = value;
        }
        UpdateGlobalConfigMode::UpdateParentFillFeeKeeperBps => {
            msg!("new={} prev={}", value, global_config.parent_fill_fee_keeper_bps);
            global_config.parent_fill_fee_keeper_bps = value;
        }
        UpdateGlobalConfigMode::UpdateParentFillFeeProtocolBps => {
            msg!("new={} prev={}", value, global_config.parent_fill_fee_protocol_bps);
            global_config.parent_fill_fee_protocol_bps = value;
        }
        UpdateGlobalConfigMode::UpdateTpSlChildFeeKeeperBps => {
            msg!("new={} prev={}", value, global_config.tp_sl_child_fee_keeper_bps);
            global_config.tp_sl_child_fee_keeper_bps = value;
        }
        UpdateGlobalConfigMode::UpdateTpSlChildFeeProtocolBps => {
            msg!("new={} prev={}", value, global_config.tp_sl_child_fee_protocol_bps);
            global_config.tp_sl_child_fee_protocol_bps = value;
        }
        _ => return Err(LimoError::InvalidConfigOption.into()),
    }

    Ok(())
}

fn update_global_config_pubkey(
    global_config: &mut GlobalConfig,
    mode: UpdateGlobalConfigMode,
    value: Pubkey,
    ts: u64,
) -> Result<()> {
    msg!("update_global_config_pubkey mode={:?} ts={}", mode, ts);

    match mode {
        UpdateGlobalConfigMode::UpdateAdminAuthorityCached => {
            msg!(
                "new={} prev={}",
                value,
                global_config.admin_authority_cached,
            );
            global_config.admin_authority_cached = value;
        }
        _ => return Err(LimoError::InvalidConfigOption.into()),
    }

    Ok(())
}
