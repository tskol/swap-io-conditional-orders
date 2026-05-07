use anchor_lang::prelude::Pubkey;
use limo::{
    operations::{create_order, take_order, take_order_calcs},
    state::{GlobalConfig, Order, OrderStatus, OrderType},
};

fn empty_global_config() -> GlobalConfig {
    GlobalConfig {
        emergency_mode: 0,
        flash_take_order_blocked: 0,
        new_orders_blocked: 0,
        orders_taking_blocked: 0,
        tp_sl_enabled: 0,
        padding3: [0; 1],
        host_fee_bps: 0,
        create_order_fee_bps: 0,
        sl_max_upward_deviation_bps: 0,
        tp_sl_min_distance_bps: 0,
        parent_fill_fee_keeper_bps: 0,
        parent_fill_fee_protocol_bps: 0,
        tp_sl_child_fee_keeper_bps: 0,
        tp_sl_child_fee_protocol_bps: 0,
        keeper_take_fee_bps: 0,
        order_close_delay_seconds: 0,
        keeper_close_fee_bps: 0,
        padding0: [0; 3],
        padding1: [0; 8],
        pda_authority_previous_lamports_balance: 0,
        total_tip_amount: 0,
        host_tip_amount: 0,
        pda_authority: Pubkey::default(),
        pda_authority_bump: 0,
        admin_authority: Pubkey::default(),
        admin_authority_cached: Pubkey::default(),
        allowed_taker: Pubkey::default(),
        txn_fee_cost: 0,
        ata_creation_cost: 0,
        oracle_max_staleness_seconds: 0,
        padding2: [0; 241],
    }
}

fn global_config_with_keeper_fee(keeper_take_fee_bps: u16) -> GlobalConfig {
    let mut global_config = empty_global_config();
    global_config.keeper_take_fee_bps = keeper_take_fee_bps;
    global_config
}

fn active_order_of_type(order_type: OrderType) -> Order {
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
        order_type as u8,
        254,
        100,
        0,
    )
    .unwrap();

    order
}

fn active_vanilla_order() -> Order {
    active_order_of_type(OrderType::Vanilla)
}

fn assert_take_order_rejected(order: &Order, input_amount: u64, output_amount: u64) {
    let global_config = empty_global_config();

    assert!(take_order_calcs(order, &global_config, input_amount, output_amount).is_err());
    assert_eq!(order.status, OrderStatus::Active as u8);
}

#[test]
fn take_order_calcs_partial_vanilla_fill_with_keeper_fee() {
    let order = active_vanilla_order();
    let global_config = global_config_with_keeper_fee(100);

    let effects = take_order_calcs(&order, &global_config, 500, 1_000).unwrap();

    assert_eq!(effects.input_to_send_to_taker, 500);
    assert_eq!(effects.output_to_send_to_maker, 1_000);
    assert_eq!(effects.output_to_send_to_protocol, 0);
    assert_eq!(effects.output_keeper_fee, 10);
    assert_eq!(order.status, OrderStatus::Active as u8);
}

#[test]
fn take_order_calcs_limit_parent_splits_fee_pot() {
    let order = active_order_of_type(OrderType::LimitParent);
    let mut global_config = empty_global_config();
    global_config.parent_fill_fee_protocol_bps = 1_000;
    global_config.parent_fill_fee_keeper_bps = 500;

    let effects = take_order_calcs(&order, &global_config, 500, 1_100).unwrap();

    assert_eq!(effects.input_to_send_to_taker, 500);
    assert_eq!(effects.output_to_send_to_maker, 1_085);
    assert_eq!(effects.output_to_send_to_protocol, 10);
    assert_eq!(effects.output_keeper_fee, 0);
}

#[test]
fn take_order_calcs_tp_child_applies_keeper_and_fee_pot_fees() {
    let order = active_order_of_type(OrderType::LimitTP);
    let mut global_config = global_config_with_keeper_fee(100);
    global_config.tp_sl_child_fee_protocol_bps = 1_000;
    global_config.tp_sl_child_fee_keeper_bps = 500;

    let effects = take_order_calcs(&order, &global_config, 500, 1_100).unwrap();

    assert_eq!(effects.input_to_send_to_taker, 500);
    assert_eq!(effects.output_to_send_to_maker, 1_085);
    assert_eq!(effects.output_to_send_to_protocol, 10);
    assert_eq!(effects.output_keeper_fee, 11);
}

#[test]
fn take_order_updates_vanilla_order_accounting() {
    let mut order = active_vanilla_order();
    let mut global_config = global_config_with_keeper_fee(100);

    let effects = take_order(
        &mut global_config,
        &mut order,
        None,
        None,
        None,
        None,
        None,
        None,
        6,
        6,
        500,
        4,
        101,
        1_000,
    )
    .unwrap();

    assert_eq!(effects.input_to_send_to_taker, 500);
    assert_eq!(effects.output_to_send_to_maker, 1_000);
    assert_eq!(effects.output_keeper_fee, 10);
    assert_eq!(order.remaining_input_amount, 500);
    assert_eq!(order.filled_output_amount, 1_000);
    assert_eq!(order.tip_amount, 4);
    assert_eq!(order.number_of_fills, 1);
    assert_eq!(order.last_updated_timestamp, 101);
    assert_eq!(global_config.total_tip_amount, 4);
}

#[test]
fn take_order_rejects_order_locked_by_flash_operation() {
    let mut order = active_vanilla_order();
    order.flash_ix_lock = 1;
    let mut global_config = empty_global_config();

    assert!(take_order(
        &mut global_config,
        &mut order,
        None,
        None,
        None,
        None,
        None,
        None,
        6,
        6,
        500,
        0,
        101,
        1_000,
    )
    .is_err());

    assert_eq!(order.remaining_input_amount, 1_000);
    assert_eq!(order.filled_output_amount, 0);
    assert_eq!(order.number_of_fills, 0);
}

#[test]
fn take_order_calcs_rejects_zero_or_oversized_input() {
    let order = active_vanilla_order();

    assert_take_order_rejected(&order, 0, 0);
    assert_take_order_rejected(&order, 1_001, 2_002);
}

#[test]
fn take_order_calcs_rejects_order_with_zero_initial_input_amount() {
    let mut order = active_vanilla_order();
    order.initial_input_amount = 0;
    order.remaining_input_amount = 1;

    assert_take_order_rejected(&order, 1, 1);
}

#[test]
fn take_order_calcs_rejects_output_below_minimum_without_fee_pot() {
    let order = active_vanilla_order();

    assert_take_order_rejected(&order, 500, 999);
}

#[test]
fn take_order_calcs_rejects_fee_pot_fees_that_exceed_output_amount() {
    let order = active_order_of_type(OrderType::LimitParent);
    let mut global_config = empty_global_config();
    global_config.parent_fill_fee_protocol_bps = 10_000;
    global_config.parent_fill_fee_keeper_bps = 10_000;

    assert!(take_order_calcs(&order, &global_config, 1, 5).is_err());
    assert_eq!(order.status, OrderStatus::Active as u8);
}
