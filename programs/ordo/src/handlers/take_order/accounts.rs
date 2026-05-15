use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;
use solana_program::sysvar::{instructions::Instructions as SysInstructions, SysvarId};

use crate::{
    seeds::{self, INTERMEDIARY_OUTPUT_TOKEN_ACCOUNT},
    state::{GlobalConfig, OraclePoolsState, Order},
};

#[event_cpi]
#[derive(Accounts)]
pub struct ExecuteOrder<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut,
        address = order.load()?.maker)]
    /// CHECK: maker is a valid account
    pub maker: AccountInfo<'info>,

    #[account(
        mut,
        has_one = pda_authority,
    )]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(mut)]
    /// CHECK: pda_authority is a valid account
    pub pda_authority: AccountInfo<'info>,

    #[account(mut,
        has_one = global_config,
        has_one = input_mint,
        has_one = output_mint
    )]
    pub order: AccountLoader<'info, Order>,

    #[account(mut)]
    pub parent_order: Option<AccountLoader<'info, Order>>,

    #[account(mut)]
    pub brother_order: Option<AccountLoader<'info, Order>>,

    #[account(
        mint::token_program = input_token_program,
    )]
    pub input_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mint::token_program = output_token_program,
    )]
    pub output_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(mut,
        seeds = [seeds::ESCROW_VAULT, global_config.key().as_ref(), input_mint.key().as_ref()],
        bump = order.load()?.in_vault_bump,
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

    #[account(mut,
        seeds = [seeds::FEE_VAULT, global_config.key().as_ref(), output_mint.key().as_ref()],
        bump,
        token::mint = output_mint,
        token::authority = pda_authority
    )]
    pub output_fee_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [seeds::ORACLE_POOL, global_config.key().as_ref(), output_mint.key().as_ref()],
        bump
    )]
    pub output_oracle_pool: Option<AccountLoader<'info, OraclePoolsState>>,

    #[account(mut,
        seeds = [seeds::ORACLE_POOL, global_config.key().as_ref(), input_mint.key().as_ref()],
        bump
    )]
    pub input_oracle_pool: Option<AccountLoader<'info, OraclePoolsState>>,

    pub input_price_update: Option<Account<'info, PriceUpdateV2>>,

    pub output_price_update: Option<Account<'info, PriceUpdateV2>>,

    #[account(mut,
        token::mint = input_mint,
        token::authority = taker
    )]
    pub taker_input_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        token::mint = output_mint,
        token::authority = taker
    )]
    pub taker_output_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(mut,
        seeds = [INTERMEDIARY_OUTPUT_TOKEN_ACCOUNT, order.key().as_ref()],
        bump
    )]
    pub intermediary_output_token_account: Option<UncheckedAccount<'info>>,

    #[account(mut,
        token::mint = output_mint,
        token::authority = maker,
    )]
    pub maker_output_ata: Option<Box<InterfaceAccount<'info, TokenAccount>>>,

    #[account(address = SysInstructions::id())]
    /// CHECK: SysInstructions is a valid sysvar
    pub sysvar_instructions: AccountInfo<'info>,

    pub input_token_program: Interface<'info, TokenInterface>,
    pub output_token_program: Interface<'info, TokenInterface>,

    pub rent: Sysvar<'info, Rent>,

    pub system_program: Program<'info, System>,
}
