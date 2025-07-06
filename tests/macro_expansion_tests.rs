// PROTOCOL NOTE: All tests must be rewritten to use only user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted. See memory-bank and systemPatterns.md for protocol.

//! Macroexpander contract and error tests for Sutra engine.

#[cfg(test)]
mod tests {
    // Helper for macro expansion in tests
    fn must_expand_ok(
        expr: sutra::ast::WithSpan<sutra::ast::Expr>,
        env: &mut sutra::macros::MacroEnv,
    ) -> sutra::ast::WithSpan<sutra::ast::Expr> {
        let result = sutra::macros::expand_macros(expr, env);
        assert!(result.is_ok(), "Macro expansion failed: {:?}", result);
        result.unwrap()
    }

    #[test]
    fn placeholder() {
        // TODO: Implement macroexpander contract tests
    }

    #[test]
    fn macro_registry_can_register_and_get_macro() {
        use sutra::macros::{MacroDef, MacroRegistry};
        let mut registry = MacroRegistry::default();
        // Register a dummy macro
        fn dummy_macro(
            _expr: &sutra::ast::WithSpan<sutra::ast::Expr>,
        ) -> Result<sutra::ast::WithSpan<sutra::ast::Expr>, sutra::error::SutraError> {
            Err(sutra::error::SutraError {
                kind: sutra::error::SutraErrorKind::Macro("dummy".to_string()),
                span: None,
            })
        }
        registry
            .macros
            .insert("inc".to_string(), MacroDef::Fn(dummy_macro));
        assert!(registry.macros.contains_key("inc"));
    }

    #[test]
    fn expand_core_macro_add() {
        use sutra::ast::{Expr, WithSpan};
        use sutra::macros::{MacroEnv, MacroRegistry};
        use sutra::parser::parse;
        // Setup: register a simple add! macro as a template (add! x y) => (+ x y)
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: sutra::ast::Span { start: 0, end: 0 },
        };
        let body = Box::new(WithSpan {
            value: Expr::List(
                vec![
                    WithSpan {
                        value: Expr::Symbol("+".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol("x".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol("y".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                ],
                sutra::ast::Span { start: 0, end: 0 },
            ),
            span: sutra::ast::Span { start: 0, end: 0 },
        });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert(
            "add!".to_string(),
            sutra::macros::MacroDef::Template(template),
        );
        let mut env = MacroEnv {
            user_macros: registry.macros,
            core_macros: MacroRegistry::default().macros,
            trace: Vec::new(),
        };
        let input = "(add! foo 1)";
        let ast = parse(input).unwrap().remove(0);
        let expanded = must_expand_ok(ast.clone(), &mut env);
        match &expanded.value {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3);
                match &items[0].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "+"),
                    _ => panic!("First element should be symbol '+'."),
                }
            }
            _ => panic!("Expanded form should be a list"),
        }
    }

    #[test]
    fn expand_macro_with_list_path() {
        // This test assumes add! macro supports a list path as first argument
        use sutra::ast::{Expr, WithSpan};
        use sutra::macros::{MacroEnv, MacroRegistry};
        use sutra::parser::parse;
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList {
            required: vec!["path".to_string(), "val".to_string()],
            rest: None,
            span: sutra::ast::Span { start: 0, end: 0 },
        };
        let body = Box::new(WithSpan {
            value: Expr::List(
                vec![
                    WithSpan {
                        value: Expr::Symbol("+".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol(
                            "path".to_string(),
                            sutra::ast::Span { start: 0, end: 0 },
                        ),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol(
                            "val".to_string(),
                            sutra::ast::Span { start: 0, end: 0 },
                        ),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                ],
                sutra::ast::Span { start: 0, end: 0 },
            ),
            span: sutra::ast::Span { start: 0, end: 0 },
        });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert(
            "add!".to_string(),
            sutra::macros::MacroDef::Template(template),
        );
        let mut env = MacroEnv {
            user_macros: registry.macros,
            core_macros: MacroRegistry::default().macros,
            trace: Vec::new(),
        };
        let input = "(add! (foo bar) 2)";
        let ast = parse(input).unwrap().remove(0);
        let expanded = must_expand_ok(ast.clone(), &mut env);
        match &expanded.value {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3);
                match &items[1].value {
                    Expr::List(path_items, _) => {
                        assert_eq!(path_items.len(), 2);
                    }
                    _ => panic!("Second element should be a list path"),
                }
            }
            _ => panic!("Expanded form should be a list"),
        }
    }

    #[test]
    fn expand_macro_with_string_path() {
        // This test assumes add! macro supports a string path as first argument
        use sutra::ast::{Expr, WithSpan};
        use sutra::macros::{MacroEnv, MacroRegistry};
        use sutra::parser::parse;
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList {
            required: vec!["path".to_string(), "val".to_string()],
            rest: None,
            span: sutra::ast::Span { start: 0, end: 0 },
        };
        let body = Box::new(WithSpan {
            value: Expr::List(
                vec![
                    WithSpan {
                        value: Expr::Symbol("+".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol(
                            "path".to_string(),
                            sutra::ast::Span { start: 0, end: 0 },
                        ),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol(
                            "val".to_string(),
                            sutra::ast::Span { start: 0, end: 0 },
                        ),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                ],
                sutra::ast::Span { start: 0, end: 0 },
            ),
            span: sutra::ast::Span { start: 0, end: 0 },
        });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert(
            "add!".to_string(),
            sutra::macros::MacroDef::Template(template),
        );
        let mut env = MacroEnv {
            user_macros: registry.macros,
            core_macros: MacroRegistry::default().macros,
            trace: Vec::new(),
        };
        let input = "(add! (\"foo\" \"bar\") 2)";
        let ast = parse(input).unwrap().remove(0);
        let expanded = must_expand_ok(ast.clone(), &mut env);
        match &expanded.value {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3);
                match &items[1].value {
                    Expr::List(path_items, _) => {
                        assert_eq!(path_items.len(), 2);
                        match &path_items[0].value {
                            Expr::String(s, _) => assert_eq!(s, "foo"),
                            _ => panic!("First path element should be string 'foo'"),
                        }
                    }
                    _ => panic!("Second element should be a list path"),
                }
            }
            _ => panic!("Expanded form should be a list"),
        }
    }

    #[test]
    fn expand_macro_with_duplicate_param_names_should_error() {
        use sutra::ast::{Expr, ParamList, Span, WithSpan};
        // Attempt to create a macro template with duplicate parameter names
        let params = ParamList {
            required: vec!["x".to_string(), "x".to_string()],
            rest: None,
            span: Span { start: 0, end: 0 },
        };
        let body = Box::new(WithSpan {
            value: Expr::Symbol("dummy".to_string(), Span { start: 0, end: 0 }),
            span: Span { start: 0, end: 0 },
        });
        let template = sutra::macros::MacroTemplate::new(params, body);
        assert!(
            template.is_err(),
            "MacroTemplate::new should error on duplicate parameter names"
        );
    }

    #[test]
    fn expand_golden_input_to_golden_output() {
        // This is a placeholder for a golden test: known macro input -> known canonical output
        // For now, just check that expansion is deterministic and matches expected output
        use sutra::ast::{Expr, WithSpan};
        use sutra::macros::{MacroEnv, MacroRegistry};
        use sutra::parser::parse;
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: sutra::ast::Span { start: 0, end: 0 },
        };
        let body = Box::new(WithSpan {
            value: Expr::List(
                vec![
                    WithSpan {
                        value: Expr::Symbol("+".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol("x".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol("y".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                ],
                sutra::ast::Span { start: 0, end: 0 },
            ),
            span: sutra::ast::Span { start: 0, end: 0 },
        });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert(
            "add!".to_string(),
            sutra::macros::MacroDef::Template(template),
        );
        let mut env = MacroEnv {
            user_macros: registry.macros,
            core_macros: MacroRegistry::default().macros,
            trace: Vec::new(),
        };
        let input = "(add! foo 1)";
        let ast = parse(input).unwrap().remove(0);
        let expanded = must_expand_ok(ast.clone(), &mut env);
        // Expected output: (+ foo 1)
        match &expanded.value {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3);
                match &items[0].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "+"),
                    _ => panic!("First element should be symbol '+'."),
                }
                match &items[1].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "foo"),
                    _ => panic!("Second element should be symbol 'foo'."),
                }
                match &items[2].value {
                    Expr::Number(n, _) => assert_eq!(*n, 1.0),
                    _ => panic!("Third element should be number 1."),
                }
            }
            _ => panic!("Expanded form should be a list"),
        }
    }

    #[test]
    fn expand_macro_with_too_few_arguments_should_error() {
        use sutra::macros::{MacroEnv, MacroRegistry};
        use sutra::parser::parse;
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: sutra::ast::Span { start: 0, end: 0 },
        };
        let body = Box::new(sutra::ast::WithSpan {
            value: sutra::ast::Expr::Symbol(
                "dummy".to_string(),
                sutra::ast::Span { start: 0, end: 0 },
            ),
            span: sutra::ast::Span { start: 0, end: 0 },
        });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert(
            "add!".to_string(),
            sutra::macros::MacroDef::Template(template),
        );
        let mut env = MacroEnv {
            user_macros: registry.macros,
            core_macros: MacroRegistry::default().macros,
            trace: Vec::new(),
        };
        let input = "(add! foo)"; // Only one argument
        let ast = parse(input).unwrap().remove(0);
        let result = sutra::macros::expand_macros(ast.clone(), &mut env);
        assert!(
            result.is_err(),
            "Macro expansion should fail due to too few arguments"
        );
    }

    #[test]
    fn expand_macro_with_too_many_arguments_should_error() {
        use sutra::macros::{MacroEnv, MacroRegistry};
        use sutra::parser::parse;
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: sutra::ast::Span { start: 0, end: 0 },
        };
        let body = Box::new(sutra::ast::WithSpan {
            value: sutra::ast::Expr::Symbol(
                "dummy".to_string(),
                sutra::ast::Span { start: 0, end: 0 },
            ),
            span: sutra::ast::Span { start: 0, end: 0 },
        });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert(
            "add!".to_string(),
            sutra::macros::MacroDef::Template(template),
        );
        let mut env = MacroEnv {
            user_macros: registry.macros,
            core_macros: MacroRegistry::default().macros,
            trace: Vec::new(),
        };
        let input = "(add! foo 1 2)"; // Three arguments
        let ast = parse(input).unwrap().remove(0);
        let result = sutra::macros::expand_macros(ast.clone(), &mut env);
        assert!(
            result.is_err(),
            "Macro expansion should fail due to too many arguments"
        );
    }

    #[test]
    fn expand_macro_with_recursion_depth_limit_should_error() {
        use sutra::ast::{Expr, WithSpan};
        use sutra::macros::{MacroEnv, MacroRegistry};
        use sutra::parser::parse;
        // Register a macro that expands to itself
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList {
            required: vec!["x".to_string()],
            rest: None,
            span: sutra::ast::Span { start: 0, end: 0 },
        };
        let body = Box::new(WithSpan {
            value: Expr::List(
                vec![
                    WithSpan {
                        value: Expr::Symbol(
                            "recur!".to_string(),
                            sutra::ast::Span { start: 0, end: 0 },
                        ),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                    WithSpan {
                        value: Expr::Symbol("x".to_string(), sutra::ast::Span { start: 0, end: 0 }),
                        span: sutra::ast::Span { start: 0, end: 0 },
                    },
                ],
                sutra::ast::Span { start: 0, end: 0 },
            ),
            span: sutra::ast::Span { start: 0, end: 0 },
        });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert(
            "recur!".to_string(),
            sutra::macros::MacroDef::Template(template),
        );
        let mut env = MacroEnv {
            user_macros: registry.macros,
            core_macros: MacroRegistry::default().macros,
            trace: Vec::new(),
        };
        let input = "(recur! foo)";
        let ast = parse(input).unwrap().remove(0);
        let result = sutra::macros::expand_macros(ast.clone(), &mut env);
        assert!(
            result.is_err(),
            "Macro expansion should fail due to recursion depth limit"
        );
    }
}
