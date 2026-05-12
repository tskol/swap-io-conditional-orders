use anchor_lang::{
    prelude::*,
    solana_program::{
        instruction::{get_stack_height, TRANSACTION_LEVEL_STACK_HEIGHT},
        sysvar::instructions::get_instruction_relative,
    },
};

use crate::LimoError;

pub(super) fn check_cpi_not_allowed(sysvar_instructions: &AccountInfo) -> Result<()> {
    let instruction_sysvar_account = sysvar_instructions.to_account_info();
    let current_ix_program_id =
        get_instruction_relative(0, &instruction_sysvar_account)?.program_id;
    require_keys_eq!(current_ix_program_id, crate::ID, LimoError::CPINotAllowed);
    require!(
        get_stack_height() <= TRANSACTION_LEVEL_STACK_HEIGHT,
        LimoError::CPINotAllowed
    );
    Ok(())
}
