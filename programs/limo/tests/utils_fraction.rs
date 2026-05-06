use limo::utils::fraction::{
    bps_u128_to_fraction, pct_u128_to_fraction, pow_fraction, to_sf, to_sf_const, BigFraction,
    Fraction, FractionExtra, FRACTION_ONE_SCALED, U128, U256,
};

#[test]
fn fraction_helpers_convert_percent_bps_and_scaled_form() {
    let quarter = Fraction::from_percent(25u64);
    let one_percent = bps_u128_to_fraction(100);

    assert_eq!(quarter.to_bps::<u64>().unwrap(), 2_500);
    assert_eq!(one_percent.to_percent::<u64>().unwrap(), 1);
    assert_eq!(pct_u128_to_fraction(100), Fraction::ONE);
    assert_eq!(bps_u128_to_fraction(10_000), Fraction::ONE);
    assert_eq!(Fraction::from_sf(to_sf(3u64)), Fraction::from_num(3u64));
    assert_eq!(to_sf_const(1), FRACTION_ONE_SCALED);
}

#[test]
fn fraction_helpers_round_and_display_values() {
    let value = Fraction::from_num(5u64) / 2;

    assert_eq!(value.to_floor::<u64>(), 2);
    assert_eq!(value.to_ceil::<u64>(), 3);
    assert_eq!(value.to_round::<u64>(), 3);
    assert_eq!(value.to_display().to_string(), "2.5000");
}

#[test]
fn pow_fraction_handles_zero_and_positive_powers() {
    let value = Fraction::from_percent(150u64);

    assert_eq!(pow_fraction(value, 0).unwrap(), Fraction::ONE);
    assert_eq!(
        value.checked_pow(2).unwrap().to_bps::<u64>().unwrap(),
        22_500
    );
}

#[test]
fn full_mul_int_ratio_preserves_precision_before_downscaling() {
    let value = Fraction::from_percent(50u64);

    let result = value.full_mul_int_ratio(3u64, 2u64);

    assert_eq!(result.to_bps::<u64>().unwrap(), 7_500);
}

#[test]
fn big_fraction_round_trips_bits_and_converts_to_fraction() {
    let value = BigFraction::from_num(7u64);
    let round_trip = BigFraction::from_bits(value.to_bits());
    let fraction: Fraction = round_trip.try_into().unwrap();

    assert_eq!(value.to_u128_sf(), Fraction::from_num(7u64).to_sf());
    assert_eq!(fraction.to_floor::<u64>(), 7);
}

#[test]
fn big_fraction_arithmetic_uses_scaled_values() {
    let two = BigFraction::from_num(2u64);
    let three = BigFraction::from_num(3u64);
    let product = two * three;
    let six: Fraction = product.try_into().unwrap();
    let five: Fraction = (two + three).try_into().unwrap();
    let one: Fraction = (three - two).try_into().unwrap();
    let three_again: Fraction = (product / two).try_into().unwrap();

    assert_eq!(six.to_floor::<u64>(), 6);
    assert_eq!(five.to_floor::<u64>(), 5);
    assert_eq!(one.to_floor::<u64>(), 1);
    assert_eq!(three_again.to_floor::<u64>(), 3);
}

#[test]
fn u128_u256_conversion_rejects_values_that_do_not_fit() {
    let small = U128([1, 2]);
    let small_as_u256 = U256::from(small);

    assert_eq!(U128::try_from(small_as_u256).unwrap(), small);
    assert!(U128::try_from(U256([0, 0, 1, 0])).is_err());
}
