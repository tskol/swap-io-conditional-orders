use anchor_lang::prelude::Pubkey;
use limo::{
    operations::update_order,
    state::{Order, UpdateOrderMode},
};

fn assert_update_order_rejected(mode: UpdateOrderMode, value: &[u8]) {
    let mut order = Order::default();

    assert!(update_order(&mut order, mode, value).is_err());
}

#[test]
fn update_order_sets_permissionless_flag() {
    let mut order = Order::default();

    update_order(&mut order, UpdateOrderMode::UpdatePermissionless, &[1]).unwrap();

    assert_eq!(order.permissionless, 1);
}

#[test]
fn update_order_sets_counterparty_pubkey() {
    let mut order = Order::default();
    let counterparty = Pubkey::new_unique();

    update_order(
        &mut order,
        UpdateOrderMode::UpdateCounterparty,
        counterparty.as_ref(),
    )
    .unwrap();

    assert_eq!(order.counterparty, counterparty);
}

#[test]
fn update_order_rejects_invalid_value_lengths() {
    assert_update_order_rejected(UpdateOrderMode::UpdatePermissionless, &[]);
    assert_update_order_rejected(UpdateOrderMode::UpdateCounterparty, &[7; 31]);
}

#[test]
fn update_order_rejects_invalid_permissionless_flag() {
    let mut order = Order::default();

    assert!(update_order(&mut order, UpdateOrderMode::UpdatePermissionless, &[2]).is_err());
    assert_eq!(order.permissionless, 0);
}
