use anchor_lang::prelude::*;

use crate::{
    handlers::take_order::ExecuteOrder, utils::constraints::is_counterparty_matching, OrdoError,
};

pub(super) fn check_permission_and_get_tip(
    ctx: &Context<ExecuteOrder>,
    order_counterparty: &Pubkey,
    allowed_taker: &Pubkey,
    tip_amount_permissionless_taking: u64,
) -> Result<u64> {
    if !is_counterparty_matching(order_counterparty, allowed_taker, &ctx.accounts.taker.key()) {
        return err!(OrdoError::CounterpartyDisallowed);
    }

    let tip = tip_amount_permissionless_taking;

    Ok(tip)
}
