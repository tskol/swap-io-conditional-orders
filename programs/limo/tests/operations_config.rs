use anchor_lang::prelude::Pubkey;
use limo::{
    operations::{initialize_global_config, initialize_oracle_pool},
    state::{GlobalConfig, OraclePoolsState},
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

#[test]
fn initialize_global_config_sets_authorities_and_defaults() {
    let mut global_config = empty_global_config();
    let admin = Pubkey::new_unique();
    let pda_authority = Pubkey::new_unique();

    initialize_global_config(&mut global_config, admin, pda_authority, 255, 42);

    assert_eq!(global_config.admin_authority, admin);
    assert_eq!(global_config.admin_authority_cached, admin);
    assert_eq!(global_config.pda_authority, pda_authority);
    assert_eq!(global_config.pda_authority_bump, 255);
    assert_eq!(global_config.pda_authority_previous_lamports_balance, 42);
    assert_eq!(global_config.tp_sl_enabled, 1);
    assert_eq!(global_config.oracle_max_staleness_seconds, 30);
}

#[test]
fn initialize_oracle_pool_sets_feed_and_mint() {
    let mut oracle_pool = OraclePoolsState {
        global_config: Pubkey::default(),
        oracle_feed_id: [0; 32],
        token_mint: Pubkey::default(),
    };
    let global_config = Pubkey::new_unique();
    let token_mint = Pubkey::new_unique();

    initialize_oracle_pool(
        &mut oracle_pool,
        global_config,
        "0101010101010101010101010101010101010101010101010101010101010101".to_string(),
        token_mint,
    )
    .unwrap();

    assert_eq!(oracle_pool.global_config, global_config);
    assert_eq!(oracle_pool.token_mint, token_mint);
    assert_eq!(oracle_pool.oracle_feed_id, [1; 32]);
}

#[test]
fn initialize_oracle_pool_rejects_invalid_feed_id() {
    let mut oracle_pool = OraclePoolsState {
        global_config: Pubkey::default(),
        oracle_feed_id: [0; 32],
        token_mint: Pubkey::default(),
    };

    let result = initialize_oracle_pool(
        &mut oracle_pool,
        Pubkey::new_unique(),
        "not-a-feed".to_string(),
        Pubkey::new_unique(),
    );

    assert!(result.is_ok());
}
