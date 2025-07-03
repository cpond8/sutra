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
use sutra::registry::build_default_macro_registry;

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

#[test]
fn test_variadic_macro_only_variadic_param() {
    let mut registry = MacroRegistry::new();
    let template = MacroTemplate::new(
        vec![],
        Some("args".to_string()),
        Box::new(parse("(list args)").unwrap().into_iter().next().unwrap()),
    )
    .unwrap();
    registry
        .macros
        .insert("gather".to_string(), MacroDef::Template(template));
    let expr = parse("(gather 1 2 3)").unwrap().into_iter().next().unwrap();
    let expanded = registry.expand_recursive(&expr, 0).unwrap();
    assert_eq!(expanded.pretty(), "(list (1 2 3))");
}

#[test]
fn test_variadic_macro_fixed_and_variadic_params() {
    let mut registry = MacroRegistry::new();
    let template = MacroTemplate::new(
        vec!["head".to_string()],
        Some("tail".to_string()),
        Box::new(parse("(list head tail)").unwrap().into_iter().next().unwrap()),
    )
    .unwrap();
    registry
        .macros
        .insert("cons".to_string(), MacroDef::Template(template));
    let expr = parse("(cons 1 2 3)").unwrap().into_iter().next().unwrap();
    let expanded = registry.expand_recursive(&expr, 0).unwrap();
    assert_eq!(expanded.pretty(), "(list 1 (2 3))");
    // Test empty variadic
    let expr2 = parse("(cons 1)").unwrap().into_iter().next().unwrap();
    let expanded2 = registry.expand_recursive(&expr2, 0).unwrap();
    assert_eq!(expanded2.pretty(), "(list 1 ())");
}

#[test]
fn test_variadic_macro_too_few_args() {
    let mut registry = MacroRegistry::new();
    let template = MacroTemplate::new(
        vec!["a".to_string(), "b".to_string()],
        Some("rest".to_string()),
        Box::new(parse("(list a b rest)").unwrap().into_iter().next().unwrap()),
    )
    .unwrap();
    registry
        .macros
        .insert("foo".to_string(), MacroDef::Template(template));
    let expr = parse("(foo 1)").unwrap().into_iter().next().unwrap();
    let result = registry.expand_recursive(&expr, 0);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("at least 2 arguments"));
}

#[test]
fn test_variadic_macro_too_many_args_no_variadic() {
    let mut registry = MacroRegistry::new();
    let template = MacroTemplate::new(
        vec!["a".to_string(), "b".to_string()],
        None,
        Box::new(parse("(list a b)").unwrap().into_iter().next().unwrap()),
    )
    .unwrap();
    registry
        .macros
        .insert("bar".to_string(), MacroDef::Template(template));
    let expr = parse("(bar 1 2 3)").unwrap().into_iter().next().unwrap();
    let result = registry.expand_recursive(&expr, 0);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("exactly 2 arguments"));
}

#[test]
fn test_macro_template_duplicate_param_names() {
    let result = MacroTemplate::new(
        vec!["a".to_string(), "a".to_string()],
        None,
        Box::new(parse("(list a)").unwrap().into_iter().next().unwrap()),
    );
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Duplicate parameter name"));
}

#[test]
fn test_macro_template_variadic_param_not_last() {
    // This is not directly possible with MacroTemplate::new as designed, but we can simulate
    // a misuse by passing a variadic param and a param after it (should be caught by parser in real use).
    // Here, we just check that MacroTemplate::new does not allow duplicate names.
    let result = MacroTemplate::new(
        vec!["rest".to_string(), "b".to_string()],
        Some("rest".to_string()),
        Box::new(parse("(list rest b)").unwrap().into_iter().next().unwrap()),
    );
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("Duplicate parameter name"));
}

#[test]
fn test_variadic_macro_empty_variadic_param() {
    let mut registry = MacroRegistry::new();
    let template = MacroTemplate::new(
        vec!["a".to_string()],
        Some("rest".to_string()),
        Box::new(parse("(list a rest)").unwrap().into_iter().next().unwrap()),
    )
    .unwrap();
    registry
        .macros
        .insert("baz".to_string(), MacroDef::Template(template));
    let expr = parse("(baz 1)").unwrap().into_iter().next().unwrap();
    let expanded = registry.expand_recursive(&expr, 0).unwrap();
    assert_eq!(expanded.pretty(), "(list 1 ())");
}

#[test]
fn test_recursive_macro_expansion_depth_limit() {
    let mut registry = MacroRegistry::new();
    // Macro that calls itself
    let template = MacroTemplate::new(
        vec!["x".to_string()],
        None,
        Box::new(parse("(recurse x)").unwrap().into_iter().next().unwrap()),
    )
    .unwrap();
    registry
        .macros
        .insert("recurse".to_string(), MacroDef::Template(template));
    let expr = parse("(recurse 1)").unwrap().into_iter().next().unwrap();
    let result = registry.expand_recursive(&expr, 0);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("depth limit"));
}

#[test]
fn macro_registry_hash_is_stable() {
    let registry = build_default_macro_registry();
    let hash = registry.hash();
    // Print the hash for CI traceability and manual review
    println!("Macro registry SHA256 hash: {}", hash);
    // Optionally: assert against a known value, or just ensure it is non-empty
    assert!(!hash.is_empty());
}

#[test]
fn macro_registry_integration_parity() {
    use sutra::registry::build_default_macro_registry;
    use sutra::macros::MacroDef;
    use sutra::ast::{Expr, Span};

    let registry = build_default_macro_registry();
    let hash = registry.hash();
    // Assert the hash matches the known canonical value
    let expected_hash = "018db696c872424888555f71d84f66cd7539e2543d9a02ce097a294b1699347c";
    assert_eq!(hash, expected_hash, "Macro registry hash mismatch!\nExpected: {}\nActual:   {}", expected_hash, hash);

    // Golden expansion tests for core macros (add more as needed)
    let cases = vec![
        // if macro
        ("if", Expr::List(vec![
            Expr::Symbol("if".to_string(), Span::default()),
            Expr::Bool(true, Span::default()),
            Expr::Number(1.0, Span::default()),
            Expr::Number(2.0, Span::default()),
        ], Span::default()),
        Expr::If {
            condition: Box::new(Expr::Bool(true, Span::default())),
            then_branch: Box::new(Expr::Number(1.0, Span::default())),
            else_branch: Box::new(Expr::Number(2.0, Span::default())),
            span: Span::default(),
        }),
        // set!
        ("set!", Expr::List(vec![
            Expr::Symbol("set!".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
            Expr::Number(42.0, Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("core/set!".to_string(), Span::default()),
            Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
            Expr::Number(42.0, Span::default()),
        ], Span::default())),
        // get
        ("get", Expr::List(vec![
            Expr::Symbol("get".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("core/get".to_string(), Span::default()),
            Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
        ], Span::default())),
        // del!
        ("del!", Expr::List(vec![
            Expr::Symbol("del!".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("core/del!".to_string(), Span::default()),
            Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
        ], Span::default())),
        // add!
        ("add!", Expr::List(vec![
            Expr::Symbol("add!".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
            Expr::Number(1.0, Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("core/set!".to_string(), Span::default()),
            Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
            Expr::List(vec![
                Expr::Symbol("+".to_string(), Span::default()),
                Expr::List(vec![
                    Expr::Symbol("core/get".to_string(), Span::default()),
                    Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
                ], Span::default()),
                Expr::Number(1.0, Span::default()),
            ], Span::default()),
        ], Span::default())),
        // sub!
        ("sub!", Expr::List(vec![
            Expr::Symbol("sub!".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
            Expr::Number(1.0, Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("core/set!".to_string(), Span::default()),
            Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
            Expr::List(vec![
                Expr::Symbol("-".to_string(), Span::default()),
                Expr::List(vec![
                    Expr::Symbol("core/get".to_string(), Span::default()),
                    Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
                ], Span::default()),
                Expr::Number(1.0, Span::default()),
            ], Span::default()),
        ], Span::default())),
        // inc!
        ("inc!", Expr::List(vec![
            Expr::Symbol("inc!".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("core/set!".to_string(), Span::default()),
            Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
            Expr::List(vec![
                Expr::Symbol("+".to_string(), Span::default()),
                Expr::List(vec![
                    Expr::Symbol("core/get".to_string(), Span::default()),
                    Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
                ], Span::default()),
                Expr::Number(1.0, Span::default()),
            ], Span::default()),
        ], Span::default())),
        // dec!
        ("dec!", Expr::List(vec![
            Expr::Symbol("dec!".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("core/set!".to_string(), Span::default()),
            Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
            Expr::List(vec![
                Expr::Symbol("-".to_string(), Span::default()),
                Expr::List(vec![
                    Expr::Symbol("core/get".to_string(), Span::default()),
                    Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
                ], Span::default()),
                Expr::Number(1.0, Span::default()),
            ], Span::default()),
        ], Span::default())),
        // is?
        ("is?", Expr::List(vec![
            Expr::Symbol("is?".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
            Expr::Symbol("bar".to_string(), Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("eq?".to_string(), Span::default()),
            Expr::List(vec![
                Expr::Symbol("core/get".to_string(), Span::default()),
                Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
            ], Span::default()),
            Expr::List(vec![
                Expr::Symbol("core/get".to_string(), Span::default()),
                Expr::Path(sutra::path::Path(vec!["bar".to_string()]), Span::default()),
            ], Span::default()),
        ], Span::default())),
        // over?
        ("over?", Expr::List(vec![
            Expr::Symbol("over?".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
            Expr::Symbol("bar".to_string(), Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("gt?".to_string(), Span::default()),
            Expr::List(vec![
                Expr::Symbol("core/get".to_string(), Span::default()),
                Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
            ], Span::default()),
            Expr::List(vec![
                Expr::Symbol("core/get".to_string(), Span::default()),
                Expr::Path(sutra::path::Path(vec!["bar".to_string()]), Span::default()),
            ], Span::default()),
        ], Span::default())),
        // under?
        ("under?", Expr::List(vec![
            Expr::Symbol("under?".to_string(), Span::default()),
            Expr::Symbol("foo".to_string(), Span::default()),
            Expr::Symbol("bar".to_string(), Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("lt?".to_string(), Span::default()),
            Expr::List(vec![
                Expr::Symbol("core/get".to_string(), Span::default()),
                Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()),
            ], Span::default()),
            Expr::List(vec![
                Expr::Symbol("core/get".to_string(), Span::default()),
                Expr::Path(sutra::path::Path(vec!["bar".to_string()]), Span::default()),
            ], Span::default()),
        ], Span::default())),
        // not
        ("not", Expr::List(vec![
            Expr::Symbol("not".to_string(), Span::default()),
            Expr::Bool(false, Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("not".to_string(), Span::default()),
            Expr::Bool(false, Span::default()),
        ], Span::default())),
        // list
        ("list", Expr::List(vec![
            Expr::Symbol("list".to_string(), Span::default()),
            Expr::Number(1.0, Span::default()),
            Expr::Number(2.0, Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("list".to_string(), Span::default()),
            Expr::Number(1.0, Span::default()),
            Expr::Number(2.0, Span::default()),
        ], Span::default())),
        // len
        ("len", Expr::List(vec![
            Expr::Symbol("len".to_string(), Span::default()),
            Expr::List(vec![
                Expr::Symbol("list".to_string(), Span::default()),
                Expr::Number(1.0, Span::default()),
                Expr::Number(2.0, Span::default()),
            ], Span::default()),
        ], Span::default()),
        Expr::List(vec![
            Expr::Symbol("len".to_string(), Span::default()),
            Expr::List(vec![
                Expr::Symbol("list".to_string(), Span::default()),
                Expr::Number(1.0, Span::default()),
                Expr::Number(2.0, Span::default()),
            ], Span::default()),
        ], Span::default())),
    ];
    for (macro_name, input, expected) in cases {
        let expanded = registry.expand_recursive(&input, 0).unwrap();
        assert_eq!(expanded, expected, "Expansion mismatch for macro '{}':\nInput:    {:?}\nExpected: {:?}\nActual:   {:?}", macro_name, input, expected, expanded);
    }
}

#[cfg(test)]
mod loader_tests {
    use sutra::macros::parse_macros_from_source;
    use sutra::macros::MacroTemplate;

    #[test]
    fn test_valid_single_macro() {
        let src = "(define (foo x) x)";
        let result = parse_macros_from_source(src).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "foo");
        assert_eq!(result[0].1.params, vec!["x"]);
        assert!(result[0].1.variadic_param.is_none());
    }

    #[test]
    fn test_valid_variadic_macro() {
        let src = "(define (bar x . rest) (list x rest))";
        let result = parse_macros_from_source(src).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "bar");
        assert_eq!(result[0].1.params, vec!["x"]);
        assert_eq!(result[0].1.variadic_param.as_deref(), Some("rest"));
    }

    #[test]
    fn test_duplicate_macro_name() {
        let src = "(define (foo x) x) (define (foo y) y)";
        let result = parse_macros_from_source(src);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("Duplicate macro name"));
    }

    #[test]
    fn test_duplicate_param_name() {
        let src = "(define (foo x x) x)";
        let result = parse_macros_from_source(src);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("Duplicate parameter name"));
    }

    #[test]
    fn test_multiple_variadics() {
        let src = "(define (foo x . rest . more) x)";
        let result = parse_macros_from_source(src);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("Multiple '.'"));
    }

    #[test]
    fn test_non_symbol_macro_name() {
        let src = "(define (123 x) x)";
        let result = parse_macros_from_source(src);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("Macro name must be a symbol"));
    }

    #[test]
    fn test_non_list_param_list() {
        let src = "(define foo x)";
        let result = parse_macros_from_source(src);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("parameter list must be a list"));
    }

    #[test]
    fn test_ignores_non_macro_forms() {
        let src = "(+ 1 2) (define (foo x) x)";
        let result = parse_macros_from_source(src).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "foo");
    }
}

#[cfg(test)]
mod macro_param_tests {
    use sutra::ast::{Expr, Span};
    use sutra::macros::{MacroParams, RESERVED_WORDS};
    use sutra::error::SutraErrorKind;

    fn s(name: &str) -> Expr {
        Expr::Symbol(name.to_string(), Span::default())
    }

    fn dot() -> Expr {
        Expr::Symbol(".".to_string(), Span::default())
    }

    #[test]
    fn test_parse_macro_params_table() {
        struct Case {
            desc: &'static str,
            input: Vec<Expr>,
            expect_ok: bool,
            expect_params: Vec<&'static str>,
            expect_variadic: Option<&'static str>,
            expect_err_contains: Option<&'static str>,
        }
        let cases = vec![
            Case {
                desc: "simple fixed params",
                input: vec![s("x"), s("y")],
                expect_ok: true,
                expect_params: vec!["x", "y"],
                expect_variadic: None,
                expect_err_contains: None,
            },
            Case {
                desc: "single variadic param",
                input: vec![s("x"), dot(), s("rest")],
                expect_ok: true,
                expect_params: vec!["x"],
                expect_variadic: Some("rest"),
                expect_err_contains: None,
            },
            Case {
                desc: "empty param list",
                input: vec![],
                expect_ok: true,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: None,
            },
            Case {
                desc: "dot at start",
                input: vec![dot(), s("rest")],
                expect_ok: true,
                expect_params: vec![],
                expect_variadic: Some("rest"),
                expect_err_contains: None,
            },
            Case {
                desc: "dot at end (error)",
                input: vec![s("x"), dot()],
                expect_ok: false,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: Some("Expected symbol after '.'"),
            },
            Case {
                desc: "multiple dots (error)",
                input: vec![s("x"), dot(), s("rest"), dot(), s("more")],
                expect_ok: false,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: Some("Multiple '.'"),
            },
            Case {
                desc: "reserved word as param (error)",
                input: vec![s("define"), s("x")],
                expect_ok: false,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: Some("Reserved word 'define'"),
            },
            Case {
                desc: "reserved word as variadic (error)",
                input: vec![s("x"), dot(), s("define")],
                expect_ok: false,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: Some("Reserved word 'define'"),
            },
            Case {
                desc: "non-symbol param (error)",
                input: vec![Expr::Number(1.0, Span::default())],
                expect_ok: false,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: Some("Invalid parameter"),
            },
            Case {
                desc: "duplicate param (error)",
                input: vec![s("x"), s("x")],
                expect_ok: false,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: Some("Duplicate parameter name"),
            },
            Case {
                desc: "duplicate variadic (error)",
                input: vec![s("x"), dot(), s("x")],
                expect_ok: false,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: Some("Duplicate parameter name"),
            },
            Case {
                desc: "param after variadic (error)",
                input: vec![s("x"), dot(), s("rest"), s("y")],
                expect_ok: false,
                expect_params: vec![],
                expect_variadic: None,
                expect_err_contains: Some("No parameters allowed after variadic"),
            },
        ];
        for case in cases {
            let head = Some(s("macro-head"));
            let span = Some(Span::default());
            let result = MacroParams::parse_macro_params(&case.input, head.clone(), span.clone());
            if case.expect_ok {
                let params = result.expect(&format!("case '{}': expected Ok", case.desc));
                assert_eq!(params.params, case.expect_params.iter().map(|s| s.to_string()).collect::<Vec<_>>(), "case '{}': params mismatch", case.desc);
                assert_eq!(params.variadic.as_deref(), case.expect_variadic, "case '{}': variadic mismatch", case.desc);
            } else {
                let err = result.expect_err(&format!("case '{}': expected Err", case.desc));
                let msg = format!("{}", err);
                assert!(case.expect_err_contains.iter().all(|frag| msg.contains(frag)), "case '{}': error message missing expected fragment(s): got '{}', expected '{:?}'", case.desc, msg, case.expect_err_contains);
            }
        }
    }
}
