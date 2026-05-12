use anchor_lang::{err, prelude::*, Result};

use crate::{GlobalConfig, LimoError, Order};

pub fn emergency_mode_disabled(global_config: &AccountLoader<GlobalConfig>) -> Result<()> {
    guard_disabled_flag(
        global_config.load()?.emergency_mode,
        LimoError::EmergencyModeEnabled,
    )
}

pub fn flash_taking_orders_disabled(global_config: &AccountLoader<GlobalConfig>) -> Result<()> {
    guard_disabled_flag(
        global_config.load()?.flash_take_order_blocked,
        LimoError::FlashTakeOrderBlocked,
    )
}

pub fn create_new_orders_disabled(global_config: &AccountLoader<GlobalConfig>) -> Result<()> {
    guard_disabled_flag(
        global_config.load()?.new_orders_blocked,
        LimoError::CreatingNewOrdersBlocked,
    )
}

pub fn taking_orders_disabled(global_config: &AccountLoader<GlobalConfig>) -> Result<()> {
    guard_disabled_flag(
        global_config.load()?.orders_taking_blocked,
        LimoError::OrderTakingBlocked,
    )
}

pub fn order_expired(order: &AccountLoader<Order>) -> Result<()> {
    let expiry = order.load()?.expiry_timestamp;
    // 0 means "no expiry" (backward compatibility: old orders had padding here)
    if expiry == 0 {
        return Ok(());
    }
    let timestamp: u64 = Clock::get()?
        .unix_timestamp
        .try_into()
        .expect("Negative timestamp");
    if expiry < timestamp {
        return err!(LimoError::OrderExpired);
    }
    Ok(())
}

fn guard_disabled_flag(flag_value: u8, error: LimoError) -> Result<()> {
    if flag_value > 0 {
        return Err(error.into());
    }
    Ok(())
}
