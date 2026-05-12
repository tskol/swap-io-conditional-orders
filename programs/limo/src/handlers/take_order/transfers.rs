use anchor_lang::prelude::*;

use crate::{
    global_seeds,
    handlers::take_order::TakeOrder,
    intermediary_seeds,
    operations::validate_pda_authority_balance_and_update_accounting,
    seeds::{GLOBAL_AUTH, INTERMEDIARY_OUTPUT_TOKEN_ACCOUNT},
    state::GlobalConfig,
    token_operations::{
        close_ata_accounts_with_signer_seeds,
        initialize_intermediary_token_account_with_signer_seeds,
        native_transfer_from_authority_to_user, transfer_from_user_to_token_account,
        transfer_from_vault_to_token_account,
    },
    utils::constraints::{is_wsol, verify_ata},
    LimoError, OrderType,
};

pub(super) fn transfer_output_and_input(
    ctx: &Context<TakeOrder>,
    global_config: &mut GlobalConfig,
    order_type: u8,
    input_to_send_to_taker: u64,
    output_to_send_to_maker: u64,
    output_to_send_to_protocol: u64,
    output_keeper_fee: u64,
) -> Result<()> {
    let gc = ctx.accounts.global_config.key();
    let seeds: &[&[u8]] = global_seeds!(global_config.pda_authority_bump as u8, &gc);

    let output_is_wsol = is_wsol(&ctx.accounts.output_mint.key());
    let order_is_limit_parent = order_type == OrderType::LimitParent as u8;
    let output_destination_token_account = if order_is_limit_parent {
        let output_vault = ctx
            .accounts
            .output_vault
            .as_ref()
            .ok_or(LimoError::OutputVaultRequired)?;
        output_vault.to_account_info()
    } else if output_is_wsol {
        let intermediary_output_token_account = ctx
            .accounts
            .intermediary_output_token_account
            .as_ref()
            .ok_or(LimoError::IntermediaryOutputTokenAccountRequired)?;
        let order_key = ctx.accounts.order.key();
        let token_account_signer_seeds: &[&[u8]] =
            intermediary_seeds!(ctx.bumps.intermediary_output_token_account, &order_key);
        initialize_intermediary_token_account_with_signer_seeds(
            intermediary_output_token_account.to_account_info().clone(),
            ctx.accounts.output_mint.to_account_info(),
            ctx.accounts.output_token_program.to_account_info(),
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.rent.to_account_info(),
            token_account_signer_seeds,
            seeds,
        )?;

        intermediary_output_token_account.to_account_info()
    } else {
        let maker_output_ata_account = ctx
            .accounts
            .maker_output_ata
            .as_ref()
            .ok_or(LimoError::MakerOutputAtaRequired)?;
        verify_ata(
            &ctx.accounts.maker.key(),
            &ctx.accounts.output_mint.key(),
            &maker_output_ata_account.key(),
            &ctx.accounts.output_token_program.key(),
        )?;
        maker_output_ata_account.to_account_info()
    };

    let output_to_send_to_maker_with_keeper_fee = output_to_send_to_maker
        .checked_sub(output_keeper_fee)
        .ok_or(LimoError::MathOverflow)
        .unwrap();

    transfer_from_user_to_token_account(
        ctx.accounts.taker_output_ata.to_account_info(),
        output_destination_token_account.clone(),
        ctx.accounts.taker.to_account_info(),
        ctx.accounts.output_mint.to_account_info(),
        ctx.accounts.output_token_program.to_account_info(),
        output_to_send_to_maker_with_keeper_fee,
        ctx.accounts.output_mint.decimals,
    )?;

    if !order_is_limit_parent && output_is_wsol {
        close_ata_accounts_with_signer_seeds(
            output_destination_token_account,
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.output_token_program.to_account_info(),
            seeds,
        )?;
        native_transfer_from_authority_to_user(
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.maker.to_account_info(),
            seeds,
            output_to_send_to_maker_with_keeper_fee,
        )?;
    }

    if output_to_send_to_protocol > 0 {
        transfer_from_user_to_token_account(
            ctx.accounts.taker_output_ata.to_account_info(),
            ctx.accounts.output_fee_vault.to_account_info(),
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.output_mint.to_account_info(),
            ctx.accounts.output_token_program.to_account_info(),
            output_to_send_to_protocol,
            ctx.accounts.output_mint.decimals,
        )?;
    }

    transfer_from_vault_to_token_account(
        ctx.accounts.taker_input_ata.to_account_info(),
        ctx.accounts.input_vault.to_account_info(),
        ctx.accounts.pda_authority.to_account_info(),
        ctx.accounts.input_mint.to_account_info(),
        ctx.accounts.input_token_program.to_account_info(),
        seeds,
        input_to_send_to_taker,
        ctx.accounts.input_mint.decimals,
    )?;

    Ok(())
}

pub(super) fn tip_transfer_and_validation(
    ctx: &Context<TakeOrder>,
    global_config: &mut GlobalConfig,
    tip: u64,
) -> Result<()> {
    let pda_authority_balance = ctx.accounts.pda_authority.lamports();
    validate_pda_authority_balance_and_update_accounting(
        global_config,
        pda_authority_balance,
        tip,
    )?;
    Ok(())
}
