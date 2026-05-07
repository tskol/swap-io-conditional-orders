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
            if found_swap_ix {
                msg!("More than one swap instruction found between start and end");
                return err!(LimoError::FlashTxWithUnexpectedIxs);
            }
            found_swap_ix = true;
        } else {
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
    use anchor_lang::AnchorSerialize;

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

    fn shared_accounts() -> Vec<AccountMeta> {
        vec![AccountMeta::new_readonly(Pubkey::new_unique(), false)]
    }

    fn instruction_with_accounts(
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

    fn limo_instruction(data: Vec<u8>, accounts: Vec<AccountMeta>) -> Instruction {
        instruction_with_accounts(crate::id(), data, accounts)
    }

    fn test_args_data(value: u64) -> Vec<u8> {
        let mut data = TestArgs::discriminator().to_vec();
        TestArgs { value }.serialize(&mut data).unwrap();
        data
    }

    #[test]
    fn ensure_start_ix_match_internal_deserializes_matching_start_ix() {
        let swap_program_id = Pubkey::new_unique();
        let accounts = shared_accounts();
        let loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(test_args_data(7), accounts.clone()),
                instruction(swap_program_id),
                limo_instruction(vec![1; 8], accounts),
            ],
            current_index: 2,
        };

        let args = ensure_start_ix_match_internal::<TestArgs>(&loader, &swap_program_id).unwrap();

        assert_eq!(args, TestArgs { value: 7 });
    }

    #[test]
    fn ensure_end_ix_match_internal_deserializes_matching_end_ix() {
        let swap_program_id = Pubkey::new_unique();
        let accounts = shared_accounts();
        let loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(vec![1; 8], accounts.clone()),
                instruction(swap_program_id),
                limo_instruction(test_args_data(42), accounts),
            ],
            current_index: 0,
        };

        let args = ensure_end_ix_match_internal::<TestArgs>(&loader, &swap_program_id).unwrap();

        assert_eq!(args, TestArgs { value: 42 });
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

    #[test]
    fn search_start_ix_accepts_single_swap_between_start_and_end() {
        let swap_program_id = Pubkey::new_unique();
        let loader = FakeInstructionLoader {
            instructions: vec![
                instruction(crate::id()),
                instruction(swap_program_id),
                instruction(crate::id()),
            ],
            current_index: 2,
        };

        let start_ix = search_start_ix(2, &loader, &swap_program_id).unwrap();

        assert_eq!(start_ix.program_id, crate::id());
    }

    #[test]
    fn search_end_ix_accepts_single_swap_between_start_and_end() {
        let swap_program_id = Pubkey::new_unique();
        let loader = FakeInstructionLoader {
            instructions: vec![
                instruction(crate::id()),
                instruction(swap_program_id),
                instruction(crate::id()),
            ],
            current_index: 0,
        };

        let end_ix = search_end_ix(0, &loader, &swap_program_id).unwrap();

        assert_eq!(end_ix.program_id, crate::id());
    }

    #[test]
    fn search_start_ix_rejects_missing_or_repeated_swap() {
        let swap_program_id = Pubkey::new_unique();
        let missing_swap_loader = FakeInstructionLoader {
            instructions: vec![instruction(crate::id()), instruction(crate::id())],
            current_index: 1,
        };
        let repeated_swap_loader = FakeInstructionLoader {
            instructions: vec![
                instruction(crate::id()),
                instruction(swap_program_id),
                instruction(swap_program_id),
                instruction(crate::id()),
            ],
            current_index: 3,
        };

        assert!(search_start_ix(1, &missing_swap_loader, &swap_program_id).is_err());
        assert!(search_start_ix(3, &repeated_swap_loader, &swap_program_id).is_err());
    }

    #[test]
    fn search_end_ix_rejects_unexpected_instruction_between_start_and_end() {
        let swap_program_id = Pubkey::new_unique();
        let loader = FakeInstructionLoader {
            instructions: vec![
                instruction(crate::id()),
                instruction(swap_program_id),
                instruction(Pubkey::new_unique()),
                instruction(crate::id()),
            ],
            current_index: 0,
        };

        assert!(search_end_ix(0, &loader, &swap_program_id).is_err());
    }

    #[test]
    fn ensure_log_ixs_reject_bad_discriminator_or_accounts() {
        let swap_program_id = Pubkey::new_unique();
        let accounts = shared_accounts();
        let wrong_discriminator_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(vec![8; 8], accounts.clone()),
                instruction(swap_program_id),
                limo_instruction(vec![1; 8], accounts),
            ],
            current_index: 2,
        };
        let short_discriminator_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(vec![1; 4], shared_accounts()),
                instruction(swap_program_id),
                limo_instruction(vec![1; 8], shared_accounts()),
            ],
            current_index: 2,
        };
        let account_mismatch_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(test_args_data(5), shared_accounts()),
                instruction(swap_program_id),
                limo_instruction(vec![1; 8], shared_accounts()),
            ],
            current_index: 2,
        };

        assert!(ensure_start_ix_match_internal::<TestArgs>(
            &wrong_discriminator_loader,
            &swap_program_id,
        )
        .is_err());
        assert!(ensure_start_ix_match_internal::<TestArgs>(
            &short_discriminator_loader,
            &swap_program_id,
        )
        .is_err());
        assert!(ensure_start_ix_match_internal::<TestArgs>(
            &account_mismatch_loader,
            &swap_program_id,
        )
        .is_err());
    }

    #[test]
    fn search_end_ix_rejects_missing_or_repeated_swap() {
        let swap_program_id = Pubkey::new_unique();
        let missing_swap_loader = FakeInstructionLoader {
            instructions: vec![instruction(crate::id()), instruction(crate::id())],
            current_index: 0,
        };
        let repeated_swap_loader = FakeInstructionLoader {
            instructions: vec![
                instruction(crate::id()),
                instruction(swap_program_id),
                instruction(swap_program_id),
                instruction(crate::id()),
            ],
            current_index: 0,
        };

        assert!(search_end_ix(0, &missing_swap_loader, &swap_program_id).is_err());
        assert!(search_end_ix(0, &repeated_swap_loader, &swap_program_id).is_err());
    }
}
