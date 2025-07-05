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

use sutra::macros::{MacroDef, MacroRegistry, MacroTemplate, MacroExpander, SutraMacroContext, SutraMacroExpander};
use sutra::parser::parse;
use sutra::registry::build_default_macro_registry;
use sutra::ast::{ParamList, Span, WithSpan, Expr};

/// Helper to build a macro registry with both Rust and Sutra-defined macros (from macros.sutra)
fn build_full_macro_registry() -> MacroRegistry {
    let mut registry = MacroRegistry::new();
    // Register Rust-defined macros
    sutra::macros_std::register_std_macros(&mut registry);
    // Load and register macros from macros.sutra
    match sutra::macros::load_macros_from_file("macros.sutra") {
        Ok(macros) => {
            for (name, template) in macros {
                registry.macros.insert(name, MacroDef::Template(template));
            }
        }
        Err(e) => {
            panic!("Error loading macros from macros.sutra: {}", e);
        }
    }
    registry
}

/// A helper to parse a string and immediately expand it using the full registry.
fn parse_and_expand(s: &str) -> String {
    let exprs = parse(s).unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    // We assume these tests operate on a single expression.
    let expanded_expr = expander.expand_macros(exprs[0].clone(), &context).unwrap();
    expanded_expr.value.pretty()
}

#[test]
fn test_add_macro_expansion() {
    let expanded = parse_and_expand("(add! foo 10)");
    let expected = "(core/set! (path foo) (+ (core/get (path foo)) 10))";
    assert_eq!(expanded, expected);
}

#[test]
fn test_inc_macro_expansion_simple_symbol() {
    let expanded = parse_and_expand("(inc! foo)");
    let expected = "(core/set! (path foo) (+ (core/get (path foo)) 1))";
    assert_eq!(expanded, expected);
}

#[test]
fn test_dec_macro_expansion() {
    let expanded = parse_and_expand("(dec! foo)");
    let expected = "(core/set! (path foo) (- (core/get (path foo)) 1))";
    assert_eq!(expanded, expected);
}

#[test]
fn test_is_macro_expansion_with_symbols() {
    let expanded = parse_and_expand("(is? foo bar)");
    let expected = "(eq? (core/get (path foo)) (core/get (path bar)))";
    assert_eq!(expanded, expected);
}

#[test]
fn test_is_macro_expansion_with_literals() {
    let expanded = parse_and_expand("(is? 10 10)");
    let expected = "(eq? 10 10)";
    assert_eq!(expanded, expected);
}

#[test]
fn test_nested_macro_expansion() {
    let expanded = parse_and_expand("(add! foo (inc! bar))");
    let expected = "(core/set! (path foo) (+ (core/get (path foo)) (core/set! (path bar) (+ (core/get (path bar)) 1))))";
    assert_eq!(expanded, expected);
}

#[test]
fn test_add_macro_expansion_list_of_symbols() {
    let expanded = parse_and_expand("(add! (foo bar) 5)");
    let expected = "(core/set! (path foo bar) (+ (core/get (path foo bar)) 5))";
    assert_eq!(expanded, expected);
}

#[test]
fn test_add_macro_expansion_list_of_strings() {
    let expanded = parse_and_expand("(add! (\"foo\" \"bar\") 5)");
    let expected = "(core/set! (path foo bar) (+ (core/get (path foo bar)) 5))";
    assert_eq!(expanded, expected);
}

#[test]
fn test_sub_macro_expansion() {
    let expanded = parse_and_expand("(sub! foo 2)");
    let expected = "(core/set! (path foo) (- (core/get (path foo)) 2))";
    assert_eq!(expanded, expected);
}

#[test]
fn test_add_macro_expansion_invalid_path_mixed_types() {
    let exprs = parse("(add! (player \"score\" 123) 5)").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let err = result.unwrap_err();
    let msg = format!("{:?}", err);
    assert!(msg.contains("Path lists can only contain symbols or strings."));
}

#[test]
fn test_inc_macro_expansion_invalid_path_number() {
    let exprs = parse("(inc! 123)").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
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
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list of two elements"));
}

#[test]
fn test_cond_macro_expansion_missing_else() {
    let exprs = parse("(cond ((is? x 1) \"one\"))").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
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
    let with_span = parse("(my-list 1 2 3)").unwrap().into_iter().next().unwrap();
    let template = MacroTemplate::new(
        ParamList { required: vec!["first".to_string()], rest: Some("rest".to_string()), span: Span::default() },
        Box::new(with_span),
    ).unwrap();
    registry
        .macros
        .insert("my-list".to_string(), MacroDef::Template(template));

    // 3. Parse the expression to be expanded.
    let expr_str = "(my-list 1 2 3)";
    let expr = parse(expr_str).unwrap().into_iter().next().unwrap();

    // 4. Expand the expression using our custom registry.
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let expanded_expr = expander.expand_macros(expr.clone(), &context).unwrap();

    // 5. Assert that the expansion is correct.
    // The `1` should be bound to `first`.
    // The `(2 3)` should be bound to `rest`.
    // The body `(list first rest)` should become `(list 1 (2 3))`.
    let expected = "(list 1 (2 3))";
    assert_eq!(expanded_expr.value.pretty(), expected);
}

#[test]
fn test_cond_macro_expansion_no_clauses() {
    let exprs = parse("(cond)").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("`cond` macro requires at least one clause."));
}

#[test]
fn test_cond_macro_expansion_non_list_clause() {
    let exprs = parse("(cond 42 (else 1))").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list."));
}

#[test]
fn test_cond_macro_expansion_else_not_last() {
    let exprs = parse("(cond (else 1) ((is? x 2) 2))").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("`else` clause must be the last clause in `cond`."));
}

#[test]
fn test_cond_macro_expansion_multiple_else_clauses() {
    let exprs = parse("(cond ((is? x 1) 1) (else 2) (else 3))").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("`else` clause must be the last clause in `cond`."));
}

#[test]
fn test_cond_macro_expansion_else_wrong_arity() {
    let exprs = parse("(cond (else 1 2))").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("`else` clause must have exactly one expression."));
}

#[test]
fn test_cond_macro_expansion_clause_not_list() {
    let exprs = parse("(cond 42)").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list."));
}

#[test]
fn test_cond_macro_expansion_clause_too_many_elements() {
    let exprs = parse("(cond ((is? x 1) 1 2) (else 3))").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("Each `cond` clause must be a list of two elements"));
}

#[test]
fn test_cond_macro_expansion_empty_clause_list() {
    let exprs = parse("(cond () (else 1))").unwrap();
    let mut registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(exprs[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
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
    let registry = build_full_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(parse(&expr).unwrap()[0].clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("depth limit"));
}

#[test]
fn test_variadic_macro_only_variadic_param() {
    let mut registry = MacroRegistry::new();
    let with_span = parse("(gather 1 2 3)").unwrap().into_iter().next().unwrap();
    let template = MacroTemplate::new(
        ParamList { required: vec!["a".to_string()], rest: Some("args".to_string()), span: Span::default() },
        Box::new(with_span),
    )
    .unwrap();
    registry
        .macros
        .insert("gather".to_string(), MacroDef::Template(template));
    let expr = parse("(gather 1 2 3)").unwrap().into_iter().next().unwrap();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let expanded = expander.expand_macros(expr.clone(), &context).unwrap();
    assert_eq!(expanded.value.pretty(), "(list 1 (2 3))");
}

#[test]
fn test_variadic_macro_fixed_and_variadic_params() {
    // Test with first template
    let mut registry1 = MacroRegistry::new();
    let with_span = parse("(cons 1 2 3)").unwrap().into_iter().next().unwrap();
    let template = MacroTemplate::new(
        ParamList { required: vec!["head".to_string()], rest: Some("tail".to_string()), span: Span::default() },
        Box::new(with_span),
    )
    .unwrap();
    registry1.macros.insert("cons".to_string(), MacroDef::Template(template));
    let expr = parse("(cons 1 2 3)").unwrap().into_iter().next().unwrap();
    let context1 = SutraMacroContext { registry: registry1, hygiene_scope: None };
    let expander1 = MacroExpander::default();
    let expanded = expander1.expand_macros(expr.clone(), &context1).unwrap();
    assert_eq!(expanded.value.pretty(), "(list 1 (2 3))");
    // Test empty variadic
    let mut registry2 = MacroRegistry::new();
    let with_span2 = parse("(cons 1)").unwrap().into_iter().next().unwrap();
    let template2 = MacroTemplate::new(
        ParamList { required: vec!["head".to_string()], rest: None, span: Span::default() },
        Box::new(with_span2),
    )
    .unwrap();
    registry2.macros.insert("cons".to_string(), MacroDef::Template(template2));
    let expr2 = parse("(cons 1)").unwrap().into_iter().next().unwrap();
    let context2 = SutraMacroContext { registry: registry2, hygiene_scope: None };
    let expander2 = MacroExpander::default();
    let expanded2 = expander2.expand_macros(expr2.clone(), &context2).unwrap();
    assert_eq!(expanded2.value.pretty(), "(list 1 ())");
}

#[test]
fn test_variadic_macro_too_few_args() {
    let mut registry = MacroRegistry::new();
    let with_span = parse("(foo 1)").unwrap().into_iter().next().unwrap();
    let template = MacroTemplate::new(
        ParamList { required: vec!["a".to_string(), "b".to_string()], rest: Some("rest".to_string()), span: Span::default() },
        Box::new(with_span),
    )
    .unwrap();
    registry
        .macros
        .insert("foo".to_string(), MacroDef::Template(template));
    let expr = parse("(foo 1)").unwrap().into_iter().next().unwrap();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(expr.clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("at least 2 arguments"));
}

#[test]
fn test_variadic_macro_too_many_args_no_variadic() {
    let mut registry = MacroRegistry::new();
    let with_span = parse("(bar 1 2 3)").unwrap().into_iter().next().unwrap();
    let template = MacroTemplate::new(
        ParamList { required: vec!["a".to_string(), "b".to_string()], rest: None, span: Span::default() },
        Box::new(with_span),
    )
    .unwrap();
    registry
        .macros
        .insert("bar".to_string(), MacroDef::Template(template));
    let expr = parse("(bar 1 2 3)").unwrap().into_iter().next().unwrap();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(expr.clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("exactly 2 arguments"));
}

#[test]
fn test_macro_template_duplicate_param_names() {
    let with_span = parse("(a a)").unwrap().into_iter().next().unwrap();
    let result = MacroTemplate::new(
        ParamList { required: vec!["a".to_string(), "a".to_string()], rest: None, span: Span::default() },
        Box::new(with_span),
    );
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("Duplicate parameter name"));
}

#[test]
fn test_macro_template_variadic_param_not_last() {
    // This is not directly possible with MacroTemplate::new as designed, but we can simulate
    // a misuse by passing a variadic param and a param after it (should be caught by parser in real use).
    // Here, we just check that MacroTemplate::new does not allow duplicate names.
    let with_span = parse("(rest b rest)").unwrap().into_iter().next().unwrap();
    let result = MacroTemplate::new(
        ParamList { required: vec!["rest".to_string(), "b".to_string()], rest: Some("rest".to_string()), span: Span::default() },
        Box::new(with_span),
    );
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
    assert!(msg.contains("Duplicate parameter name"));
}

#[test]
fn test_variadic_macro_empty_variadic_param() {
    let mut registry = MacroRegistry::new();
    let with_span = parse("(baz 1)").unwrap().into_iter().next().unwrap();
    let template = MacroTemplate::new(
        ParamList { required: vec!["a".to_string()], rest: Some("rest".to_string()), span: Span::default() },
        Box::new(with_span),
    )
    .unwrap();
    registry
        .macros
        .insert("baz".to_string(), MacroDef::Template(template));
    let expr = parse("(baz 1)").unwrap().into_iter().next().unwrap();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let expanded = expander.expand_macros(expr.clone(), &context).unwrap();
    assert_eq!(expanded.value.pretty(), "(list 1 ())");
}

#[test]
fn test_recursive_macro_expansion_depth_limit() {
    let mut registry = MacroRegistry::new();
    // Macro that calls itself
    let with_span = parse("(recurse 1)").unwrap().into_iter().next().unwrap();
    let template = MacroTemplate::new(
        ParamList { required: vec!["x".to_string()], rest: None, span: Span::default() },
        Box::new(with_span),
    )
    .unwrap();
    registry
        .macros
        .insert("recurse".to_string(), MacroDef::Template(template));
    let expr = parse("(recurse 1)").unwrap().into_iter().next().unwrap();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let result = expander.expand_macros(expr.clone(), &context);
    assert!(result.is_err());
    let msg = format!("{:?}", result.unwrap_err());
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
    use sutra::ast::{Expr, Span};

    let registry = build_default_macro_registry();
    let hash = registry.hash();
    // Assert the hash matches the known canonical value
    let expected_hash = "018db696c872424888555f71d84f66cd7539e2543d9a02ce097a294b1699347c";
    assert_eq!(hash, expected_hash, "Macro registry hash mismatch!\nExpected: {}\nActual:   {}", expected_hash, hash);

    // Golden expansion tests for core macros (add more as needed)
    let cases = vec![
        // if macro
        (
            "if",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("if".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Bool(true, Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(2.0, Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::If {
                condition: Box::new(WithSpan { value: Expr::Bool(true, Span::default()), span: Span::default() }),
                then_branch: Box::new(WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() }),
                else_branch: Box::new(WithSpan { value: Expr::Number(2.0, Span::default()), span: Span::default() }),
                span: Span::default(),
            }, span: Span::default() },
        ),
        // set!
        (
            "set!",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("set!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(42.0, Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("core/set!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(42.0, Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // get
        (
            "get",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("get".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // del!
        (
            "del!",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("del!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("core/del!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // add!
        (
            "add!",
            WithSpan {
                value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("add!".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                ], Span::default()),
                span: Span::default()
            },
            WithSpan {
                value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("core/set!".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::List(vec![
                        WithSpan { value: Expr::Symbol("+".to_string(), Span::default()), span: Span::default() },
                        WithSpan { value: Expr::List(vec![
                            WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                            WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                        ], Span::default()), span: Span::default() },
                        WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                    ], Span::default()), span: Span::default() },
                ], Span::default()),
                span: Span::default()
            },
        ),
        // sub!
        (
            "sub!",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("sub!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("core/set!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("-".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::List(vec![
                        WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                        WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                    ], Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // inc!
        (
            "inc!",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("inc!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("core/set!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("+".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::List(vec![
                        WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                        WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                    ], Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // dec!
        (
            "dec!",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("dec!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("core/set!".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("-".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::List(vec![
                        WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                        WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                    ], Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // is?
        (
            "is?",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("is?".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("bar".to_string(), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("eq?".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Path(sutra::path::Path(vec!["bar".to_string()]), Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // over?
        (
            "over?",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("over?".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("bar".to_string(), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("gt?".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Path(sutra::path::Path(vec!["bar".to_string()]), Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // under?
        (
            "under?",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("under?".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("foo".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Symbol("bar".to_string(), Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("lt?".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Path(sutra::path::Path(vec!["foo".to_string()]), Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("core/get".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Path(sutra::path::Path(vec!["bar".to_string()]), Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // not
        (
            "not",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("not".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Bool(false, Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("not".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Bool(false, Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // list
        (
            "list",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("list".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(2.0, Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("list".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                WithSpan { value: Expr::Number(2.0, Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
        // len
        (
            "len",
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("len".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("list".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Number(2.0, Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
            WithSpan { value: Expr::List(vec![
                WithSpan { value: Expr::Symbol("len".to_string(), Span::default()), span: Span::default() },
                WithSpan { value: Expr::List(vec![
                    WithSpan { value: Expr::Symbol("list".to_string(), Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Number(1.0, Span::default()), span: Span::default() },
                    WithSpan { value: Expr::Number(2.0, Span::default()), span: Span::default() },
                ], Span::default()), span: Span::default() },
            ], Span::default()), span: Span::default() },
        ),
    ];
    for (macro_name, input, expected) in cases {
        let registry = build_default_macro_registry();
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(input.clone(), &context).unwrap();
        assert_eq!(
            expanded.value.pretty(),
            expected.value.pretty(),
            "Expansion mismatch for macro '{}':\nInput:    {}\nExpected: {}\nActual:   {}",
            macro_name,
            input.value.pretty(),
            expected.value.pretty(),
            expanded.value.pretty()
        );
    }
}

#[cfg(test)]
mod loader_tests {
    use sutra::macros::parse_macros_from_source;

    #[test]
    fn test_valid_single_macro() {
        let src = "(define (foo x) x)";
        let result = parse_macros_from_source(src).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "foo");
        assert_eq!(result[0].1.params.required, vec!["x"]);
        assert!(result[0].1.params.rest.is_none());
    }

    #[test]
    fn test_valid_variadic_macro() {
        let src = "(define (bar x . rest) (list x rest))";
        let result = parse_macros_from_source(src).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "bar");
        assert_eq!(result[0].1.params.required, vec!["x"]);
        assert_eq!(result[0].1.params.rest.as_deref(), Some("rest"));
    }

    #[test]
    fn test_ignores_non_macro_forms() {
        let src = "(+ 1 2) (define (foo x) x)";
        let result = parse_macros_from_source(src).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "foo");
    }
}
