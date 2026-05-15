use anchor_lang::{
    prelude::*, solana_program::instruction::Instruction, AnchorDeserialize, Discriminator,
};

use super::flash_ixs::ix_utils;
use super::ix_introspection::{
    deserialize_checked_paired_ordo_ix, ordo_ix_discriminator, PairedIxRole,
};
use crate::OrdoError;

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

    deserialize_checked_paired_ordo_ix(instruction_loader, current_idx, &end_ix, PairedIxRole::End)
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
                let discriminator = ordo_ix_discriminator(&ix)?;
                if discriminator.eq(end_ix_discriminator) {
                    if found_end_ix.is_some() {
                        msg!("Unexpected repeated end ix");
                        return err!(OrdoError::FlashTxWithUnexpectedIxs);
                    }
                    found_end_ix = Some(ix.clone());
                }
                if discriminator.eq(start_ix_discriminator) {
                    msg!("Unexpected repeated start ix");
                    return err!(OrdoError::FlashTxWithUnexpectedIxs);
                }
            }
        } else {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(ix_result.unwrap_err().into());
        }
    }

    let end_ix = found_end_ix.ok_or_else(|| error!(OrdoError::FlashIxsNotEnded))?;

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

    deserialize_checked_paired_ordo_ix(
        instruction_loader,
        current_idx,
        &start_ix,
        PairedIxRole::Start,
    )
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
            let discriminator = ordo_ix_discriminator(&ix)?;
            if discriminator.eq(start_ix_discriminator) {
                if found_start_ix.is_some() {
                    msg!("Unexpected instruction between start and end");
                    return err!(OrdoError::FlashTxWithUnexpectedIxs);
                }
                found_start_ix = Some(ix);
            } else if discriminator.eq(end_ix_discriminator) {
                msg!("Unexpected instruction between start and end");
                return err!(OrdoError::FlashTxWithUnexpectedIxs);
            }
        }
    }

    let start_ix = found_start_ix.ok_or_else(|| error!(OrdoError::FlashIxsNotStarted))?;

    Ok(start_ix)
}

#[cfg(test)]
#[path = "assert_user_swap_balance_introspection_tests.rs"]
mod tests;
