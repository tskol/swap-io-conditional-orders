use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    seeds,
    state::{GlobalConfig, Order},
};

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
