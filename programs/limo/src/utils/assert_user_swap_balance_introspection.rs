use anchor_lang::{
    prelude::*, solana_program::instruction::Instruction, AnchorDeserialize, Discriminator,
};

use super::flash_ixs::{check_same_accounts, ix_utils};
use crate::LimoError;

pub fn ensure_end_ix_match<T>(
    instruction_sysvar_account_info: &AccountInfo,
    start_ix_discriminator: &[u8; 8],
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let instruction_loader = ix_utils::BpfInstructionLoader {
        instruction_sysvar_account_info,
    };
    ensure_end_ix_match_internal(&instruction_loader, start_ix_discriminator)
}

fn ensure_end_ix_match_internal<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    start_ix_discriminator: &[u8; 8],
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let current_idx = instruction_loader.load_current_index()?.into();
    let end_ix = search_end_ix(
        current_idx,
        instruction_loader,
        start_ix_discriminator,
        &T::discriminator(),
    )?;

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
    start_ix_discriminator: &[u8; 8],
    end_ix_discriminator: &[u8; 8],
) -> Result<Instruction> {
    let mut found_end_ix = None;
    let mut ix_iterator =
        ix_utils::IxIterator::new_at(current_idx.checked_add(1).unwrap(), instruction_loader);

    for ix_result in ix_iterator.by_ref() {
        if let Ok(ix) = ix_result {
            if ix.program_id == crate::id() {
                let discriminator = limo_ix_discriminator(&ix)?;
                if discriminator.eq(end_ix_discriminator) {
                    if found_end_ix.is_some() {
                        msg!("Unexpected repeated end ix");
                        return err!(LimoError::FlashTxWithUnexpectedIxs);
                    }
                    found_end_ix = Some(ix.clone());
                }
                if discriminator.eq(start_ix_discriminator) {
                    msg!("Unexpected repeated start ix");
                    return err!(LimoError::FlashTxWithUnexpectedIxs);
                }
            }
        } else {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(ix_result.unwrap_err().into());
        }
    }

    let end_ix = found_end_ix.ok_or_else(|| error!(LimoError::FlashIxsNotEnded))?;

    Ok(end_ix)
}

pub fn ensure_start_ix_match<T>(
    instruction_sysvar_account_info: &AccountInfo,
    end_ix_discriminator: &[u8; 8],
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let instruction_loader = ix_utils::BpfInstructionLoader {
        instruction_sysvar_account_info,
    };
    ensure_start_ix_match_internal(&instruction_loader, end_ix_discriminator)
}

fn ensure_start_ix_match_internal<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    end_ix_discriminator: &[u8; 8],
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let current_idx = instruction_loader.load_current_index()?.into();
    let start_ix = search_start_ix(
        current_idx,
        instruction_loader,
        &T::discriminator(),
        end_ix_discriminator,
    )?;

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
    start_ix_discriminator: &[u8; 8],
    end_ix_discriminator: &[u8; 8],
) -> Result<Instruction> {
    let mut found_start_ix = None;

    for idx in (0..current_idx).rev() {
        let ix = instruction_loader.load_instruction_at(idx)?;
        if ix.program_id == crate::id() {
            let discriminator = limo_ix_discriminator(&ix)?;
            if discriminator.eq(start_ix_discriminator) {
                if found_start_ix.is_some() {
                    msg!("Unexpected instruction between start and end");
                    return err!(LimoError::FlashTxWithUnexpectedIxs);
                }
                found_start_ix = Some(ix);
            } else if discriminator.eq(end_ix_discriminator) {
                msg!("Unexpected instruction between start and end");
                return err!(LimoError::FlashTxWithUnexpectedIxs);
            }
        }
    }

    let start_ix = found_start_ix.ok_or_else(|| error!(LimoError::FlashIxsNotStarted))?;

    Ok(start_ix)
}

fn limo_ix_discriminator(ix: &Instruction) -> Result<&[u8]> {
    let Some(discriminator) = ix.data.get(..8) else {
        msg!("Instruction has no valid discriminator");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    };

    Ok(discriminator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::solana_program::{instruction::AccountMeta, program_error::ProgramError};
    use anchor_lang::AnchorSerialize;

    const START_DISCRIMINATOR: [u8; 8] = [1; 8];
    const END_DISCRIMINATOR: [u8; 8] = [2; 8];
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

    fn limo_instruction(data: Vec<u8>) -> Instruction {
        instruction_with_accounts(crate::id(), data, shared_accounts())
    }

    fn limo_instruction_with_accounts(data: Vec<u8>, accounts: Vec<AccountMeta>) -> Instruction {
        instruction_with_accounts(crate::id(), data, accounts)
    }

    fn test_args_data(value: u64) -> Vec<u8> {
        let mut data = TestArgs::discriminator().to_vec();
        TestArgs { value }.serialize(&mut data).unwrap();
        data
    }

    #[test]
    fn ensure_end_ix_match_internal_deserializes_matching_end_ix() {
        let accounts = shared_accounts();
        let loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction_with_accounts(START_DISCRIMINATOR.to_vec(), accounts.clone()),
                instruction_with_accounts(Pubkey::new_unique(), vec![3; 8], shared_accounts()),
                limo_instruction_with_accounts(test_args_data(42), accounts),
            ],
            current_index: 0,
        };

        let args = ensure_end_ix_match_internal::<TestArgs>(&loader, &START_DISCRIMINATOR).unwrap();

        assert_eq!(args, TestArgs { value: 42 });
    }

    #[test]
    fn ensure_start_ix_match_internal_deserializes_matching_start_ix() {
        let accounts = shared_accounts();
        let loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction_with_accounts(test_args_data(7), accounts.clone()),
                instruction_with_accounts(Pubkey::new_unique(), vec![3; 8], shared_accounts()),
                limo_instruction_with_accounts(END_DISCRIMINATOR.to_vec(), accounts),
            ],
            current_index: 2,
        };

        let args = ensure_start_ix_match_internal::<TestArgs>(&loader, &END_DISCRIMINATOR).unwrap();

        assert_eq!(args, TestArgs { value: 7 });
    }

    #[test]
    fn search_end_ix_rejects_short_limo_discriminator() {
        let loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(START_DISCRIMINATOR.to_vec()),
                limo_instruction(vec![7; 4]),
            ],
            current_index: 0,
        };

        assert!(search_end_ix(0, &loader, &START_DISCRIMINATOR, &END_DISCRIMINATOR,).is_err());
    }

    #[test]
    fn search_end_ix_rejects_repeated_end_or_start_ix() {
        let repeated_end_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(START_DISCRIMINATOR.to_vec()),
                limo_instruction(END_DISCRIMINATOR.to_vec()),
                limo_instruction(END_DISCRIMINATOR.to_vec()),
            ],
            current_index: 0,
        };
        let repeated_start_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(START_DISCRIMINATOR.to_vec()),
                limo_instruction(START_DISCRIMINATOR.to_vec()),
            ],
            current_index: 0,
        };

        assert!(search_end_ix(
            0,
            &repeated_end_loader,
            &START_DISCRIMINATOR,
            &END_DISCRIMINATOR,
        )
        .is_err());
        assert!(search_end_ix(
            0,
            &repeated_start_loader,
            &START_DISCRIMINATOR,
            &END_DISCRIMINATOR,
        )
        .is_err());
    }

    #[test]
    fn search_start_ix_rejects_short_limo_discriminator() {
        let loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(vec![7; 4]),
                limo_instruction(END_DISCRIMINATOR.to_vec()),
            ],
            current_index: 1,
        };

        assert!(search_start_ix(1, &loader, &START_DISCRIMINATOR, &END_DISCRIMINATOR,).is_err());
    }

    #[test]
    fn search_start_ix_rejects_repeated_start_or_end_ix() {
        let repeated_start_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(START_DISCRIMINATOR.to_vec()),
                limo_instruction(START_DISCRIMINATOR.to_vec()),
                limo_instruction(END_DISCRIMINATOR.to_vec()),
            ],
            current_index: 2,
        };
        let repeated_end_loader = FakeInstructionLoader {
            instructions: vec![
                limo_instruction(START_DISCRIMINATOR.to_vec()),
                limo_instruction(END_DISCRIMINATOR.to_vec()),
                limo_instruction(END_DISCRIMINATOR.to_vec()),
            ],
            current_index: 2,
        };

        assert!(search_start_ix(
            2,
            &repeated_start_loader,
            &START_DISCRIMINATOR,
            &END_DISCRIMINATOR,
        )
        .is_err());
        assert!(search_start_ix(
            2,
            &repeated_end_loader,
            &START_DISCRIMINATOR,
            &END_DISCRIMINATOR,
        )
        .is_err());
    }
}
