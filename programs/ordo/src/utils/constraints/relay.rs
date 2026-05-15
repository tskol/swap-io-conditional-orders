use anchor_lang::{prelude::*, require, Result, ToAccountInfo};
use express_relay::{cpi::accounts::CheckPermission, sdk::cpi::check_permission_cpi};

use crate::OrdoError;

pub fn check_permission_express_relay_and_get_fees<'a>(
    sysvar_instructions: &AccountInfo<'a>,
    permission: &AccountInfo<'a>,
    pda_authority: &AccountInfo<'a>,
    config_router: &AccountInfo<'a>,
    express_relay_metadata: &AccountInfo<'a>,
    express_relay_program: &AccountInfo<'a>,
    order_key: Pubkey,
) -> Result<u64> {
    let express_relay_check_permission_accounts = CheckPermission {
        sysvar_instructions: sysvar_instructions.to_account_info(),
        permission: permission.to_account_info(),
        router: pda_authority.to_account_info(),
        config_router: config_router.to_account_info(),
        express_relay_metadata: express_relay_metadata.to_account_info(),
    };

    require!(
        permission.key() == order_key,
        OrdoError::PermissionDoesNotMatchOrder
    );

    let fees = check_permission_cpi(
        express_relay_check_permission_accounts,
        express_relay_program.to_account_info(),
    )?;

    Ok(fees)
}
