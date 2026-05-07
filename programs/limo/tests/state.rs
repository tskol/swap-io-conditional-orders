use anchor_lang::prelude::Pubkey;
use limo::state::{
    OrderStatus, OrderType, UpdateGlobalConfigMode, UpdateGlobalConfigValue, UpdateOrderMode,
};

#[test]
fn order_status_converts_known_values() {
    assert_eq!(u8::from(OrderStatus::Active), 0);
    assert_eq!(u8::from(OrderStatus::Filled), 1);
    assert_eq!(u8::from(OrderStatus::Cancelled), 2);

    assert_eq!(OrderStatus::from(0), OrderStatus::Active);
    assert_eq!(OrderStatus::from(1), OrderStatus::Filled);
    assert_eq!(OrderStatus::from(2), OrderStatus::Cancelled);
}

#[test]
#[should_panic(expected = "Invalid OrderStatus")]
fn order_status_panics_on_invalid_value() {
    let _ = OrderStatus::from(9);
}

#[test]
fn order_type_try_from_accepts_known_values_and_rejects_invalid_value() {
    assert_eq!(u8::from(OrderType::Vanilla), 0);
    assert_eq!(u8::from(OrderType::LimitParent), 1);
    assert_eq!(u8::from(OrderType::LimitTP), 2);
    assert_eq!(u8::from(OrderType::LimitSL), 3);

    assert_eq!(OrderType::try_from(0).unwrap(), OrderType::Vanilla);
    assert_eq!(OrderType::try_from(1).unwrap(), OrderType::LimitParent);
    assert_eq!(OrderType::try_from(2).unwrap(), OrderType::LimitTP);
    assert_eq!(OrderType::try_from(3).unwrap(), OrderType::LimitSL);
    assert!(OrderType::try_from(4).is_err());
}

#[test]
fn update_global_config_modes_decode_known_values() {
    assert_eq!(
        UpdateGlobalConfigMode::try_from(0).unwrap(),
        UpdateGlobalConfigMode::UpdateEmergencyMode,
    );
    assert_eq!(
        UpdateGlobalConfigMode::try_from(20).unwrap(),
        UpdateGlobalConfigMode::UpdateKeeperCloseFeeBps,
    );
    assert!(UpdateGlobalConfigMode::try_from(21).is_err());
}

#[test]
fn update_order_modes_decode_known_values() {
    assert_eq!(
        UpdateOrderMode::try_from(0).unwrap(),
        UpdateOrderMode::UpdatePermissionless,
    );
    assert_eq!(
        UpdateOrderMode::try_from(1).unwrap(),
        UpdateOrderMode::UpdateCounterparty,
    );
    assert!(UpdateOrderMode::try_from(2).is_err());
}

#[test]
fn update_global_config_value_serializes_to_raw_bytes() {
    let pubkey = Pubkey::new_unique();

    assert_eq!(
        UpdateGlobalConfigValue::Bool(true).to_raw_bytes_array()[0],
        1
    );
    assert_eq!(
        &UpdateGlobalConfigValue::U16(0x1234).to_raw_bytes_array()[..2],
        &0x1234u16.to_le_bytes(),
    );
    assert_eq!(
        &UpdateGlobalConfigValue::U64(0x1234_5678).to_raw_bytes_array()[..8],
        &0x1234_5678u64.to_le_bytes(),
    );
    assert_eq!(
        &UpdateGlobalConfigValue::Pubkey(pubkey).to_raw_bytes_array()[..32],
        pubkey.as_ref(),
    );
}
