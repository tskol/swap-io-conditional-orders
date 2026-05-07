mod common;

use anchor_lang::prelude::{
    AccountLoader, Context, InterfaceAccount, Program, Pubkey, Signer, System,
};
use anchor_spl::token_interface::{spl_token_2022, Mint};
use bytemuck::{from_bytes, Zeroable};
use common::{
    executable_account, mint_account, zero_copy_account_data, zeroed_zero_copy_account_data,
    TestAccount,
};
use limo::{
    limo as program,
    handlers::initialize_oracle_pool::{
        InitializeOraclePool, InitializeOraclePoolBumps,
    },
    state::{GlobalConfig, OraclePoolsState},
};

const TEST_FEED_ID: &str = "0202020202020202020202020202020202020202020202020202020202020202";

#[test]
fn initialize_oracle_pool_handler_initializes_feed_and_mint() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let admin_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let token_mint_key = Pubkey::new_unique();
    let oracle_pool_key = Pubkey::new_unique();

    let mut global_config = GlobalConfig::zeroed();
    global_config.admin_authority = admin_key;

    let mut admin = TestAccount::new(admin_key, owner).signer().writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config));
    let mut token_mint = mint_account(token_mint_key, spl_token_2022::ID);
    let mut oracle_pool = TestAccount::new(oracle_pool_key, owner)
        .with_data(zeroed_zero_copy_account_data::<OraclePoolsState>())
        .writable();
    let mut system_program = executable_account(anchor_lang::system_program::ID, owner);

    let admin_info = admin.info();
    let global_config_info = global_config.info();
    let token_mint_info = token_mint.info();
    let oracle_pool_info = oracle_pool.info();
    let system_program_info = system_program.info();

    let mut accounts = InitializeOraclePool {
        admin_authority: Signer::try_from(&admin_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        token_mint: Box::new(InterfaceAccount::<Mint>::try_from(&token_mint_info).unwrap()),
        oracle_pool: AccountLoader::try_from_unchecked(&program_id, &oracle_pool_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], InitializeOraclePoolBumps {
        oracle_pool: 251,
    });

    program::initialize_oracle_pool(ctx, TEST_FEED_ID.to_string()).unwrap();

    let oracle_pool_data = oracle_pool_info.try_borrow_data().unwrap();
    let oracle_pool = from_bytes::<OraclePoolsState>(
        &oracle_pool_data[8..8 + std::mem::size_of::<OraclePoolsState>()],
    );
    assert_eq!(oracle_pool.global_config, global_config_key);
    assert_eq!(oracle_pool.token_mint, token_mint_key);
    assert_eq!(oracle_pool.oracle_feed_id, [2; 32]);
}
