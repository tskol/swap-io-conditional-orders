use anchor_lang::{prelude::*, Accounts};
use anchor_spl::token_interface::Mint;

use crate::{
    operations, seeds,
    state::{GlobalConfig, OraclePoolsState},
};

pub fn handler_update_oracle_pool(ctx: Context<UpdateOraclePool>, feed_id: String) -> Result<()> {
    let oracle_pool = &mut ctx.accounts.oracle_pool.load_mut()?;

    operations::update_oracle_pool(oracle_pool, feed_id)?;

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
        seeds = [seeds::ORACLE_POOL, global_config.key().as_ref(), token_mint.key().as_ref()],
        bump
    )]
    pub oracle_pool: AccountLoader<'info, OraclePoolsState>,

    pub token_mint: Box<InterfaceAccount<'info, Mint>>,
}
