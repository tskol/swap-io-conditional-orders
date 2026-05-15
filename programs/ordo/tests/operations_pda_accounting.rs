use anchor_lang::prelude::Pubkey;
use ordo::{operations::validate_pda_authority_balance_and_update_accounting, state::GlobalConfig};

fn global_config_with_accounting(
    pda_authority_previous_lamports_balance: u64,
    total_tip_amount: u64,
) -> GlobalConfig {
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
        pda_authority_previous_lamports_balance,
        total_tip_amount,
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

fn assert_pda_accounting_rejected(
    global_config: &mut GlobalConfig,
    pda_authority_balance: u64,
    tip: u64,
    expected_previous_lamports_balance: u64,
) {
    assert!(validate_pda_authority_balance_and_update_accounting(
        global_config,
        pda_authority_balance,
        tip
    )
    .is_err());
    assert_eq!(
        global_config.pda_authority_previous_lamports_balance,
        expected_previous_lamports_balance
    );
}

#[test]
fn pda_accounting_accepts_tip_transfer_and_updates_previous_balance() {
    let mut global_config = global_config_with_accounting(50, 70);

    validate_pda_authority_balance_and_update_accounting(&mut global_config, 75, 25).unwrap();

    assert_eq!(global_config.pda_authority_previous_lamports_balance, 75);
}

#[test]
fn pda_accounting_rejects_when_pda_balance_is_below_total_tip_accounting() {
    let mut global_config = global_config_with_accounting(10, 100);

    assert_pda_accounting_rejected(&mut global_config, 50, 5, 10);
}

#[test]
fn pda_accounting_rejects_when_pda_balance_drops_below_previous_balance() {
    let mut global_config = global_config_with_accounting(50, 40);

    assert_pda_accounting_rejected(&mut global_config, 49, 0, 50);
}
