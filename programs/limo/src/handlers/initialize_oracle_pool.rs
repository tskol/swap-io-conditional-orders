use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenAccount, TokenInterface};

use crate::state::{GlobalConfig, OraclePoolsState};
use crate::{seeds};

pub fn handler_initialize_oracle_pool(ctx: Context<InitializeOraclePool>, feed_id: String) -> Result<()> {
    let oracle_pool = &mut ctx.accounts.oracle_pool;
    let global_config = ctx.accounts.global_config.key();

    let token_mint = ctx.accounts.token_mint.key();
    let bump = ctx.bumps.oracle_pool;

    crate::operations::initialize_oracle_pool(
        oracle_pool,
        global_config,
        feed_id,
        token_mint,
        bump,
    );

    msg!(
        "Initializing oracle pool for token mint {}",
        oracle_pool.token_mint
    );

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeOraclePool<'info> {
    #[account(mut)]
    admin_authority: Signer<'info>,

    #[account(has_one = admin_authority)]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init,
        space = 8 + 32 + 32,
        payer = admin_authority,
        seeds = [seeds::ORACLE_POOL, token_mint.key().as_ref()],
        bump
    )]
    pub oracle_pool: Account<'info, OraclePoolsState>,

    pub system_program: Program<'info, System>,
}
