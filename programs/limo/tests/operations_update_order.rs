use anchor_lang::prelude::Pubkey;
use limo::{
    operations::update_order,
    state::{Order, UpdateOrderMode},
};

#[test]
fn update_order_sets_permissionless_flag() {
    let mut order = Order::default();

    update_order(&mut order, UpdateOrderMode::UpdatePermissionless, &[1]).unwrap();

    assert_eq!(order.permissionless, 0);
}
