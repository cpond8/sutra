//! Comprehensive Unit Tests for Sutra Engine
//!
//! These tests focus on internal functions, error message formatting,
//! and edge cases that complement the script-based integration tests.
//! Specifically targets the kinds of issues that caused the variadic macro bug.

use std::sync::Arc;
use sutra::ast::value::Value;
use sutra::ast::{AstNode, Expr, ParamList, Span, WithSpan};
use sutra::macros::check_arity;
use sutra::syntax::error::{eval_arity_error, eval_general_error, eval_type_error};

#[cfg(test)]
mod arity_tests {
    use super::*;

    #[test]
    fn test_enhanced_arity_error_formatting() {
        let span = Span::default();
        let args = vec![WithSpan {
            value: Arc::new(Expr::Number(1.0, span.clone())),
            span: span.clone(),
        }];

        let error = eval_arity_error(Some(span.clone()), &args, "+", "at least 2");

        // Check that the enhanced error message contains expected components
        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("Arity mismatch"));
        assert!(error_msg.contains("+"));
        assert!(error_msg.contains("at least 2"));
        assert!(error_msg.contains("number 1")); // The number should be in the message
    }

    #[test]
    fn test_arity_error_with_no_args() {
        let span = Span::default();
        let args: Vec<AstNode> = vec![];

        let error = eval_arity_error(Some(span.clone()), &args, "*", "at least 2");

        let error_msg = format!("{:?}", error);

        // Check for enhanced error message components
        assert!(error_msg.contains("Arity mismatch"));
        assert!(error_msg.contains("*"));
        assert!(error_msg.contains("at least 2"));
        assert!(error_msg.contains("No arguments provided"));
    }

    #[test]
    fn test_arity_error_with_many_args() {
        let span = Span::default();
        let args: Vec<AstNode> = (1..=5)
            .map(|i| WithSpan {
                value: Arc::new(Expr::Number(i as f64, span.clone())),
                span: span.clone(),
            })
            .collect();

        let error = eval_arity_error(Some(span.clone()), &args, "not", "exactly 1");

        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("Arguments provided (5)"));
        assert!(error_msg.contains("number 1"));
        assert!(error_msg.contains("number 5"));
    }
}

#[cfg(test)]
mod type_error_tests {
    use super::*;

    #[test]
    fn test_enhanced_type_error_formatting() {
        let span = Span::default();
        let arg = WithSpan {
            value: Arc::new(Expr::String("hello".to_string(), span.clone())),
            span: span.clone(),
        };
        let value = Value::String("hello".to_string());

        let error = eval_type_error(Some(span.clone()), &arg, "+", "a Number", &value);

        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("Type mismatch"));
        assert!(error_msg.contains("Expected argument of type Number"));
        assert!(error_msg.contains("but received String"));
    }

    #[test]
    fn test_type_error_with_conversion_suggestion() {
        let span = Span::default();
        let arg = WithSpan {
            value: Arc::new(Expr::String("123".to_string(), span.clone())),
            span: span.clone(),
        };
        let value = Value::String("123".to_string());

        let error = eval_type_error(Some(span.clone()), &arg, "*", "a Number", &value);

        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("Consider parsing the string to a number"));
    }

    #[test]
    fn test_type_error_different_types() {
        let span = Span::default();

        // Test with various value types
        let test_cases = vec![
            (Value::Bool(true), "Boolean"),
            (Value::List(vec![]), "List"),
            (Value::Nil, "Nil"),
        ];

        for (value, expected_type) in test_cases {
            let arg = WithSpan {
                value: Arc::new(Expr::Number(42.0, span.clone())),
                span: span.clone(),
            };

            let error = eval_type_error(Some(span.clone()), &arg, "test_fn", "a Number", &value);
            let error_msg = format!("{:?}", error);
            assert!(error_msg.contains(&format!("but received {}", expected_type)));
        }
    }
}

#[cfg(test)]
mod macro_arity_tests {
    use super::*;

    #[test]
    fn test_macro_arity_check_exact_match() {
        let span = Span::default();
        let params = ParamList {
            required: vec!["a".to_string(), "b".to_string()],
            rest: None,
            span: span.clone(),
        };

        // Should succeed with exact number of args
        assert!(check_arity(2, &params, &span).is_ok());

        // Should fail with too few args
        assert!(check_arity(1, &params, &span).is_err());

        // Should fail with too many args (no variadic)
        assert!(check_arity(3, &params, &span).is_err());
    }

    #[test]
    fn test_macro_arity_check_variadic() {
        let span = Span::default();
        let params = ParamList {
            required: vec!["a".to_string()],
            rest: Some("rest".to_string()),
            span: span.clone(),
        };

        // Should succeed with minimum args
        assert!(check_arity(1, &params, &span).is_ok());

        // Should succeed with extra args (variadic)
        assert!(check_arity(3, &params, &span).is_ok());

        // Should fail with too few args
        assert!(check_arity(0, &params, &span).is_err());
    }

    #[test]
    fn test_macro_arity_error_messages() {
        let span = Span::default();
        let params = ParamList {
            required: vec!["a".to_string(), "b".to_string()],
            rest: None,
            span: span.clone(),
        };

        let error = check_arity(1, &params, &span).unwrap_err();
        let error_msg = format!("{:?}", error);

        // Check that enhanced macro arity error contains expected information
        assert!(error_msg.contains("Macro parameters: a, b"));
        assert!(error_msg.contains("Expected exactly 2 arguments, but received 1"));
    }

    #[test]
    fn test_macro_arity_variadic_error_messages() {
        let span = Span::default();
        let params = ParamList {
            required: vec!["required".to_string()],
            rest: Some("rest".to_string()),
            span: span.clone(),
        };

        let error = check_arity(0, &params, &span).unwrap_err();
        let error_msg = format!("{:?}", error);

        // Check variadic macro error messaging
        assert!(error_msg.contains("Expected at least 1"));
        assert!(error_msg.contains("additional arguments via '...' parameter"));
    }
}

#[cfg(test)]
mod general_error_tests {
    use super::*;

    #[test]
    fn test_enhanced_general_error_formatting() {
        let span = Span::default();
        let arg = WithSpan {
            value: Arc::new(Expr::Number(42.0, span.clone())),
            span: span.clone(),
        };

        let error = eval_general_error(Some(span.clone()), &arg, "Division by zero");

        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("Division by zero"));
        assert!(error_msg.contains("Error occurred while evaluating"));
        assert!(error_msg.contains("number 42"));
    }

    #[test]
    fn test_general_error_with_complex_expression() {
        let span = Span::default();
        let inner_list = vec![
            WithSpan {
                value: Arc::new(Expr::Number(1.0, span.clone())),
                span: span.clone(),
            },
            WithSpan {
                value: Arc::new(Expr::Number(2.0, span.clone())),
                span: span.clone(),
            },
        ];
        let arg = WithSpan {
            value: Arc::new(Expr::List(inner_list, span.clone())),
            span: span.clone(),
        };

        let error = eval_general_error(Some(span.clone()), &arg, "Complex error case");

        let error_msg = format!("{:?}", error);
        assert!(error_msg.contains("Complex error case"));
        assert!(error_msg.contains("list with 2 elements"));
    }
}

#[cfg(test)]
mod error_integration_tests {
    use super::*;

    #[test]
    fn test_error_message_consistency() {
        // Test that all error types follow consistent formatting patterns
        let span = Span::default();
        let arg = WithSpan {
            value: Arc::new(Expr::String("test".to_string(), span.clone())),
            span: span.clone(),
        };

        let arity_error = eval_arity_error(Some(span.clone()), &[arg.clone()], "test_fn", "2");
        let type_error = eval_type_error(
            Some(span.clone()),
            &arg,
            "test_fn",
            "Number",
            &Value::String("test".to_string()),
        );
        let general_error = eval_general_error(Some(span.clone()), &arg, "General error");

        // All errors should have spans
        assert!(arity_error.span.is_some());
        assert!(type_error.span.is_some());
        assert!(general_error.span.is_some());

        // All error messages should be non-empty and contain relevant context
        let arity_msg = format!("{:?}", arity_error);
        let type_msg = format!("{:?}", type_error);
        let general_msg = format!("{:?}", general_error);

        assert!(!arity_msg.is_empty());
        assert!(!type_msg.is_empty());
        assert!(!general_msg.is_empty());

        // All should contain contextual information
        assert!(arity_msg.contains("Arity"));
        assert!(type_msg.contains("Type"));
        assert!(general_msg.contains("General error"));
    }
}
