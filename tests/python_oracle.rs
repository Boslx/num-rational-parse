//! Integration test using Python's `fractions.Fraction` as a test oracle.
//!
//! This test verifies that `num-rational-parse` produces the same results as
//! Python's `fractions.Fraction` for a wide variety of inputs — both valid
//! (should parse successfully) and invalid (should both reject).

use num_rational::Ratio;
use num_rational_parse::RationalParse;
use pyo3::types::PyAnyMethods;

/// Try to parse `input` with Python's `fractions.Fraction`.
///
/// Returns `Some((numerator, denominator))` on success, or `None` if Python
/// rejects the input (raises `ValueError` or `ZeroDivisionError`).
fn python_fraction_opt(input: &str) -> Option<(i128, i128)> {
    pyo3::Python::with_gil(|py| {
        let fractions = py.import("fractions").ok()?;
        let f = fractions.call_method1("Fraction", (input,)).ok()?;
        let num: i128 = f.getattr("numerator").ok()?.extract().ok()?;
        let den: i128 = f.getattr("denominator").ok()?.extract().ok()?;
        Some((num, den))
    })
}

#[test]
#[ignore = "Requires Python 3 development headers; run locally with: cargo test test_against_python_fractions_oracle -- --ignored"]
fn test_valid_inputs_agree_with_python() {
    let test_cases: &[&str] = &[
        // --- Integers ---
        "0", "5", "-3", "005", // --- Fractions ---
        "3/4", "-5/2", "3/2", "-3/2", // --- Decimals ---
        "1.25", "-0.5", "3.1415", "2.25", "-3.", ".6", "3.2", "1.01", "0.0", "-0.0",
        // --- Scientific notation ---
        "1.2e-3", "-47e-2", "32.e-5", "1E+06", "-1.23e4", ".0e+0", "-0.000e0", "1E5",
    ];

    for &input in test_cases {
        let py = python_fraction_opt(input)
            .unwrap_or_else(|| panic!("Python Fraction({input:?}) rejected a valid input"));

        let rust_ratio = Ratio::<i128>::from_str_flex(input)
            .unwrap_or_else(|e| panic!("Rust from_str_flex failed for '{}': {:?}", input, e));
        let rust_num = *rust_ratio.numer();
        let rust_den = *rust_ratio.denom();

        assert_eq!(
            (py.0, py.1),
            (rust_num, rust_den),
            "Mismatch for input '{}':\n  Python: {}/{}\n  Rust:   {}/{}",
            input,
            py.0,
            py.1,
            rust_num,
            rust_den
        );
    }
}

#[test]
#[ignore = "Requires Python 3 development headers; run locally with: cargo test test_against_python_fractions_oracle -- --ignored"]
fn test_invalid_inputs_rejected_by_both() {
    // Inputs that Python's Fraction rejects *and* from_str_flex should also reject.
    // Both sides must agree these are unparseable.
    let test_cases: &[&str] = &[
        // Garbage
        "", "invalid", "abc", "3a2", "--3", "++3", // Malformed fractions
        "3/", "/2", "3/+2", "3/-2", "3/0", "1/0", // Mixed formats
        "3/7.2", "3.2/7", // Whitespace between tokens
        "+ 3/2", "- 3/2", "3 .2", "3. 2", "3.2 e1", "3.2e 1",
        // Leading/trailing operators
        "3/2/1", "3/2/", ".e5",
    ];

    for &input in test_cases {
        let python_rejected = python_fraction_opt(input).is_none();
        let rust_rejected = Ratio::<i128>::from_str_flex(input).is_err();

        assert!(
            python_rejected && rust_rejected,
            "Both should reject '{}', but:\n  Python rejects: {}\n  Rust rejects:  {}",
            input,
            python_rejected,
            rust_rejected,
        );
    }
}
