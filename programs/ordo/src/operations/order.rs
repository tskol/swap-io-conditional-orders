use anchor_lang::prelude::*;

use crate::{
    dbg_msg,
    state::{GlobalConfig, Order, OrderStatus, UpdateOrderMode},
    OrdoError,
};

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
    active_duration_seconds: u64,
) -> Result<()> {
    let timestamp = current_timestamp.try_into().map_err(OrdoError::from)?;
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
    order.last_updated_timestamp = timestamp;
    order.counterparty = Pubkey::default();
    order.permissionless = 0;
    order.expiry_timestamp = if active_duration_seconds == 0 {
        0
    } else {
        timestamp
            .checked_add(active_duration_seconds)
            .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?
    };
    Ok(())
}

pub fn update_order(order: &mut Order, mode: UpdateOrderMode, value: &[u8]) -> Result<()> {
    match mode {
        UpdateOrderMode::UpdatePermissionless => {
            require!(value.len() == 1, OrdoError::InvalidParameterType);
            require!(value[0] == 0 || value[0] == 1, OrdoError::InvalidFlag);
            msg!("update_order mode={:?}", mode);
            msg!("new={} prev={}", value[0], order.permissionless);
            order.permissionless = value[0];
        }
        UpdateOrderMode::UpdateCounterparty => {
            require!(value.len() == 32, OrdoError::InvalidParameterType);
            msg!("update_order mode={:?}", mode);
            msg!("new={:?} prev={}", &value[..32], order.counterparty);
            order.counterparty = Pubkey::new_from_array(
                value[..32]
                    .try_into()
                    .map_err(|_| OrdoError::InvalidParameterType)?,
            );
        }
    }
    Ok(())
}

pub fn close_order_and_claim_tip(
    order: &mut Order,
    global_config: &mut GlobalConfig,
    current_timestamp: u64,
) -> Result<()> {
    require!(
        order.status == OrderStatus::Active as u8 || order.status == OrderStatus::Filled as u8,
        OrdoError::OrderCanNotBeCanceled
    );

    let close_after_timestamp = order
        .last_updated_timestamp
        .checked_add(global_config.order_close_delay_seconds)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;

    require!(
        current_timestamp >= close_after_timestamp,
        OrdoError::NotEnoughTimePassedSinceLastUpdate
    );

    require!(
        order.flash_ix_lock == 0,
        OrdoError::OrderWithinFlashOperation
    );

    let total_tip_amount = global_config
        .total_tip_amount
        .checked_sub(order.tip_amount)
        .ok_or_else(|| dbg_msg!(OrdoError::InvalidTipBalance))?;

    order.status = OrderStatus::Cancelled as u8;
    global_config.total_tip_amount = total_tip_amount;

    Ok(())
}
