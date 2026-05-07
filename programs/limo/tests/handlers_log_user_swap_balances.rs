use anchor_lang::prelude::{AccountInfo, InterfaceAccount, Pubkey, Signer, UncheckedAccount};
use anchor_spl::token_interface::{spl_token_2022, Mint};
use limo::handlers::log_user_swap_balances::{get_balances_checked, LogUserSwapBalances};
use solana_program::{program_option::COption, program_pack::Pack};

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
