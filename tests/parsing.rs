use num_rational::{Ratio, Rational32, Rational64};
use num_rational_parse::{RatioErrorKind, RationalParse};

fn components(s: &str) -> (i32, i32) {
    let r = Rational32::from_str_flex(s).unwrap_or_else(|_| panic!("Failed to parse {}", s));
    (*r.numer(), *r.denom())
}

fn check_invalid(s: &str, expected: RatioErrorKind) {
    let res = Rational32::from_str_flex(s);
    match res {
        Ok(val) => panic!(
            "Expected error {:?} for input {:?}, but got Ok({:?})",
            expected, s, val
        ),
        Err(e) => assert_eq!(
            *e.kind(),
            expected,
            "For input {:?}, expected error {:?}, got {:?}",
            s,
            expected,
            e.kind()
        ),
    }
}

#[test]
fn test_examples() {
    assert_eq!((314, 1), components("314"));
    assert_eq!((-35, 4), components("-35/4"));
    assert_eq!((6283, 2000), components("3.1415"));
    assert_eq!((-47, 100), components("-47e-2"));
    assert_eq!((9, 4), components("2.25"));
    assert_eq!((1000, 1), components("1_000/1"));
    assert_eq!((3, 2), components("1.50_0"));
}

#[test]
fn test_integers() {
    assert_eq!((5, 1), components("5"));
    assert_eq!((5, 1), components("005"));
    assert_eq!((123, 1), components("1_2_3"));
}

#[test]
fn test_fractions() {
    assert_eq!((3, 2), components("3/2"));
    assert_eq!((3, 2), components("3 / 2"));
    assert_eq!((3, 2), components(" \n  +3/2"));
    assert_eq!((-3, 2), components("-3/2  "));
    assert_eq!((13, 2), components("    0013/002 \n  "));
    assert_eq!((41, 107), components("1_2_3/3_2_1"));
}

#[test]
fn test_decimals() {
    assert_eq!((16, 5), components(" 3.2 "));
    assert_eq!((16, 5), components("003.2"));
    assert_eq!((-16, 5), components(" -3.2 "));
    assert_eq!((-3, 1), components(" -3. "));
    assert_eq!((3, 5), components(" .6 "));
    assert_eq!((6283, 2000), components("3.14_15"));
    assert_eq!((101, 100), components("1.01"));
    assert_eq!((101, 100), components("1.0_1"));
}

#[test]
fn test_scientific() {
    assert_eq!((1, 3125), components("32.e-5"));
    assert_eq!((1000000, 1), components("1E+06"));
    assert_eq!((-12300, 1), components("-1.23e4"));
    assert_eq!((0, 1), components(" .0e+0\t"));
    assert_eq!((0, 1), components("-0.000e0"));
}

#[test]
fn test_underscores() {
    assert_eq!((123, 1), components("1_2_3"));
    assert_eq!((41, 107), components("1_2_3/3_2_1"));
    assert_eq!((6283, 2000), components("3.14_15"));
}

#[test]
fn test_overflow() {
    // Integer overflow: exceeds i32::MAX (2147483647)
    check_invalid("2147483648", RatioErrorKind::Overflow);
    check_invalid("99999999999", RatioErrorKind::Overflow);
    check_invalid("-2147483648", RatioErrorKind::Overflow);

    // Fraction overflow: numerator exceeds i32::MAX
    check_invalid("2147483648/1", RatioErrorKind::Overflow);
    check_invalid("-2147483648/1", RatioErrorKind::Overflow);

    // Fraction overflow: denominator exceeds i32::MAX
    check_invalid("1/2147483648", RatioErrorKind::Overflow);

    // Trailing zeros are stripped to prevent unnecessary overflow
    assert_eq!((1, 1), components("1.0000000000"));
    assert_eq!((123, 100), components("1.2300000"));

    // But actual overflow with significant digits still caught
    check_invalid("1.12345678901", RatioErrorKind::Overflow);

    // Scientific notation overflow: positive exponent too large
    check_invalid("1e10", RatioErrorKind::Overflow);
    check_invalid("2147483648e0", RatioErrorKind::Overflow);

    // Scientific notation overflow: negative exponent causing denominator overflow
    check_invalid("3.14_15e-1_0", RatioErrorKind::Overflow);
    check_invalid("1e-10", RatioErrorKind::Overflow);
}

#[test]
fn test_invalid() {
    check_invalid("invalid", RatioErrorKind::ParseError);
    check_invalid("1/0", RatioErrorKind::ZeroDenominator);
    check_invalid("3/0", RatioErrorKind::ZeroDenominator);
    check_invalid("3/", RatioErrorKind::ParseError);
    check_invalid("/2", RatioErrorKind::ParseError);
    check_invalid("3/+2", RatioErrorKind::ParseError);
    check_invalid("3/-2", RatioErrorKind::ParseError);
    check_invalid("+ 3/2", RatioErrorKind::ParseError);
    check_invalid("- 3/2", RatioErrorKind::ParseError);
    check_invalid("3a2", RatioErrorKind::ParseError);
    check_invalid("3/7.2", RatioErrorKind::ParseError);
    check_invalid("3.2/7", RatioErrorKind::ParseError);
    check_invalid("3 .2", RatioErrorKind::ParseError);
    check_invalid("3. 2", RatioErrorKind::ParseError);
    check_invalid("3.2 e1", RatioErrorKind::ParseError);
    check_invalid("3.2e 1", RatioErrorKind::ParseError);
    check_invalid("3.+2", RatioErrorKind::ParseError);
    check_invalid("3.-2", RatioErrorKind::ParseError);
    check_invalid("0x10", RatioErrorKind::ParseError);
    check_invalid("0x10/1", RatioErrorKind::ParseError);
    check_invalid("1/0x10", RatioErrorKind::ParseError);
    check_invalid("0x10.", RatioErrorKind::ParseError);
    check_invalid("0x10.1", RatioErrorKind::ParseError);
    check_invalid("1.0x10", RatioErrorKind::ParseError);
    check_invalid("1.0e0x10", RatioErrorKind::ParseError);

    check_invalid("³", RatioErrorKind::ParseError);
    check_invalid("³/2", RatioErrorKind::ParseError);
    check_invalid("3/²", RatioErrorKind::ParseError);
    check_invalid("³.2", RatioErrorKind::ParseError);
    check_invalid("3.²", RatioErrorKind::ParseError);
    check_invalid("3.2e²", RatioErrorKind::ParseError);
    check_invalid("¼", RatioErrorKind::ParseError);

    check_invalid(".", RatioErrorKind::ParseError);
    check_invalid("_", RatioErrorKind::ParseError);
    check_invalid("_1", RatioErrorKind::ParseError);
    check_invalid("1__2", RatioErrorKind::ParseError);
    check_invalid("/_", RatioErrorKind::ParseError);
    check_invalid("1_/", RatioErrorKind::ParseError);
    check_invalid("_1/", RatioErrorKind::ParseError);
    check_invalid("1__2/", RatioErrorKind::ParseError);
    check_invalid("1/_", RatioErrorKind::ParseError);
    check_invalid("1/_1", RatioErrorKind::ParseError);
    check_invalid("1/1__2", RatioErrorKind::ParseError);
    check_invalid("1._111", RatioErrorKind::ParseError);
    check_invalid("1.1__1", RatioErrorKind::ParseError);
    check_invalid("1.1e+_1", RatioErrorKind::ParseError);
    check_invalid("1.1e+1__1", RatioErrorKind::ParseError);
    check_invalid("123.dd", RatioErrorKind::ParseError);
    check_invalid("123.5_dd", RatioErrorKind::ParseError);
    check_invalid("dd.5", RatioErrorKind::ParseError);
    check_invalid("7_dd", RatioErrorKind::ParseError);
    check_invalid("1/dd", RatioErrorKind::ParseError);
    check_invalid("1/123_dd", RatioErrorKind::ParseError);
    check_invalid("789edd", RatioErrorKind::ParseError);
    check_invalid("789e2_dd", RatioErrorKind::ParseError);
}

#[test]
fn test_backtracking() {
    // Catastrophic backtracking test
    let val = "9".repeat(50) + "_";
    check_invalid(&val, RatioErrorKind::ParseError);
    check_invalid(&("1/".to_owned() + &val), RatioErrorKind::ParseError);
    check_invalid(&("1.".to_owned() + &val), RatioErrorKind::ParseError);
    check_invalid(&(".".to_owned() + &val), RatioErrorKind::ParseError);
    check_invalid(&("1.1+e".to_owned() + &val), RatioErrorKind::ParseError);
    check_invalid(&("1.1e".to_owned() + &val), RatioErrorKind::ParseError);
}

#[test]
fn test_aliases() {
    // Test Rational64 (i64)
    let r64 = Rational64::from_str_flex("3.1415926535").unwrap();
    assert_eq!(r64, Rational64::new(6283185307, 2000000000));

    // Test Ratio<isize>
    type RationalIsize = Ratio<isize>;
    let risize = RationalIsize::from_str_flex("1/3").unwrap();
    assert_eq!(risize, RationalIsize::new(1, 3));

    // Test Ratio<i8>
    type Rational8 = Ratio<i8>;
    assert_eq!(
        *Rational8::from_str_flex("128").unwrap_err().kind(),
        RatioErrorKind::Overflow
    );
    assert_eq!(
        Rational8::from_str_flex("127").unwrap(),
        Rational8::new(127, 1)
    );
}
