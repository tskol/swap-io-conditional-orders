use anchor_lang::{
    prelude::{AccountInfo, AccountLoader, Context, Pubkey, Signer},
    Discriminator,
};
use bytemuck::{bytes_of, Pod, Zeroable};
use limo::{
    handlers::update_order::{handler_update_order, UpdateOrder, UpdateOrderBumps},
    operations::create_order,
    state::{GlobalConfig, Order, OrderType, UpdateOrderMode},
};

fn zero_copy_account_data<T: Discriminator + Pod>(account: &T) -> Vec<u8> {
    let mut data = vec![0; 8 + std::mem::size_of::<T>()];
    data[..8].copy_from_slice(&T::discriminator());
    data[8..].copy_from_slice(bytes_of(account));
    data
}

fn active_order(global_config: Pubkey, maker: Pubkey) -> Order {
    let mut order = Order::default();
    create_order(
        &mut order,
        global_config,
        maker,
        1_000,
        2_000,
        Pubkey::default(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        OrderType::Vanilla as u8,
        254,
        100,
        0,
    )
    .unwrap();
    order
}

#[test]
fn update_order_handler_sets_permissionless_flag() {
    let program_id = limo::ID;
    let owner = limo::ID;
    let maker_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let order_key = Pubkey::new_unique();

    let mut maker_lamports = 0;
    let mut maker_data = [];
    let maker_info = AccountInfo::new(
        &maker_key,
        true,
        false,
        &mut maker_lamports,
        &mut maker_data,
        &owner,
        false,
        0,
    );

    let mut global_config_lamports = 0;
    let mut global_config_data = zero_copy_account_data(&GlobalConfig::zeroed());
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

    let mut order_lamports = 0;
    let mut order_data = zero_copy_account_data(&active_order(global_config_key, maker_key));
    let order_info = AccountInfo::new(
        &order_key,
        false,
        true,
        &mut order_lamports,
        &mut order_data,
        &owner,
        false,
        0,
    );

    let mut accounts = UpdateOrder {
        maker: Signer::try_from(&maker_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        order: AccountLoader::try_from(&order_info).unwrap(),
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], UpdateOrderBumps {});

    handler_update_order(ctx, UpdateOrderMode::UpdatePermissionless, &[1]).unwrap();

    let order = AccountLoader::<Order>::try_from(&order_info).unwrap();
    assert_eq!(order.load().unwrap().permissionless, 1);
}
