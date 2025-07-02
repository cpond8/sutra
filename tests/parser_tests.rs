// tests/parser_tests.rs

use sutra::ast::Expr;
use sutra::parser::parse;

// A helper to get the inner expressions from a parsed program.
// Our `parse` function wraps the program in a root `Expr::List`.
fn get_inner_exprs(program: Expr) -> Vec<Expr> {
    if let Expr::List(items, _) = program {
        items
    } else {
        panic!("Expected a root list expression");
    }
}

// ---
// Migrated and Updated Tests
// ---

#[test]
fn test_parse_simple_s_expression() {
    let program = parse("(+ 1 2)").unwrap();
    let items = get_inner_exprs(program);
    assert_eq!(items.len(), 1);

    if let Expr::List(inner_items, _) = &items[0] {
        assert_eq!(inner_items.len(), 3);
        assert!(matches!(&inner_items[0], Expr::Symbol(s, _) if s == "+"));
        assert!(matches!(inner_items[1], Expr::Number(n, _) if n == 1.0));
        assert!(matches!(inner_items[2], Expr::Number(n, _) if n == 2.0));
    } else {
        panic!("Expected a list inside the program");
    }
}

#[test]
fn test_parse_nested_s_expression() {
    let program = parse("(+ 1 (* 2 3))").unwrap();
    let items = get_inner_exprs(program);
    assert_eq!(items[0].pretty(), "(+ 1 (* 2 3))");
}

#[test]
fn test_parse_string_literal() {
    let program = parse(r#"(set! (list "name") "sutra")"#).unwrap();
    let items = get_inner_exprs(program);
    assert_eq!(items[0].pretty(), r#"(set! (list "name") "sutra")"#);
}

// ---
// New Tests for Unified Parser
// ---

#[test]
fn test_parse_brace_block_equivalent() {
    let s_expr_ast = parse("(+ 1 2)").unwrap();
    let brace_block_ast = parse("{+ 1 2}").unwrap();
    assert_eq!(s_expr_ast, brace_block_ast);
}

#[test]
fn test_parse_with_comments() {
    let source = "; this is a comment\n (+ 1 2) ; another comment";
    let program = parse(source).unwrap();
    let items = get_inner_exprs(program);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].pretty(), "(+ 1 2)");
}

#[test]
fn test_parse_string_with_escapes() {
    let source = r#"("hello \"world\" \n\t")"#;
    let program = parse(source).unwrap();
    let items = get_inner_exprs(program);
    assert_eq!(items.len(), 1);

    if let Expr::List(inner, _) = &items[0] {
        if let Expr::String(s, _) = &inner[0] {
            assert_eq!(s, "hello \"world\"\n\t");
        } else {
            panic!("Expected a string literal");
        }
    } else {
        panic!("Expected a list");
    }
}

#[test]
fn test_unclosed_list_fails() {
    let source = "(+ 1";
    let result = parse(source);
    assert!(result.is_err());
    // Optionally, assert on the error type/message
    let err = result.unwrap_err();
    assert!(matches!(err.kind, sutra::error::SutraErrorKind::Parse(_)));
}

#[test]
fn test_mismatched_brackets_fail() {
    let source = "(+ 1}";
    let result = parse(source);
    assert!(result.is_err());
}
