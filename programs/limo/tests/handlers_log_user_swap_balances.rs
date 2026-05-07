mod common;

use anchor_lang::prelude::{AccountInfo, InterfaceAccount, Pubkey, Signer, UncheckedAccount};
use anchor_lang::{
    prelude::{AccountLoader, Context, Program, Rent, Sysvar, System},
    AnchorSerialize, Discriminator,
};
use anchor_spl::token_interface::{spl_token_2022, Mint};
use bytemuck::from_bytes;
use common::{
    install_noop_syscall_stubs, instructions_sysvar_data, zero_copy_account_data,
    zeroed_zero_copy_account_data, TestAccount,
};
use limo::{
    handlers::log_user_swap_balances::{
        get_balances_checked, LogUserSwapBalances, LogUserSwapBalancesEndContext,
        LogUserSwapBalancesBumps, LogUserSwapBalancesEndContextBumps,
        LogUserSwapBalancesStartContext,
        LogUserSwapBalancesStartContextBumps,
    },
    instruction::{LogUserSwapBalancesEnd, LogUserSwapBalancesStart},
    limo as program,
    state::UserSwapBalancesState,
};
use solana_program::{program_option::COption, program_pack::Pack};
use solana_program::instruction::{AccountMeta, Instruction};

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

fn token_account_data(mint: Pubkey, owner: Pubkey, amount: u64) -> Vec<u8> {
    let mut data = vec![0; spl_token_2022::state::Account::LEN];
    let token_account = spl_token_2022::state::Account {
        mint,
        owner,
        amount,
        delegate: COption::None,
        state: spl_token_2022::state::AccountState::Initialized,
        is_native: COption::None,
        delegated_amount: 0,
        close_authority: COption::None,
    };
    spl_token_2022::state::Account::pack(token_account, &mut data).unwrap();
    data
}

fn instruction_data<T: AnchorSerialize + Discriminator>(args: &T) -> Vec<u8> {
    let mut data = T::discriminator().to_vec();
    args.serialize(&mut data).unwrap();
    data
}

#[allow(clippy::too_many_arguments)]
fn log_balance_ixs(
    accounts: Vec<AccountMeta>,
    swap_program_key: Pubkey,
    simulated_swap_amount_out: u64,
    simulated_ts: u64,
    minimum_amount_out: u64,
    swap_amount_in: u64,
    simulated_amount_out_next_best: u64,
    aggregator: u8,
    next_best_aggregator: u8,
) -> [Instruction; 3] {
    [
        Instruction {
            program_id: limo::ID,
            accounts: accounts.clone(),
            data: instruction_data(&LogUserSwapBalancesStart {}),
        },
        Instruction {
            program_id: swap_program_key,
            accounts: vec![],
            data: vec![1, 2, 3],
        },
        Instruction {
            program_id: limo::ID,
            accounts,
            data: instruction_data(&LogUserSwapBalancesEnd {
                simulated_swap_amount_out,
                simulated_ts,
                minimum_amount_out,
                swap_amount_in,
                simulated_amount_out_next_best,
                aggregator,
                next_best_aggregator,
                _padding: [0; 2],
            }),
        },
    ]
}

#[test]
fn get_balances_checked_reads_token_account_balances() {
    let token_program = spl_token_2022::ID;
    let maker_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let input_ta_key = Pubkey::new_unique();
    let output_ta_key = Pubkey::new_unique();
    let swap_program_key = Pubkey::new_unique();

    let mut maker_lamports = 42;
    let mut maker_data = [];
    let maker_info = AccountInfo::new(
        &maker_key,
        true,
        false,
        &mut maker_lamports,
        &mut maker_data,
        &limo::ID,
        false,
        0,
    );

    let mut input_mint_lamports = 1;
    let mut input_mint_data = mint_account_data();
    let input_mint_info = AccountInfo::new(
        &input_mint_key,
        false,
        false,
        &mut input_mint_lamports,
        &mut input_mint_data,
        &token_program,
        false,
        0,
    );

    let mut output_mint_lamports = 1;
    let mut output_mint_data = mint_account_data();
    let output_mint_info = AccountInfo::new(
        &output_mint_key,
        false,
        false,
        &mut output_mint_lamports,
        &mut output_mint_data,
        &token_program,
        false,
        0,
    );

    let mut input_ta_lamports = 1;
    let mut input_ta_data = token_account_data(input_mint_key, maker_key, 123);
    let input_ta_info = AccountInfo::new(
        &input_ta_key,
        false,
        false,
        &mut input_ta_lamports,
        &mut input_ta_data,
        &token_program,
        false,
        0,
    );

    let mut output_ta_lamports = 1;
    let mut output_ta_data = token_account_data(output_mint_key, maker_key, 456);
    let output_ta_info = AccountInfo::new(
        &output_ta_key,
        false,
        false,
        &mut output_ta_lamports,
        &mut output_ta_data,
        &token_program,
        false,
        0,
    );

    let mut swap_program_lamports = 0;
    let mut swap_program_data = [];
    let swap_program_info = AccountInfo::new(
        &swap_program_key,
        false,
        false,
        &mut swap_program_lamports,
        &mut swap_program_data,
        &limo::ID,
        false,
        0,
    );

    let accounts = LogUserSwapBalances {
        maker: Signer::try_from(&maker_info).unwrap(),
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        input_ta: UncheckedAccount::try_from(&input_ta_info),
        output_ta: UncheckedAccount::try_from(&output_ta_info),
        pda_referrer: None,
        swap_program_id: swap_program_info,
    };

    let balances = get_balances_checked(&accounts).unwrap();

    assert_eq!(balances.lamports_balance, 42);
    assert_eq!(balances.input_balance, 123);
    assert_eq!(balances.output_balance, 456);
}

#[test]
fn get_balances_checked_treats_missing_token_accounts_as_zero() {
    let token_program = spl_token_2022::ID;
    let maker_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let input_ta_key = Pubkey::new_unique();
    let output_ta_key = Pubkey::new_unique();
    let swap_program_key = Pubkey::new_unique();

    let mut maker_lamports = 24;
    let mut maker_data = [];
    let maker_info = AccountInfo::new(
        &maker_key,
        true,
        false,
        &mut maker_lamports,
        &mut maker_data,
        &limo::ID,
        false,
        0,
    );

    let mut input_mint_lamports = 1;
    let mut input_mint_data = mint_account_data();
    let input_mint_info = AccountInfo::new(
        &input_mint_key,
        false,
        false,
        &mut input_mint_lamports,
        &mut input_mint_data,
        &token_program,
        false,
        0,
    );

    let mut output_mint_lamports = 1;
    let mut output_mint_data = mint_account_data();
    let output_mint_info = AccountInfo::new(
        &output_mint_key,
        false,
        false,
        &mut output_mint_lamports,
        &mut output_mint_data,
        &token_program,
        false,
        0,
    );

    let mut input_ta_lamports = 0;
    let mut input_ta_data = [];
    let input_ta_info = AccountInfo::new(
        &input_ta_key,
        false,
        false,
        &mut input_ta_lamports,
        &mut input_ta_data,
        &limo::ID,
        false,
        0,
    );

    let mut output_ta_lamports = 0;
    let mut output_ta_data = [];
    let output_ta_info = AccountInfo::new(
        &output_ta_key,
        false,
        false,
        &mut output_ta_lamports,
        &mut output_ta_data,
        &limo::ID,
        false,
        0,
    );

    let mut swap_program_lamports = 0;
    let mut swap_program_data = [];
    let swap_program_info = AccountInfo::new(
        &swap_program_key,
        false,
        false,
        &mut swap_program_lamports,
        &mut swap_program_data,
        &limo::ID,
        false,
        0,
    );

    let accounts = LogUserSwapBalances {
        maker: Signer::try_from(&maker_info).unwrap(),
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        input_ta: UncheckedAccount::try_from(&input_ta_info),
        output_ta: UncheckedAccount::try_from(&output_ta_info),
        pda_referrer: None,
        swap_program_id: swap_program_info,
    };

    let balances = get_balances_checked(&accounts).unwrap();

    assert_eq!(balances.lamports_balance, 24);
    assert_eq!(balances.input_balance, 0);
    assert_eq!(balances.output_balance, 0);
}

#[test]
#[allow(clippy::too_many_lines)]
fn log_user_swap_balances_start_records_current_balances() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let token_program = spl_token_2022::ID;
    let maker_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let input_ta_key = Pubkey::new_unique();
    let output_ta_key = Pubkey::new_unique();
    let swap_program_key = Pubkey::new_unique();
    let state_key = Pubkey::new_unique();
    let event_authority_key = Pubkey::new_unique();

    let account_metas = vec![
        AccountMeta::new_readonly(maker_key, true),
        AccountMeta::new_readonly(input_mint_key, false),
        AccountMeta::new_readonly(output_mint_key, false),
        AccountMeta::new_readonly(input_ta_key, false),
        AccountMeta::new_readonly(output_ta_key, false),
        AccountMeta::new_readonly(swap_program_key, false),
        AccountMeta::new(state_key, false),
        AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        AccountMeta::new_readonly(solana_program::sysvar::rent::ID, false),
        AccountMeta::new_readonly(solana_program::sysvar::instructions::ID, false),
        AccountMeta::new_readonly(event_authority_key, false),
        AccountMeta::new_readonly(limo::ID, false),
    ];
    let ixs = log_balance_ixs(account_metas, swap_program_key, 77, 88, 5, 10, 70, 1, 2);
    let sysvar_data = instructions_sysvar_data(&ixs, 0);

    let mut maker = TestAccount::new(maker_key, owner)
        .with_lamports(42)
        .signer()
        .writable();
    let mut input_mint = TestAccount::new(input_mint_key, token_program)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut output_mint = TestAccount::new(output_mint_key, token_program)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut input_ta = TestAccount::new(input_ta_key, token_program)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, maker_key, 123));
    let mut output_ta = TestAccount::new(output_ta_key, token_program)
        .with_lamports(1)
        .with_data(token_account_data(output_mint_key, maker_key, 456));
    let mut swap_program = TestAccount::new(swap_program_key, owner);
    let mut user_swap_balance_state = TestAccount::new(state_key, owner)
        .with_data(zeroed_zero_copy_account_data::<UserSwapBalancesState>())
        .writable();
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut rent = TestAccount::new(solana_program::sysvar::rent::ID, owner)
        .with_data(bincode::serialize(&Rent::default()).unwrap());
    let mut sysvar_instructions =
        TestAccount::new(solana_program::sysvar::instructions::ID, owner).with_data(sysvar_data);
    let mut event_authority = TestAccount::new(event_authority_key, owner);
    let mut program = TestAccount::new(limo::ID, owner).executable();

    let maker_info = maker.info();
    let input_mint_info = input_mint.info();
    let output_mint_info = output_mint.info();
    let input_ta_info = input_ta.info();
    let output_ta_info = output_ta.info();
    let swap_program_info = swap_program.info();
    let user_swap_balance_state_info = user_swap_balance_state.info();
    let system_program_info = system_program.info();
    let rent_info = rent.info();
    let sysvar_instructions_info = sysvar_instructions.info();
    let event_authority_info = event_authority.info();
    let program_info = program.info();

    let base_accounts = LogUserSwapBalances {
        maker: Signer::try_from(&maker_info).unwrap(),
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        input_ta: UncheckedAccount::try_from(&input_ta_info),
        output_ta: UncheckedAccount::try_from(&output_ta_info),
        pda_referrer: None,
        swap_program_id: swap_program_info,
    };
    let mut accounts = LogUserSwapBalancesStartContext {
        base_accounts,
        user_swap_balance_state: AccountLoader::try_from_unchecked(
            &program_id,
            &user_swap_balance_state_info,
        )
        .unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        rent: Sysvar::<Rent>::from_account_info(&rent_info).unwrap(),
        sysvar_instructions: sysvar_instructions_info,
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        LogUserSwapBalancesStartContextBumps {
            base_accounts: LogUserSwapBalancesBumps {},
            user_swap_balance_state: 254,
            event_authority: 253,
        },
    );

    program::log_user_swap_balances_start(ctx).unwrap();

    let state_data = user_swap_balance_state_info.try_borrow_data().unwrap();
    let state = from_bytes::<UserSwapBalancesState>(
        &state_data[8..8 + std::mem::size_of::<UserSwapBalancesState>()],
    );
    assert_eq!(state.user_lamports, 42);
    assert_eq!(state.input_ta_balance, 123);
    assert_eq!(state.output_ta_balance, 456);
}

#[test]
#[allow(clippy::too_many_lines)]
fn log_user_swap_balances_end_emits_diffs_and_closes_state() {
    install_noop_syscall_stubs();

    let program_id = limo::ID;
    let owner = limo::ID;
    let token_program = spl_token_2022::ID;
    let maker_key = Pubkey::new_unique();
    let input_mint_key = Pubkey::new_unique();
    let output_mint_key = Pubkey::new_unique();
    let input_ta_key = Pubkey::new_unique();
    let output_ta_key = Pubkey::new_unique();
    let swap_program_key = Pubkey::new_unique();
    let state_key = Pubkey::new_unique();
    let event_authority_key = Pubkey::new_unique();

    let account_metas = vec![
        AccountMeta::new_readonly(maker_key, true),
        AccountMeta::new_readonly(input_mint_key, false),
        AccountMeta::new_readonly(output_mint_key, false),
        AccountMeta::new_readonly(input_ta_key, false),
        AccountMeta::new_readonly(output_ta_key, false),
        AccountMeta::new_readonly(swap_program_key, false),
        AccountMeta::new(state_key, false),
        AccountMeta::new_readonly(anchor_lang::system_program::ID, false),
        AccountMeta::new_readonly(solana_program::sysvar::rent::ID, false),
        AccountMeta::new_readonly(solana_program::sysvar::instructions::ID, false),
        AccountMeta::new_readonly(event_authority_key, false),
        AccountMeta::new_readonly(limo::ID, false),
    ];
    let ixs = log_balance_ixs(account_metas, swap_program_key, 77, 88, 5, 10, 70, 1, 2);
    let sysvar_data = instructions_sysvar_data(&ixs, 2);

    let start_state = UserSwapBalancesState {
        user_lamports: 42,
        input_ta_balance: 123,
        output_ta_balance: 456,
    };

    let mut maker = TestAccount::new(maker_key, owner)
        .with_lamports(42)
        .signer()
        .writable();
    let mut input_mint = TestAccount::new(input_mint_key, token_program)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut output_mint = TestAccount::new(output_mint_key, token_program)
        .with_lamports(1)
        .with_data(mint_account_data());
    let mut input_ta = TestAccount::new(input_ta_key, token_program)
        .with_lamports(1)
        .with_data(token_account_data(input_mint_key, maker_key, 113));
    let mut output_ta = TestAccount::new(output_ta_key, token_program)
        .with_lamports(1)
        .with_data(token_account_data(output_mint_key, maker_key, 461));
    let mut swap_program = TestAccount::new(swap_program_key, owner);
    let mut state_lamports = 1;
    let state_owner = owner;
    let state_data = zero_copy_account_data(&start_state);
    let mut state_backing = vec![0; 8 + state_data.len()];
    state_backing[..8].copy_from_slice(&(state_data.len() as u64).to_le_bytes());
    state_backing[8..].copy_from_slice(&state_data);
    let mut system_program = TestAccount::new(anchor_lang::system_program::ID, owner).executable();
    let mut rent = TestAccount::new(solana_program::sysvar::rent::ID, owner)
        .with_data(bincode::serialize(&Rent::default()).unwrap());
    let mut sysvar_instructions =
        TestAccount::new(solana_program::sysvar::instructions::ID, owner).with_data(sysvar_data);
    let mut event_authority = TestAccount::new(event_authority_key, owner);
    let mut program = TestAccount::new(limo::ID, owner).executable();

    let maker_info = maker.info();
    let input_mint_info = input_mint.info();
    let output_mint_info = output_mint.info();
    let input_ta_info = input_ta.info();
    let output_ta_info = output_ta.info();
    let swap_program_info = swap_program.info();
    let user_swap_balance_state_info = AccountInfo::new(
        &state_key,
        false,
        true,
        &mut state_lamports,
        &mut state_backing[8..],
        &state_owner,
        false,
        0,
    );
    let system_program_info = system_program.info();
    let rent_info = rent.info();
    let sysvar_instructions_info = sysvar_instructions.info();
    let event_authority_info = event_authority.info();
    let program_info = program.info();

    let base_accounts = LogUserSwapBalances {
        maker: Signer::try_from(&maker_info).unwrap(),
        input_mint: Box::new(InterfaceAccount::<Mint>::try_from(&input_mint_info).unwrap()),
        output_mint: Box::new(InterfaceAccount::<Mint>::try_from(&output_mint_info).unwrap()),
        input_ta: UncheckedAccount::try_from(&input_ta_info),
        output_ta: UncheckedAccount::try_from(&output_ta_info),
        pda_referrer: None,
        swap_program_id: swap_program_info,
    };
    let mut accounts = LogUserSwapBalancesEndContext {
        base_accounts,
        user_swap_balance_state: AccountLoader::try_from(&user_swap_balance_state_info).unwrap(),
        system_program: Program::<System>::try_from(&system_program_info).unwrap(),
        rent: Sysvar::<Rent>::from_account_info(&rent_info).unwrap(),
        sysvar_instructions: sysvar_instructions_info,
        event_authority: event_authority_info,
        program: program_info,
    };
    let ctx = Context::new(
        &program_id,
        &mut accounts,
        &[],
        LogUserSwapBalancesEndContextBumps {
            base_accounts: LogUserSwapBalancesBumps {},
            user_swap_balance_state: 254,
            event_authority: 253,
        },
    );

    program::log_user_swap_balances_end(ctx, 77, 88, 5, 10, 70, 1, 2, [0; 2]).unwrap();
}
