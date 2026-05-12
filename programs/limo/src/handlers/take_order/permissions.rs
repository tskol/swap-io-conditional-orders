use anchor_lang::prelude::*;

use crate::{
    handlers::take_order::TakeOrder, utils::constraints::is_counterparty_matching, LimoError,
};

pub(super) fn check_permission_and_get_tip(
    ctx: &Context<TakeOrder>,
    order_counterparty: &Pubkey,
    allowed_taker: &Pubkey,
    tip_amount_permissionless_taking: u64,
) -> Result<u64> {
    if !is_counterparty_matching(order_counterparty, allowed_taker, &ctx.accounts.taker.key()) {
        return err!(LimoError::CounterpartyDisallowed);
    }

    let tip = tip_amount_permissionless_taking;

    Ok(tip)
}
