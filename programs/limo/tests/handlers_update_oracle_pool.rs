use anchor_lang::{
    prelude::{AccountInfo, AccountLoader, Context, InterfaceAccount, Pubkey, Signer},
    Discriminator,
};
use anchor_spl::token_interface::{spl_token_2022, Mint};
use bytemuck::{bytes_of, Pod, Zeroable};
use limo::{
    handlers::update_oracle_pool::{
        handler_update_oracle_pool, UpdateOraclePool, UpdateOraclePoolBumps,
    },
    state::{GlobalConfig, OraclePoolsState},
};
use solana_program::{program_option::COption, program_pack::Pack};

const TEST_FEED_ID: &str = "0202020202020202020202020202020202020202020202020202020202020202";

fn zero_copy_account_data<T: Discriminator + Pod>(account: &T) -> Vec<u8> {
    let mut data = vec![0; 8 + std::mem::size_of::<T>()];
    data[..8].copy_from_slice(&T::discriminator());
    data[8..].copy_from_slice(bytes_of(account));
    data
}

fn mint_account_data() -> Vec<u8> {
    let mut data = vec![0; spl_token_2022::state::Mint::LEN];
    let mint = spl_token_2022::state::Mint {
        mint_authority: COption::None,
        supply: 0,
        decimals: 6,
        is_initialized: true,
        freeze_authority: COption::None,
    };
    spl_token_2022::state::Mint::pack(mint, &mut data).unwrap();
    data
}

#[test]
fn update_oracle_pool_handler_updates_feed_id() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let admin_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let oracle_pool_key = Pubkey::new_unique();
    let token_mint_key = Pubkey::new_unique();

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

    let mut global_config = GlobalConfig::zeroed();
    global_config.admin_authority = admin_key;
    let mut global_config_lamports = 0;
    let mut global_config_data = zero_copy_account_data(&global_config);
    let global_config_info = AccountInfo::new(
        &global_config_key,
        false,
        false,
        &mut global_config_lamports,
        &mut global_config_data,
        &owner,
        false,
        0,
    );

    let mut oracle_pool = OraclePoolsState {
        global_config: global_config_key,
        oracle_feed_id: [1; 32],
        token_mint: token_mint_key,
    };
    let mut oracle_pool_lamports = 0;
    let mut oracle_pool_data = zero_copy_account_data(&oracle_pool);
    let oracle_pool_info = AccountInfo::new(
        &oracle_pool_key,
        false,
        true,
        &mut oracle_pool_lamports,
        &mut oracle_pool_data,
        &owner,
        false,
        0,
    );

    let token_program = spl_token_2022::ID;
    let mut token_mint_lamports = 1;
    let mut token_mint_data = mint_account_data();
    let token_mint_info = AccountInfo::new(
        &token_mint_key,
        false,
        false,
        &mut token_mint_lamports,
        &mut token_mint_data,
        &token_program,
        false,
        0,
    );

    let mut accounts = UpdateOraclePool {
        admin_authority: Signer::try_from(&admin_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        oracle_pool: AccountLoader::try_from(&oracle_pool_info).unwrap(),
        token_mint: Box::new(InterfaceAccount::<Mint>::try_from(&token_mint_info).unwrap()),
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], UpdateOraclePoolBumps {
        oracle_pool: 254,
    });

    handler_update_oracle_pool(ctx, TEST_FEED_ID.to_string()).unwrap();

    oracle_pool = *AccountLoader::<OraclePoolsState>::try_from(&oracle_pool_info)
        .unwrap()
        .load()
        .unwrap();
    assert_eq!(oracle_pool.global_config, global_config_key);
    assert_eq!(oracle_pool.token_mint, token_mint_key);
    assert_eq!(oracle_pool.oracle_feed_id, [2; 32]);
}
