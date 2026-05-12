use anchor_lang::prelude::*;

use crate::{
    require_lte,
    state::{GetBalancesCheckedResult, UserSwapBalancesState},
    LimoError,
};

pub fn record_user_swap_balances(
    user_swap_balance_state: &mut UserSwapBalancesState,
    balances: GetBalancesCheckedResult,
) {
    user_swap_balance_state.user_lamports = balances.lamports_balance;
    user_swap_balance_state.input_ta_balance = balances.input_balance;
    user_swap_balance_state.output_ta_balance = balances.output_balance;
}

pub fn validate_user_swap_balances(
    start_balance_state: &UserSwapBalancesState,
    end_balance_state: GetBalancesCheckedResult,
    max_input_amount_change: u64,
    min_output_amount_change: u64,
) -> Result<()> {
    require_gte!(
        start_balance_state.input_ta_balance,
        end_balance_state.input_balance,
        LimoError::SwapInputInvalidBalanceChange
    );

    require_lte!(
        start_balance_state.output_ta_balance,
        end_balance_state.output_balance,
        LimoError::SwapOutputInvalidBalanceChange
    );

    require_lte!(
        start_balance_state.input_ta_balance - end_balance_state.input_balance,
        max_input_amount_change,
        LimoError::SwapInputAmountTooLarge
    );
    require_gte!(
        end_balance_state.output_balance - start_balance_state.output_ta_balance,
        min_output_amount_change,
        LimoError::SwapOutputAmountTooSmall
    );
    Ok(())
}
