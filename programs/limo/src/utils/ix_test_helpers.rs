use anchor_lang::{
    prelude::Pubkey,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
    },
    AnchorDeserialize, AnchorSerialize, Discriminator,
};

use super::flash_ixs::ix_utils;

const TEST_ARGS_DISCRIMINATOR: [u8; 8] = [9; 8];

#[derive(AnchorSerialize, AnchorDeserialize, Debug, PartialEq, Eq)]
pub(crate) struct TestArgs {
    pub value: u64,
}

impl Discriminator for TestArgs {
    const DISCRIMINATOR: [u8; 8] = TEST_ARGS_DISCRIMINATOR;
}

pub(crate) struct FakeInstructionLoader {
    pub instructions: Vec<Instruction>,
    pub current_index: u16,
}

impl ix_utils::InstructionLoader for FakeInstructionLoader {
    fn load_instruction_at(&self, index: usize) -> std::result::Result<Instruction, ProgramError> {
        self.instructions
            .get(index)
            .cloned()
            .ok_or(ProgramError::InvalidArgument)
    }

    fn load_current_index(&self) -> std::result::Result<u16, ProgramError> {
        Ok(self.current_index)
    }
}

pub(crate) fn shared_accounts() -> Vec<AccountMeta> {
    vec![AccountMeta::new_readonly(Pubkey::new_unique(), false)]
}

pub(crate) fn instruction(program_id: Pubkey) -> Instruction {
    instruction_with_accounts(program_id, vec![1; 8], shared_accounts())
}

pub(crate) fn instruction_with_accounts(
    program_id: Pubkey,
    data: Vec<u8>,
    accounts: Vec<AccountMeta>,
) -> Instruction {
    Instruction {
        program_id,
        accounts,
        data,
    }
}

pub(crate) fn limo_instruction(data: Vec<u8>) -> Instruction {
    limo_instruction_with_accounts(data, shared_accounts())
}

pub(crate) fn limo_instruction_with_accounts(
    data: Vec<u8>,
    accounts: Vec<AccountMeta>,
) -> Instruction {
    instruction_with_accounts(crate::id(), data, accounts)
}

pub(crate) fn test_args_data(value: u64) -> Vec<u8> {
    let mut data = TestArgs::discriminator().to_vec();
    TestArgs { value }.serialize(&mut data).unwrap();
    data
}
