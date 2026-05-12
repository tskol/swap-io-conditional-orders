use anchor_lang::{
    prelude::*, solana_program::instruction::Instruction, AnchorDeserialize, Discriminator,
};

use super::flash_ixs::{check_same_accounts, ix_utils};
use crate::LimoError;

pub(crate) enum PairedIxRole {
    Start,
    End,
}

impl PairedIxRole {
    fn label(&self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::End => "End",
        }
    }
}

pub(crate) fn deserialize_checked_paired_limo_ix<T>(
    instruction_loader: &impl ix_utils::InstructionLoader,
    current_idx: usize,
    paired_ix: &Instruction,
    paired_ix_role: PairedIxRole,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let deserialized_ix = deserialize_limo_ix(paired_ix, paired_ix_role.label())?;

    let current_ix = instruction_loader.load_instruction_at(current_idx)?;
    match paired_ix_role {
        PairedIxRole::Start => check_same_accounts(paired_ix, &current_ix)?,
        PairedIxRole::End => check_same_accounts(&current_ix, paired_ix)?,
    }

    Ok(deserialized_ix)
}

pub(crate) fn deserialize_limo_ix<T>(ix: &Instruction, label: &str) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let Some(discriminator) = ix.data.get(..8) else {
        msg!("{} ix has no valid discriminator", label);
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    };

    if discriminator != T::discriminator() {
        msg!("{} ix is not the expected one", label);
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    }

    Ok(T::try_from_slice(&ix.data[8..])?)
}

pub(crate) fn limo_ix_discriminator(ix: &Instruction) -> Result<&[u8]> {
    let Some(discriminator) = ix.data.get(..8) else {
        msg!("Instruction has no valid discriminator");
        return err!(LimoError::FlashTxWithUnexpectedIxs);
    };

    Ok(discriminator)
}
