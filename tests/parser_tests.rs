// tests/parser_tests.rs

use sutra::ast::Expr;
use sutra::parser::parse;

// A helper to get the inner expressions from a parsed program.
fn get_inner_exprs(program: Result<Vec<Expr>, sutra::error::SutraError>) -> Vec<Expr> {
    program.unwrap()
}

// ---
// Migrated and Updated Tests
// ---

#[test]
fn test_parse_simple_s_expression() {
    let items = get_inner_exprs(parse("(+ 1 2)"));
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
    let items = get_inner_exprs(parse("(+ 1 (* 2 3))"));
    assert_eq!(items[0].pretty(), "(+ 1 (* 2 3))");
}

#[test]
fn test_parse_string_literal() {
    let items = get_inner_exprs(parse(r#"(set! (list "name") "sutra")"#));
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
    let items = get_inner_exprs(parse(source));
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].pretty(), "(+ 1 2)");
}

#[test]
fn test_parse_string_with_escapes() {
    let source = r#"("hello \"world\"\n\t")"#;
    let items = get_inner_exprs(parse(source));
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

#[test]
fn test_dotted_macro_param_list_ast() {
    use sutra::parser::parse;
    use sutra::ast::Expr;
    let src = "(define (bar x . rest) 42)";
    let ast = parse(src).expect("parse should succeed");
    // The first expr should be the define form
    if let Expr::List(items, _) = &ast[0] {
        if let Expr::List(param_items, _) = &items[1] {
            println!("[TEST DEBUG] param_items: {:?}", param_items);
            // Optionally, assert the expected shape
            // e.g., [Symbol("bar"), Symbol("x"), Symbol("."), Symbol("rest")]
        } else {
            panic!("Expected parameter list to be a list");
        }
    } else {
        panic!("Expected top-level expr to be a list");
    }
}

#[test]
fn test_round_trip_parse_pretty_parse() {
    let cases = vec![
        "(+ 1 2)",
        "(+ 1 (* 2 3))",
        "{+ 1 2}",
        "(define (foo x . rest) {+ x (sum rest)})",
        "{define (foo x . rest) (+ x (sum rest))}",
    ];
    for src in cases {
        let ast1 = parse(src).expect("parse should succeed");
        let pretty = ast1.iter().map(|e| e.pretty()).collect::<Vec<_>>().join("\n");
        let ast2 = parse(&pretty).expect("re-parse should succeed");
        assert_eq!(ast1, ast2, "Round-trip failed for: {}", src);
    }
}

#[test]
fn test_dotted_list_edge_cases() {
    let valid = vec![
        ("(a b . c)", vec!["a", "b", ".", "c"]),
    ];
    for (src, expected_syms) in valid {
        let ast = parse(src).expect("parse should succeed");
        let param_items = match &ast[0] {
            Expr::List(items, _) => items,
            _ => panic!("Expected list AST"),
        };
        let syms: Vec<_> = param_items.iter().map(|e| match e {
            Expr::Symbol(s, _) => s.as_str(),
            _ => "?",
        }).collect();
        assert_eq!(syms, expected_syms, "Dotted list AST shape for: {}", src);
    }
    let invalid = vec![
        "(. a b)",
        "(a .)",
        "(a b . c d)",
        "(a b . . c)",
        "( . )",
    ];
    for src in invalid {
        let result = parse(src);
        assert!(result.is_err(), "Should error for malformed dotted list: {}", src);
        let err = result.unwrap_err();
        assert!(matches!(err.kind, sutra::error::SutraErrorKind::Parse(_)), "Error kind for: {}", src);
        assert!(err.span.is_some(), "Error should have span for: {}", src);
    }
}

#[test]
fn test_golden_macro_param_list() {
    let src = "(define (foo x . rest) {+ x (sum rest)})";
    let ast = parse(src).expect("parse should succeed");
    let param_items = match &ast[0] {
        Expr::List(items, _) => match &items[1] {
            Expr::List(params, _) => params,
            _ => panic!("Expected param list as second item"),
        },
        _ => panic!("Expected top-level list"),
    };
    let syms: Vec<_> = param_items.iter().map(|e| match e {
        Expr::Symbol(s, _) => s.as_str(),
        _ => "?",
    }).collect();
    assert_eq!(syms, vec!["foo", "x", ".", "rest"]);
}

#[test]
fn test_parse_malformed_input() {
    let cases = vec![
        "(+ 1 2",
        "(a b . . c)",
        "(. a b)",
        "(a b .)",
        "( . )",
    ];
    for src in cases {
        let result = parse(src);
        assert!(result.is_err(), "Should error for malformed input: {}", src);
        let err = result.unwrap_err();
        assert!(err.span.is_some(), "Error should have span for: {}", src);
    }
}

#[test]
fn test_parse_mixed_syntax_and_comments() {
    let src1 = "{+ 1 ; comment\n 2}";
    let src2 = "(+ 1 ; comment\n 2)";
    for src in [src1, src2] {
        let ast = parse(src).expect("parse should succeed");
        assert_eq!(ast[0].pretty(), "(+ 1 2)");
    }
}

#[test]
fn test_string_literal_escapes() {
    let cases = vec![
        (r#""hello \n world""#, "hello \n world"),
        (r#""tab\tend""#, "tab\tend"),
        (r#""quote: \"""#, "quote: \""),
        (r#""backslash: \\""#, "backslash: \\"),
    ];
    for (src, expected) in cases {
        let ast = parse(&format!("(+ {})", src)).expect("parse should succeed");
        let inner = match &ast[0] {
            Expr::List(items, _) => &items[1],
            _ => panic!("Expected list AST"),
        };
        if let Expr::String(s, _) = inner {
            assert_eq!(s, expected, "String literal escape for: {}", src);
        } else {
            panic!("Expected string AST");
        }
    }
}

#[test]
fn test_span_coverage() {
    let src = "(+ 1 (* 2 3))";
    let ast = parse(src).expect("parse should succeed");
    // Top-level list should cover the whole input
    let span = ast[0].span();
    assert_eq!(span.start, 0);
    assert_eq!(span.end, src.len());
}
