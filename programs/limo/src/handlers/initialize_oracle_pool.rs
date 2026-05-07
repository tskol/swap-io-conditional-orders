use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint};

use crate::{seeds, state::{GlobalConfig, OraclePoolsState}};

pub fn handler_initialize_oracle_pool(ctx: Context<InitializeOraclePool>, feed_id: String) -> Result<()> {
    let oracle_pool = &mut ctx.accounts.oracle_pool.load_init()?;
    let global_config = ctx.accounts.global_config.key();

    let token_mint = ctx.accounts.token_mint.key();

    crate::operations::initialize_oracle_pool(
        oracle_pool,
        global_config,
        feed_id,
        token_mint,
    )?;

    msg!(
        "Initializing oracle pool for token mint {}",
        oracle_pool.token_mint
    );

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeOraclePool<'info> {
    #[account(mut)]
    pub admin_authority: Signer<'info>,

    #[account(has_one = admin_authority)]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(init,
        payer = admin_authority,
        space = 8 + 32 + 32 + 32, // 8 (AccountLoader header) + 32 (global_config) + 32 (oracle_feed_id) + 32 (token_mint)
        seeds = [seeds::ORACLE_POOL, global_config.key().as_ref(), token_mint.key().as_ref()],
        bump
    )]
    pub oracle_pool: AccountLoader<'info, OraclePoolsState>,

    pub system_program: Program<'info, System>,
}
