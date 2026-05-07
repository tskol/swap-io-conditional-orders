mod common;

use anchor_lang::prelude::{
    AccountLoader, Context, Interface, InterfaceAccount, Program, Pubkey, Signer, System,
};
use anchor_spl::token_interface::{spl_token_2022, Mint, TokenAccount, TokenInterface};
use bytemuck::{from_bytes, Zeroable};
use common::{
    install_noop_syscall_stubs, mint_account_data, token_account_data,
    zero_copy_account_data, zeroed_zero_copy_account_data, TestAccount,
};
use limo::{
    handlers::create_order::{CreateOrder, CreateOrderBumps},
    limo as program,
    state::{GlobalConfig, Order, OrderStatus, OrderType},
};

#[test]
fn create_order_handler_initializes_vanilla_order() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let maker_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;

    let mut global_config = GlobalConfig::zeroed();
    global_config.pda_authority = pda_authority_key;

    let mut maker = TestAccount::new(maker_key, owner).signer().writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();
    let mut pda_authority = TestAccount::new(pda_authority_key, owner);
    let mut order = TestAccount::new(Pubkey::new_unique(), owner)
        .with_data(zeroed_zero_copy_account_data::<Order>())
        .writable();
    let mut input_mint = TestAccount::new(input_mint_key, token_program_key)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut output_mint = TestAccount::new(output_mint_key, token_program_key)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut maker_ata = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, maker_key, 1_000))
        .writable();
    let mut input_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, pda_authority_key, 0))
        .writable();
    let mut input_fee_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, pda_authority_key, 0))
        .writable();
    let mut input_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut output_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut event_authority = TestAccount::new(Pubkey::new_unique(), owner);
    let mut program = TestAccount::new(limo::ID, owner).executable();

    let maker_info = maker.info();
    let global_config_info = global_config.info();
    let pda_authority_info = pda_authority.info();
    let order_info = order.info();
    let input_mint_info = input_mint.info();
    let output_mint_info = output_mint.info();
    let maker_ata_info = maker_ata.info();
    let input_vault_info = input_vault.info();
    let input_fee_vault_info = input_fee_vault.info();
    let input_token_program_info = input_token_program.info();
    let output_token_program_info = output_token_program.info();
    let system_program_info = system_program.info();
    let event_authority_info = event_authority.info();
    let program_info = program.info();

    let mut accounts = CreateOrder {
        maker: Signer::try_from(&maker_info).unwrap(),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_authority_info,
        order: AccountLoader::try_from_unchecked(&program_id, &order_info).unwrap(),
        tp_order: None,
        sl_order: None,
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        maker_ata: Box::new(InterfaceAccount::<TokenAccount>::try_from(&maker_ata_info).unwrap()),
        input_vault: Box::new(InterfaceAccount::<TokenAccount>::try_from(&input_vault_info).unwrap()),
        input_fee_vault: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&input_fee_vault_info).unwrap(),
        ),
        output_vault: None,
        input_token_program: Interface::<TokenInterface>::try_from(&input_token_program_info).unwrap(),
        output_token_program: Interface::<TokenInterface>::try_from(&output_token_program_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], CreateOrderBumps {
        input_vault: 254,
        input_fee_vault: 253,
        output_vault: 0,
        event_authority: 252,
    });

    program::create_order(ctx, 1_000, 2_000, OrderType::Vanilla as u8, 0, 0, 0).unwrap();

    let order_data = order_info.try_borrow_data().unwrap();
    let order = from_bytes::<Order>(&order_data[8..8 + std::mem::size_of::<Order>()]);
    assert_eq!(order.maker, maker_key);
    assert_eq!(order.input_mint, input_mint_key);
    assert_eq!(order.output_mint, output_mint_key);
    assert_eq!(order.remaining_input_amount, 1_000);
    assert_eq!(order.expected_output_amount, 2_000);
    assert_eq!(order.status, OrderStatus::Active as u8);
}
