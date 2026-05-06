use anchor_lang::prelude::Pubkey;
use limo::utils::flash_ixs::{
    check_same_accounts,
    ix_utils::{InstructionLoader, IxIterator},
};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
};

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
