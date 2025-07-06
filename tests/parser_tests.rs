//! Parser contract and error tests for Sutra engine.

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {
        // TODO: Implement parser contract tests
    }

    #[test]
    fn parse_valid_sexpr() {
        // Import the public parser API and AST types
        use sutra::ast::Expr;
        use sutra::parser::parse;

        let input = "(add! foo 1)";
        let result = parse(input);
        assert!(result.is_ok(), "Parser should succeed on valid S-expr");
        let ast = result.unwrap();
        assert_eq!(ast.len(), 1, "Should parse one top-level form");
        let list = &ast[0].value;
        match list {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3, "List should have three elements");
                match &items[0].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "add!"),
                    _ => panic!("First element should be a symbol 'add!'"),
                }
                match &items[1].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "foo"),
                    _ => panic!("Second element should be a symbol 'foo'"),
                }
                match &items[2].value {
                    Expr::Number(n, _) => assert_eq!(*n, 1.0),
                    _ => panic!("Third element should be a number 1"),
                }
            }
            _ => panic!("Top-level form should be a list"),
        }
    }

    #[test]
    fn parse_valid_brace_block() {
        use sutra::ast::Expr;
        use sutra::parser::parse;
        let input = "{add! foo 1}";
        let result = parse(input);
        assert!(result.is_ok(), "Parser should succeed on valid brace block");
        let ast = result.unwrap();
        assert_eq!(ast.len(), 1, "Should parse one top-level form");
        let list = &ast[0].value;
        match list {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3, "List should have three elements");
                match &items[0].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "add!"),
                    _ => panic!("First element should be a symbol 'add!'"),
                }
                match &items[1].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "foo"),
                    _ => panic!("Second element should be a symbol 'foo'"),
                }
                match &items[2].value {
                    Expr::Number(n, _) => assert_eq!(*n, 1.0),
                    _ => panic!("Third element should be a number 1"),
                }
            }
            _ => panic!("Top-level form should be a list"),
        }
    }

    #[test]
    fn parse_with_comments_and_whitespace() {
        use sutra::ast::Expr;
        use sutra::parser::parse;
        let input = "\n  ; this is a comment\n  (add!  ; inline comment\n    foo\n    1\n  )\n  ; trailing comment\n";
        let result = parse(input);
        assert!(
            result.is_ok(),
            "Parser should succeed with comments and whitespace"
        );
        let ast = result.unwrap();
        assert_eq!(ast.len(), 1, "Should parse one top-level form");
        let list = &ast[0].value;
        match list {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3, "List should have three elements");
                match &items[0].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "add!"),
                    _ => panic!("First element should be a symbol 'add!'"),
                }
                match &items[1].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "foo"),
                    _ => panic!("Second element should be a symbol 'foo'"),
                }
                match &items[2].value {
                    Expr::Number(n, _) => assert_eq!(*n, 1.0),
                    _ => panic!("Third element should be a number 1"),
                }
            }
            _ => panic!("Top-level form should be a list"),
        }
    }

    #[test]
    fn parse_string_literals_with_escapes() {
        use sutra::ast::Expr;
        use sutra::parser::parse;
        let input = "(print \"hello\\nworld\")";
        let result = parse(input);
        assert!(
            result.is_ok(),
            "Parser should succeed on string literal with escapes"
        );
        let ast = result.unwrap();
        assert_eq!(ast.len(), 1, "Should parse one top-level form");
        let list = &ast[0].value;
        match list {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 2, "List should have two elements");
                match &items[0].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "print"),
                    _ => panic!("First element should be a symbol 'print'"),
                }
                match &items[1].value {
                    Expr::String(s, _) => assert_eq!(s, "hello\nworld"),
                    _ => panic!("Second element should be a string literal with escape"),
                }
            }
            _ => panic!("Top-level form should be a list"),
        }
    }

    #[test]
    fn parse_empty_input_should_return_empty_vec() {
        use sutra::parser::parse;
        // Per the grammar, empty input is a valid (empty) program and should return Ok(vec![])
        let input = "";
        let result = parse(input);
        assert!(result.is_ok(), "Parser should succeed on empty input");
        let ast = result.unwrap();
        assert_eq!(
            ast.len(),
            0,
            "Empty input should yield an empty vector of forms"
        );
    }

    #[test]
    fn parse_malformed_input_should_error() {
        use sutra::parser::parse;
        let unclosed = "(add! foo 1";
        let mismatched = "(add! {foo 1)";
        let result1 = parse(unclosed);
        assert!(result1.is_err(), "Parser should error on unclosed list");
        let result2 = parse(mismatched);
        assert!(
            result2.is_err(),
            "Parser should error on mismatched brackets"
        );
    }

    #[test]
    fn parse_deeply_nested_forms() {
        use sutra::ast::Expr;
        use sutra::parser::parse;
        // Per the language spec, the parser's recursion depth max is 100.
        // Exceeding this may cause stack overflow or parser error.
        let depth_limit = 100;
        let mut input = String::new();
        for _ in 0..depth_limit {
            input.push('(');
        }
        input.push_str("foo");
        for _ in 0..depth_limit {
            input.push(')');
        }
        let result = parse(&input);
        assert!(
            result.is_ok(),
            "Parser should handle up to 100 levels of nesting"
        );
        let ast = result.unwrap();
        assert_eq!(ast.len(), 1, "Should parse one top-level form");
        // Optionally, check the nesting depth by traversing the AST
        let mut node = &ast[0].value;
        let mut depth = 0;
        while let Expr::List(items, _) = node {
            if items.len() == 1 {
                node = &items[0].value;
                depth += 1;
            } else {
                break;
            }
        }
        assert_eq!(
            depth, depth_limit,
            "Nesting depth should be {}",
            depth_limit
        );
        match node {
            Expr::Symbol(s, _) => assert_eq!(s, "foo"),
            _ => panic!("Innermost node should be symbol 'foo'"),
        }
    }

    #[test]
    fn parse_round_trip_pretty() {
        use sutra::parser::parse;
        let input = "(add! foo 1)";
        let result = parse(input);
        assert!(result.is_ok(), "Parser should succeed on valid input");
        let ast = result.unwrap();
        assert_eq!(ast.len(), 1, "Should parse one top-level form");
        let pretty = ast[0].value.pretty();
        let result2 = parse(&pretty);
        assert!(
            result2.is_ok(),
            "Parser should succeed on pretty-printed output"
        );
        let ast2 = result2.unwrap();
        assert_eq!(
            ast2.len(),
            1,
            "Pretty-printed parse should yield one top-level form"
        );
        assert_eq!(
            ast[0].value, ast2[0].value,
            "ASTs should be equivalent after round-trip"
        );
    }
}
