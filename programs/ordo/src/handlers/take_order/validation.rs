use anchor_lang::prelude::*;

use crate::{
    handlers::take_order::ExecuteOrder, utils::constraints::token_2022::validate_token_extensions,
    OrdoError,
};

pub(super) fn validate_take_order_token_extensions(ctx: &Context<ExecuteOrder>) -> Result<()> {
    validate_token_extensions(
        &ctx.accounts.input_mint.to_account_info(),
        vec![&ctx.accounts.taker_input_ata.to_account_info()],
    )?;

    if let Some(maker_output_ata_account) = ctx.accounts.maker_output_ata.as_ref() {
        validate_token_extensions(
            &ctx.accounts.output_mint.to_account_info(),
            vec![
                &ctx.accounts.taker_output_ata.to_account_info(),
                &maker_output_ata_account.to_account_info(),
            ],
        )?;
    } else {
        validate_token_extensions(
            &ctx.accounts.output_mint.to_account_info(),
            vec![&ctx.accounts.taker_output_ata.to_account_info()],
        )?;
    }

    Ok(())
}

pub(super) fn validate_child_order_accounts(ctx: &Context<ExecuteOrder>) -> Result<(bool, Pubkey)> {
    let order = &ctx.accounts.order.load()?;

    // Child order: require/validate parent, and validate brother iff it exists in the parent.
    if order.parent_order != Pubkey::default() {
        let parent_loader = ctx
            .accounts
            .parent_order
            .as_ref()
            .ok_or(OrdoError::InvalidAccount)?;
        require!(
            parent_loader.key() == order.parent_order,
            OrdoError::InvalidAccount
        );

        let parent = parent_loader.load()?;
        let this_order_key = ctx.accounts.order.key();
        let brother_key = if parent.tp_child_order == this_order_key {
            parent.sl_child_order
        } else if parent.sl_child_order == this_order_key {
            parent.tp_child_order
        } else {
            return err!(OrdoError::InvalidAccount);
        };

        if brother_key == Pubkey::default() {
            require!(ctx.accounts.brother_order.is_none(), OrdoError::InvalidAccount);
        } else {
            require!(
                ctx.accounts
                    .brother_order
                    .as_ref()
                    .map(|bo| bo.key() == brother_key)
                    .unwrap_or(false),
                OrdoError::InvalidAccount
            );
        }
    }

    Ok((order.permissionless != 0, order.counterparty))
}
