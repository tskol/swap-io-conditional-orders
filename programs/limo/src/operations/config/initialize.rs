use anchor_lang::prelude::*;

use crate::state::GlobalConfig;

pub fn initialize_global_config(
    global_config: &mut GlobalConfig,
    admin_authority: Pubkey,
    pda_authority: Pubkey,
    pda_bump: u64,
    pda_authority_previous_lamports_balance: u64,
) {
    global_config.emergency_mode = 0;
    global_config.pda_authority = pda_authority;
    global_config.pda_authority_bump = pda_bump;
    global_config.admin_authority = admin_authority;
    global_config.admin_authority_cached = admin_authority;
    global_config.total_tip_amount = 0;
    global_config.host_tip_amount = 0;
    global_config.pda_authority_previous_lamports_balance = pda_authority_previous_lamports_balance;
    global_config.tp_sl_enabled = 1;
    global_config.oracle_max_staleness_seconds = 30;
}
