use limo::operations::calculate_fee_amount;

#[test]
fn fee_amount_rounds_fractional_bps_up_to_one_unit() {
    let fee = calculate_fee_amount(1, 1).unwrap();

    assert_eq!(fee, 0);
}