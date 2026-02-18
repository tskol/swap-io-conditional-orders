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
    GlobalConfig, OrderDisplay, LimoError,
};

pub fn handler_close_order_and_claim_tip(ctx: Context<CloseOrderAndClaimTip>) -> Result<()> {
    validate_token_extensions(
        &ctx.accounts.input_mint.to_account_info(),
        vec![&ctx.accounts.maker_input_ata.to_account_info()],
    )?;
    let order = &mut ctx.accounts.order.load_mut()?;
    let global_config = &mut ctx.accounts.global_config.load_mut()?;

    let parsed_order_type = OrderType::try_from(order.order_type).map_err(|_| LimoError::OrderTypeInvalid)?;
    require!(parsed_order_type == OrderType::LimitParent || parsed_order_type == OrderType::Vanilla, LimoError::OrderTypeInvalid);

    let ts = u64::try_from(Clock::get()?.unix_timestamp).unwrap();

    validate_closer(
        ctx.accounts.closer.key(),
        ctx.accounts.maker.key(),
        global_config.allowed_taker,
        order.expiry_timestamp < ts,
    )?;

    if parsed_order_type == OrderType::LimitParent {
        if order.tp_child_order != Pubkey::default() {
            let tp_child_order_loader = ctx
                .accounts
                .tp_child_order
                .as_ref()
                .ok_or(LimoError::InvalidAccount)?;
            require!(tp_child_order_loader.key() == order.tp_child_order, LimoError::InvalidAccount);
            let tp_child_order = &mut tp_child_order_loader.load_mut()?;

            close_order_and_claim_tip(
                tp_child_order,
                global_config,
                ts,
                ctx.accounts.global_config.key(),
                &ctx.accounts.pda_authority.to_account_info(),
                &ctx.accounts.maker.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
            )?;

            emit_cpi!(OrderDisplay {
                initial_input_amount: tp_child_order.initial_input_amount,
                expected_output_amount: tp_child_order.expected_output_amount,
                remaining_input_amount: tp_child_order.remaining_input_amount,
                filled_output_amount: tp_child_order.filled_output_amount,
                tip_amount: tp_child_order.tip_amount,
                number_of_fills: tp_child_order.number_of_fills,
                on_event_output_amount_filled: 0,
                on_event_tip_amount: 0,
                order_type: tp_child_order.order_type,
                status: tp_child_order.status,
                last_updated_timestamp: tp_child_order.last_updated_timestamp,
            });
        }
        if order.sl_child_order != Pubkey::default() {
            let sl_child_order_loader = ctx
                .accounts
                .sl_child_order
                .as_ref()
                .ok_or(LimoError::InvalidAccount)?;
            require!(sl_child_order_loader.key() == order.sl_child_order, LimoError::InvalidAccount);
            let sl_child_order = &mut sl_child_order_loader.load_mut()?;

            close_order_and_claim_tip(
                sl_child_order,
                global_config,
                ts,
                ctx.accounts.global_config.key(),
                &ctx.accounts.pda_authority.to_account_info(),
                &ctx.accounts.maker.to_account_info(),
                &ctx.accounts.system_program.to_account_info(),
            )?;

            emit_cpi!(OrderDisplay {
                initial_input_amount: sl_child_order.initial_input_amount,
                expected_output_amount: sl_child_order.expected_output_amount,
                remaining_input_amount: sl_child_order.remaining_input_amount,
                filled_output_amount: sl_child_order.filled_output_amount,
                tip_amount: sl_child_order.tip_amount,
                number_of_fills: sl_child_order.number_of_fills,
                on_event_output_amount_filled: 0,
                on_event_tip_amount: 0,
                order_type: sl_child_order.order_type,
                status: sl_child_order.status,
                last_updated_timestamp: sl_child_order.last_updated_timestamp,
            });
        }
    }

    close_order_and_claim_tip(
        order,
        global_config,
        ts,
        ctx.accounts.global_config.key(),
        &ctx.accounts.pda_authority.to_account_info(),
        &ctx.accounts.maker.to_account_info(),
        &ctx.accounts.system_program.to_account_info(),
    )?;

    let gc = ctx.accounts.global_config.key();
    let seeds: &[&[u8]] = global_seeds!(global_config.pda_authority_bump as u8, &gc);

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
        let maker_output_ata = ctx.accounts.maker_output_ata.as_ref().ok_or(LimoError::MakerOutputAtaRequired)?.to_account_info();
        let output_vault = ctx.accounts.output_vault.as_ref().ok_or(LimoError::OutputVaultRequired)?.to_account_info();
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

    global_config.pda_authority_previous_lamports_balance = ctx.accounts.pda_authority.lamports();

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
        close = maker
    )]
    pub order: AccountLoader<'info, Order>,

    #[account(mut,
        has_one = maker,
        has_one = global_config,
        close = maker,
    )]
    pub tp_child_order: Option<AccountLoader<'info, Order>>,
    
    #[account(mut,
        has_one = maker,
        has_one = global_config,
        close = maker,
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
    require!(is_order_expired && closer == allowed_taker || closer == maker, LimoError::InvalidAccount);
    Ok(())
}
