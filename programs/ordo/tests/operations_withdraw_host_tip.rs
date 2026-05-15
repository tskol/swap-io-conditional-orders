use anchor_lang::prelude::Pubkey;
use ordo::{operations::withdraw_host_tip, state::GlobalConfig};

fn global_config_with_tips(total_tip_amount: u64, host_tip_amount: u64) -> GlobalConfig {
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
        total_tip_amount,
        host_tip_amount,
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

fn assert_withdraw_host_tip_rejected(
    global_config: &mut GlobalConfig,
    pda_authority_balance: u64,
    expected_total_tip_amount: u64,
    expected_host_tip_amount: u64,
) {
    assert!(withdraw_host_tip(global_config, pda_authority_balance).is_err());
    assert_eq!(global_config.total_tip_amount, expected_total_tip_amount);
    assert_eq!(global_config.host_tip_amount, expected_host_tip_amount);
}

#[test]
fn withdraw_host_tip_returns_host_amount_and_resets_accounting() {
    let mut global_config = global_config_with_tips(100, 20);

    let withdrawn_amount = withdraw_host_tip(&mut global_config, 100).unwrap();

    assert_eq!(withdrawn_amount, 20);
    assert_eq!(global_config.total_tip_amount, 80);
    assert_eq!(global_config.host_tip_amount, 0);
}

#[test]
fn withdraw_host_tip_rejects_when_host_tip_exceeds_total_tip_accounting() {
    let mut global_config = global_config_with_tips(5, 10);

    assert_withdraw_host_tip_rejected(&mut global_config, 10, 5, 10);
}

#[test]
fn withdraw_host_tip_rejects_when_pda_balance_is_below_host_tip() {
    let mut global_config = global_config_with_tips(20, 10);

    assert_withdraw_host_tip_rejected(&mut global_config, 9, 20, 10);
}
