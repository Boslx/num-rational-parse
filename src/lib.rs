//! Flexible string parsing for `num_rational`.
//!
//! This crate provides flexible string parsing for rational numbers, inspired by
//! Python's `fractions` module, allowing `num_rational::Ratio` to be parsed from
//! strings with flexible formatting.
//!
//! # Examples
//!
//! ```rust
//! use num_rational::Ratio;
//! use num_rational_parse::RationalParse;
//!
//! let r = Ratio::<i32>::from_str_flex("3.14").unwrap();
//! assert_eq!(r, Ratio::new(157, 50));
//!
//! let r2 = Ratio::<i32>::from_str_flex("1.2e-2").unwrap();
//! assert_eq!(r2, Ratio::new(3, 250));
//!
//! let r3 = Ratio::<i32>::from_str_flex("-1_000/2_000").unwrap();
//! assert_eq!(r3, Ratio::new(-1, 2));
//! ```

use num_integer::Integer;
use num_rational::Ratio;
use num_traits::{CheckedAdd, CheckedMul, FromPrimitive, Signed};
use regex::Regex;
use std::str::FromStr;

/// An error which can be returned when parsing a ratio.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ParseRatioError {
    kind: RatioErrorKind,
}

impl ParseRatioError {
    /// Returns the specific type of error that occurred.
    pub fn kind(&self) -> &RatioErrorKind {
        &self.kind
    }
}

impl std::fmt::Display for ParseRatioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.description().fmt(f)
    }
}

/// The specific type of error that occurred during parsing.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RatioErrorKind {
    /// The string could not be parsed as a ratio.
    ///
    /// This occurs if the input string does not match the expected format
    /// (e.g., contains invalid characters or is empty).
    ParseError,
    /// The denominator was zero.
    ///
    /// Ratios cannot have a zero denominator.
    ZeroDenominator,
    /// The parsed value cannot be represented by the target type.
    ///
    /// This occurs if the numerator, denominator, or intermediate values
    /// overflow the capacity of the integer type `T`.
    Overflow,
}

impl RatioErrorKind {
    fn description(&self) -> &'static str {
        match *self {
            RatioErrorKind::ParseError => "failed to parse integer",
            RatioErrorKind::ZeroDenominator => "zero value denominator",
            RatioErrorKind::Overflow => "overflow",
        }
    }
}

impl std::fmt::Display for RatioErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.description().fmt(f)
    }
}

/// A trait for parsing a string into a rational number with flexible formats.
///
/// This trait extends `num_rational::Ratio` to support parsing strings in formats
/// accepted by Python's `fractions.Fraction` class, including:
/// - Fractions: `"1/2"`
/// - Decimals: `"1.5"`
/// - Scientific notation: `"1.2e-3"`, `"1E5"`
pub trait RationalParse: Sized {
    /// Parses a string into a rational number.
    ///
    /// The input string can be in various formats:
    /// - `"-35/4"` (Fraction)
    /// - `"3.1415"` (Decimal)
    /// - `"-47e-2"` (Scientific notation)
    ///
    /// # Errors
    ///
    /// Returns [`ParseRatioError`] if the string is not a valid rational number string
    /// or if it represents a valid number that cannot be represented by the target type
    /// (e.g. overflow).
    fn from_str_flex(s: &str) -> Result<Self, ParseRatioError>;
}

use std::sync::LazyLock;

/// Returns the regular expression for parsing rational numbers.
///
/// This regex is adapted from Python's `fractions` module, with additional capture
/// groups and detailed comments for clarity.
///
/// Note: The lookahead `(?=\d|\.\d)` present in the Python reference is omitted here
/// as it is not supported by the `regex` crate; the check is performed manually
/// in the parsing logic.
///
/// Python reference:
/// https://github.com/python/cpython/blob/888d101445c72c7cf23923e99ed567732f42fb79/Lib/fractions.py#L56
static RATIONAL_FORMAT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?xi)                                # Case-insensitive, verbose mode
        \A\s*                                  # optional whitespace at the start,
        (?P<sign>[-+]?)                        # an optional sign, then
        (?P<num>\d*|\d+(_\d+)*)                # numerator (possibly empty)
        (?:                                    # followed by
           (?:\s*/\s*(?P<denom>\d+(_\d+)*))?   # an optional denominator
        |                                      # or
           (?:\.(?P<decimal>\d*|\d+(_\d+)*))?  # an optional fractional part
           (?:E(?P<exp>[-+]?\d+(_\d+)*))?      # and optional exponent
        )
        \s*\z                                  # and optional whitespace to finish
        ",
    )
    .unwrap()
});

impl<T> RationalParse for Ratio<T>
where
    T: Clone + Integer + Signed + FromStr + CheckedMul + CheckedAdd + FromPrimitive,
    <T as FromStr>::Err: std::fmt::Display,
{
    fn from_str_flex(input: &str) -> Result<Self, ParseRatioError> {
        let cap = RATIONAL_FORMAT.captures(input).ok_or(ParseRatioError {
            kind: RatioErrorKind::ParseError,
        })?;

        let sign_str = cap.name("sign").map(|m| m.as_str()).unwrap_or("");
        let num_str = cap.name("num").map(|m| m.as_str()).unwrap_or("");
        let denom_str = cap.name("denom").map(|m| m.as_str());
        let decimal_str = cap.name("decimal").map(|m| m.as_str());
        let exp_str = cap.name("exp").map(|m| m.as_str());

        // Validate "lookahead" equivalent
        let num_has_digits = !num_str.is_empty();
        let decimal_has_digits = decimal_str.is_some_and(|s| !s.is_empty());

        if !num_has_digits && !decimal_has_digits {
            return Err(ParseRatioError {
                kind: RatioErrorKind::ParseError,
            });
        }

        let parse_val = |s: &str| -> Result<T, ParseRatioError> {
            if s.is_empty() {
                return Ok(T::zero());
            }
            if s.contains('_') {
                let s_clean = s.replace('_', "");
                T::from_str(&s_clean).map_err(|_| ParseRatioError {
                    kind: RatioErrorKind::Overflow,
                })
            } else {
                T::from_str(s).map_err(|_| ParseRatioError {
                    kind: RatioErrorKind::Overflow,
                })
            }
        };

        let ten = T::from_u8(10).ok_or(ParseRatioError {
            kind: RatioErrorKind::ParseError,
        })?;

        let checked_pow = |base: &T, exp: u32| -> Result<T, ParseRatioError> {
            num_traits::checked_pow(base.clone(), exp as usize).ok_or(ParseRatioError {
                kind: RatioErrorKind::Overflow,
            })
        };

        let mut numerator: T = parse_val(num_str)?;
        let mut denominator: T;

        if let Some(d_str) = denom_str {
            denominator = parse_val(d_str)?;
        } else {
            denominator = T::one();
            if let Some(dec) = decimal_str {
                // Strip trailing zeros to avoid unnecessary overflow and create more efficient rationals
                // e.g., "1.0000000000" becomes "1.0" instead of creating denominator = 10^10
                let dec_trimmed = dec.trim_end_matches('0');
                let dec_clean_owned: String;
                let dec_final = if dec_trimmed.contains('_') {
                    dec_clean_owned = dec_trimmed.replace('_', "");
                    &dec_clean_owned
                } else {
                    dec_trimmed
                };

                // Power of 10 equal to number of significant decimal digits
                let scale = checked_pow(&ten, dec_final.len() as u32)?;

                let dec_val = if dec_final.is_empty() {
                    T::zero()
                } else {
                    T::from_str(dec_final).map_err(|_| ParseRatioError {
                        kind: RatioErrorKind::Overflow,
                    })?
                };

                numerator = numerator
                    .checked_mul(&scale)
                    .ok_or(ParseRatioError {
                        kind: RatioErrorKind::Overflow,
                    })?
                    .checked_add(&dec_val)
                    .ok_or(ParseRatioError {
                        kind: RatioErrorKind::Overflow,
                    })?;

                denominator = denominator.checked_mul(&scale).ok_or(ParseRatioError {
                    kind: RatioErrorKind::Overflow,
                })?;
            }
            if let Some(exp_s) = exp_str {
                let exp_clean_owned: String;
                let exp_final = if exp_s.contains('_') {
                    exp_clean_owned = exp_s.replace('_', "");
                    &exp_clean_owned
                } else {
                    exp_s
                };
                let exp_val = exp_final.parse::<i32>().map_err(|_| ParseRatioError {
                    kind: RatioErrorKind::ParseError,
                })?;

                let abs_exp = exp_val.unsigned_abs();
                let scale = checked_pow(&ten, abs_exp)?;

                if exp_val >= 0 {
                    numerator = numerator.checked_mul(&scale).ok_or(ParseRatioError {
                        kind: RatioErrorKind::Overflow,
                    })?;
                } else {
                    denominator = denominator.checked_mul(&scale).ok_or(ParseRatioError {
                        kind: RatioErrorKind::Overflow,
                    })?;
                }
            }
        }

        if sign_str == "-" {
            numerator = -numerator;
        }

        if denominator.is_zero() {
            return Err(ParseRatioError {
                kind: RatioErrorKind::ZeroDenominator,
            });
        }

        Ok(Ratio::new(numerator, denominator))
    }
}
