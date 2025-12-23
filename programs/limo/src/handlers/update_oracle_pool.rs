use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::{Mint, TokenInterface};

use crate::{
    operations,
    state::{GlobalConfig, OraclePoolsState},
    seeds,
};

pub fn handler_update_oracle_pool(
    ctx: Context<UpdateOraclePool>,
    feed_id: String,
) -> Result<()> {
    let oracle_pool = &mut ctx.accounts.oracle_pool;

    operations::update_oracle_pool(oracle_pool, feed_id);

    Ok(())
}

#[derive(Accounts)]
pub struct UpdateOraclePool<'info> {
    #[account(mut)]
    pub admin_authority: Signer<'info>,

    #[account(mut,
        has_one = admin_authority,)]
    pub global_config: AccountLoader<'info, GlobalConfig>,

    #[account(mut,
        has_one = global_config,
        seeds = [seeds::ORACLE_POOL, token_mint.key().as_ref()],
        bump = oracle_pool.bump,
    )]
    pub oracle_pool: Account<'info, OraclePoolsState>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,
}
