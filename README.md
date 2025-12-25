# num-rational-parse
[![Crate](https://img.shields.io/crates/v/num-rational-parse.svg)](https://crates.io/crates/num-rational-parse)
[![Docs](https://docs.rs/num-rational-parse/badge.svg)](https://docs.rs/num-rational-parse)
[![Build Status](https://github.com/Boslx/num-rational-parse/actions/workflows/rust.yml/badge.svg)](https://github.com/Boslx/num-rational-parse/actions/workflows/rust.yml)

Extension for `num_rational` providing flexible string parsing for `Ratio<T>` types.

It allows you to parse:
- Fractions: `"3/4"`
- Decimals: `"1.25"`
- [Scientific notation](https://en.wikipedia.org/wiki/Scientific_notation#E_notation): `"1.2e-3"`, `"1E5"`

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
num-rational = "0.4"
num-rational-parse = "0.1"
```

Then import the `RationalParse` trait:

```rust
use num_rational::Ratio;
use num_rational_parse::RationalParse;

fn main() {
    let r = Ratio::<i32>::from_str_flex("3.14").unwrap();
    println!("{}", r); // Prints "157/50"

    let r2 = Ratio::<i32>::from_str_flex("1.2e-2").unwrap();
    println!("{}", r2); // Prints "3/250"
}
```

The standard `from_str` implementation in `num_rational` only supports the `numerator/denominator` format. `num-rational-parse` extends this to support decimals and scientific notation.

### Note on Precision

Unlike parsing to a floating-point number (like `f64`) and then converting to a fraction, `num-rational-parse` parses decimal strings directly into their exact rational representation. This avoids precision loss or rounding errors commonly associated with floating-point math.

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Credits

The parsing approach is inspired by Python®'s [fractions](https://github.com/python/cpython/blob/main/Lib/fractions.py) module.

## Trademark Notice

"Python" is a registered trademark of the Python Software Foundation.