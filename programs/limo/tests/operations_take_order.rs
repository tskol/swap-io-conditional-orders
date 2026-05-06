use anchor_lang::prelude::Pubkey;
use limo::{
    operations::{create_order, take_order_calcs},
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

fn active_vanilla_order() -> Order {
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

#[test]
fn take_order_calcs_partial_vanilla_fill_with_keeper_fee() {
    let order = active_vanilla_order();
    let mut global_config = empty_global_config();
    global_config.keeper_take_fee_bps = 100;

    let effects = take_order_calcs(&order, &global_config, 500, 1_000).unwrap();

    assert_eq!(effects.input_to_send_to_taker, 500);
    assert_eq!(effects.output_to_send_to_maker, 1_000);
    assert_eq!(effects.output_to_send_to_protocol, 0);
    assert_eq!(effects.output_keeper_fee, 0);
    assert_eq!(order.status, OrderStatus::Active as u8);
}
