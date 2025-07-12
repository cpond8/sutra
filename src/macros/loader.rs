//! Macro definition parsing and file loading.
//!
//! Parses macro definitions from source code and files, including validation
//! and duplicate checking for robust macro loading.

use crate::ast::{AstNode, Expr, ParamList, Span};
use crate::macros::types::MacroTemplate;
use crate::syntax::error::{io_error, macro_error, SutraError};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

// =============================
// Public API for macro loading
// =============================

/// Parses Sutra macro definitions from a source string.
/// Identifies `define` forms, validates structure and parameters, and checks for duplicates.
pub fn parse_macros_from_source(source: &str) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    let exprs = crate::syntax::parser::parse(source)?;
    let mut macros = Vec::new();
    let mut names_seen = HashSet::new();

    for expr in exprs {
        if let Some((macro_name, template)) = try_parse_macro_form(&expr, &mut names_seen)? {
            macros.push((macro_name, template));
        }
    }
    Ok(macros)
}

/// Loads macro definitions from a file, with ergonomic path handling.
pub fn load_macros_from_file<P: AsRef<Path>>(
    path: P,
) -> Result<Vec<(String, MacroTemplate)>, SutraError> {
    let source = fs::read_to_string(path).map_err(|e| io_error(e.to_string(), None))?;
    parse_macros_from_source(&source)
}

/// Checks the arity of macro arguments against the parameter list.
/// Provides enhanced error reporting for mismatches, including variadic parameters.
pub fn check_arity(args_len: usize, params: &ParamList, span: &Span) -> Result<(), SutraError> {
    let required_len = params.required.len();
    let has_variadic = params.rest.is_some();

    // Too few arguments
    if args_len < required_len {
        return Err(crate::macros::error::enhanced_macro_arity_error(
            args_len, params, span,
        ));
    }

    // Too many arguments for non-variadic macro
    if args_len > required_len && !has_variadic {
        return Err(crate::macros::error::enhanced_macro_arity_error(
            args_len, params, span,
        ));
    }

    // Arity is correct
    Ok(())
}

// =============================
// Internal parsing helpers
// =============================

// Attempts to parse a macro definition form from an AST node.
fn try_parse_macro_form(
    expr: &AstNode,
    names_seen: &mut HashSet<String>,
) -> Result<Option<(String, MacroTemplate)>, SutraError> {
    let Some(items) = validate_define_form(expr) else {
        return Ok(None);
    };

    let Some(param_list) = items.get(1) else {
        return Ok(None);
    };

    let Expr::ParamList(param_list) = &*param_list.value else {
        return Ok(None);
    };

    let macro_name = extract_and_check_macro_name(param_list, names_seen)?;
    let params = build_macro_params(param_list);
    let body = Box::new(items[2].clone());
    let template = MacroTemplate::new(params, body)?;

    Ok(Some((macro_name, template)))
}

// Validates the basic `(define (name ...) body)` structure.
fn validate_define_form(expr: &AstNode) -> Option<&[AstNode]> {
    let Expr::List(items, _) = &*expr.value else {
        return None;
    };

    if items.len() != 3 {
        return None;
    }

    let Expr::Symbol(s, _) = &*items[0].value else {
        return None;
    };

    if s != "define" {
        return None;
    }

    Some(items)
}

// Extracts and validates macro name, checking for duplicates.
fn extract_and_check_macro_name(
    param_list: &ParamList,
    names_seen: &mut HashSet<String>,
) -> Result<String, SutraError> {
    let macro_name = extract_macro_name(param_list)?;

    if !names_seen.insert(macro_name.clone()) {
        return Err(macro_error(
            format!("Duplicate macro name '{}'.", macro_name),
            Some(param_list.span.clone()),
        ));
    }

    Ok(macro_name)
}

// Extracts the macro name from a parameter list (first element).
fn extract_macro_name(param_list: &ParamList) -> Result<String, SutraError> {
    let Some(name) = param_list.required.first() else {
        return Err(macro_error(
            "Macro name must be the first element of the parameter list.",
            Some(param_list.span.clone()),
        ));
    };

    Ok(name.clone())
}

// Builds macro parameters by excluding the macro name from the list.
fn build_macro_params(param_list: &ParamList) -> ParamList {
    ParamList {
        required: param_list.required[1..].to_vec(),
        rest: param_list.rest.clone(),
        span: param_list.span.clone(),
    }
}

// =============================
// Tests
// =============================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Span;

    #[test]
    fn test_parse_simple_macro() {
        let source = "(define (double x) (* x 2))";
        let result = parse_macros_from_source(source).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "double");
        assert_eq!(result[0].1.params.required.len(), 1);
        assert_eq!(result[0].1.params.required[0], "x");
        assert!(result[0].1.params.rest.is_none());
    }

    #[test]
    fn test_parse_variadic_macro() {
        let source = "(define (sum x ...rest) (+ x (apply + rest)))";
        let result = parse_macros_from_source(source).unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].0, "sum");
        assert_eq!(result[0].1.params.required.len(), 1);
        assert_eq!(result[0].1.params.required[0], "x");
        assert_eq!(result[0].1.params.rest, Some("rest".to_string()));
    }

    #[test]
    fn test_parse_multiple_macros() {
        let source = r#"
            (define (macro1 x) (+ x 1))
            (define (macro2 y) (- y 1))
        "#;
        let result = parse_macros_from_source(source).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].0, "macro1");
        assert_eq!(result[1].0, "macro2");
    }

    #[test]
    fn test_duplicate_macro_names() {
        let source = r#"
            (define (test x) (+ x 1))
            (define (test y) (- y 1))
        "#;
        let result = parse_macros_from_source(source);

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Duplicate macro name"));
    }

    #[test]
    fn test_check_arity_exact_match() {
        let params = ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(2, &params, &span).is_ok());
    }

    #[test]
    fn test_check_arity_too_few() {
        let params = ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(1, &params, &span).is_err());
    }

    #[test]
    fn test_check_arity_too_many_non_variadic() {
        let params = ParamList {
            required: vec!["x".to_string()],
            rest: None,
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(2, &params, &span).is_err());
    }

    #[test]
    fn test_check_arity_variadic_ok() {
        let params = ParamList {
            required: vec!["x".to_string()],
            rest: Some("rest".to_string()),
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(1, &params, &span).is_ok());
        assert!(check_arity(3, &params, &span).is_ok());
    }

    #[test]
    fn test_check_arity_variadic_too_few() {
        let params = ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: Some("rest".to_string()),
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(1, &params, &span).is_err());
    }
}
