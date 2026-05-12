mod common;

use anchor_lang::prelude::{AccountInfo, AccountLoader, Context, Pubkey, Signer};
use bytemuck::from_bytes;
use common::zeroed_zero_copy_account_data;
use limo::{
    handlers::initialize_global_config::{InitializeGlobalConfig, InitializeGlobalConfigBumps},
    limo as program,
    state::GlobalConfig,
};

#[test]
fn initialize_global_config_handler_sets_authorities_and_defaults() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let admin_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();

    let mut admin_lamports = 0;
    let mut admin_data = [];
    let admin_info = AccountInfo::new(
        &admin_key,
        true,
        true,
        &mut admin_lamports,
        &mut admin_data,
        &owner,
        false,
        0,
    );

    let mut pda_lamports = 123;
    let mut pda_data = [];
    let pda_info = AccountInfo::new(
        &pda_authority_key,
        false,
        true,
        &mut pda_lamports,
        &mut pda_data,
        &owner,
        false,
        0,
    );

    let mut global_config_lamports = 0;
    let mut global_config_data = zeroed_zero_copy_account_data::<GlobalConfig>();
    let global_config_info = AccountInfo::new(
        &global_config_key,
        false,
        true,
        &mut global_config_lamports,
        &mut global_config_data,
        &owner,
        false,
        0,
    );

    let mut accounts = InitializeGlobalConfig {
        admin_authority: Signer::try_from(&admin_info).unwrap(),
        pda_authority: pda_info,
        global_config: AccountLoader::try_from_unchecked(&program_id, &global_config_info).unwrap(),
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        InitializeGlobalConfigBumps { pda_authority: 201 },
    );

    program::initialize_global_config(ctx).unwrap();

    let global_config =
        from_bytes::<GlobalConfig>(&global_config_data[8..8 + std::mem::size_of::<GlobalConfig>()]);
    assert_eq!(global_config.admin_authority, admin_key);
    assert_eq!(global_config.admin_authority_cached, admin_key);
    assert_eq!(global_config.pda_authority, pda_authority_key);
    assert_eq!(global_config.pda_authority_bump, 201);
    assert_eq!(global_config.pda_authority_previous_lamports_balance, 123);
    assert_eq!(global_config.tp_sl_enabled, 1);
    assert_eq!(global_config.oracle_max_staleness_seconds, 30);
}
