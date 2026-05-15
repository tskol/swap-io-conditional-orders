use anchor_lang::prelude::Pubkey;
use ordo::{
    operations::update_global_config,
    state::{GlobalConfig, UpdateGlobalConfigMode, UpdateGlobalConfigValue},
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

fn update_config(
    global_config: &mut GlobalConfig,
    mode: UpdateGlobalConfigMode,
    value: UpdateGlobalConfigValue,
) {
    update_global_config(global_config, mode, &value.to_raw_bytes_array(), 123).unwrap();
}

#[test]
fn update_global_config_sets_flags() {
    let mut global_config = empty_global_config();

    for mode in [
        UpdateGlobalConfigMode::UpdateEmergencyMode,
        UpdateGlobalConfigMode::UpdateFlashExecuteOrderBlocked,
        UpdateGlobalConfigMode::UpdateBlockNewOrders,
        UpdateGlobalConfigMode::UpdateBlockOrderTaking,
        UpdateGlobalConfigMode::UpdateTpSlEnabled,
    ] {
        update_config(
            &mut global_config,
            mode,
            UpdateGlobalConfigValue::Bool(true),
        );
    }

    assert_eq!(global_config.emergency_mode, 1);
    assert_eq!(global_config.flash_take_order_blocked, 1);
    assert_eq!(global_config.new_orders_blocked, 1);
    assert_eq!(global_config.orders_taking_blocked, 1);
    assert_eq!(global_config.tp_sl_enabled, 1);
}

#[test]
fn update_global_config_sets_bps_fields() {
    let mut global_config = empty_global_config();

    for (mode, value) in [
        (UpdateGlobalConfigMode::UpdateHostFeeBps, 11),
        (UpdateGlobalConfigMode::UpdateSubmitOrderFeeBps, 12),
        (UpdateGlobalConfigMode::UpdateParentFillFeeKeeperBps, 13),
        (UpdateGlobalConfigMode::UpdateParentFillFeeProtocolBps, 14),
        (UpdateGlobalConfigMode::UpdateTpSlChildFeeKeeperBps, 15),
        (UpdateGlobalConfigMode::UpdateTpSlChildFeeProtocolBps, 16),
        (UpdateGlobalConfigMode::UpdateSlMaxUpwardDeviationBps, 17),
        (UpdateGlobalConfigMode::UpdateTpSlMinDistanceBps, 18),
        (UpdateGlobalConfigMode::UpdateKeeperTakeFeeBps, 19),
        (UpdateGlobalConfigMode::UpdateKeeperCloseFeeBps, 20),
    ] {
        update_config(
            &mut global_config,
            mode,
            UpdateGlobalConfigValue::U16(value),
        );
    }

    assert_eq!(global_config.host_fee_bps, 11);
    assert_eq!(global_config.create_order_fee_bps, 12);
    assert_eq!(global_config.parent_fill_fee_keeper_bps, 13);
    assert_eq!(global_config.parent_fill_fee_protocol_bps, 14);
    assert_eq!(global_config.tp_sl_child_fee_keeper_bps, 15);
    assert_eq!(global_config.tp_sl_child_fee_protocol_bps, 16);
    assert_eq!(global_config.sl_max_upward_deviation_bps, 17);
    assert_eq!(global_config.tp_sl_min_distance_bps, 18);
    assert_eq!(global_config.keeper_take_fee_bps, 19);
    assert_eq!(global_config.keeper_close_fee_bps, 20);
}

#[test]
fn update_global_config_sets_u64_and_pubkey_fields() {
    let mut global_config = empty_global_config();
    let admin_authority_cached = Pubkey::new_unique();
    let allowed_taker = Pubkey::new_unique();

    for (mode, value) in [
        (UpdateGlobalConfigMode::UpdateOrderCloseDelaySeconds, 21),
        (UpdateGlobalConfigMode::UpdateTxnFeeCost, 22),
        (UpdateGlobalConfigMode::UpdateAtaCreationCost, 23),
        (UpdateGlobalConfigMode::UpdateOracleMaxStalenessSeconds, 24),
    ] {
        update_config(
            &mut global_config,
            mode,
            UpdateGlobalConfigValue::U64(value),
        );
    }
    update_config(
        &mut global_config,
        UpdateGlobalConfigMode::UpdateAdminAuthorityCached,
        UpdateGlobalConfigValue::Pubkey(admin_authority_cached),
    );
    update_config(
        &mut global_config,
        UpdateGlobalConfigMode::UpdateCounterparty,
        UpdateGlobalConfigValue::Pubkey(allowed_taker),
    );

    assert_eq!(global_config.order_close_delay_seconds, 21);
    assert_eq!(global_config.txn_fee_cost, 22);
    assert_eq!(global_config.ata_creation_cost, 23);
    assert_eq!(global_config.oracle_max_staleness_seconds, 24);
    assert_eq!(global_config.admin_authority_cached, admin_authority_cached);
    assert_eq!(global_config.allowed_taker, allowed_taker);
}

#[test]
fn update_global_config_rejects_invalid_flag_and_bps() {
    let mut global_config = empty_global_config();

    assert!(update_global_config(
        &mut global_config,
        UpdateGlobalConfigMode::UpdateEmergencyMode,
        &UpdateGlobalConfigValue::U16(2).to_raw_bytes_array(),
        123,
    )
    .is_err());
    assert!(update_global_config(
        &mut global_config,
        UpdateGlobalConfigMode::UpdateHostFeeBps,
        &UpdateGlobalConfigValue::U16(10_001).to_raw_bytes_array(),
        123,
    )
    .is_err());

    assert_eq!(global_config.emergency_mode, 0);
    assert_eq!(global_config.host_fee_bps, 0);
}
