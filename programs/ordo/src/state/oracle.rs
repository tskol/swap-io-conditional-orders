use anchor_lang::prelude::*;
use derivative::Derivative;

#[derive(PartialEq, Derivative)]
#[derivative(Debug)]
#[account(zero_copy)]
pub struct OraclePoolsState {
    pub global_config: Pubkey,

    pub oracle_feed_id: [u8; 32],
    pub token_mint: Pubkey,
}
