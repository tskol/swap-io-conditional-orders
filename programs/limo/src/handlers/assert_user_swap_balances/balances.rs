use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenAccount;

use crate::GetBalancesCheckedResult;

pub(super) fn get_user_balances_checked(
    maker: &Signer,
    input_ta: &InterfaceAccount<TokenAccount>,
    output_ta: &InterfaceAccount<TokenAccount>,
) -> GetBalancesCheckedResult {
    GetBalancesCheckedResult {
        lamports_balance: maker.lamports(),
        input_balance: input_ta.amount,
        output_balance: output_ta.amount,
    }
}
