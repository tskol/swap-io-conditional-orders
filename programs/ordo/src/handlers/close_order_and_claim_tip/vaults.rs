use anchor_lang::prelude::*;

use crate::{
    handlers::close_order_and_claim_tip::ExitOrderAndClaimTip, state::Order,
    token_operations::transfer_from_vault_to_token_account, OrdoError,
};

pub(super) fn return_remaining_order_vaults(
    ctx: &Context<ExitOrderAndClaimTip>,
    order: &Order,
    seeds: &[&[u8]],
) -> Result<()> {
    if order.remaining_input_amount > 0 {
        ctx.accounts
            .transfer_input_vault_to(
                ctx.accounts.maker_input_ata.to_account_info(),
                seeds,
                order.remaining_input_amount,
            )
            .unwrap();
    }

    if order.available_child_input_amount > 0 {
        let maker_output_ata = ctx
            .accounts
            .maker_output_ata
            .as_ref()
            .ok_or(OrdoError::MakerOutputAtaRequired)?
            .to_account_info();
        let output_vault = ctx
            .accounts
            .output_vault
            .as_ref()
            .ok_or(OrdoError::OutputVaultRequired)?
            .to_account_info();
        ctx.accounts
            .transfer_output_vault_to(
                maker_output_ata,
                output_vault,
                seeds,
                order.available_child_input_amount,
            )
            .unwrap();
    }

    Ok(())
}

impl<'info> ExitOrderAndClaimTip<'info> {
    pub(super) fn transfer_input_vault_to(
        &self,
        destination_token_account: AccountInfo<'info>,
        seeds: &[&[u8]],
        amount: u64,
    ) -> Result<()> {
        transfer_from_vault_to_token_account(
            destination_token_account,
            self.input_vault.to_account_info(),
            self.pda_authority.to_account_info(),
            self.input_mint.to_account_info(),
            self.input_token_program.to_account_info(),
            seeds,
            amount,
            self.input_mint.decimals,
        )
    }

    pub(super) fn transfer_output_vault_to(
        &self,
        destination_token_account: AccountInfo<'info>,
        output_vault: AccountInfo<'info>,
        seeds: &[&[u8]],
        amount: u64,
    ) -> Result<()> {
        transfer_from_vault_to_token_account(
            destination_token_account,
            output_vault,
            self.pda_authority.to_account_info(),
            self.output_mint.to_account_info(),
            self.output_token_program.to_account_info(),
            seeds,
            amount,
            self.output_mint.decimals,
        )
    }
}
