use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    global_seeds, operations,
    seeds::{self, GLOBAL_AUTH},
    state::{Order, OrderType},
    token_operations::{
        lamports_transfer_from_authority_to_account, transfer_from_vault_to_token_account,
    },
    utils::constraints::token_2022::validate_token_extensions,
    GlobalConfig, LimoError, OrderDisplay,
};

pub fn handler_close_order_and_claim_tip(ctx: Context<CloseOrderAndClaimTip>) -> Result<()> {
    validate_close_order_token_extensions(&ctx)?;

    let global_config_key = ctx.accounts.global_config.key();
    let order = &mut ctx.accounts.order.load_mut()?;
    let global_config = &mut ctx.accounts.global_config.load_mut()?;
    let parsed_order_type = validate_close_order_type(order.order_type)?;
    let ts = u64::try_from(Clock::get()?.unix_timestamp).unwrap();
    // 0 means "no expiry" (backward compatibility: old orders had padding here)
    let is_order_expired = order.expiry_timestamp != 0 && order.expiry_timestamp < ts;

    validate_closer(
        ctx.accounts.closer.key(),
        ctx.accounts.maker.key(),
        global_config.allowed_taker,
        is_order_expired,
    )?;

    if parsed_order_type == OrderType::LimitParent {
        close_child_order_and_emit(
            &ctx,
            ctx.accounts.tp_child_order.as_ref(),
            order.tp_child_order,
            global_config,
            ts,
            global_config_key,
        )?;
        close_child_order_and_emit(
            &ctx,
            ctx.accounts.sl_child_order.as_ref(),
            order.sl_child_order,
            global_config,
            ts,
            global_config_key,
        )?;
    }

    close_order_and_claim_tip(
        order,
        global_config,
        ts,
        global_config_key,
        &ctx.accounts.pda_authority.to_account_info(),
        &ctx.accounts.maker.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;

    let gc = global_config_key;
    let seeds: &[&[u8]] = global_seeds!(global_config.pda_authority_bump as u8, &gc);
    pay_allowed_taker_close_fee(&ctx, global_config, order, seeds)?;
    return_remaining_order_vaults(&ctx, order, seeds)?;

    global_config.pda_authority_previous_lamports_balance = ctx.accounts.pda_authority.lamports();
    emit_order_display(&ctx, order)?;

    Ok(())
}

fn validate_close_order_token_extensions(ctx: &Context<CloseOrderAndClaimTip>) -> Result<()> {
    validate_token_extensions(
        &ctx.accounts.input_mint.to_account_info(),
        vec![&ctx.accounts.maker_input_ata.to_account_info()],
    )
}

fn validate_close_order_type(order_type: u8) -> Result<OrderType> {
    let parsed_order_type =
        OrderType::try_from(order_type).map_err(|_| LimoError::OrderTypeInvalid)?;
    require!(
        parsed_order_type == OrderType::LimitParent || parsed_order_type == OrderType::Vanilla,
        LimoError::OrderTypeInvalid
    );
    Ok(parsed_order_type)
}

fn close_child_order_and_emit(
    ctx: &Context<CloseOrderAndClaimTip>,
    child_order_loader: Option<&AccountLoader<'_, Order>>,
    expected_child_order: Pubkey,
    global_config: &mut GlobalConfig,
    ts: u64,
    global_config_key: Pubkey,
) -> Result<()> {
    if expected_child_order == Pubkey::default() {
        return Ok(());
    }

    let child_order_loader = child_order_loader.ok_or(LimoError::InvalidAccount)?;
    require!(
        child_order_loader.key() == expected_child_order,
        LimoError::InvalidAccount
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
    emit_order_display(ctx, child_order)
}

fn pay_allowed_taker_close_fee(
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
        transfer_from_vault_to_token_account(
            closer_input_ata.to_account_info(),
            ctx.accounts.input_vault.to_account_info(),
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.input_mint.to_account_info(),
            ctx.accounts.input_token_program.to_account_info(),
            seeds,
            fee_input_total,
            ctx.accounts.input_mint.decimals,
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
        transfer_from_vault_to_token_account(
            closer_output_ata.to_account_info(),
            output_vault.to_account_info(),
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.output_mint.to_account_info(),
            ctx.accounts.output_token_program.to_account_info(),
            seeds,
            fee_from_child,
            ctx.accounts.output_mint.decimals,
        )?;
        order.available_child_input_amount -= fee_from_child;
    }

    Ok(())
}

fn child_initial_input_amount(ctx: &Context<CloseOrderAndClaimTip>) -> Result<u64> {
    if let Some(ref loader) = ctx.accounts.tp_child_order {
        Ok(loader.load()?.initial_input_amount)
    } else if let Some(ref loader) = ctx.accounts.sl_child_order {
        Ok(loader.load()?.initial_input_amount)
    } else {
        Ok(0)
    }
}

fn return_remaining_order_vaults(
    ctx: &Context<CloseOrderAndClaimTip>,
    order: &Order,
    seeds: &[&[u8]],
) -> Result<()> {
    if order.remaining_input_amount > 0 {
        transfer_from_vault_to_token_account(
            ctx.accounts.maker_input_ata.to_account_info(),
            ctx.accounts.input_vault.to_account_info(),
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.input_mint.to_account_info(),
            ctx.accounts.input_token_program.to_account_info(),
            seeds,
            order.remaining_input_amount,
            ctx.accounts.input_mint.decimals,
        )
        .unwrap();
    }

    if order.available_child_input_amount > 0 {
        let maker_output_ata = ctx
            .accounts
            .maker_output_ata
            .as_ref()
            .ok_or(LimoError::MakerOutputAtaRequired)?
            .to_account_info();
        let output_vault = ctx
            .accounts
            .output_vault
            .as_ref()
            .ok_or(LimoError::OutputVaultRequired)?
            .to_account_info();
        transfer_from_vault_to_token_account(
            maker_output_ata,
            output_vault,
            ctx.accounts.pda_authority.to_account_info(),
            ctx.accounts.output_mint.to_account_info(),
            ctx.accounts.output_token_program.to_account_info(),
            seeds,
            order.available_child_input_amount,
            ctx.accounts.output_mint.decimals,
        )
        .unwrap();
    }

    Ok(())
}

fn emit_order_display(ctx: &Context<CloseOrderAndClaimTip>, order: &Order) -> Result<()> {
    emit_cpi!(OrderDisplay {
        initial_input_amount: order.initial_input_amount,
        expected_output_amount: order.expected_output_amount,
        remaining_input_amount: order.remaining_input_amount,
        filled_output_amount: order.filled_output_amount,
        tip_amount: order.tip_amount,
        number_of_fills: order.number_of_fills,
        on_event_output_amount_filled: 0,
        on_event_tip_amount: 0,
        order_type: order.order_type,
        status: order.status,
        last_updated_timestamp: order.last_updated_timestamp,
    });

    Ok(())
}

#[event_cpi]
#[derive(Accounts)]
pub struct CloseOrderAndClaimTip<'info> {
    #[account(mut)]
    pub closer: Signer<'info>,

    #[account(mut)]
    /// CHECK: maker is a valid account
    pub maker: AccountInfo<'info>,

    #[account(mut,
        has_one = maker,
        has_one = global_config,
        has_one = input_mint,
        has_one = output_mint,
        close = closer
    )]
    pub order: AccountLoader<'info, Order>,

    #[account(mut,
        has_one = maker,
        has_one = global_config,
        close = closer,
    )]
    pub tp_child_order: Option<AccountLoader<'info, Order>>,

    #[account(mut,
        has_one = maker,
        has_one = global_config,
        close = closer,
    )]
    pub sl_child_order: Option<AccountLoader<'info, Order>>,

    #[account(
        mut,
        has_one = pda_authority,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(mut)]
    /// CHECK: pda_authority is a valid account
    pub pda_authority: AccountInfo<'info>,

    #[account(
        mint::token_program = input_token_program,
    )]
    pub input_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mint::token_program = output_token_program,
    )]
    pub output_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut,
        token::mint = input_mint,
        token::authority = maker
    )]
    pub maker_input_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        token::mint = output_mint,
        token::authority = maker
    )]
    pub maker_output_ata: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(mut,
        token::mint = input_mint,
        token::authority = closer
    )]
    pub closer_input_ata: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(mut,
        token::mint = output_mint,
        token::authority = closer
    )]
    pub closer_output_ata: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(mut,
        seeds = [seeds::ESCROW_VAULT, global_config.key().as_ref(), input_mint.key().as_ref()],
        bump,
        token::mint = input_mint,
        token::authority = pda_authority
    )]
    pub input_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [seeds::ESCROW_VAULT, global_config.key().as_ref(), output_mint.key().as_ref()],
        bump,
        token::mint = output_mint,
        token::authority = pda_authority
    )]
    pub output_vault: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    pub input_token_program: Interface<'info, TokenInterface>,
    pub output_token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
}

fn close_order_and_claim_tip<'a>(
    order: &mut Order,
    global_config: &mut GlobalConfig,
    current_timestamp: u64,
    global_config_key: Pubkey,
    pda_authority: &AccountInfo<'a>,
    maker: &AccountInfo<'a>,
    system_program: &AccountInfo<'a>,
) -> Result<()> {
    operations::close_order_and_claim_tip(order, global_config, current_timestamp)?;
    let pda_authority_bump = global_config.pda_authority_bump as u8;
    let seeds: &[&[u8]] = global_seeds!(pda_authority_bump, &global_config_key);

    if order.tip_amount > 0 {
        lamports_transfer_from_authority_to_account(
            maker.to_account_info(),
            pda_authority.to_account_info(),
            system_program.to_account_info(),
            seeds,
            order.tip_amount,
        )?;
    }

    Ok(())
}

fn validate_closer(
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
