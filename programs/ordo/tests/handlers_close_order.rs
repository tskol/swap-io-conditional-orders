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
use ordo::{
    handlers::close_order_and_claim_tip::{ExitOrderAndClaimTip, ExitOrderAndClaimTipBumps},
    operations::create_order,
    ordo as program,
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
#[allow(clippy::too_many_lines)]
fn close_order_handler_cancels_vanilla_order_without_transfers() {
    install_noop_syscall_stubs();

    let program_id = ordo::ID;
    let owner = ordo::ID;
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
        global_config_key,
        maker_key,
        input_mint_key,
        output_mint_key,
        token_program_key,
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
    let mut program = TestAccount::new(ordo::ID, owner).executable();

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

    let mut accounts = ExitOrderAndClaimTip {
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
        input_token_program: Interface::<TokenInterface>::try_from(&input_token_program_info)
            .unwrap(),
        output_token_program: Interface::<TokenInterface>::try_from(&output_token_program_info)
            .unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        ExitOrderAndClaimTipBumps {
            input_vault: 254,
            output_vault: 0,
            event_authority: 253,
        },
    );

    program::close_order_and_claim_tip(ctx).unwrap();

    let order = AccountLoader::<Order>::try_from(&order_info).unwrap();
    assert_eq!(order.load().unwrap().status, OrderStatus::Cancelled as u8);
}

#[test]
#[allow(clippy::too_many_lines)]
fn close_order_handler_cancels_parent_and_children_with_vault_returns() {
    install_noop_syscall_stubs();

    let program_id = ordo::ID;
    let owner = ordo::ID;
    let maker_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let parent_order_key = Pubkey::new_unique();
    let tp_child_order_key = Pubkey::new_unique();
    let sl_child_order_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;

    let mut global_config = GlobalConfig::zeroed();
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;

    let mut parent_order = Order::default();
    create_order(
        &mut parent_order,
        global_config_key,
        maker_key,
        1_000,
        2_000,
        Pubkey::default(),
        input_mint_key,
        output_mint_key,
        token_program_key,
        token_program_key,
        OrderType::LimitParent as u8,
        254,
        1,
        0,
    )
    .unwrap();
    parent_order.tp_child_order = tp_child_order_key;
    parent_order.sl_child_order = sl_child_order_key;
    parent_order.remaining_input_amount = 300;
    parent_order.available_child_input_amount = 400;

    let mut tp_child_order = Order::default();
    create_order(
        &mut tp_child_order,
        global_config_key,
        maker_key,
        2_000,
        2_500,
        parent_order_key,
        output_mint_key,
        input_mint_key,
        token_program_key,
        token_program_key,
        OrderType::LimitTP as u8,
        253,
        1,
        0,
    )
    .unwrap();

    let mut sl_child_order = Order::default();
    create_order(
        &mut sl_child_order,
        global_config_key,
        maker_key,
        2_000,
        1_500,
        parent_order_key,
        output_mint_key,
        input_mint_key,
        token_program_key,
        token_program_key,
        OrderType::LimitSL as u8,
        252,
        1,
        0,
    )
    .unwrap();

    let mut closer = TestAccount::new(maker_key, owner).signer().writable();
    let mut maker = TestAccount::new(maker_key, owner).writable();
    let mut order = TestAccount::new(parent_order_key, owner)
        .with_data(zero_copy_account_data(&parent_order))
        .writable();
    let mut tp_child_order = TestAccount::new(tp_child_order_key, owner)
        .with_data(zero_copy_account_data(&tp_child_order))
        .writable();
    let mut sl_child_order = TestAccount::new(sl_child_order_key, owner)
        .with_data(zero_copy_account_data(&sl_child_order))
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
    let mut maker_output_ata = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(output_mint_key, maker_key, 0))
        .writable();
    let mut input_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, pda_authority_key, 1_000))
        .writable();
    let mut output_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(
            output_mint_key,
            pda_authority_key,
            1_000,
        ))
        .writable();
    let mut input_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut output_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut event_authority = TestAccount::new(Pubkey::new_unique(), owner);
    let mut program = TestAccount::new(ordo::ID, owner).executable();

    let closer_info = closer.info();
    let maker_info = maker.info();
    let order_info = order.info();
    let tp_child_order_info = tp_child_order.info();
    let sl_child_order_info = sl_child_order.info();
    let global_config_info = global_config.info();
    let pda_authority_info = pda_authority.info();
    let input_mint_info = input_mint.info();
    let output_mint_info = output_mint.info();
    let maker_input_ata_info = maker_input_ata.info();
    let maker_output_ata_info = maker_output_ata.info();
    let input_vault_info = input_vault.info();
    let output_vault_info = output_vault.info();
    let input_token_program_info = input_token_program.info();
    let output_token_program_info = output_token_program.info();
    let system_program_info = system_program.info();
    let event_authority_info = event_authority.info();
    let program_info = program.info();

    let mut accounts = ExitOrderAndClaimTip {
        closer: Signer::try_from(&closer_info).unwrap(),
        maker: maker_info,
        order: AccountLoader::try_from(&order_info).unwrap(),
        tp_child_order: Some(AccountLoader::try_from(&tp_child_order_info).unwrap()),
        sl_child_order: Some(AccountLoader::try_from(&sl_child_order_info).unwrap()),
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_authority_info,
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        maker_input_ata: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&maker_input_ata_info).unwrap(),
        ),
        maker_output_ata: Some(Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&maker_output_ata_info).unwrap(),
        )),
        closer_input_ata: None,
        closer_output_ata: None,
        input_vault: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&input_vault_info).unwrap(),
        ),
        output_vault: Some(Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&output_vault_info).unwrap(),
        )),
        input_token_program: Interface::<TokenInterface>::try_from(&input_token_program_info)
            .unwrap(),
        output_token_program: Interface::<TokenInterface>::try_from(&output_token_program_info)
            .unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        ExitOrderAndClaimTipBumps {
            input_vault: 254,
            output_vault: 253,
            event_authority: 252,
        },
    );

    program::close_order_and_claim_tip(ctx).unwrap();

    let order = AccountLoader::<Order>::try_from(&order_info).unwrap();
    let tp_child_order = AccountLoader::<Order>::try_from(&tp_child_order_info).unwrap();
    let sl_child_order = AccountLoader::<Order>::try_from(&sl_child_order_info).unwrap();

    assert_eq!(order.load().unwrap().status, OrderStatus::Cancelled as u8);
    assert_eq!(
        tp_child_order.load().unwrap().status,
        OrderStatus::Cancelled as u8
    );
    assert_eq!(
        sl_child_order.load().unwrap().status,
        OrderStatus::Cancelled as u8
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn close_order_handler_pays_allowed_taker_fee_from_remaining_input() {
    install_noop_syscall_stubs();

    let program_id = ordo::ID;
    let owner = ordo::ID;
    let closer_key = Pubkey::new_unique();
    let maker_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;

    let mut global_config = GlobalConfig::zeroed();
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;
    global_config.allowed_taker = closer_key;
    global_config.keeper_close_fee_bps = 1_000;

    let mut order = Order::default();
    create_order(
        &mut order,
        global_config_key,
        maker_key,
        1_000,
        2_000,
        Pubkey::default(),
        input_mint_key,
        output_mint_key,
        token_program_key,
        token_program_key,
        OrderType::Vanilla as u8,
        254,
        1,
        1,
    )
    .unwrap();

    let mut closer = TestAccount::new(closer_key, owner).signer().writable();
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
    let mut closer_input_ata = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, closer_key, 0))
        .writable();
    let mut input_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, pda_authority_key, 1_000))
        .writable();
    let mut input_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut output_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut event_authority = TestAccount::new(Pubkey::new_unique(), owner);
    let mut program = TestAccount::new(ordo::ID, owner).executable();

    let closer_info = closer.info();
    let maker_info = maker.info();
    let order_info = order.info();
    let global_config_info = global_config.info();
    let pda_authority_info = pda_authority.info();
    let input_mint_info = input_mint.info();
    let output_mint_info = output_mint.info();
    let maker_input_ata_info = maker_input_ata.info();
    let closer_input_ata_info = closer_input_ata.info();
    let input_vault_info = input_vault.info();
    let input_token_program_info = input_token_program.info();
    let output_token_program_info = output_token_program.info();
    let system_program_info = system_program.info();
    let event_authority_info = event_authority.info();
    let program_info = program.info();

    let mut accounts = ExitOrderAndClaimTip {
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
        closer_input_ata: Some(Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&closer_input_ata_info).unwrap(),
        )),
        closer_output_ata: None,
        input_vault: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&input_vault_info).unwrap(),
        ),
        output_vault: None,
        input_token_program: Interface::<TokenInterface>::try_from(&input_token_program_info)
            .unwrap(),
        output_token_program: Interface::<TokenInterface>::try_from(&output_token_program_info)
            .unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        ExitOrderAndClaimTipBumps {
            input_vault: 254,
            output_vault: 0,
            event_authority: 253,
        },
    );

    program::close_order_and_claim_tip(ctx).unwrap();

    let order = AccountLoader::<Order>::try_from(&order_info).unwrap();
    let order = order.load().unwrap();
    assert_eq!(order.status, OrderStatus::Cancelled as u8);
    assert_eq!(order.remaining_input_amount, 900);
}

#[test]
#[allow(clippy::too_many_lines)]
fn close_order_handler_pays_allowed_taker_fee_from_child_output() {
    install_noop_syscall_stubs();

    let program_id = ordo::ID;
    let owner = ordo::ID;
    let closer_key = Pubkey::new_unique();
    let maker_key = Pubkey::new_unique();
    let global_config_key = Pubkey::new_unique();
    let pda_authority_key = Pubkey::new_unique();
    let parent_order_key = Pubkey::new_unique();
    let tp_child_order_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;

    let mut global_config = GlobalConfig::zeroed();
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;
    global_config.allowed_taker = closer_key;
    global_config.keeper_close_fee_bps = 1_000;
    global_config.total_tip_amount = 18;

    let mut parent_order = Order::default();
    create_order(
        &mut parent_order,
        global_config_key,
        maker_key,
        1_000,
        2_000,
        Pubkey::default(),
        input_mint_key,
        output_mint_key,
        token_program_key,
        token_program_key,
        OrderType::LimitParent as u8,
        254,
        1,
        1,
    )
    .unwrap();
    parent_order.tp_child_order = tp_child_order_key;
    parent_order.remaining_input_amount = 50;
    parent_order.available_child_input_amount = 200;
    parent_order.tip_amount = 11;

    let mut tp_child_order = Order::default();
    create_order(
        &mut tp_child_order,
        global_config_key,
        maker_key,
        2_000,
        2_500,
        parent_order_key,
        output_mint_key,
        input_mint_key,
        token_program_key,
        token_program_key,
        OrderType::LimitTP as u8,
        253,
        1,
        1,
    )
    .unwrap();
    tp_child_order.tip_amount = 7;

    let mut closer = TestAccount::new(closer_key, owner).signer().writable();
    let mut maker = TestAccount::new(maker_key, owner).writable();
    let mut order = TestAccount::new(parent_order_key, owner)
        .with_data(zero_copy_account_data(&parent_order))
        .writable();
    let mut tp_child_order = TestAccount::new(tp_child_order_key, owner)
        .with_data(zero_copy_account_data(&tp_child_order))
        .writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();
    let mut pda_authority = TestAccount::new(pda_authority_key, owner)
        .with_lamports(18)
        .writable();
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
    let mut closer_output_ata = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(output_mint_key, closer_key, 0))
        .writable();
    let mut input_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, pda_authority_key, 50))
        .writable();
    let mut output_vault = TestAccount::new(Pubkey::new_unique(), token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(output_mint_key, pda_authority_key, 200))
        .writable();
    let mut input_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut output_token_program = TestAccount::new(token_program_key, owner).executable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut event_authority = TestAccount::new(Pubkey::new_unique(), owner);
    let mut program = TestAccount::new(ordo::ID, owner).executable();

    let closer_info = closer.info();
    let maker_info = maker.info();
    let order_info = order.info();
    let tp_child_order_info = tp_child_order.info();
    let global_config_info = global_config.info();
    let pda_authority_info = pda_authority.info();
    let input_mint_info = input_mint.info();
    let output_mint_info = output_mint.info();
    let maker_input_ata_info = maker_input_ata.info();
    let closer_output_ata_info = closer_output_ata.info();
    let input_vault_info = input_vault.info();
    let output_vault_info = output_vault.info();
    let input_token_program_info = input_token_program.info();
    let output_token_program_info = output_token_program.info();
    let system_program_info = system_program.info();
    let event_authority_info = event_authority.info();
    let program_info = program.info();

    let mut accounts = ExitOrderAndClaimTip {
        closer: Signer::try_from(&closer_info).unwrap(),
        maker: maker_info,
        order: AccountLoader::try_from(&order_info).unwrap(),
        tp_child_order: Some(AccountLoader::try_from(&tp_child_order_info).unwrap()),
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
        closer_output_ata: Some(Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&closer_output_ata_info).unwrap(),
        )),
        input_vault: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&input_vault_info).unwrap(),
        ),
        output_vault: Some(Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&output_vault_info).unwrap(),
        )),
        input_token_program: Interface::<TokenInterface>::try_from(&input_token_program_info)
            .unwrap(),
        output_token_program: Interface::<TokenInterface>::try_from(&output_token_program_info)
            .unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        ExitOrderAndClaimTipBumps {
            input_vault: 254,
            output_vault: 253,
            event_authority: 252,
        },
    );

    program::close_order_and_claim_tip(ctx).unwrap();

    let order = AccountLoader::<Order>::try_from(&order_info).unwrap();
    let tp_child_order = AccountLoader::<Order>::try_from(&tp_child_order_info).unwrap();
    let global_config = AccountLoader::<GlobalConfig>::try_from(&global_config_info).unwrap();
    let order = order.load().unwrap();

    assert_eq!(order.status, OrderStatus::Cancelled as u8);
    assert_eq!(order.remaining_input_amount, 50);
    assert_eq!(order.available_child_input_amount, 0);
    assert_eq!(
        tp_child_order.load().unwrap().status,
        OrderStatus::Cancelled as u8
    );
    assert_eq!(global_config.load().unwrap().total_tip_amount, 0);
}
