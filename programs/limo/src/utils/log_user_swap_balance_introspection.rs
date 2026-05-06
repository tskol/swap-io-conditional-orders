use anchor_lang::{
    prelude::*, solana_program::instruction::Instruction, AnchorDeserialize, Discriminator,
};

use super::flash_ixs::{check_same_accounts, ix_utils};
use crate::LimoError;

pub fn ensure_end_ix_match<T>(
    instruction_sysvar_account_info: &AccountInfo,
    swap_program_id: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let instruction_loader = ix_utils::BpfInstructionLoader {
        instruction_sysvar_account_info,
    };
    ensure_end_ix_match_internal(&instruction_loader, swap_program_id)
}

fn ensure_end_ix_match_internal<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    swap_program_id: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let current_idx = instruction_loader.load_current_index()?.into();
    let end_ix = search_end_ix(current_idx, instruction_loader, swap_program_id)?;

    if let Some(discriminator) = end_ix.data.get(..8) {
        if discriminator != T::discriminator() {
            msg!("End ix is not the expected one");
            return err!(LimoError::FlashTxWithUnexpectedIxs);
        }
    } else {
        msg!("End ix has no valid discriminator");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    }

    let current_ix = instruction_loader.load_instruction_at(current_idx)?;
    check_same_accounts(&current_ix, &end_ix)?;

    Ok(T::try_from_slice(&end_ix.data[8..])?)
}

fn search_end_ix(
    current_idx: usize,
    instruction_loader: &impl ix_utils::InstructionLoader,
    swap_program_id: &Pubkey,
) -> Result<Instruction> {
    let mut found_swap_ix = false;
    let mut found_end_ix = None;
    let mut ix_iterator =
        ix_utils::IxIterator::new_at(current_idx.checked_add(1).unwrap(), instruction_loader);

    for ix in ix_iterator.by_ref() {
        if let Err(error) = ix {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(error.into());
        }
        let ix = ix?;

        if ix.program_id == crate::id() {
            found_end_ix = Some(ix);
            break;
        } else if ix.program_id == *swap_program_id {
            if found_swap_ix {
                msg!("More than one swap instruction found between start and end");
                return err!(LimoError::FlashTxWithUnexpectedIxs);
            }
            found_swap_ix = true;
        } else {
            msg!("Unexpected instruction found between start and end");
            return err!(LimoError::FlashTxWithUnexpectedIxs);
        }
    }

    let end_ix = found_end_ix.ok_or_else(|| error!(LimoError::FlashIxsNotEnded))?;

    if !found_swap_ix {
        msg!("No swap instruction found between start and end");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    }

    Ok(end_ix)
}

pub fn ensure_start_ix_match<T>(
    instruction_sysvar_account_info: &AccountInfo,
    swap_program_id: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let instruction_loader = ix_utils::BpfInstructionLoader {
        instruction_sysvar_account_info,
    };
    ensure_start_ix_match_internal(&instruction_loader, swap_program_id)
}

fn ensure_start_ix_match_internal<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    swap_program_id: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let current_idx = instruction_loader.load_current_index()?.into();
    let start_ix = search_start_ix(current_idx, instruction_loader, swap_program_id)?;

    if let Some(discriminator) = start_ix.data.get(..8) {
        if discriminator != T::discriminator() {
            msg!("Start ix is not the expected one");
            return err!(LimoError::FlashTxWithUnexpectedIxs);
        }
    } else {
        msg!("Start ix has no valid discriminator");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    }

    let current_ix = instruction_loader.load_instruction_at(current_idx)?;
    check_same_accounts(&start_ix, &current_ix)?;

    Ok(T::try_from_slice(&start_ix.data[8..])?)
}

fn search_start_ix(
    current_idx: usize,
    instruction_loader: &impl ix_utils::InstructionLoader,
    swap_program_id: &Pubkey,
) -> Result<Instruction> {
    let mut found_swap_ix = false;
    let mut found_start_ix = None;

    for idx in (0..current_idx).rev() {
        let ix = instruction_loader.load_instruction_at(idx)?;
        msg!("ix {} ix program: {:?}", idx, ix.program_id);
        if ix.program_id == crate::id() {
            found_start_ix = Some(ix);
            break;
        } else if ix.program_id == *swap_program_id {
            if found_swap_ix || found_start_ix.is_some() {
                msg!("Multiple swap instructions or swap instruction before start ix");
                return err!(LimoError::FlashTxWithUnexpectedIxs);
            }
            found_swap_ix = true;
        } else if found_start_ix.is_some() {
            msg!("Unexpected instruction between start and end");
            return err!(LimoError::FlashTxWithUnexpectedIxs);
        }
    }

    let start_ix = found_start_ix.ok_or_else(|| error!(LimoError::FlashIxsNotStarted))?;

    if !found_swap_ix {
        msg!("No swap instruction found between start and end");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    }

    Ok(start_ix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::solana_program::{instruction::AccountMeta, program_error::ProgramError};

    struct FakeInstructionLoader {
        instructions: Vec<Instruction>,
        current_index: u16,
    }

    impl ix_utils::InstructionLoader for FakeInstructionLoader {
        fn load_instruction_at(
            &self,
            index: usize,
        ) -> std::result::Result<Instruction, ProgramError> {
            self.instructions
                .get(index)
                .cloned()
                .ok_or(ProgramError::InvalidArgument)
        }

        fn load_current_index(&self) -> std::result::Result<u16, ProgramError> {
            Ok(self.current_index)
        }
    }

    fn instruction(program_id: Pubkey) -> Instruction {
        Instruction {
            program_id,
            accounts: vec![AccountMeta::new_readonly(Pubkey::new_unique(), false)],
            data: vec![1; 8],
        }
    }

    #[test]
    fn search_start_ix_rejects_unexpected_instruction_between_start_and_end() {
        let swap_program_id = Pubkey::new_unique();
        let loader = FakeInstructionLoader {
            instructions: vec![
                instruction(crate::id()),
                instruction(Pubkey::new_unique()),
                instruction(swap_program_id),
                instruction(crate::id()),
            ],
            current_index: 3,
        };

        assert!(search_start_ix(3, &loader, &swap_program_id).is_err());
    }
}
