mod common;

use anchor_lang::prelude::{
    AccountLoader, Context, Interface, InterfaceAccount, Program, Pubkey, Rent, Signer, Sysvar,
    System,
};
use anchor_spl::{
    associated_token::get_associated_token_address_with_program_id,
    token_interface::{spl_token_2022, Mint, TokenAccount, TokenInterface},
};
use bytemuck::Zeroable;
use common::{
    executable_account, install_noop_syscall_stubs, mint_account, token_account,
    zero_copy_account_data, TestAccount,
};
use limo::{
    handlers::take_order::{TakeOrder, TakeOrderBumps},
    limo as program,
    operations::create_order,
    state::{GlobalConfig, Order, OrderType},
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
    order
}

fn active_limit_parent_order(
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
        OrderType::LimitParent as u8,
        254,
        1,
        0,
    )
    .unwrap();
    order
}

#[test]
#[allow(clippy::too_many_lines)]
fn take_order_handler_fills_vanilla_order_partially() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let (
        taker_key,
        maker_key,
        global_config_key,
        pda_authority_key,
        input_mint_key,
        output_mint_key,
    ) = (
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    );
    let token_program_key = spl_token_2022::ID;
    let maker_output_ata_key = get_associated_token_address_with_program_id(
        &maker_key,
        &output_mint_key,
        &token_program_key,
    );

    let mut global_config = GlobalConfig::zeroed();
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;
    global_config.allowed_taker = taker_key;
    let order = active_vanilla_order(
        global_config_key, maker_key, input_mint_key, output_mint_key, token_program_key,
    );

    let mut taker = TestAccount::new(taker_key, owner).signer().writable();
    let mut maker = TestAccount::new(maker_key, owner).writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();
    let mut pda_authority = TestAccount::new(pda_authority_key, owner).writable();
    let mut order = TestAccount::new(Pubkey::new_unique(), owner)
        .with_data(zero_copy_account_data(&order))
        .writable();
    let mut input_mint = mint_account(input_mint_key, token_program_key);
    let mut output_mint = mint_account(output_mint_key, token_program_key);
    let mut input_vault = token_account(
        Pubkey::new_unique(), token_program_key, input_mint_key, pda_authority_key, 1_000,
    );
    let mut output_fee_vault = token_account(
        Pubkey::new_unique(), token_program_key, output_mint_key, pda_authority_key, 0,
    );
    let mut taker_input_ata = token_account(
        Pubkey::new_unique(), token_program_key, input_mint_key, taker_key, 0,
    );
    let mut taker_output_ata = token_account(
        Pubkey::new_unique(), token_program_key, output_mint_key, taker_key, 1_000,
    );
    let mut maker_output_ata = token_account(
        maker_output_ata_key, token_program_key, output_mint_key, maker_key, 0,
    );
    let mut sysvar_instructions =
        TestAccount::new(solana_program::sysvar::instructions::ID, owner);
    let mut input_token_program = executable_account(token_program_key, owner);
    let mut output_token_program = executable_account(token_program_key, owner);
    let mut rent = TestAccount::new(solana_program::sysvar::rent::ID, owner)
        .with_data(bincode::serialize(&Rent::default()).unwrap());
    let mut system_program = executable_account(anchor_lang::system_program::ID, owner);
    let mut event_authority = TestAccount::new(Pubkey::new_unique(), owner);
    let mut program = executable_account(limo::ID, owner);

    let (
        taker_info,
        maker_info,
        global_config_info,
        pda_authority_info,
        order_info,
        input_mint_info,
        output_mint_info,
        input_vault_info,
        output_fee_vault_info,
        taker_input_ata_info,
        taker_output_ata_info,
        maker_output_ata_info,
        sysvar_instructions_info,
        input_token_program_info,
        output_token_program_info,
        rent_info,
        system_program_info,
        event_authority_info,
        program_info,
    ) = (
        taker.info(),
        maker.info(),
        global_config.info(),
        pda_authority.info(),
        order.info(),
        input_mint.info(),
        output_mint.info(),
        input_vault.info(),
        output_fee_vault.info(),
        taker_input_ata.info(),
        taker_output_ata.info(),
        maker_output_ata.info(),
        sysvar_instructions.info(),
        input_token_program.info(),
        output_token_program.info(),
        rent.info(),
        system_program.info(),
        event_authority.info(),
        program.info(),
    );

    let mut accounts = TakeOrder {
        taker: Signer::try_from(&taker_info).unwrap(),
        maker: maker_info,
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_authority_info,
        order: AccountLoader::try_from(&order_info).unwrap(),
        parent_order: None,
        brother_order: None,
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        input_vault: Box::new(InterfaceAccount::<TokenAccount>::try_from(&input_vault_info).unwrap()),
        output_vault: None,
        output_fee_vault: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&output_fee_vault_info).unwrap(),
        ),
        output_oracle_pool: None,
        input_oracle_pool: None,
        input_price_update: None,
        output_price_update: None,
        taker_input_ata: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&taker_input_ata_info).unwrap(),
        ),
        taker_output_ata: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&taker_output_ata_info).unwrap(),
        ),
        intermediary_output_token_account: None,
        maker_output_ata: Some(Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&maker_output_ata_info).unwrap(),
        )),
        sysvar_instructions: sysvar_instructions_info,
        input_token_program: Interface::<TokenInterface>::try_from(&input_token_program_info).unwrap(),
        output_token_program: Interface::<TokenInterface>::try_from(&output_token_program_info).unwrap(),
        rent: Sysvar::<Rent>::from_account_info(&rent_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], TakeOrderBumps {
        output_vault: 0,
        output_fee_vault: 253,
        output_oracle_pool: 0,
        input_oracle_pool: 0,
        intermediary_output_token_account: 0,
        event_authority: 252,
    });

    program::take_order(ctx, 500, 1_000, 0).unwrap();

    let order = AccountLoader::<Order>::try_from(&order_info).unwrap();
    let order = order.load().unwrap();
    assert_eq!(order.remaining_input_amount, 500);
    assert_eq!(order.filled_output_amount, 1_000);
    assert_eq!(order.number_of_fills, 1);
}

#[test]
#[allow(clippy::too_many_lines)]
fn take_order_handler_fills_limit_parent_into_output_vault() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let (
        taker_key,
        maker_key,
        global_config_key,
        pda_authority_key,
        input_mint_key,
        output_mint_key,
    ) = (
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
        Pubkey::new_unique(),
    );
    let token_program_key = spl_token_2022::ID;

    let mut global_config = GlobalConfig::zeroed();
    global_config.pda_authority = pda_authority_key;
    global_config.pda_authority_bump = 254;
    global_config.allowed_taker = taker_key;
    global_config.parent_fill_fee_protocol_bps = 1_000;
    let order = active_limit_parent_order(
        global_config_key, maker_key, input_mint_key, output_mint_key, token_program_key,
    );

    let mut taker = TestAccount::new(taker_key, owner).signer().writable();
    let mut maker = TestAccount::new(maker_key, owner).writable();
    let mut global_config = TestAccount::new(global_config_key, owner)
        .with_data(zero_copy_account_data(&global_config))
        .writable();
    let mut pda_authority = TestAccount::new(pda_authority_key, owner).writable();
    let mut order = TestAccount::new(Pubkey::new_unique(), owner)
        .with_data(zero_copy_account_data(&order))
        .writable();
    let mut input_mint = mint_account(input_mint_key, token_program_key);
    let mut output_mint = mint_account(output_mint_key, token_program_key);
    let mut input_vault = token_account(
        Pubkey::new_unique(), token_program_key, input_mint_key, pda_authority_key, 1_000,
    );
    let mut output_vault = token_account(
        Pubkey::new_unique(), token_program_key, output_mint_key, pda_authority_key, 0,
    );
    let mut output_fee_vault = token_account(
        Pubkey::new_unique(), token_program_key, output_mint_key, pda_authority_key, 0,
    );
    let mut taker_input_ata = token_account(
        Pubkey::new_unique(), token_program_key, input_mint_key, taker_key, 0,
    );
    let mut taker_output_ata = token_account(
        Pubkey::new_unique(), token_program_key, output_mint_key, taker_key, 2_000,
    );
    let mut sysvar_instructions =
        TestAccount::new(solana_program::sysvar::instructions::ID, owner);
    let mut input_token_program = executable_account(token_program_key, owner);
    let mut output_token_program = executable_account(token_program_key, owner);
    let mut rent = TestAccount::new(solana_program::sysvar::rent::ID, owner)
        .with_data(bincode::serialize(&Rent::default()).unwrap());
    let mut system_program = executable_account(anchor_lang::system_program::ID, owner);
    let mut event_authority = TestAccount::new(Pubkey::new_unique(), owner);
    let mut program = executable_account(limo::ID, owner);

    let (
        taker_info,
        maker_info,
        global_config_info,
        pda_authority_info,
        order_info,
        input_mint_info,
        output_mint_info,
        input_vault_info,
        output_vault_info,
        output_fee_vault_info,
        taker_input_ata_info,
        taker_output_ata_info,
        sysvar_instructions_info,
        input_token_program_info,
        output_token_program_info,
        rent_info,
        system_program_info,
        event_authority_info,
        program_info,
    ) = (
        taker.info(),
        maker.info(),
        global_config.info(),
        pda_authority.info(),
        order.info(),
        input_mint.info(),
        output_mint.info(),
        input_vault.info(),
        output_vault.info(),
        output_fee_vault.info(),
        taker_input_ata.info(),
        taker_output_ata.info(),
        sysvar_instructions.info(),
        input_token_program.info(),
        output_token_program.info(),
        rent.info(),
        system_program.info(),
        event_authority.info(),
        program.info(),
    );

    let mut accounts = TakeOrder {
        taker: Signer::try_from(&taker_info).unwrap(),
        maker: maker_info,
        global_config: AccountLoader::try_from(&global_config_info).unwrap(),
        pda_authority: pda_authority_info,
        order: AccountLoader::try_from(&order_info).unwrap(),
        parent_order: None,
        brother_order: None,
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        input_vault: Box::new(InterfaceAccount::<TokenAccount>::try_from(&input_vault_info).unwrap()),
        output_vault: Some(Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&output_vault_info).unwrap(),
        )),
        output_fee_vault: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&output_fee_vault_info).unwrap(),
        ),
        output_oracle_pool: None,
        input_oracle_pool: None,
        input_price_update: None,
        output_price_update: None,
        taker_input_ata: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&taker_input_ata_info).unwrap(),
        ),
        taker_output_ata: Box::new(
            InterfaceAccount::<TokenAccount>::try_from(&taker_output_ata_info).unwrap(),
        ),
        intermediary_output_token_account: None,
        maker_output_ata: None,
        sysvar_instructions: sysvar_instructions_info,
        input_token_program: Interface::<TokenInterface>::try_from(&input_token_program_info).unwrap(),
        output_token_program: Interface::<TokenInterface>::try_from(&output_token_program_info).unwrap(),
        rent: Sysvar::<Rent>::from_account_info(&rent_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(&program_id, &mut accounts, &[], TakeOrderBumps {
        output_vault: 252,
        output_fee_vault: 253,
        output_oracle_pool: 0,
        input_oracle_pool: 0,
        intermediary_output_token_account: 0,
        event_authority: 251,
    });

    program::take_order(ctx, 500, 1_100, 0).unwrap();

    let order = AccountLoader::<Order>::try_from(&order_info).unwrap();
    let order = order.load().unwrap();
    assert_eq!(order.remaining_input_amount, 500);
    assert_eq!(order.filled_output_amount, 1_090);
    assert_eq!(order.available_child_input_amount, 1_090);
}
