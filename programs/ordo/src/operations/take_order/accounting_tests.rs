use anchor_lang::{prelude::*, Discriminator};
use bytemuck::bytes_of;

use crate::{
    operations::create_order,
    state::{GlobalConfig, Order, OrderStatus, OrderType},
};

use super::{update_take_child_order_accounting_and_tips, update_take_order_accounting_and_tips};

fn active_order() -> Order {
    let mut order = Order::default();
    create_order(
        &mut order,
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        1_000,
        2_000,
        Pubkey::default(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        OrderType::Vanilla as u8,
        254,
        100,
        0,
    )
    .unwrap();
    order
}

fn zero_copy_account_data<T: Discriminator + bytemuck::Pod>(account: &T) -> Vec<u8> {
    let mut data = vec![0; 8 + std::mem::size_of::<T>()];
    data[..8].copy_from_slice(&T::discriminator());
    data[8..].copy_from_slice(bytes_of(account));
    data
}

fn assert_active_order_unchanged(order: &Order, number_of_fills: u64) {
    assert_eq!(order.remaining_input_amount, 1_000);
    assert_eq!(order.filled_output_amount, 0);
    assert_eq!(order.number_of_fills, number_of_fills);
    assert_eq!(order.last_updated_timestamp, 100);
}

#[test]
fn take_order_accounting_rejects_negative_timestamp_without_mutating() {
    let mut global_config = GlobalConfig::default();
    let mut order = active_order();

    assert!(
        update_take_order_accounting_and_tips(&mut global_config, &mut order, 100, 200, 0, -1,)
            .is_err()
    );

    assert_active_order_unchanged(&order, 0);
}

#[test]
fn take_order_accounting_rejects_number_of_fills_overflow_without_mutating() {
    let mut global_config = GlobalConfig::default();
    let mut order = active_order();
    order.number_of_fills = u64::MAX;

    assert!(update_take_order_accounting_and_tips(
        &mut global_config,
        &mut order,
        100,
        200,
        0,
        101,
    )
    .is_err());

    assert_active_order_unchanged(&order, u64::MAX);
}

#[test]
fn take_order_accounting_updates_vanilla_order_and_tips() {
    let mut global_config = GlobalConfig {
        host_fee_bps: 2_500,
        ..GlobalConfig::default()
    };
    let mut order = active_order();

    update_take_order_accounting_and_tips(&mut global_config, &mut order, 250, 500, 8, 101)
        .unwrap();

    assert_eq!(order.remaining_input_amount, 750);
    assert_eq!(order.filled_output_amount, 500);
    assert_eq!(order.tip_amount, 6);
    assert_eq!(order.number_of_fills, 1);
    assert_eq!(order.status, OrderStatus::Active as u8);
    assert_eq!(order.last_updated_timestamp, 101);
    assert_eq!(global_config.host_tip_amount, 2);
    assert_eq!(global_config.total_tip_amount, 8);
}

#[test]
fn take_order_accounting_marks_parent_order_filled_and_tracks_child_input() {
    let mut global_config = GlobalConfig::default();
    let mut order = active_order();
    order.order_type = OrderType::LimitParent as u8;

    update_take_order_accounting_and_tips(&mut global_config, &mut order, 1_000, 2_000, 0, 101)
        .unwrap();

    assert_eq!(order.remaining_input_amount, 0);
    assert_eq!(order.filled_output_amount, 2_000);
    assert_eq!(order.available_child_input_amount, 2_000);
    assert_eq!(order.number_of_fills, 1);
    assert_eq!(order.status, OrderStatus::Filled as u8);
    assert_eq!(order.last_updated_timestamp, 101);
}

#[test]
fn take_child_order_accounting_rejects_input_above_parent_available() {
    let mut global_config = GlobalConfig::default();
    let mut parent_order = active_order();
    let mut child_order = active_order();
    child_order.order_type = OrderType::LimitTP as u8;
    parent_order.available_child_input_amount = 99;

    assert!(update_take_child_order_accounting_and_tips(
        &mut global_config,
        &mut child_order,
        &mut parent_order,
        None,
        None,
        None,
        None,
        None,
        6,
        6,
        100,
        200,
        0,
        101,
    )
    .is_err());

    assert_eq!(parent_order.available_child_input_amount, 99);
    assert_active_order_unchanged(&child_order, 0);
}

#[test]
fn take_child_order_accounting_updates_parent_child_and_tips() {
    let mut global_config = GlobalConfig {
        host_fee_bps: 5_000,
        ..GlobalConfig::default()
    };
    let mut parent_order = active_order();
    let mut child_order = active_order();
    child_order.order_type = OrderType::LimitTP as u8;
    parent_order.available_child_input_amount = 500;

    update_take_child_order_accounting_and_tips(
        &mut global_config,
        &mut child_order,
        &mut parent_order,
        None,
        None,
        None,
        None,
        None,
        6,
        6,
        250,
        600,
        9,
        101,
    )
    .unwrap();

    assert_eq!(parent_order.available_child_input_amount, 250);
    assert_eq!(child_order.remaining_input_amount, 750);
    assert_eq!(child_order.filled_output_amount, 600);
    assert_eq!(child_order.tip_amount, 4);
    assert_eq!(global_config.host_tip_amount, 5);
    assert_eq!(global_config.total_tip_amount, 9);
}

#[test]
fn take_child_order_accounting_marks_brother_filled_after_child_fill() {
    let mut global_config = GlobalConfig::default();
    let mut parent_order = active_order();
    let mut child_order = active_order();
    let brother_order = active_order();
    child_order.order_type = OrderType::LimitTP as u8;
    parent_order.status = OrderStatus::Filled as u8;
    parent_order.available_child_input_amount = 1_000;

    let brother_key = Pubkey::new_unique();
    parent_order.sl_child_order = brother_key;
    let owner = crate::ID;
    let mut lamports = 0;
    let mut data = zero_copy_account_data(&brother_order);
    let brother_info = AccountInfo::new(
        &brother_key,
        false,
        true,
        &mut lamports,
        &mut data,
        &owner,
        false,
        0,
    );
    let brother_loader = AccountLoader::<Order>::try_from(&brother_info).unwrap();

    update_take_child_order_accounting_and_tips(
        &mut global_config,
        &mut child_order,
        &mut parent_order,
        Some(&brother_loader),
        None,
        None,
        None,
        None,
        6,
        6,
        1_000,
        2_000,
        0,
        101,
    )
    .unwrap();

    assert_eq!(parent_order.available_child_input_amount, 0);
    assert_eq!(child_order.status, OrderStatus::Filled as u8);
    assert_eq!(
        brother_loader.load().unwrap().status,
        OrderStatus::Filled as u8
    );
}

#[test]
fn take_child_order_accounting_ignores_supplied_brother_when_parent_has_no_sibling_child() {
    let mut global_config = GlobalConfig::default();
    let mut parent_order = active_order();
    let mut child_order = active_order();
    let brother_order = active_order();
    child_order.order_type = OrderType::LimitTP as u8;
    parent_order.status = OrderStatus::Filled as u8;
    parent_order.available_child_input_amount = 1_000;
    parent_order.sl_child_order = Pubkey::default();

    let brother_key = Pubkey::new_unique();
    let owner = crate::ID;
    let mut lamports = 0;
    let mut data = zero_copy_account_data(&brother_order);
    let brother_info = AccountInfo::new(
        &brother_key,
        false,
        true,
        &mut lamports,
        &mut data,
        &owner,
        false,
        0,
    );
    let brother_loader = AccountLoader::<Order>::try_from(&brother_info).unwrap();

    update_take_child_order_accounting_and_tips(
        &mut global_config,
        &mut child_order,
        &mut parent_order,
        Some(&brother_loader),
        None,
        None,
        None,
        None,
        6,
        6,
        1_000,
        2_000,
        0,
        101,
    )
    .unwrap();

    let brother_after = brother_loader.load().unwrap();
    assert_eq!(brother_after.status, OrderStatus::Active as u8);
}
