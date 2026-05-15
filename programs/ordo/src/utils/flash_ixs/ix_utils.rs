use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::Instruction,
        program_error::ProgramError,
        sysvar::instructions::{load_current_index_checked, load_instruction_at_checked},
    },
};

pub trait InstructionLoader {
    fn load_instruction_at(&self, index: usize) -> std::result::Result<Instruction, ProgramError>;
    fn load_current_index(&self) -> std::result::Result<u16, ProgramError>;
}

pub struct BpfInstructionLoader<'a, 'info> {
    pub instruction_sysvar_account_info: &'a AccountInfo<'info>,
}

impl<'a, 'info> InstructionLoader for BpfInstructionLoader<'a, 'info> {
    fn load_instruction_at(&self, index: usize) -> std::result::Result<Instruction, ProgramError> {
        load_instruction_at_checked(index, self.instruction_sysvar_account_info)
    }

    fn load_current_index(&self) -> std::result::Result<u16, ProgramError> {
        load_current_index_checked(self.instruction_sysvar_account_info)
    }
}

pub struct IxIterator<'a, IxLoader: InstructionLoader> {
    current_ix: usize,
    instruction_loader: &'a IxLoader,
}

impl<'a, IxLoader> IxIterator<'a, IxLoader>
where
    IxLoader: InstructionLoader,
{
    pub fn new_at(start_ix_index: usize, instruction_loader: &'a IxLoader) -> Self {
        Self {
            current_ix: start_ix_index,
            instruction_loader,
        }
    }
}

impl<IxLoader> Iterator for IxIterator<'_, IxLoader>
where
    IxLoader: InstructionLoader,
{
    type Item = std::result::Result<Instruction, ProgramError>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.instruction_loader.load_instruction_at(self.current_ix) {
            Ok(ix) => {
                self.current_ix = self.current_ix.checked_add(1).unwrap();
                Some(Ok(ix))
            }
            Err(ProgramError::InvalidArgument) => None,
            Err(e) => Some(Err(e)),
        }
    }
}
