use anchor_lang::prelude::*;

use crate::{
    dbg_msg,
    state::GlobalConfig,
    utils::fraction::{Fraction, FractionExtra},
    OrdoError,
};

pub fn calculate_fee_amount(amount: u64, fee_bps: u16) -> Result<u64> {
    let fee_amount = (Fraction::from_bps(fee_bps) * Fraction::from(amount)).to_ceil::<u64>();
    Ok(fee_amount)
}

pub fn withdraw_host_tip(
    global_config: &mut GlobalConfig,
    pda_authority_balance: u64,
) -> Result<u64> {
    require_gte!(
        pda_authority_balance,
        global_config.host_tip_amount,
        OrdoError::InvalidHostTipBalance
    );
    let host_tip_amount = global_config.host_tip_amount;
    global_config.total_tip_amount = global_config
        .total_tip_amount
        .checked_sub(host_tip_amount)
        .ok_or_else(|| dbg_msg!(OrdoError::MathOverflow))?;
    global_config.host_tip_amount = 0;
    Ok(host_tip_amount)
}

pub fn validate_pda_authority_balance_and_update_accounting(
    global_config: &mut GlobalConfig,
    pda_authority_balance: u64,
    tip: u64,
) -> Result<()> {
    let tip_transfer_amount = pda_authority_balance
        .checked_sub(global_config.pda_authority_previous_lamports_balance)
        .ok_or_else(|| dbg_msg!(OrdoError::InvalidTipTransferAmount))?;

    require_gte!(
        tip_transfer_amount,
        tip,
        OrdoError::InvalidTipTransferAmount
    );
    require_gte!(
        pda_authority_balance,
        global_config.total_tip_amount,
        OrdoError::InvalidTipBalance
    );

    global_config.pda_authority_previous_lamports_balance = pda_authority_balance;

    Ok(())
}
