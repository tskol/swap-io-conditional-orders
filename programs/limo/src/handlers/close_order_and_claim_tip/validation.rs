use anchor_lang::prelude::*;

use crate::{
    handlers::close_order_and_claim_tip::CloseOrderAndClaimTip, state::OrderType,
    utils::constraints::token_2022::validate_token_extensions, LimoError,
};

pub(super) fn validate_close_order_token_extensions(
    ctx: &Context<CloseOrderAndClaimTip>,
) -> Result<()> {
    validate_token_extensions(
        &ctx.accounts.input_mint.to_account_info(),
        vec![&ctx.accounts.maker_input_ata.to_account_info()],
    )
}

pub(super) fn validate_close_order_type(order_type: u8) -> Result<OrderType> {
    let parsed_order_type =
        OrderType::try_from(order_type).map_err(|_| LimoError::OrderTypeInvalid)?;
    require!(
        parsed_order_type == OrderType::LimitParent || parsed_order_type == OrderType::Vanilla,
        LimoError::OrderTypeInvalid
    );
    Ok(parsed_order_type)
}

pub(super) fn validate_closer(
    closer: Pubkey,
    maker: Pubkey,
    allowed_taker: Pubkey,
    is_order_expired: bool,
) -> Result<()> {
    require!(
        is_order_expired && closer == allowed_taker || closer == maker,
        LimoError::InvalidAccount
    );
    Ok(())
}
