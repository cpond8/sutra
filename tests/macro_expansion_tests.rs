//! tests/macro_expansion_tests.rs

//! # Macro Expansion Integration Tests
//!
//! This test suite is dedicated to verifying the correctness of the macro
//! expansion pipeline stage in isolation. It ensures that author-facing macros
//! expand into the expected canonical AST forms before they are passed to the
//! evaluation engine.
//!
//! ## Principles
//!
//! - **No Evaluation**: These tests do not run the `eval` pipeline. They stop
//!   at macro expansion.
//! - **Structural Correctness**: The primary goal is to assert that the *structure*
//!   of the expanded AST is correct.
//! - **Contract Verification**: These tests verify the contract between the macro
//!   system and the atom system, ensuring macros produce valid input for atoms.

use sutra::macros::expand;
use sutra::parser::parse;

/// A helper to parse a string and immediately expand it.
fn parse_and_expand(s: &str) -> String {
    let exprs = parse(s).unwrap();
    // We assume these tests operate on a single expression.
    let expanded_expr = expand(&exprs[0]).unwrap();
    expanded_expr.pretty()
}

#[test]
fn test_add_macro_expansion() {
    let expanded = parse_and_expand("(add! score 10)");
    let expected = r#"(core/set! (path score) (+ (core/get (path score)) 10))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_inc_macro_expansion_simple_symbol() {
    let expanded = parse_and_expand("(inc! score)");
    let expected = r#"(core/set! (path score) (+ (core/get (path score)) 1))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_inc_macro_expansion_dotted_symbol() {
    let expanded = parse_and_expand("(inc! player.score)");
    let expected = r#"(core/set! (path player score) (+ (core/get (path player score)) 1))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_dec_macro_expansion() {
    let expanded = parse_and_expand("(dec! player.health)");
    let expected = r#"(core/set! (path player health) (- (core/get (path player health)) 1))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_is_macro_expansion_with_symbols() {
    let expanded = parse_and_expand(r#"(is? player.state "active")"#);
    let expected = r#"(eq? (core/get (path player state)) "active")"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_is_macro_expansion_with_literals() {
    let expanded = parse_and_expand("(is? 10 10)");
    let expected = r#"(eq? 10 10)"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_nested_macro_expansion() {
    let expanded = parse_and_expand("(add! score (inc! other.value))");
    let expected = r#"(core/set! (path score) (+ (core/get (path score)) (core/set! (path other value) (+ (core/get (path other value)) 1))))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_add_macro_expansion_list_of_symbols() {
    let expanded = parse_and_expand("(add! (player score) 5)");
    let expected = r#"(core/set! (path player score) (+ (core/get (path player score)) 5))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_add_macro_expansion_list_of_strings() {
    let expanded = parse_and_expand("(add! (\"player\" \"score\") 5)");
    let expected = r#"(core/set! (path player score) (+ (core/get (path player score)) 5))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_sub_macro_expansion() {
    let expanded = parse_and_expand("(sub! player.score 2)");
    let expected = r#"(core/set! (path player score) (- (core/get (path player score)) 2))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_add_macro_expansion_invalid_path_mixed_types() {
    let exprs = parse("(add! (player \"score\" 123) 5)").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{}", err);
    assert!(msg.contains("Path lists can only contain symbols or strings."));
}

#[test]
fn test_inc_macro_expansion_invalid_path_number() {
    let exprs = parse("(inc! 123)").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Invalid path format: expected a symbol or a list."));
}
