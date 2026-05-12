use anchor_lang::prelude::*;

use crate::{
    state::{GlobalConfig, UpdateGlobalConfigMode},
    utils::consts::UPDATE_GLOBAL_CONFIG_BYTE_SIZE,
};

use super::field_updates::{
    update_global_config_bps, update_global_config_flag, update_global_config_pubkey,
};

pub fn update_global_config(
    global_config: &mut GlobalConfig,
    mode: UpdateGlobalConfigMode,
    value: &[u8; UPDATE_GLOBAL_CONFIG_BYTE_SIZE],
    ts: u64,
) -> Result<()> {
    match mode {
        UpdateGlobalConfigMode::UpdateEmergencyMode
        | UpdateGlobalConfigMode::UpdateFlashTakeOrderBlocked
        | UpdateGlobalConfigMode::UpdateBlockNewOrders
        | UpdateGlobalConfigMode::UpdateBlockOrderTaking
        | UpdateGlobalConfigMode::UpdateTpSlEnabled => {
            let value = value[0];
            update_global_config_flag(global_config, mode, value, ts)?;
        }
        UpdateGlobalConfigMode::UpdateHostFeeBps
        | UpdateGlobalConfigMode::UpdateCreateOrderFeeBps
        | UpdateGlobalConfigMode::UpdateSlMaxUpwardDeviationBps
        | UpdateGlobalConfigMode::UpdateTpSlMinDistanceBps
        | UpdateGlobalConfigMode::UpdateParentFillFeeKeeperBps
        | UpdateGlobalConfigMode::UpdateParentFillFeeProtocolBps
        | UpdateGlobalConfigMode::UpdateTpSlChildFeeKeeperBps
        | UpdateGlobalConfigMode::UpdateTpSlChildFeeProtocolBps
        | UpdateGlobalConfigMode::UpdateKeeperTakeFeeBps
        | UpdateGlobalConfigMode::UpdateKeeperCloseFeeBps => {
            let value = u16::from_le_bytes(value[0..2].try_into().unwrap());
            update_global_config_bps(global_config, mode, value, ts)?;
        }
        UpdateGlobalConfigMode::UpdateOrderCloseDelaySeconds => {
            let value = u64::from_le_bytes(value[0..8].try_into().unwrap());
            msg!("update_global_config mode={:?} ts={}", mode, ts);
            msg!(
                "new={} prev={}",
                value,
                global_config.order_close_delay_seconds
            );
            global_config.order_close_delay_seconds = value;
        }
        UpdateGlobalConfigMode::UpdateAdminAuthorityCached => {
            let value = Pubkey::new_from_array(value[0..32].try_into().unwrap());
            update_global_config_pubkey(global_config, mode, value, ts)?
        }
        UpdateGlobalConfigMode::UpdateAllowedTaker => {
            let value = Pubkey::new_from_array(value[0..32].try_into().unwrap());
            update_global_config_pubkey(global_config, mode, value, ts)?
        }
        UpdateGlobalConfigMode::UpdateTxnFeeCost => {
            let value = u64::from_le_bytes(value[0..8].try_into().unwrap());
            msg!("update_global_config mode={:?} ts={}", mode, ts);
            msg!("new={} prev={}", value, global_config.txn_fee_cost);
            global_config.txn_fee_cost = value;
        }
        UpdateGlobalConfigMode::UpdateAtaCreationCost => {
            let value = u64::from_le_bytes(value[0..8].try_into().unwrap());
            msg!("update_global_config mode={:?} ts={}", mode, ts);
            msg!("new={} prev={}", value, global_config.ata_creation_cost);
            global_config.ata_creation_cost = value;
        }
        UpdateGlobalConfigMode::UpdateOracleMaxStalenessSeconds => {
            let value = u64::from_le_bytes(value[0..8].try_into().unwrap());
            msg!("update_global_config mode={:?} ts={}", mode, ts);
            msg!(
                "new={} prev={}",
                value,
                global_config.oracle_max_staleness_seconds
            );
            global_config.oracle_max_staleness_seconds = value;
        }
    }
    Ok(())
}
