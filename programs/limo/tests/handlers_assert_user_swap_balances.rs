mod common;

use anchor_lang::prelude::{
    AccountLoader, Context, InterfaceAccount, Program, Pubkey, Rent, Signer, System, Sysvar,
};
use anchor_spl::token_interface::{spl_token_2022, TokenAccount};
use bytemuck::from_bytes;
use common::{
    install_noop_syscall_stubs, instruction_data, instructions_sysvar_data, token_account_data,
    zero_copy_account_data, zeroed_zero_copy_account_data, TestAccount,
};
use limo::{
    handlers::assert_user_swap_balances::{
        AssertUserSwapBalancesEndContext, AssertUserSwapBalancesEndContextBumps,
        AssertUserSwapBalancesStartContext, AssertUserSwapBalancesStartContextBumps,
    },
    instruction::{AssertUserSwapBalancesEnd, AssertUserSwapBalancesStart},
    limo as program,
    state::UserSwapBalancesState,
};
use solana_program::instruction::{AccountMeta, Instruction};

fn assert_balance_ixs(
    accounts: Vec<AccountMeta>,
    max_input_amount_change: u64,
    min_output_amount_change: u64,
) -> [Instruction; 2] {
    [
        Instruction {
            program_id: limo::ID,
            accounts: accounts.clone(),
            data: instruction_data(&AssertUserSwapBalancesStart {}),
        },
        Instruction {
            program_id: limo::ID,
            accounts,
            data: instruction_data(&AssertUserSwapBalancesEnd {
                max_input_amount_change,
                min_output_amount_change,
            }),
        },
    ]
}

#[test]
fn assert_user_swap_balances_start_records_current_balances() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let maker_key = Pubkey::new_unique();
    let input_ta_key = Pubkey::new_unique();
    let output_ta_key = Pubkey::new_unique();
    let state_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;

    let account_metas = vec![
        AccountMeta::new(maker_key, true),
        AccountMeta::new_readonly(input_ta_key, false),
        AccountMeta::new_readonly(output_ta_key, false),
        AccountMeta::new(state_key, false),
    ];
    let ixs = assert_balance_ixs(account_metas, 10, 5);
    let sysvar_data = instructions_sysvar_data(&ixs, 0);

    let mut maker = TestAccount::new(maker_key, owner)
        .with_lamports(42)
        .signer()
        .writable();
    let mut input_ta = TestAccount::new(input_ta_key, token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, maker_key, 100))
        .writable();
    let mut output_ta = TestAccount::new(output_ta_key, token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(output_mint_key, maker_key, 200))
        .writable();
    let mut user_swap_balance_state = TestAccount::new(state_key, owner)
        .with_data(zeroed_zero_copy_account_data::<UserSwapBalancesState>())
        .writable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut rent = TestAccount::new(solana_program::sysvar::rent::ID, owner)
        .with_data(bincode::serialize(&Rent::default()).unwrap());
    let mut sysvar_instructions =
        TestAccount::new(solana_program::sysvar::instructions::ID, owner).with_data(sysvar_data);

    let maker_info = maker.info();
    let input_ta_info = input_ta.info();
    let output_ta_info = output_ta.info();
    let user_swap_balance_state_info = user_swap_balance_state.info();
    let system_program_info = system_program.info();
    let rent_info = rent.info();
    let sysvar_instructions_info = sysvar_instructions.info();

    let mut accounts = AssertUserSwapBalancesStartContext {
        maker: Signer::try_from(&maker_info).unwrap(),
        input_ta: Box::new(InterfaceAccount::<TokenAccount>::try_from(&input_ta_info).unwrap()),
        output_ta: Box::new(InterfaceAccount::<TokenAccount>::try_from(&output_ta_info).unwrap()),
        user_swap_balance_state: AccountLoader::try_from_unchecked(
            &program_id,
            &user_swap_balance_state_info,
        )
        .unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        rent: Sysvar::<Rent>::from_account_info(&rent_info).unwrap(),
        sysvar_instructions: sysvar_instructions_info,
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        AssertUserSwapBalancesStartContextBumps {
            user_swap_balance_state: 254,
        },
    );

    program::assert_user_swap_balances_start(ctx).unwrap();

    let state_data = user_swap_balance_state_info.try_borrow_data().unwrap();
    let state = from_bytes::<UserSwapBalancesState>(
        &state_data[8..8 + std::mem::size_of::<UserSwapBalancesState>()],
    );
    assert_eq!(state.user_lamports, 42);
    assert_eq!(state.input_ta_balance, 100);
    assert_eq!(state.output_ta_balance, 200);
}

#[test]
fn assert_user_swap_balances_end_validates_balance_changes() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let maker_key = Pubkey::new_unique();
    let input_ta_key = Pubkey::new_unique();
    let output_ta_key = Pubkey::new_unique();
    let state_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let token_program_key = spl_token_2022::ID;

    let account_metas = vec![
        AccountMeta::new(maker_key, true),
        AccountMeta::new_readonly(input_ta_key, false),
        AccountMeta::new_readonly(output_ta_key, false),
        AccountMeta::new(state_key, false),
    ];
    let ixs = assert_balance_ixs(account_metas, 10, 5);
    let sysvar_data = instructions_sysvar_data(&ixs, 1);

    let start_state = UserSwapBalancesState {
        user_lamports: 42,
        input_ta_balance: 100,
        output_ta_balance: 200,
    };

    let mut maker = TestAccount::new(maker_key, owner)
        .with_lamports(42)
        .signer()
        .writable();
    let mut input_ta = TestAccount::new(input_ta_key, token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, maker_key, 90))
        .writable();
    let mut output_ta = TestAccount::new(output_ta_key, token_program_key)
        .with_lamports(1)
        .with_data(token_account_data(output_mint_key, maker_key, 205))
        .writable();
    let mut user_swap_balance_state = TestAccount::new(state_key, owner)
        .with_data(zero_copy_account_data(&start_state))
        .writable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut rent = TestAccount::new(solana_program::sysvar::rent::ID, owner)
        .with_data(bincode::serialize(&Rent::default()).unwrap());
    let mut sysvar_instructions =
        TestAccount::new(solana_program::sysvar::instructions::ID, owner).with_data(sysvar_data);

    let maker_info = maker.info();
    let input_ta_info = input_ta.info();
    let output_ta_info = output_ta.info();
    let user_swap_balance_state_info = user_swap_balance_state.info();
    let system_program_info = system_program.info();
    let rent_info = rent.info();
    let sysvar_instructions_info = sysvar_instructions.info();

    let mut accounts = AssertUserSwapBalancesEndContext {
        maker: Signer::try_from(&maker_info).unwrap(),
        input_ta: Box::new(InterfaceAccount::<TokenAccount>::try_from(&input_ta_info).unwrap()),
        output_ta: Box::new(InterfaceAccount::<TokenAccount>::try_from(&output_ta_info).unwrap()),
        user_swap_balance_state: AccountLoader::try_from(&user_swap_balance_state_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        rent: Sysvar::<Rent>::from_account_info(&rent_info).unwrap(),
        sysvar_instructions: sysvar_instructions_info,
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        AssertUserSwapBalancesEndContextBumps {
            user_swap_balance_state: 254,
        },
    );

    program::assert_user_swap_balances_end(ctx, 10, 5).unwrap();
}
