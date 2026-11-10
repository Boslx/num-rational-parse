# Changelog

## [0.2.1] - 2026-11-11

- Replaced the `regex` parser with [`winnow`](https://crates.io/crates/winnow),
  making parsing 3.7×–5.6× faster.
- Changed error kinds: Unicode digits (`"٣"`) give `ParseError` instead of
  `Overflow`, and an exponent that does not fit `i32` gives `Overflow` instead
  of `ParseError`.

## [0.2.0] - 2026-07-27

- Errors now implement `std::error::Error` via `thiserror`, with clearer
  messages.
- Parser consolidated into a single regex

## [0.1.1] - 2026-07-24

- Fixed parsing of `+` prefixes and signed integer minimum values (e.g.
  `i8::MIN`, `i128::MIN`).

## [0.1.0] - 2025-12-25

- Initial release: `RationalParse::from_str_flex` for integers, fractions,
  decimals, and scientific notation, with underscore digit separators, exact
  rational parsing, and `ParseRatioError`/`RatioErrorKind`.
