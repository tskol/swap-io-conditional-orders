use anchor_lang::prelude::*;

use crate::{
    handlers::{
        close_order_and_claim_tip::{close_flow::close_order_and_claim_tip, ExitOrderAndClaimTip},
        order_events,
    },
    state::{GlobalConfig, Order},
    OrdoError,
};

pub(super) fn close_child_order_and_emit(
    ctx: &Context<ExitOrderAndClaimTip>,
    child_order_loader: Option<&AccountLoader<'_, Order>>,
    expected_child_order: Pubkey,
    global_config: &mut GlobalConfig,
    ts: u64,
    global_config_key: Pubkey,
) -> Result<()> {
    if expected_child_order == Pubkey::default() {
        return Ok(());
    }

    let child_order_loader = child_order_loader.ok_or(OrdoError::InvalidAccount)?;
    require!(
        child_order_loader.key() == expected_child_order,
        OrdoError::InvalidAccount
    );

    let child_order = &mut child_order_loader.load_mut()?;
    close_order_and_claim_tip(
        child_order,
        global_config,
        ts,
        global_config_key,
        &ctx.accounts.pda_authority.to_account_info(),
        &ctx.accounts.maker.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;
    emit_cpi!(order_events::order_display(child_order, 0, 0));
    Ok(())
}
