//! Flexible string parsing for [`num_rational::Ratio`].
//!
//! This crate extends [`Ratio`] with a parser that accepts the same formats as
//! Python's `fractions.Fraction`:
//!
//! | Format                  | Example           |
//! |-------------------------|-------------------|
//! | Integer                 | `"42"`, `"-5"`    |
//! | Fraction                | `"3/4"`, `"-5/2"` |
//! | Decimal                 | `"1.25"`, `".5"`  |
//! | Scientific notation     | `"1.2e-3"`, `"1E5"` |
//!
//! Underscores are allowed as digit separators in all formats:
//! `"1_000/2_000"`, `"3.14_15e-1_0"`.
//!
//! # Examples
//!
//! ```
//! use num_rational::Ratio;
//! use num_rational_parse::RationalParse;
//!
//! assert_eq!(Ratio::<i32>::from_str_flex("3.14").unwrap(), Ratio::new(157, 50));
//! assert_eq!(Ratio::<i32>::from_str_flex("1.2e-2").unwrap(), Ratio::new(3, 250));
//! assert_eq!(Ratio::<i32>::from_str_flex("-1_000/2_000").unwrap(), Ratio::new(-1, 2));
//! ```

use num_integer::Integer;
use num_rational::Ratio;
use num_traits::{CheckedAdd, CheckedMul, CheckedSub, FromPrimitive, One, Signed, Zero};
use regex::{regex, Regex};
use std::str::FromStr;

#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{kind}")]
pub struct ParseRatioError {
    pub kind: RatioErrorKind,
}

/// The specific category of a [`ParseRatioError`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, thiserror::Error)]
#[non_exhaustive]
pub enum RatioErrorKind {
    #[error("Input does not match a valid rational-number format")]
    ParseError,
    #[error("The denominator is zero")]
    ZeroDenominator,
    #[error("Value overflowed the target integer type")]
    Overflow,
}

// ---------------------------------------------------------------------------
// Regex
//
// Adapted from CPython's fractions.py. The Python original uses a lookahead
// `(?=\d|\.\d)` that the `regex` crate does not support; we validate the
// equivalent constraint in code instead.
//
// Reference: https://github.com/python/cpython/blob/888d101/Lib/fractions.py#L56
// ---------------------------------------------------------------------------

fn rational_re() -> &'static Regex {
    regex!(
        r"(?xi)
        \A\s*
        (?P<sign>[-+]?)                     # optional sign
        (?P<num>\d*|\d+(_\d+)*)             # numerator / integer part (may be empty)
        (?:
            (?:\s*/\s*(?P<denom>\d+(_\d+)*))?  # optional denominator
        |
            (?:\.(?P<decimal>\d*|\d+(_\d+)*))?  # optional decimal part
            (?:E(?P<exp>[-+]?\d+(_\d+)*))?      # optional exponent
        )
        \s*\z
        "
    )
}

/// Extension trait that adds flexible rational-number parsing to [`Ratio`].
pub trait RationalParse: Sized {
    /// Parse a string into a rational number.
    ///
    /// Accepts integers, fractions (`a/b`), decimals (`a.b`), and
    /// scientific notation (`a.bEe`). Underscores are permitted as digit
    /// separators in all numeric parts.
    ///
    /// # Errors
    ///
    /// Returns [`ParseRatioError`] if the input is malformed, contains a zero
    /// denominator, or causes integer overflow for the chosen type `T`.
    fn from_str_flex(s: &str) -> Result<Self, ParseRatioError>;
}

impl<T> RationalParse for Ratio<T>
where
    T: Clone + Integer + Signed + FromStr + CheckedMul + CheckedAdd + CheckedSub + FromPrimitive,
{
    fn from_str_flex(input: &str) -> Result<Self, ParseRatioError> {
        let caps = rational_re()
            .captures(input)
            .ok_or(parse_error(RatioErrorKind::ParseError))?;

        let sign_negative = caps.name("sign").is_some_and(|m| m.as_str() == "-");
        let num_str = caps.name("num").map_or("", |m| m.as_str());
        let denom = caps.name("denom").map(|m| m.as_str());
        let decimal = caps.name("decimal").map(|m| m.as_str());
        let exp = caps.name("exp").map(|m| m.as_str());

        // Must have at least one digit somewhere.
        if num_str.is_empty() && !decimal.is_some_and(|s| !s.is_empty()) {
            return Err(parse_error(RatioErrorKind::ParseError));
        }

        let mut numerator = parse_int(num_str, sign_negative)?;

        // Fraction: "a / b"
        if let Some(d) = denom {
            let denominator: T = parse_int(d, false)?;
            if denominator.is_zero() {
                return Err(parse_error(RatioErrorKind::ZeroDenominator));
            }
            return Ok(Ratio::new(numerator, denominator));
        }

        // Pure integer (no decimal point, no exponent).
        if decimal.is_none() && exp.is_none() {
            return Ok(Ratio::from_integer(numerator));
        }

        // Decimal / scientific notation.
        let mut denominator = T::one();

        if let Some(dec) = decimal {
            let dec_clean = dec.replace('_', "");
            let dec_trimmed = dec_clean.trim_end_matches('0');
            let scale = pow10(
                u32::try_from(dec_trimmed.len())
                    .map_err(|_| parse_error(RatioErrorKind::Overflow))?,
            )?;

            let dec_val = if dec_trimmed.is_empty() {
                T::zero()
            } else {
                parse_int(dec_trimmed, sign_negative)?
            };

            numerator = numerator
                .checked_mul(&scale)
                .and_then(|n| n.checked_add(&dec_val))
                .ok_or_else(|| parse_error(RatioErrorKind::Overflow))?;

            denominator = denominator
                .checked_mul(&scale)
                .ok_or_else(|| parse_error(RatioErrorKind::Overflow))?;
        }

        if let Some(exp_str) = exp {
            let exp_val = exp_str
                .replace('_', "")
                .parse::<i32>()
                .map_err(|_| parse_error(RatioErrorKind::ParseError))?;

            let scale = pow10(exp_val.unsigned_abs())?;
            if exp_val >= 0 {
                numerator = numerator
                    .checked_mul(&scale)
                    .ok_or_else(|| parse_error(RatioErrorKind::Overflow))?;
            } else {
                denominator = denominator
                    .checked_mul(&scale)
                    .ok_or_else(|| parse_error(RatioErrorKind::Overflow))?;
            }
        }

        Ok(Ratio::new(numerator, denominator))
    }
}

#[inline]
fn parse_error(kind: RatioErrorKind) -> ParseRatioError {
    ParseRatioError { kind }
}

#[inline]
fn parse_int<T>(digits: &str, negative: bool) -> Result<T, ParseRatioError>
where
    T: FromStr + Zero,
{
    if digits.is_empty() {
        return Ok(T::zero());
    }
    // Fast path: positive number without underscores — zero allocations.
    if !negative && !digits.contains('_') {
        return T::from_str(digits).map_err(|_| parse_error(RatioErrorKind::Overflow));
    }
    // Slow path: prepend sign and/or strip underscores in one pass.
    let mut cleaned = String::with_capacity(digits.len() + 1);
    if negative {
        cleaned.push('-');
    }
    for c in digits.chars() {
        if c != '_' {
            cleaned.push(c);
        }
    }
    if cleaned.is_empty() || cleaned == "-" {
        return Ok(T::zero());
    }
    T::from_str(&cleaned).map_err(|_| parse_error(RatioErrorKind::Overflow))
}

#[inline]
fn pow10<T>(exp: u32) -> Result<T, ParseRatioError>
where
    T: Clone + CheckedMul + One + FromPrimitive,
{
    let ten = T::from_u8(10).ok_or_else(|| parse_error(RatioErrorKind::ParseError))?;
    if exp == 0 {
        return Ok(T::one());
    }
    let mut result = T::one();
    let mut base = ten;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result
                .checked_mul(&base)
                .ok_or_else(|| parse_error(RatioErrorKind::Overflow))?;
        }
        e >>= 1;
        if e > 0 {
            base = base
                .checked_mul(&base)
                .ok_or_else(|| parse_error(RatioErrorKind::Overflow))?;
        }
    }
    Ok(result)
}
