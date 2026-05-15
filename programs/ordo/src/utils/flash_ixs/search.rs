use super::*;

pub(super) fn ensure_second_ix_match_internal<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let current_idx = instruction_loader.load_current_index()?.into();
    let second_ix = search_second_ix(current_idx, instruction_loader, input_mint, output_mint)?;
    if let Some(discriminator) = second_ix.data.get(..8) {
        if discriminator != T::discriminator() {
            msg!("Extra ix is not the expected one");
            return err!(OrdoError::FlashTxWithUnexpectedIxs);
        }
    } else {
        msg!("Extra ix has no valid discriminator");
        return err!(OrdoError::FlashTxWithUnexpectedIxs);
    }

    let current_ix = instruction_loader.load_instruction_at(current_idx)?;
    check_same_accounts(&current_ix, &second_ix)?;

    Ok(T::try_from_slice(&second_ix.data[8..])?)
}

pub(super) fn search_second_ix(
    current_idx: usize,
    instruction_loader: &impl ix_utils::InstructionLoader,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<Instruction> {
    for idx in 0..current_idx {
        let ix = instruction_loader.load_instruction_at(idx)?;

        require!(
            program_id_allowed(ix.program_id),
            OrdoError::FlashTxWithUnexpectedIxs
        );

        if ix.program_id == token_2022::ID {
            token_2022_verify_ix_and_mints(&ix, input_mint, output_mint)?;
        }
    }

    let mut found_extra_ix = None;
    let mut ix_iterator =
        ix_utils::IxIterator::new_at(current_idx.checked_add(1).unwrap(), instruction_loader);

    for ix in ix_iterator.by_ref() {
        if let Err(error) = ix {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(error.into());
        }
        let ix = ix?;
        if ix.program_id == crate::id() {
            found_extra_ix = Some(ix);
            break;
        }
    }

    let extra_ix = found_extra_ix.ok_or_else(|| error!(OrdoError::FlashIxsNotEnded))?;

    for ix in ix_iterator.by_ref() {
        if let Err(error) = ix {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(error.into());
        }
        let ix = ix?;
        require!(
            program_id_allowed(ix.program_id),
            OrdoError::FlashTxWithUnexpectedIxs
        );
        if ix.program_id == token_2022::ID {
            token_2022_verify_ix_and_mints(&ix, input_mint, output_mint)?;
        }
    }

    Ok(extra_ix)
}

pub(super) fn ensure_first_ix_match_internal<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let current_idx = instruction_loader.load_current_index()?.into();
    let first_ix = search_first_ix(current_idx, instruction_loader, input_mint, output_mint)?;
    if let Some(discriminator) = first_ix.data.get(..8) {
        if discriminator != T::discriminator() {
            msg!("Extra ix is not the expected one");
            return err!(OrdoError::FlashTxWithUnexpectedIxs);
        }
    } else {
        msg!("Extra ix has no valid discriminator");
        return err!(OrdoError::FlashTxWithUnexpectedIxs);
    }

    let current_ix = instruction_loader.load_instruction_at(current_idx)?;
    check_same_accounts(&first_ix, &current_ix)?;

    Ok(T::try_from_slice(&first_ix.data[8..])?)
}

pub(super) fn search_first_ix(
    current_idx: usize,
    instruction_loader: &impl ix_utils::InstructionLoader,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<Instruction> {
    let mut ix_iterator =
        ix_utils::IxIterator::new_at(current_idx.checked_add(1).unwrap(), instruction_loader);

    for ix in ix_iterator.by_ref() {
        if let Err(error) = ix {
            msg!("Unexpected error encountered while iterating over instructions");
            return Err(error.into());
        }
        let ix = ix?;
        require!(
            program_id_allowed(ix.program_id),
            OrdoError::FlashTxWithUnexpectedIxs
        );
        if ix.program_id == token_2022::ID {
            token_2022_verify_ix_and_mints(&ix, input_mint, output_mint)?;
        }
    }

    let mut found_extra_ix = None;

    for idx in 0..current_idx {
        let ix = instruction_loader.load_instruction_at(idx)?;
        if ix.program_id == crate::id() {
            found_extra_ix = Some(ix);
            break;
        } else {
            require!(
                program_id_allowed(ix.program_id),
                OrdoError::FlashTxWithUnexpectedIxs
            );
            if ix.program_id == token_2022::ID {
                token_2022_verify_ix_and_mints(&ix, input_mint, output_mint)?;
            }
        }
    }

    let extra_ix = found_extra_ix.ok_or_else(|| error!(OrdoError::FlashIxsNotStarted))?;

    Ok(extra_ix)
}
