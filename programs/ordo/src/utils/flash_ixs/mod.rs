use anchor_lang::{
    prelude::*, solana_program::instruction::Instruction, AnchorDeserialize, Discriminator,
};
use anchor_spl::{
    associated_token,
    token::spl_token,
    token_2022::{self, spl_token_2022::instruction::TokenInstruction},
};
use solana_program::pubkey;

const COMPUTE_BUDGET_PUBKEY: Pubkey = pubkey!("ComputeBudget111111111111111111111111111111");

mod accounts;
pub mod ix_utils;
mod search;
mod token_2022_guard;

pub use accounts::check_same_accounts;
use search::{ensure_first_ix_match_internal, ensure_second_ix_match_internal};
#[cfg(test)]
use search::{search_first_ix, search_second_ix};
use token_2022_guard::token_2022_verify_ix_and_mints;

use crate::OrdoError;

pub fn ensure_second_ix_match<T>(
    instruction_sysvar_account_info: &AccountInfo,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let instruction_loader = ix_utils::BpfInstructionLoader {
        instruction_sysvar_account_info,
    };
    ensure_second_ix_match_internal(&instruction_loader, input_mint, output_mint)
}

pub fn ensure_first_ix_match<T>(
    instruction_sysvar_account_info: &AccountInfo,
    input_mint: &Pubkey,
    output_mint: &Pubkey,
) -> Result<T>
where
    T: Discriminator + AnchorDeserialize,
{
    let instruction_loader = ix_utils::BpfInstructionLoader {
        instruction_sysvar_account_info,
    };
    ensure_first_ix_match_internal(&instruction_loader, input_mint, output_mint)
}

fn program_id_allowed(program_id: Pubkey) -> bool {
    program_id == COMPUTE_BUDGET_PUBKEY
        || program_id == spl_token::ID
        || program_id == token_2022::ID
        || program_id == associated_token::ID
}

#[cfg(test)]
#[path = "../flash_ixs_tests.rs"]
mod tests;
