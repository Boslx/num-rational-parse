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
fn test_valid_inputs_agree_with_python() {
    let test_cases: &[&str] = &[
        // --- Integers ---
        "0",
        "5",
        "-3",
        "+3",
        "005",
        // --- Fractions ---
        "3/4",
        "-5/2",
        "3/2",
        "-3/2",
        "+3/4",
        // --- Underscore-separated integers and fractions ---
        "1_234",
        "1_234/5_678",
        "1_2_3_4",
        "1_2/3_4",
        // --- Decimals ---
        "1.25",
        "-0.5",
        "3.1415",
        "2.25",
        "-3.",
        ".6",
        "3.2",
        "1.01",
        "0.0",
        "-0.0",
        ".123",
        "-.456",
        // --- Leading-dot edge cases ---
        "+.5",
        "-.5e-2",
        ".5E2",
        "-.5E-2",
        ".0_0_0_5",
        " .5 ",
        "\t.5\t",
        "-.0_0_0_5",
        ".5e+2",
        "+.5e-2",
        ".000_5",
        "-.000_5",
        ".5_0_0",
        "-.5_0_0",
        // --- Underscore decimals ---
        "1.5_000",
        "3.14_15",
        "1.0_1",
        // --- Scientific notation ---
        "1.2e-3",
        "-47e-2",
        "32.e-5",
        "1E+06",
        "-1.23e4",
        ".0e+0",
        "-0.000e0",
        "1E5",
        ".5e2",
        "-.5e2",
        // --- Underscore scientific ---
        "1.5_000e2",
        "1.1e+2_3",
        "1e+2_3",
        // --- Leading zeros ---
        "00",
        "00/01",
        "0.00",
        "00.5",
        // --- Zero variations ---
        "-0",
        "+0",
        "0/1",
        "0/5",
        "-0/5",
        ".0",
        ".0e+0",
        // --- Whitespace around slash ---
        "3 / 4",
        "3/ 4",
        "3 /4",
        // --- Whitespace padding ---
        " 3 ",
        "3 ",
        // --- Trailing newlines (Python uses \Z, Rust uses \s*\z — both accept) ---
        "3\n",
        "3/4\n",
        "1.5\n",
        "1e2\n",
        " 3 \n",
        "3\n\n",
        "3\r\n",
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
fn test_invalid_inputs_rejected_by_both() {
    // Inputs that Python's Fraction rejects *and* from_str_flex should also reject.
    // Both sides must agree these are unparseable.
    let test_cases: &[&str] = &[
        // Garbage
        "", "   ", "invalid", "abc", "3a2", "a3", "3a", // Double signs
        "--3", "++3", "+-3", "-+3", // Just sign
        "+", "-", // Malformed fractions
        "3/", "/2", "3/+2", "3/-2", "3/0", "1/0",
        // Whitespace between sign and number
        "+ 3/2", "- 3/2", // Whitespace inside decimal
        "3 .2", "3. 2", // Whitespace around exponent
        "3.2 e1", "3.2e 1", "3 e5", "3 E5", // Mixed formats
        "3/7.2", "3.2/7", // Multiple slashes
        "3/2/1", "3/2/", // Trailing slash after decimal
        "3./", "3.0/", // Fraction followed by decimal
        "3/4.5", "3.5/4", // Just slash
        "/",     // Just dot variants
        ".", "..", "...", // Exponent without digits
        ".e5", "3e", "e5", ".e", "3.e", "3e+", "3e-",
        // Fraction with trailing space after slash (no denom digits)
        "3/ ", "3 / ", // Leading slash
        " /4",
        // Underscore edge cases (accepted by Python but not as shown here — these specific patterns are rejected)
        "_3", "3_", "3._", "3_.5", "1__2", "3__4/5", // Leading-dot with bad underscores
        ".5_", "._5", "_.5", ".5__0", ".__5", "-._5", "+._5",
        // Underscore in exponent right after sign
        "5e-_2", ".5e-_2",
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
