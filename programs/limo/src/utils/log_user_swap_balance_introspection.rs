use anchor_lang::{
    prelude::*, solana_program::instruction::Instruction, AnchorDeserialize, Discriminator,
};

use super::flash_ixs::ix_utils;
use super::ix_introspection::{deserialize_checked_paired_limo_ix, PairedIxRole};
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

    deserialize_checked_paired_limo_ix(instruction_loader, current_idx, &end_ix, PairedIxRole::End)
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

        if classify_boundary_or_single_swap_ix(
            &ix,
            swap_program_id,
            &mut found_swap_ix,
            "Unexpected instruction found between start and end",
        )? {
            found_end_ix = Some(ix);
            break;
        }
    }

    let end_ix = found_end_ix.ok_or_else(|| error!(LimoError::FlashIxsNotEnded))?;

    ensure_swap_ix_found(found_swap_ix)?;

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

    deserialize_checked_paired_limo_ix(
        instruction_loader,
        current_idx,
        &start_ix,
        PairedIxRole::Start,
    )
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
        if classify_boundary_or_single_swap_ix(
            &ix,
            swap_program_id,
            &mut found_swap_ix,
            "Unexpected instruction between start and end",
        )? {
            found_start_ix = Some(ix);
            break;
        }
    }

    let start_ix = found_start_ix.ok_or_else(|| error!(LimoError::FlashIxsNotStarted))?;

    ensure_swap_ix_found(found_swap_ix)?;

    Ok(start_ix)
}

fn classify_boundary_or_single_swap_ix(
    ix: &Instruction,
    swap_program_id: &Pubkey,
    found_swap_ix: &mut bool,
    unexpected_ix_message: &'static str,
) -> Result<bool> {
    if ix.program_id == crate::id() {
        return Ok(true);
    }

    if ix.program_id == *swap_program_id {
        if *found_swap_ix {
            msg!("More than one swap instruction found between start and end");
            return err!(LimoError::FlashTxWithUnexpectedIxs);
        }
        *found_swap_ix = true;
        return Ok(false);
    }

    msg!("{}", unexpected_ix_message);
    err!(LimoError::FlashTxWithUnexpectedIxs)
}

fn ensure_swap_ix_found(found_swap_ix: bool) -> Result<()> {
    if !found_swap_ix {
        msg!("No swap instruction found between start and end");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    }

    Ok(())
}

#[cfg(test)]
#[path = "log_user_swap_balance_introspection_tests.rs"]
mod tests;
