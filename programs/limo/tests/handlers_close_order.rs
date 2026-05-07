mod common;

use anchor_lang::prelude::{
    AccountLoader, Context, Interface, InterfaceAccount, Program, Pubkey, Signer, System,
};
use anchor_spl::token_interface::{spl_token_2022, Mint, TokenAccount, TokenInterface};
use bytemuck::Zeroable;
use common::{
    install_noop_syscall_stubs, mint_account_data, token_account_data, zero_copy_account_data,
    TestAccount,
};
use limo::{
    handlers::close_order_and_claim_tip::{
        handler_close_order_and_claim_tip, CloseOrderAndClaimTip, CloseOrderAndClaimTipBumps,
    },
    operations::create_order,
    state::{GlobalConfig, Order, OrderStatus, OrderType},
};

fn active_vanilla_order(
    global_config: Pubkey,
    maker: Pubkey,
    input_mint: Pubkey,
    output_mint: Pubkey,
    token_program: Pubkey,
) -> Order {
    let mut order = Order::default();
    create_order(
        &mut order,
        global_config,
        maker,
        1_000,
        2_000,
        Pubkey::default(),
        input_mint,
        output_mint,
        token_program,
        token_program,
        OrderType::Vanilla as u8,
        254,
        1,
        0,
    )
    .unwrap();
    order.remaining_input_amount = 0;
    order.filled_output_amount = 2_000;
    order.status = OrderStatus::Filled as u8;
    order
}

#[test]
fn close_order_handler_cancels_vanilla_order_without_transfers() {
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
    global_config.pda_authority_bump = 254;

    let order = active_vanilla_order(
        global_config_key, maker_key, input_mint_key, output_mint_key, token_program_key,
    );

    let mut closer = TestAccount::new(maker_key, owner).signer().writable();
    let mut maker = TestAccount::new(maker_key, owner).writable();
    let mut order = TestAccount::new(Pubkey::new_unique(), owner)
        .with_data(zero_copy_account_data(&order))
        .writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();
    let mut pda_authority = TestAccount::new(pda_authority_key, owner).writable();
    let mut input_mint = TestAccount::new(input_mint_key, token_program_key)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut output_mint = TestAccount::new(output_mint_key, token_program_key)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut maker_input_ata = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, maker_key, 0))
        .writable();
    let mut input_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, pda_authority_key, 0))
        .writable();
    let mut input_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut output_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut event_authority = TestAccount::new(Pubkey::new_unique(), owner);
    let mut program = TestAccount::new(limo::ID, owner).executable();

    let closer_info = closer.info();
    let maker_info = maker.info();
    let order_info = order.info();
    let global_config_info = global_config.info();
    let pda_authority_info = pda_authority.info();
    let input_mint_info = input_mint.info();
    let output_mint_info = output_mint.info();
    let maker_input_ata_info = maker_input_ata.info();
    let input_vault_info = input_vault.info();
    let input_token_program_info = input_token_program.info();
    let output_token_program_info = output_token_program.info();
    let system_program_info = system_program.info();
    let event_authority_info = event_authority.info();
    let program_info = program.info();

    let mut accounts = CloseOrderAndClaimTip {
        closer: Signer::try_from(&closer_info).unwrap(),
        maker: maker_info,
        order: AccountLoader::try_from(&order_info).unwrap(),
        tp_child_order: None,
        sl_child_order: None,
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_authority_info,
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        maker_input_ata: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&maker_input_ata_info).unwrap(),
        ),
        maker_output_ata: None,
        closer_input_ata: None,
        closer_output_ata: None,
        input_vault: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&input_vault_info).unwrap(),
        ),
        output_vault: None,
        input_token_program: Interface::<TokenInterface>::try_from(&input_token_program_info).unwrap(),
        output_token_program: Interface::<TokenInterface>::try_from(&output_token_program_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], CloseOrderAndClaimTipBumps {
        input_vault: 254,
        output_vault: 0,
        event_authority: 253,
    });

    handler_close_order_and_claim_tip(ctx).unwrap();

    let order = AccountLoader::<Order>::try_from(&order_info).unwrap();
    assert_eq!(order.load().unwrap().status, OrderStatus::Cancelled as u8);
}
