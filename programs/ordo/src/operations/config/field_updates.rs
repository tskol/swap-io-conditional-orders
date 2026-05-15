use anchor_lang::prelude::*;

use crate::{
    state::{GlobalConfig, UpdateGlobalConfigMode},
    OrdoError,
};

pub(super) fn update_global_config_flag(
    global_config: &mut GlobalConfig,
    mode: UpdateGlobalConfigMode,
    value: u8,
    ts: u64,
) -> Result<()> {
    msg!("update_global_config_flag mode={:?} ts={}", mode, ts);

    if value != 0 && value != 1 {
        return err!(OrdoError::InvalidFlag);
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
        _ => return Err(OrdoError::InvalidConfigOption.into()),
    }

    Ok(())
}

pub(super) fn update_global_config_bps(
    global_config: &mut GlobalConfig,
    mode: UpdateGlobalConfigMode,
    value: u16,
    ts: u64,
) -> Result<()> {
    msg!("update_global_config_bps mode={:?} ts={}", mode, ts);

    if value > 10000 {
        return err!(OrdoError::InvalidBps);
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
            msg!(
                "new={} prev={}",
                value,
                global_config.sl_max_upward_deviation_bps
            );
            global_config.sl_max_upward_deviation_bps = value;
        }
        UpdateGlobalConfigMode::UpdateTpSlMinDistanceBps => {
            msg!(
                "new={} prev={}",
                value,
                global_config.tp_sl_min_distance_bps
            );
            global_config.tp_sl_min_distance_bps = value;
        }
        UpdateGlobalConfigMode::UpdateParentFillFeeKeeperBps => {
            msg!(
                "new={} prev={}",
                value,
                global_config.parent_fill_fee_keeper_bps
            );
            global_config.parent_fill_fee_keeper_bps = value;
        }
        UpdateGlobalConfigMode::UpdateParentFillFeeProtocolBps => {
            msg!(
                "new={} prev={}",
                value,
                global_config.parent_fill_fee_protocol_bps
            );
            global_config.parent_fill_fee_protocol_bps = value;
        }
        UpdateGlobalConfigMode::UpdateTpSlChildFeeKeeperBps => {
            msg!(
                "new={} prev={}",
                value,
                global_config.tp_sl_child_fee_keeper_bps
            );
            global_config.tp_sl_child_fee_keeper_bps = value;
        }
        UpdateGlobalConfigMode::UpdateTpSlChildFeeProtocolBps => {
            msg!(
                "new={} prev={}",
                value,
                global_config.tp_sl_child_fee_protocol_bps
            );
            global_config.tp_sl_child_fee_protocol_bps = value;
        }
        UpdateGlobalConfigMode::UpdateKeeperTakeFeeBps => {
            msg!("new={} prev={}", value, global_config.keeper_take_fee_bps);
            global_config.keeper_take_fee_bps = value;
        }
        UpdateGlobalConfigMode::UpdateKeeperCloseFeeBps => {
            msg!("new={} prev={}", value, global_config.keeper_close_fee_bps);
            global_config.keeper_close_fee_bps = value;
        }
        _ => return Err(OrdoError::InvalidConfigOption.into()),
    }

    Ok(())
}

pub(super) fn update_global_config_pubkey(
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
        UpdateGlobalConfigMode::UpdateAllowedTaker => {
            msg!("new={} prev={}", value, global_config.allowed_taker,);
            global_config.allowed_taker = value;
        }
        _ => return Err(OrdoError::InvalidConfigOption.into()),
    }

    Ok(())
}
