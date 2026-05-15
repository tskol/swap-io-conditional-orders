use ordo::{
    operations::validate_user_swap_balances,
    state::{GetBalancesCheckedResult, UserSwapBalancesState},
};

fn start_balances() -> UserSwapBalancesState {
    UserSwapBalancesState {
        user_lamports: 10_000,
        input_ta_balance: 1_000,
        output_ta_balance: 200,
    }
}

fn end_balances(input_balance: u64, output_balance: u64) -> GetBalancesCheckedResult {
    GetBalancesCheckedResult {
        lamports_balance: 10_000,
        input_balance,
        output_balance,
    }
}

#[test]
fn validate_user_swap_balances_accepts_change_within_limits() {
    validate_user_swap_balances(&start_balances(), end_balances(900, 350), 100, 150).unwrap();
}

#[test]
fn validate_user_swap_balances_rejects_invalid_input_or_output_direction() {
    assert!(
        validate_user_swap_balances(&start_balances(), end_balances(1_001, 350), 100, 150).is_err()
    );
    assert!(
        validate_user_swap_balances(&start_balances(), end_balances(900, 199), 100, 150).is_err()
    );
}

#[test]
fn validate_user_swap_balances_rejects_changes_outside_limits() {
    assert!(
        validate_user_swap_balances(&start_balances(), end_balances(899, 350), 100, 150).is_err()
    );
    assert!(
        validate_user_swap_balances(&start_balances(), end_balances(900, 349), 100, 150).is_err()
    );
}
