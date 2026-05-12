mod common;

use anchor_lang::prelude::{AccountLoader, Context, Pubkey, Signer};
use bytemuck::Zeroable;
use common::{install_noop_syscall_stubs, zero_copy_account_data, TestAccount};
use limo::{
    handlers::update_global_config::{UpdateGlobalConfig, UpdateGlobalConfigBumps},
    limo as program,
    state::{GlobalConfig, UpdateGlobalConfigMode, UpdateGlobalConfigValue},
};

#[test]
fn update_global_config_handler_updates_txn_fee_cost() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let admin_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();

    let mut global_config = GlobalConfig::zeroed();
    global_config.admin_authority = admin_key;

    let mut admin = TestAccount::new(admin_key, owner).signer().writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();

    let admin_info = admin.info();
    let global_config_info = global_config.info();

    let mut accounts = UpdateGlobalConfig {
        admin_authority: Signer::try_from(&admin_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], UpdateGlobalConfigBumps {});
    let value = UpdateGlobalConfigValue::U64(333).to_raw_bytes_array();

    program::update_global_config(ctx, UpdateGlobalConfigMode::UpdateTxnFeeCost as u16, value)
        .unwrap();

    let global_config = AccountLoader::<GlobalConfig>::try_from(&global_config_info).unwrap();
    assert_eq!(global_config.load().unwrap().txn_fee_cost, 333);
}
