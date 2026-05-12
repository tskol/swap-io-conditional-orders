use anchor_lang::prelude::*;

use crate::{
    handlers::close_order_and_claim_tip::CloseOrderAndClaimTip,
    operations,
    state::{GlobalConfig, Order},
    LimoError,
};

pub(super) fn pay_allowed_taker_close_fee(
    ctx: &Context<CloseOrderAndClaimTip>,
    global_config: &GlobalConfig,
    order: &mut Order,
    seeds: &[&[u8]],
) -> Result<()> {
    let is_allowed_taker_closer = ctx.accounts.closer.key() == global_config.allowed_taker;
    if !is_allowed_taker_closer || global_config.keeper_close_fee_bps == 0 {
        return Ok(());
    }

    let fee_input_total = operations::calculate_fee_amount(
        order.initial_input_amount,
        global_config.keeper_close_fee_bps,
    )?;

    if order.remaining_input_amount >= fee_input_total {
        let closer_input_ata = ctx
            .accounts
            .closer_input_ata
            .as_ref()
            .ok_or(LimoError::InvalidAccount)?;
        ctx.accounts.transfer_input_vault_to(
            closer_input_ata.to_account_info(),
            seeds,
            fee_input_total,
        )?;
        order.remaining_input_amount -= fee_input_total;
        return Ok(());
    }

    let child_initial = child_initial_input_amount(ctx)?;
    let fee_from_child =
        operations::calculate_fee_amount(child_initial, global_config.keeper_close_fee_bps)?;
    if order.available_child_input_amount >= fee_from_child && fee_from_child > 0 {
        let closer_output_ata = ctx
            .accounts
            .closer_output_ata
            .as_ref()
            .ok_or(LimoError::InvalidAccount)?;
        let output_vault = ctx
            .accounts
            .output_vault
            .as_ref()
            .ok_or(LimoError::OutputVaultRequired)?;
        ctx.accounts.transfer_output_vault_to(
            closer_output_ata.to_account_info(),
            output_vault.to_account_info(),
            seeds,
            fee_from_child,
        )?;
        order.available_child_input_amount -= fee_from_child;
    }

    Ok(())
}

pub(super) fn child_initial_input_amount(ctx: &Context<CloseOrderAndClaimTip>) -> Result<u64> {
    if let Some(ref loader) = ctx.accounts.tp_child_order {
        Ok(loader.load()?.initial_input_amount)
    } else if let Some(ref loader) = ctx.accounts.sl_child_order {
        Ok(loader.load()?.initial_input_amount)
    } else {
        Ok(0)
    }
}
