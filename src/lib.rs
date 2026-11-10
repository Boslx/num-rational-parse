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
use num_traits::pow::checked_pow;
use num_traits::{CheckedAdd, CheckedMul, FromPrimitive, One, Zero};
use std::borrow::Cow;
use std::str::FromStr;
use winnow::ascii::digit1;
use winnow::combinator::{alt, eof, opt, peek, preceded};
use winnow::prelude::*;
use winnow::token::{one_of, take_while};

#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{kind}")]
pub struct ParseRatioError {
    #[from]
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
// Parser
//
// Port of CPython's fractions._RATIONAL_FORMAT
//
// Reference: https://github.com/python/cpython/blob/888d101/Lib/fractions.py#L56
// ---------------------------------------------------------------------------

/// The syntactic parts of a rational literal.
struct Parts<'a> {
    sign_negative: bool,
    num: &'a str,
    denom: Option<&'a str>,
    decimal: Option<&'a str>,
    exp: Option<&'a str>,
}

/// Digits with underscores allowed only between them (`\d*|\d+(_\d+)*`).
/// May match an empty string.
fn digits<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    take_while(0.., |c: char| c.is_ascii_digit() || c == '_')
        .verify(|s: &&str| !s.starts_with('_') && !s.ends_with('_') && !s.contains("__"))
        .parse_next(input)
}

/// Like [`digits`], but requires at least one digit (`\d+(_\d+)*`).
fn digits1<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    digits.verify(|s: &&str| !s.is_empty()).parse_next(input)
}

/// Optional whitespace around the number: anything Python's `\s` matches.
fn ws0(input: &mut &str) -> ModalResult<()> {
    take_while(0.., |c: char| c.is_whitespace())
        .void()
        .parse_next(input)
}

/// `(?=\d|\.\d)`: a digit, or a dot followed by a digit, must come next.
fn num_start(input: &mut &str) -> ModalResult<()> {
    peek(alt((digit1.void(), ('.', digit1).void()))).parse_next(input)
}

/// `[-+]?\d+(_\d+)*`, returned with its sign included.
fn exponent<'a>(input: &mut &'a str) -> ModalResult<&'a str> {
    let start = *input;
    preceded(opt(one_of(['+', '-'])), digits1).parse_next(input)?;
    Ok(&start[..start.len() - input.len()])
}

fn parts<'a>(input: &mut &'a str) -> ModalResult<Parts<'a>> {
    ws0.parse_next(input)?;
    let sign_negative = opt(one_of(['+', '-']))
        .map(|sign| sign == Some('-'))
        .parse_next(input)?;
    num_start.parse_next(input)?;
    let num = digits.parse_next(input)?;

    let (denom, decimal, exp) = alt((
        // Fraction: optional whitespace around '/'.
        preceded(ws0, preceded('/', preceded(ws0, digits1))).map(|d| (Some(d), None, None)),
        // Decimal part and/or exponent.
        (
            opt(preceded('.', digits)),
            opt(preceded(one_of(['E', 'e']), exponent)),
        )
            .map(|(decimal, exp)| (None, decimal, exp)),
    ))
    .parse_next(input)?;

    ws0.parse_next(input)?;
    eof.parse_next(input)?;

    Ok(Parts {
        sign_negative,
        num,
        denom,
        decimal,
        exp,
    })
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
    ///
    /// Intermediates must fit `T`: `0.125` is built as `125/10^3` before
    /// `Ratio::new` reduces it to `1/8`, so `Ratio::<i8>` reports
    /// [`RatioErrorKind::Overflow`] although `1/8` fits. There is no gcd pass
    /// on the digits, only the basic build-then-reduce. Parse into a wider
    /// type such as `i128` first, then convert the reduced result to `T`.
    fn from_str_flex(s: &str) -> Result<Self, ParseRatioError>;
}

impl<T> RationalParse for Ratio<T>
where
    T: Clone + Integer + FromStr + CheckedMul + CheckedAdd + FromPrimitive,
{
    fn from_str_flex(input: &str) -> Result<Self, ParseRatioError> {
        let Parts {
            sign_negative,
            num,
            denom,
            decimal,
            exp,
        } = parts.parse(input).map_err(|_| RatioErrorKind::ParseError)?;

        // Strip `_` separators once per field; everything below is underscore-free.
        let num = strip_underscores(num);
        let denom = denom.map(strip_underscores);
        let decimal = decimal.map(strip_underscores);
        let exp = exp.map(strip_underscores);

        let mut numerator = parse_int(&num, sign_negative)?;

        // Fraction: "a / b"
        if let Some(d) = denom {
            let denominator: T = parse_int(&d, false)?;
            if denominator.is_zero() {
                return Err(RatioErrorKind::ZeroDenominator.into());
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
            let dec_trimmed = dec.trim_end_matches('0');
            let scale = pow10(dec_trimmed.len())?;

            let dec_val = if dec_trimmed.is_empty() {
                T::zero()
            } else {
                parse_int(dec_trimmed, sign_negative)?
            };

            numerator = numerator
                .checked_mul(&scale)
                .and_then(|n| n.checked_add(&dec_val))
                .ok_or(RatioErrorKind::Overflow)?;

            denominator = denominator
                .checked_mul(&scale)
                .ok_or(RatioErrorKind::Overflow)?;
        }

        // Zero stays zero for any exponent; T may not hold 10^|exp|.
        if numerator.is_zero() {
            return Ok(Self::zero());
        }

        if let Some(exp_str) = exp {
            let exp_val = exp_str
                .parse::<i32>()
                .map_err(|_| RatioErrorKind::Overflow)?;

            let scale = pow10(exp_val.unsigned_abs() as usize)?;
            if exp_val >= 0 {
                numerator = numerator
                    .checked_mul(&scale)
                    .ok_or(RatioErrorKind::Overflow)?;
            } else {
                denominator = denominator
                    .checked_mul(&scale)
                    .ok_or(RatioErrorKind::Overflow)?;
            }
        }

        Ok(Ratio::new(numerator, denominator))
    }
}

/// Strip `_` digit separators, borrowing when there is nothing to strip.
#[inline]
fn strip_underscores(s: &str) -> Cow<'_, str> {
    if s.contains('_') {
        Cow::Owned(s.replace('_', ""))
    } else {
        Cow::Borrowed(s)
    }
}

/// Parse already underscore-free digits, applying the number's sign.
#[inline]
fn parse_int<T>(digits: &str, negative: bool) -> Result<T, RatioErrorKind>
where
    T: FromStr + Zero,
{
    if digits.is_empty() {
        return Ok(T::zero());
    }
    if negative {
        // "-0" is still zero, but unsigned types reject the sign.
        if digits.bytes().all(|b| b == b'0') {
            return Ok(T::zero());
        }
        return T::from_str(&format!("-{digits}")).map_err(|_| RatioErrorKind::Overflow);
    }
    T::from_str(digits).map_err(|_| RatioErrorKind::Overflow)
}

#[inline]
fn pow10<T>(exp: usize) -> Result<T, RatioErrorKind>
where
    T: Clone + CheckedMul + One + FromPrimitive,
{
    let ten = T::from_u8(10).ok_or(RatioErrorKind::ParseError)?;
    checked_pow(ten, exp).ok_or(RatioErrorKind::Overflow)
}
