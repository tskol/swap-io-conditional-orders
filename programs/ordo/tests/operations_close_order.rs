use anchor_lang::prelude::Pubkey;
use ordo::{
    operations::{close_order_and_claim_tip, create_order},
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

fn global_config_with_tip_and_delay(
    total_tip_amount: u64,
    close_delay_seconds: u64,
) -> GlobalConfig {
    let mut global_config = empty_global_config();
    global_config.total_tip_amount = total_tip_amount;
    global_config.order_close_delay_seconds = close_delay_seconds;
    global_config
}

fn active_order_with_tip(tip_amount: u64) -> Order {
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
    order.tip_amount = tip_amount;

    order
}

fn active_order_with_tip_and_last_updated(tip_amount: u64, last_updated_timestamp: u64) -> Order {
    let mut order = active_order_with_tip(tip_amount);
    order.last_updated_timestamp = last_updated_timestamp;
    order
}

fn assert_close_order_rejected(
    order: &mut Order,
    global_config: &mut GlobalConfig,
    current_timestamp: u64,
    expected_total_tip_amount: u64,
) {
    assert!(close_order_and_claim_tip(order, global_config, current_timestamp).is_err());
    assert_eq!(order.status, OrderStatus::Active as u8);
    assert_eq!(global_config.total_tip_amount, expected_total_tip_amount);
}

#[test]
fn close_order_cancels_active_order_and_claims_tip() {
    let mut order = active_order_with_tip(10);
    let mut global_config = global_config_with_tip_and_delay(50, 5);

    close_order_and_claim_tip(&mut order, &mut global_config, 105).unwrap();

    assert_eq!(order.status, OrderStatus::Cancelled as u8);
    assert_eq!(global_config.total_tip_amount, 40);
}

#[test]
fn close_order_rejects_before_close_delay_passes() {
    let mut order = active_order_with_tip(10);
    let mut global_config = global_config_with_tip_and_delay(50, 6);

    assert_close_order_rejected(&mut order, &mut global_config, 105, 50);
}

#[test]
fn close_order_rejects_when_order_tip_exceeds_total_tip_accounting() {
    let mut order = active_order_with_tip(10);
    let mut global_config = global_config_with_tip_and_delay(5, 5);

    assert_close_order_rejected(&mut order, &mut global_config, 105, 5);
}

#[test]
fn close_order_rejects_when_close_delay_timestamp_overflows() {
    let mut order = active_order_with_tip_and_last_updated(0, u64::MAX);
    let mut global_config = global_config_with_tip_and_delay(0, 1);

    assert_close_order_rejected(&mut order, &mut global_config, u64::MAX, 0);
}

#[test]
fn close_order_rejects_cancelled_order() {
    let mut order = active_order_with_tip(0);
    order.status = OrderStatus::Cancelled as u8;
    let mut global_config = global_config_with_tip_and_delay(0, 0);

    assert!(close_order_and_claim_tip(&mut order, &mut global_config, 100).is_err());
    assert_eq!(order.status, OrderStatus::Cancelled as u8);
    assert_eq!(global_config.total_tip_amount, 0);
}

#[test]
fn close_order_rejects_flash_locked_order() {
    let mut order = active_order_with_tip(0);
    order.flash_ix_lock = 1;
    let mut global_config = global_config_with_tip_and_delay(0, 0);

    assert_close_order_rejected(&mut order, &mut global_config, 100, 0);
}
