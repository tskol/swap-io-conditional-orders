mod common;

use anchor_lang::prelude::{AccountLoader, Context, Pubkey, Signer};
use bytemuck::Zeroable;
use common::{zero_copy_account_data, TestAccount};
use ordo::{
    handlers::update_global_config_admin::{UpdateGlobalConfigAdmin, UpdateGlobalConfigAdminBumps},
    ordo as program,
    state::GlobalConfig,
};

#[test]
fn update_global_config_admin_handler_promotes_cached_admin() {
    let program_id = ordo::ID;
    let owner = ordo::ID;
    let old_admin_key = Pubkey::new_unique();
    let new_admin_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();

    let mut global_config = GlobalConfig::zeroed();
    global_config.admin_authority = old_admin_key;
    global_config.admin_authority_cached = new_admin_key;

    let mut new_admin = TestAccount::new(new_admin_key, owner).signer();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();

    let new_admin_info = new_admin.info();
    let global_config_info = global_config.info();

    let mut accounts = UpdateGlobalConfigAdmin {
        admin_authority_cached: Signer::try_from(&new_admin_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        UpdateGlobalConfigAdminBumps {},
    );

    program::update_global_config_admin(ctx).unwrap();

    let global_config = AccountLoader::<GlobalConfig>::try_from(&global_config_info).unwrap();
    assert_eq!(global_config.load().unwrap().admin_authority, new_admin_key);
}
