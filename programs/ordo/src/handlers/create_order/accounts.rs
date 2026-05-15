use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::{
    seeds,
    state::{GlobalConfig, Order},
};

#[event_cpi]
#[derive(Accounts)]
pub struct SubmitOrder<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut, has_one = pda_authority)]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account()]
    /// CHECK: pda_authority is a valid account
    pub pda_authority: AccountInfo<'info>,

    #[account(zero)]
    pub order: AccountLoader<'info, Order>,

    #[account(zero)]
    pub tp_order: Option<AccountLoader<'info, Order>>,

    #[account(zero)]
    pub sl_order: Option<AccountLoader<'info, Order>>,

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
    pub maker_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [seeds::ESCROW_VAULT, global_config.key().as_ref(), input_mint.key().as_ref()],
        bump,
        token::mint = input_mint,
        token::authority = pda_authority
    )]
    pub input_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [seeds::FEE_VAULT, global_config.key().as_ref(), input_mint.key().as_ref()],
        bump,
        token::mint = input_mint,
        token::authority = pda_authority
    )]
    pub input_fee_vault: Box<InterfaceAccount<'info, TokenAccount>>,

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
