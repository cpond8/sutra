//!
//! Parses macro definitions from source code and files, including validation
//! and duplicate checking for robust macro loading.
//!
//! ## Error Handling
//!
//! All errors in this module are reported via the unified `SutraError` type and must be constructed using the `sutra_err!` macro. See `src/diagnostics.rs` for macro arms and usage rules.
//!
//! Example:
//! ```rust
//! return Err(sutra_err!(Validation, "Duplicate macro name '{}'", name));
//! ```
//!
//! All macro loading, parsing, and arity errors use this system.

use crate::ast::{AstNode, Expr, ParamList, Span};
use crate::macros::types::MacroTemplate;
use crate::{SutraError, sutra_err};
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
    let source = fs::read_to_string(path)
        .map_err(|e| sutra_err!(Internal, "Failed to read file: {}", e))?;
    parse_macros_from_source(&source)
}

/// Checks the arity of macro arguments against the parameter list.
/// Provides enhanced error reporting for mismatches, including variadic parameters.
pub fn check_arity(args_len: usize, params: &ParamList, _span: &Span) -> Result<(), SutraError> { // TODO: address the unused span
    let required_len = params.required.len();
    let has_variadic = params.rest.is_some();

    // Too few arguments
    if args_len < required_len {
        return Err(sutra_err!(Eval, "Macro arity error: expected at least {} arguments, got {}", required_len, args_len));
    }

    // Too many arguments for non-variadic macro
    if args_len > required_len && !has_variadic {
        return Err(sutra_err!(Eval, "Macro arity error: expected exactly {} arguments, got {}", required_len, args_len));
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
    let Expr::Define { name, params, body, span: _ } = &*expr.value else {
        return Ok(None); // Not a define form
    };

    if !names_seen.insert(name.clone()) {
        return Err(sutra_err!(Validation, "Duplicate macro name '{}'", name));
    }

    let template = MacroTemplate::new(params.clone(), body.clone())?;

    Ok(Some((name.clone(), template)))
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
