use anchor_lang::prelude::*;
use num_enum::TryFromPrimitive;

use crate::OrdoError;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OrderStatus {
    Active = 0,
    Filled = 1,
    Cancelled = 2,
}

impl From<OrderStatus> for u8 {
    fn from(val: OrderStatus) -> Self {
        match val {
            OrderStatus::Active => 0,
            OrderStatus::Filled => 1,
            OrderStatus::Cancelled => 2,
        }
    }
}

impl From<u8> for OrderStatus {
    fn from(val: u8) -> Self {
        match val {
            0 => OrderStatus::Active,
            1 => OrderStatus::Filled,
            2 => OrderStatus::Cancelled,
            _ => panic!("Invalid OrderStatus"),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum OrderType {
    Vanilla = 0,
    LimitParent = 1,
    LimitTP = 2,
    LimitSL = 3,
}

impl From<OrderType> for u8 {
    fn from(val: OrderType) -> Self {
        match val {
            OrderType::Vanilla => 0,
            OrderType::LimitParent => 1,
            OrderType::LimitTP => 2,
            OrderType::LimitSL => 3,
        }
    }
}

impl TryFrom<u8> for OrderType {
    type Error = OrdoError;
    fn try_from(val: u8) -> core::result::Result<Self, OrdoError> {
        match val {
            0 => Ok(OrderType::Vanilla),
            1 => Ok(OrderType::LimitParent),
            2 => Ok(OrderType::LimitTP),
            3 => Ok(OrderType::LimitSL),
            _ => Err(OrdoError::OrderTypeInvalid),
        }
    }
}

#[derive(TryFromPrimitive, PartialEq, Eq, Clone, Copy, Debug)]
#[repr(u16)]
pub enum UpdateGlobalConfigMode {
    UpdateEmergencyMode = 0,
    UpdateFlashTakeOrderBlocked = 1,
    UpdateBlockNewOrders = 2,
    UpdateBlockOrderTaking = 3,
    UpdateHostFeeBps = 4,
    UpdateAdminAuthorityCached = 5,
    UpdateOrderCloseDelaySeconds = 6,
    UpdateTxnFeeCost = 7,
    UpdateAtaCreationCost = 8,
    UpdateTpSlEnabled = 9,
    UpdateCreateOrderFeeBps = 10,
    UpdateParentFillFeeKeeperBps = 11,
    UpdateParentFillFeeProtocolBps = 12,
    UpdateTpSlChildFeeKeeperBps = 13,
    UpdateTpSlChildFeeProtocolBps = 14,
    UpdateOracleMaxStalenessSeconds = 15,
    UpdateSlMaxUpwardDeviationBps = 16,
    UpdateTpSlMinDistanceBps = 17,
    UpdateAllowedTaker = 18,
    UpdateKeeperTakeFeeBps = 19,
    UpdateKeeperCloseFeeBps = 20,
}

#[derive(
    TryFromPrimitive, PartialEq, Eq, Clone, Copy, Debug, AnchorSerialize, AnchorDeserialize,
)]
#[repr(u16)]
pub enum UpdateOrderMode {
    UpdatePermissionless = 0,
    UpdateCounterparty = 1,
}
