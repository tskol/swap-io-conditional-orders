use ordo::operations::calculate_fee_amount;

#[test]
fn fee_amount_rounds_fractional_bps_up_to_one_unit() {
    let fee = calculate_fee_amount(1, 1).unwrap();

    assert_eq!(fee, 1);
}
#[test]
fn fee_amount_handles_zero_and_full_bps() {
    assert_eq!(calculate_fee_amount(123_456, 0).unwrap(), 0);
    assert_eq!(calculate_fee_amount(123_456, 10_000).unwrap(), 123_456);
}
