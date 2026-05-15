use anchor_lang::prelude::*;

mod errors;
pub mod handlers;
pub mod operations;
pub mod seeds;
pub mod state;
pub mod token_operations;
pub mod utils;
use utils::{
    constraints::{
        create_new_orders_disabled, emergency_mode_disabled, order_expired, taking_orders_disabled,
    },
    consts::UPDATE_GLOBAL_CONFIG_BYTE_SIZE,
};

pub use crate::errors::{OrdoError, OrdoResult};
use crate::handlers::*;
pub use crate::state::*;

declare_id!("EfsKVSxQxwoR9NpZfjgWuuwpbbF14BQgrh3rt4kAs181");

#[cfg(not(feature = "no-entrypoint"))]
solana_security_txt::security_txt! {
    name: "swap-io-conditional-orders",
    project_url: "https://swap.io",
    contacts: "link:https://swap.io/contact",
    policy: "https://github.com/swap-dot-io/swap-io-conditional-orders/SECURITY.md",

    source_code: "https://github.com/swap-dot-io/swap-io-conditional-orders",
    preferred_languages: "en"
}

#[program]
pub mod ordo {

    use super::*;

    pub fn initialize_global_config(ctx: Context<InitializeGlobalConfig>) -> Result<()> {
        handlers::initialize_global_config::handler_initialize_global_config(ctx)
    }

    pub fn initialize_oracle_pool(
        ctx: Context<InitializeOraclePool>,
        feed_id: String,
    ) -> Result<()> {
        handlers::initialize_oracle_pool::handler_initialize_oracle_pool(ctx, feed_id)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.global_config))]
    pub fn initialize_vault(ctx: Context<InitializeVault>) -> Result<()> {
        handlers::initialize_vault::handler_initialize_vault(ctx)
    }

    #[access_control(create_new_orders_disabled(&ctx.accounts.global_config))]
    #[access_control(emergency_mode_disabled(&ctx.accounts.global_config))]
    pub fn create_order(
        ctx: Context<SubmitOrder>,
        input_amount: u64,
        output_amount: u64,
        order_type: u8,
        tp_output_amount: u64,
        sl_output_amount: u64,
        active_duration_seconds: u64,
    ) -> Result<()> {
        handlers::create_order::handler_create_order(
            ctx,
            input_amount,
            output_amount,
            order_type,
            tp_output_amount,
            sl_output_amount,
            active_duration_seconds,
        )
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.global_config))]
    #[access_control(order_expired(&ctx.accounts.order))]
    pub fn update_order(
        ctx: Context<UpdateOrder>,
        mode: UpdateOrderMode,
        value: Vec<u8>,
    ) -> Result<()> {
        handlers::update_order::handler_update_order(ctx, mode, &value)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.global_config))]
    pub fn close_order_and_claim_tip(ctx: Context<ExitOrderAndClaimTip>) -> Result<()> {
        handlers::close_order_and_claim_tip::handler_close_order_and_claim_tip(ctx)
    }

    #[access_control(taking_orders_disabled(&ctx.accounts.global_config))]
    #[access_control(emergency_mode_disabled(&ctx.accounts.global_config))]
    #[access_control(order_expired(&ctx.accounts.order))]
    pub fn take_order(
        ctx: Context<ExecuteOrder>,
        input_amount: u64,
        min_output_amount: u64,
        tip_amount_permissionless_taking: u64,
    ) -> Result<()> {
        handlers::take_order::handler_take_order(
            ctx,
            input_amount,
            min_output_amount,
            tip_amount_permissionless_taking,
        )
    }

    pub fn update_global_config(
        ctx: Context<UpdateGlobalConfig>,
        mode: u16,
        value: [u8; UPDATE_GLOBAL_CONFIG_BYTE_SIZE],
    ) -> Result<()> {
        handlers::update_global_config::handler_update_global_config(ctx, mode, &value)
    }

    pub fn update_global_config_admin(ctx: Context<UpdateGlobalConfigAdmin>) -> Result<()> {
        handlers::update_global_config_admin::handler_update_global_config_admin(ctx)
    }

    #[access_control(emergency_mode_disabled(&ctx.accounts.global_config))]
    pub fn withdraw_host_tip(ctx: Context<WithdrawHostTip>) -> Result<()> {
        handlers::withdraw_host_tip::withdraw_host_tip(ctx)
    }

    pub fn log_user_swap_balances_start(
        ctx: Context<LogUserSwapBalancesStartContext>,
    ) -> Result<()> {
        handlers::log_user_swap_balances::handler_log_user_swap_balances_start(ctx)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn log_user_swap_balances_end(
        ctx: Context<LogUserSwapBalancesEndContext>,
        simulated_swap_amount_out: u64,
        simulated_ts: u64,
        minimum_amount_out: u64,
        swap_amount_in: u64,
        simulated_amount_out_next_best: u64,
        aggregator: u8,
        next_best_aggregator: u8,
        _padding: [u8; 2],
    ) -> Result<()> {
        handlers::log_user_swap_balances::handler_log_user_swap_balances_end(
            ctx,
            simulated_swap_amount_out,
            simulated_ts,
            minimum_amount_out,
            swap_amount_in,
            simulated_amount_out_next_best,
            aggregator,
            next_best_aggregator,
        )
    }

    pub fn assert_user_swap_balances_start(
        ctx: Context<AssertUserSwapBalancesStartContext>,
    ) -> Result<()> {
        handlers::assert_user_swap_balances::handler_assert_user_swap_balances_start(ctx)
    }

    pub fn assert_user_swap_balances_end(
        ctx: Context<AssertUserSwapBalancesEndContext>,
        max_input_amount_change: u64,
        min_output_amount_change: u64,
    ) -> Result<()> {
        handlers::assert_user_swap_balances::handler_assert_user_swap_balances_end(
            ctx,
            max_input_amount_change,
            min_output_amount_change,
        )
    }
}
