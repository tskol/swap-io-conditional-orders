mod common;

use anchor_lang::{
    prelude::{AccountInfo, AccountLoader, Context, Program, Pubkey, Signer, System},
    Discriminator,
};
use bytemuck::{bytes_of, Pod, Zeroable};
use common::install_noop_syscall_stubs;
use limo::{
    handlers::withdraw_host_tip::{WithdrawHostTip, WithdrawHostTipBumps},
    limo as program,
    state::GlobalConfig,
};

fn zero_copy_account_data<T: Discriminator + Pod>(account: &T) -> Vec<u8> {
    let mut data = vec![0; 8 + std::mem::size_of::<T>()];
    data[..8].copy_from_slice(&T::discriminator());
    data[8..].copy_from_slice(bytes_of(account));
    data
}

#[test]
fn withdraw_host_tip_handler_updates_previous_balance_when_no_tip_is_due() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let admin_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let system_program_key = anchor_lang::system_program::ID;

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

    let mut pda_lamports = 42;
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

    let mut system_lamports = 0;
    let mut system_data = [];
    let system_info = AccountInfo::new(
        &system_program_key,
        false,
        false,
        &mut system_lamports,
        &mut system_data,
        &owner,
        true,
        0,
    );

    let mut global_config = GlobalConfig::zeroed();
    global_config.admin_authority = admin_key;
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;
    global_config.total_tip_amount = 7;
    global_config.pda_authority_previous_lamports_balance = 10;

    let mut global_config_lamports = 0;
    let mut global_config_data = zero_copy_account_data(&global_config);
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

    let mut accounts = WithdrawHostTip {
        admin_authority: Signer::try_from(&admin_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_info,
        system_program: Program::<System>::try_from(&system_info).unwrap(),
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], WithdrawHostTipBumps {});

    program::withdraw_host_tip(ctx).unwrap();

    let global_config = AccountLoader::<GlobalConfig>::try_from(&global_config_info).unwrap();
    let global_config = global_config.load().unwrap();
    assert_eq!(global_config.host_tip_amount, 0);
    assert_eq!(global_config.total_tip_amount, 7);
    assert_eq!(global_config.pda_authority_previous_lamports_balance, 42);
}

#[test]
fn withdraw_host_tip_handler_transfers_host_tip_and_updates_accounting() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let admin_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let system_program_key = anchor_lang::system_program::ID;

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

    let mut pda_lamports = 42;
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

    let mut system_lamports = 0;
    let mut system_data = [];
    let system_info = AccountInfo::new(
        &system_program_key,
        false,
        false,
        &mut system_lamports,
        &mut system_data,
        &owner,
        true,
        0,
    );

    let mut global_config = GlobalConfig::zeroed();
    global_config.admin_authority = admin_key;
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;
    global_config.total_tip_amount = 10;
    global_config.host_tip_amount = 7;
    global_config.pda_authority_previous_lamports_balance = 42;

    let mut global_config_lamports = 0;
    let mut global_config_data = zero_copy_account_data(&global_config);
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

    let mut accounts = WithdrawHostTip {
        admin_authority: Signer::try_from(&admin_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_info,
        system_program: Program::<System>::try_from(&system_info).unwrap(),
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], WithdrawHostTipBumps {});

    program::withdraw_host_tip(ctx).unwrap();

    let global_config = AccountLoader::<GlobalConfig>::try_from(&global_config_info).unwrap();
    let global_config = global_config.load().unwrap();
    assert_eq!(global_config.host_tip_amount, 0);
    assert_eq!(global_config.total_tip_amount, 3);
    assert_eq!(global_config.pda_authority_previous_lamports_balance, 42);
}
