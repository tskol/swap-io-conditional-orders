use anchor_lang::prelude::*;
use derivative::Derivative;

#[derive(PartialEq, Derivative)]
#[derivative(Debug)]
#[account(zero_copy)]
pub struct UserSwapBalancesState {
    pub user_lamports: u64,
    pub input_ta_balance: u64,
    pub output_ta_balance: u64,
}

#[event]
pub struct UserSwapBalanceDiffs {
    pub user_lamports_before: u64,
    pub input_ta_balance_before: u64,
    pub output_ta_balance_before: u64,
    pub user_lamports_after: u64,
    pub input_ta_balance_after: u64,
    pub output_ta_balance_after: u64,
    pub swap_program: Pubkey,
    pub simulated_swap_amount_out: u64,
    pub simulated_ts: u64,
    pub minimum_amount_out: u64,
    pub swap_amount_in: u64,
    pub simulated_amount_out_next_best: u64,
    pub aggregator: u8,
    pub next_best_aggregator: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetBalancesCheckedResult {
    pub lamports_balance: u64,
    pub input_balance: u64,
    pub output_balance: u64,
}
