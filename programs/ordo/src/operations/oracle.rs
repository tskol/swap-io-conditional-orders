use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::get_feed_id_from_hex;

use crate::{state::OraclePoolsState, OrdoError};

pub fn initialize_oracle_pool(
    oracle_pool: &mut OraclePoolsState,
    global_config: Pubkey,
    feed_id: String,
    token_mint: Pubkey,
) -> Result<()> {
    let feed_id_bytes = get_feed_id_from_hex(&feed_id).map_err(|_| OrdoError::InvalidFeedId)?;
    oracle_pool.global_config = global_config;
    oracle_pool.oracle_feed_id = feed_id_bytes;
    oracle_pool.token_mint = token_mint;

    Ok(())
}

pub fn update_oracle_pool(oracle_pool: &mut OraclePoolsState, feed_id: String) -> Result<()> {
    let feed_id_bytes = get_feed_id_from_hex(&feed_id).map_err(|_| OrdoError::InvalidFeedId)?;
    oracle_pool.oracle_feed_id = feed_id_bytes;

    Ok(())
}
