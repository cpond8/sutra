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

use sutra::macros::{expand, MacroDef, MacroRegistry, MacroTemplate};
use sutra::parser::parse;

/// A helper to parse a string and immediately expand it using the standard registry.
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

#[test]
fn test_cond_macro_expansion_two_clauses() {
    let expanded = parse_and_expand("(cond ((is? x 1) \"one\") (else \"other\"))");
    let expected = r#"(if (eq? (core/get (path x)) 1) "one" "other")"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_cond_macro_expansion_three_clauses() {
    let expanded = parse_and_expand("(cond ((is? x 1) \"one\") ((is? x 2) \"two\") (else \"other\"))");
    let expected = r#"(if (eq? (core/get (path x)) 1) "one" (if (eq? (core/get (path x)) 2) "two" "other"))"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_cond_macro_expansion_only_else() {
    let expanded = parse_and_expand("(cond (else 42))");
    let expected = r#"42"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_cond_macro_expansion_invalid_clause() {
    let exprs = parse("(cond ((is? x 1)) (else \"other\"))").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list of two elements"));
}

#[test]
fn test_cond_macro_expansion_missing_else() {
    let exprs = parse("(cond ((is? x 1) \"one\"))").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("`cond` macro requires at least one clause."));
}

#[test]
fn test_cond_macro_expansion_nested() {
    let expanded = parse_and_expand("(cond ((is? x 1) (cond ((is? y 2) \"a\") (else \"b\"))) (else \"c\"))");
    let expected = r#"(if (eq? (core/get (path x)) 1) (if (eq? (core/get (path y)) 2) "a" "b") "c")"#;
    assert_eq!(expanded, expected);
}

#[test]
fn test_declarative_variadic_macro_expansion() {
    // 1. Define a custom registry for this test.
    let mut registry = MacroRegistry::new();

    // 2. Define a simple variadic macro `my-list` that takes one required
    //    argument `first` and a variadic argument `rest`.
    let template = MacroTemplate {
        params: vec!["first".to_string()],
        variadic_param: Some("rest".to_string()),
        // The body just constructs a new list with the elements in a different order.
        body: Box::new(
            parse("(list first rest)")
                .unwrap()
                .into_iter()
                .next()
                .unwrap(),
        ),
    };
    registry
        .macros
        .insert("my-list".to_string(), MacroDef::Template(template));

    // 3. Parse the expression to be expanded.
    let expr_str = "(my-list 1 2 3)";
    let expr = parse(expr_str).unwrap().into_iter().next().unwrap();

    // 4. Expand the expression using our custom registry.
    let expanded_expr = registry.expand_recursive(&expr, 0).unwrap();

    // 5. Assert that the expansion is correct.
    // The `1` should be bound to `first`.
    // The `(2 3)` should be bound to `rest`.
    // The body `(list first rest)` should become `(list 1 (2 3))`.
    let expected = "(list 1 (2 3))";
    assert_eq!(expanded_expr.pretty(), expected);
}

#[test]
fn test_cond_macro_expansion_no_clauses() {
    let exprs = parse("(cond)").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("`cond` macro requires at least one clause."));
}

#[test]
fn test_cond_macro_expansion_non_list_clause() {
    let exprs = parse("(cond 42 (else 1))").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list."));
}

#[test]
fn test_cond_macro_expansion_else_not_last() {
    let exprs = parse("(cond (else 1) ((is? x 2) 2))").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("`else` clause must be the last clause in `cond`."));
}

#[test]
fn test_cond_macro_expansion_multiple_else_clauses() {
    let exprs = parse("(cond ((is? x 1) 1) (else 2) (else 3))").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("`else` clause must be the last clause in `cond`."));
}

#[test]
fn test_cond_macro_expansion_else_wrong_arity() {
    let exprs = parse("(cond (else 1 2))").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("`else` clause must have exactly one expression."));
}

#[test]
fn test_cond_macro_expansion_clause_not_list() {
    let exprs = parse("(cond 42)").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list."));
}

#[test]
fn test_cond_macro_expansion_clause_too_many_elements() {
    let exprs = parse("(cond ((is? x 1) 1 2) (else 3))").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list of two elements"));
}

#[test]
fn test_cond_macro_expansion_empty_clause_list() {
    let exprs = parse("(cond () (else 1))").unwrap();
    let result = sutra::macros::expand(&exprs[0]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list of two elements"));
}

#[test]
fn test_cond_macro_expansion_deeply_nested() {
    // 10-level deep cond
    let mut expr = String::from("(cond");
    for i in 0..10 {
        expr.push_str(&format!(" ((is? x {}) {} )", i, i));
    }
    expr.push_str(" (else 42))");
    let expanded = parse_and_expand(&expr);
    // Just check that the expansion contains the expected final else value
    assert!(expanded.contains("42"));
}
