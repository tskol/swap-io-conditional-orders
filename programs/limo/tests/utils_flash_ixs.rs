mod common;

use anchor_lang::{prelude::Pubkey, AnchorDeserialize, AnchorSerialize, Discriminator};
use common::{instructions_sysvar_data, TestAccount};
use limo::utils::flash_ixs::{
    check_same_accounts, ensure_first_ix_match, ensure_second_ix_match,
    ix_utils::{InstructionLoader, IxIterator},
};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
};

const TEST_ARGS_DISCRIMINATOR: [u8; 8] = [9; 8];

#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq)]
struct TestArgs {
    value: u64,
}

impl Discriminator for TestArgs {
    const DISCRIMINATOR: [u8; 8] = TEST_ARGS_DISCRIMINATOR;
}

struct FakeInstructionLoader {
    instructions: Vec<Instruction>,
    error_at: Option<usize>,
    current_index: u16,
}

impl InstructionLoader for FakeInstructionLoader {
    fn load_instruction_at(&self, index: usize) -> Result<Instruction, ProgramError> {
        if self.error_at == Some(index) {
            return Err(ProgramError::InvalidAccountData);
        }

        self.instructions
            .get(index)
            .cloned()
            .ok_or(ProgramError::InvalidArgument)
    }

    fn load_current_index(&self) -> Result<u16, ProgramError> {
        Ok(self.current_index)
    }
}

fn instruction_with_accounts(accounts: Vec<AccountMeta>) -> Instruction {
    Instruction {
        program_id: Pubkey::new_unique(),
        accounts,
        data: vec![1, 2, 3],
    }
}

fn limo_instruction(data: Vec<u8>, accounts: Vec<AccountMeta>) -> Instruction {
    Instruction {
        program_id: limo::ID,
        accounts,
        data,
    }
}

fn test_args_data(value: u64) -> Vec<u8> {
    let mut data = TestArgs::discriminator().to_vec();
    TestArgs { value }.serialize(&mut data).unwrap();
    data
}

#[test]
fn check_same_accounts_accepts_identical_account_lists() {
    let account = Pubkey::new_unique();
    let start_ix = instruction_with_accounts(vec![AccountMeta::new(account, false)]);
    let end_ix = instruction_with_accounts(vec![AccountMeta::new(account, false)]);

    check_same_accounts(&start_ix, &end_ix).unwrap();
}

#[test]
fn check_same_accounts_rejects_length_or_pubkey_mismatch() {
    let first = Pubkey::new_unique();
    let second = Pubkey::new_unique();
    let start_ix = instruction_with_accounts(vec![AccountMeta::new(first, false)]);
    let empty_end_ix = instruction_with_accounts(vec![]);
    let mismatched_end_ix = instruction_with_accounts(vec![AccountMeta::new(second, false)]);

    assert!(check_same_accounts(&start_ix, &empty_end_ix).is_err());
    assert!(check_same_accounts(&start_ix, &mismatched_end_ix).is_err());
}

#[test]
fn public_flash_ix_match_wrappers_read_instruction_sysvar() {
    let input_mint = Pubkey::new_unique();
    let output_mint = Pubkey::new_unique();
    let shared_account = Pubkey::new_unique();
    let accounts = vec![AccountMeta::new_readonly(shared_account, false)];

    let first_ixs = vec![
        limo_instruction(test_args_data(7), accounts.clone()),
        limo_instruction(vec![1; 8], accounts.clone()),
    ];
    let mut first_sysvar = TestAccount::new(solana_program::sysvar::instructions::ID, limo::ID)
        .with_data(instructions_sysvar_data(&first_ixs, 1));
    let first_sysvar_info = first_sysvar.info();

    let first_args =
        ensure_first_ix_match::<TestArgs>(&first_sysvar_info, &input_mint, &output_mint).unwrap();
    assert_eq!(first_args, TestArgs { value: 7 });

    let second_ixs = vec![
        limo_instruction(vec![1; 8], accounts.clone()),
        limo_instruction(test_args_data(42), accounts),
    ];
    let mut second_sysvar = TestAccount::new(solana_program::sysvar::instructions::ID, limo::ID)
        .with_data(instructions_sysvar_data(&second_ixs, 0));
    let second_sysvar_info = second_sysvar.info();

    let second_args =
        ensure_second_ix_match::<TestArgs>(&second_sysvar_info, &input_mint, &output_mint)
            .unwrap();
    assert_eq!(second_args, TestArgs { value: 42 });
}

#[test]
fn ix_iterator_stops_at_invalid_argument() {
    let first_ix = instruction_with_accounts(vec![]);
    let second_ix = instruction_with_accounts(vec![]);
    let loader = FakeInstructionLoader {
        instructions: vec![first_ix.clone(), second_ix.clone()],
        error_at: None,
        current_index: 0,
    };
    let collected = IxIterator::new_at(0, &loader)
        .collect::<Result<Vec<_>, ProgramError>>()
        .unwrap();

    assert_eq!(collected, vec![first_ix, second_ix]);
}

#[test]
fn ix_iterator_propagates_non_terminal_loader_errors() {
    let loader = FakeInstructionLoader {
        instructions: vec![instruction_with_accounts(vec![])],
        error_at: Some(1),
        current_index: 0,
    };
    let mut iterator = IxIterator::new_at(0, &loader);

    assert!(iterator.next().unwrap().is_ok());
    assert_eq!(
        iterator.next().unwrap().unwrap_err(),
        ProgramError::InvalidAccountData
    );
}
