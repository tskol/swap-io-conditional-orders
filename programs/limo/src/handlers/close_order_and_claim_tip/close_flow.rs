use anchor_lang::prelude::*;

use crate::{
    global_seeds, operations,
    seeds::GLOBAL_AUTH,
    state::{GlobalConfig, Order},
    token_operations::lamports_transfer_from_authority_to_account,
};

pub(super) fn close_order_and_claim_tip<'a>(
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
