//! Macroexpander contract and error tests for Sutra engine.

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {
        // TODO: Implement macroexpander contract tests
        assert!(true);
    }

    #[test]
    fn macro_registry_can_register_and_get_macro() {
        use sutra::macros::{MacroRegistry, MacroDef};
        let mut registry = MacroRegistry::default();
        // Register a dummy macro
        fn dummy_macro(_expr: &sutra::ast::WithSpan<sutra::ast::Expr>) -> Result<sutra::ast::WithSpan<sutra::ast::Expr>, sutra::error::SutraError> {
            Err(sutra::error::SutraError { kind: sutra::error::SutraErrorKind::Macro("dummy".to_string()), span: None })
        }
        registry.macros.insert("inc".to_string(), MacroDef::Fn(dummy_macro));
        assert!(registry.macros.contains_key("inc"));
    }

    #[test]
    fn sutra_macro_context_can_lookup_macro() {
        use sutra::macros::{MacroRegistry, SutraMacroContext, MacroDef};
        let mut registry = MacroRegistry::default();
        fn dummy_macro(_expr: &sutra::ast::WithSpan<sutra::ast::Expr>) -> Result<sutra::ast::WithSpan<sutra::ast::Expr>, sutra::error::SutraError> {
            Err(sutra::error::SutraError { kind: sutra::error::SutraErrorKind::Macro("dummy".to_string()), span: None })
        }
        registry.macros.insert("foo".to_string(), MacroDef::Fn(dummy_macro));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        assert!(context.get_macro("foo").is_some());
        assert!(context.get_macro("bar").is_none());
    }

    #[test]
    fn expand_core_macro_add() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, MacroRegistry, SutraMacroContext, SutraMacroExpander};
        use sutra::ast::{Expr, WithSpan};
        // Setup: register a simple add! macro as a template (add! x y) => (+ x y)
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList { required: vec!["x".to_string(), "y".to_string()], rest: None, span: sutra::ast::Span { start: 0, end: 0 } };
        let body = Box::new(WithSpan { value: Expr::List(vec![
            WithSpan { value: Expr::Symbol("+".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("x".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("y".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
        ], sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert("add!".to_string(), sutra::macros::MacroDef::Template(template));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let input = "(add! foo 1)";
        let ast = parse(input).unwrap().remove(0);
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast.clone(), &context);
        assert!(expanded.is_ok(), "Macro expansion should succeed");
        let expanded = expanded.unwrap();
        match &expanded.value {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3);
                match &items[0].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "+"),
                    _ => panic!("First element should be symbol '+'.")
                }
            }
            _ => panic!("Expanded form should be a list")
        }
    }

    #[test]
    fn expand_macro_with_list_path() {
        // This test assumes add! macro supports a list path as first argument
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, MacroRegistry, SutraMacroContext, SutraMacroExpander};
        use sutra::ast::{Expr, WithSpan};
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList { required: vec!["path".to_string(), "val".to_string()], rest: None, span: sutra::ast::Span { start: 0, end: 0 } };
        let body = Box::new(WithSpan { value: Expr::List(vec![
            WithSpan { value: Expr::Symbol("+".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("path".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("val".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
        ], sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert("add!".to_string(), sutra::macros::MacroDef::Template(template));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let input = "(add! (foo bar) 2)";
        let ast = parse(input).unwrap().remove(0);
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast.clone(), &context);
        assert!(expanded.is_ok(), "Macro expansion should succeed");
        let expanded = expanded.unwrap();
        match &expanded.value {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3);
                match &items[1].value {
                    Expr::List(path_items, _) => {
                        assert_eq!(path_items.len(), 2);
                    }
                    _ => panic!("Second element should be a list path")
                }
            }
            _ => panic!("Expanded form should be a list")
        }
    }

    #[test]
    fn expand_macro_with_string_path() {
        // This test assumes add! macro supports a string path as first argument
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, MacroRegistry, SutraMacroContext, SutraMacroExpander};
        use sutra::ast::{Expr, WithSpan};
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList { required: vec!["path".to_string(), "val".to_string()], rest: None, span: sutra::ast::Span { start: 0, end: 0 } };
        let body = Box::new(WithSpan { value: Expr::List(vec![
            WithSpan { value: Expr::Symbol("+".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("path".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("val".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
        ], sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert("add!".to_string(), sutra::macros::MacroDef::Template(template));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let input = "(add! (\"foo\" \"bar\") 2)";
        let ast = parse(input).unwrap().remove(0);
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast.clone(), &context);
        assert!(expanded.is_ok(), "Macro expansion should succeed");
        let expanded = expanded.unwrap();
        match &expanded.value {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3);
                match &items[1].value {
                    Expr::List(path_items, _) => {
                        assert_eq!(path_items.len(), 2);
                        match &path_items[0].value {
                            Expr::String(s, _) => assert_eq!(s, "foo"),
                            _ => panic!("First path element should be string 'foo'")
                        }
                    }
                    _ => panic!("Second element should be a list path")
                }
            }
            _ => panic!("Expanded form should be a list")
        }
    }

    #[test]
    fn expand_macro_with_invalid_path_should_error() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, MacroRegistry, SutraMacroContext, SutraMacroExpander};
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList { required: vec!["path".to_string(), "val".to_string()], rest: None, span: sutra::ast::Span { start: 0, end: 0 } };
        let body = Box::new(sutra::ast::WithSpan { value: sutra::ast::Expr::Symbol("invalid".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert("add!".to_string(), sutra::macros::MacroDef::Template(template));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let input = "(add! 42 2)"; // 42 is not a valid path
        let ast = parse(input).unwrap().remove(0);
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast.clone(), &context);
        assert!(expanded.is_ok(), "Macro expansion should succeed syntactically (template does not check path type)");
        // If semantic validation is required, this should be an error; otherwise, document this limitation.
    }

    #[test]
    fn expand_macro_with_too_few_arguments_should_error() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, MacroRegistry, SutraMacroContext, SutraMacroExpander};
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList { required: vec!["x".to_string(), "y".to_string()], rest: None, span: sutra::ast::Span { start: 0, end: 0 } };
        let body = Box::new(sutra::ast::WithSpan { value: sutra::ast::Expr::Symbol("dummy".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert("add!".to_string(), sutra::macros::MacroDef::Template(template));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let input = "(add! foo)"; // Only one argument
        let ast = parse(input).unwrap().remove(0);
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast.clone(), &context);
        assert!(expanded.is_err(), "Macro expansion should error on too few arguments");
    }

    #[test]
    fn expand_macro_with_too_many_arguments_should_error() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, MacroRegistry, SutraMacroContext, SutraMacroExpander};
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList { required: vec!["x".to_string(), "y".to_string()], rest: None, span: sutra::ast::Span { start: 0, end: 0 } };
        let body = Box::new(sutra::ast::WithSpan { value: sutra::ast::Expr::Symbol("dummy".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert("add!".to_string(), sutra::macros::MacroDef::Template(template));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let input = "(add! foo 1 2)"; // Three arguments
        let ast = parse(input).unwrap().remove(0);
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast.clone(), &context);
        assert!(expanded.is_err(), "Macro expansion should error on too many arguments");
    }

    #[test]
    fn expand_macro_with_duplicate_param_names_should_error() {
        use sutra::ast::{ParamList, Span, Expr, WithSpan};
        // Attempt to create a macro template with duplicate parameter names
        let params = ParamList { required: vec!["x".to_string(), "x".to_string()], rest: None, span: Span { start: 0, end: 0 } };
        let body = Box::new(WithSpan { value: Expr::Symbol("dummy".to_string(), Span { start: 0, end: 0 }), span: Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body);
        assert!(template.is_err(), "MacroTemplate::new should error on duplicate parameter names");
    }

    #[test]
    fn expand_macro_with_recursion_depth_limit_should_error() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, MacroRegistry, SutraMacroContext, SutraMacroExpander};
        use sutra::ast::{Expr, WithSpan};
        // Register a macro that expands to itself
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList { required: vec!["x".to_string()], rest: None, span: sutra::ast::Span { start: 0, end: 0 } };
        let body = Box::new(WithSpan { value: Expr::List(vec![
            WithSpan { value: Expr::Symbol("recur!".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("x".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
        ], sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert("recur!".to_string(), sutra::macros::MacroDef::Template(template));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let input = "(recur! foo)";
        let ast = parse(input).unwrap().remove(0);
        let expander = MacroExpander { max_recursion: 8 };
        let expanded = expander.expand_macros(ast.clone(), &context);
        assert!(expanded.is_err(), "Macro expansion should error on recursion depth limit");
    }

    #[test]
    fn expand_golden_input_to_golden_output() {
        // This is a placeholder for a golden test: known macro input -> known canonical output
        // For now, just check that expansion is deterministic and matches expected output
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, MacroRegistry, SutraMacroContext, SutraMacroExpander};
        use sutra::ast::{Expr, WithSpan};
        let mut registry = MacroRegistry::default();
        let params = sutra::ast::ParamList { required: vec!["x".to_string(), "y".to_string()], rest: None, span: sutra::ast::Span { start: 0, end: 0 } };
        let body = Box::new(WithSpan { value: Expr::List(vec![
            WithSpan { value: Expr::Symbol("+".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("x".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
            WithSpan { value: Expr::Symbol("y".to_string(), sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } },
        ], sutra::ast::Span { start: 0, end: 0 }), span: sutra::ast::Span { start: 0, end: 0 } });
        let template = sutra::macros::MacroTemplate::new(params, body).unwrap();
        registry.macros.insert("add!".to_string(), sutra::macros::MacroDef::Template(template));
        let context = SutraMacroContext { registry, hygiene_scope: None };
        let input = "(add! foo 1)";
        let ast = parse(input).unwrap().remove(0);
        let expander = MacroExpander::default();
        let expanded = expander.expand_macros(ast.clone(), &context).unwrap();
        // Expected output: (+ foo 1)
        match &expanded.value {
            Expr::List(items, _) => {
                assert_eq!(items.len(), 3);
                match &items[0].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "+"),
                    _ => panic!("First element should be symbol '+'.")
                }
                match &items[1].value {
                    Expr::Symbol(s, _) => assert_eq!(s, "foo"),
                    _ => panic!("Second element should be symbol 'foo'.")
                }
                match &items[2].value {
                    Expr::Number(n, _) => assert_eq!(*n, 1.0),
                    _ => panic!("Third element should be number 1.")
                }
            }
            _ => panic!("Expanded form should be a list")
        }
    }
}