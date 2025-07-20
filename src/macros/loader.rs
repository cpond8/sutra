//!
//! Parses macro definitions from source code and files, including validation
//! and duplicate checking for robust macro loading.

use std::{collections::HashSet, fs, path::Path};

// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::{ast::ParamList, syntax::parser, MacroTemplate};

/// Type alias for macro parsing results
type MacroParseResult = Result<Vec<(String, MacroTemplate)>, SutraError>;

// =============================
// Public API for macro loading
// =============================

/// Parses Sutra macro definitions from a source string.
/// Identifies `define` forms, validates structure and parameters, and checks for duplicates.
pub fn parse_macros_from_source(source: &str) -> MacroParseResult {
    let exprs = parser::parse(source)?;
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
pub fn load_macros_from_file<P: AsRef<Path>>(path: P) -> MacroParseResult {
    let path_str = path.as_ref().to_string_lossy();
    let src_arc = to_error_source(&*path_str);
    let source = fs::read_to_string(&path).map_err(|e| {
        err_ctx!(
            Internal,
            format!("Failed to read file: {}", e.to_string()),
            &src_arc,
            Span::default(),
            "Check that the macro file exists and is readable."
        )
    })?;
    parse_macros_from_source(&source)
}

/// Checks the arity of macro arguments against the parameter list.
/// Provides enhanced error reporting for mismatches, including variadic parameters.
pub fn check_arity(
    args_len: usize,
    params: &ParamList,
    macro_name: &str,
    span: &Span,
) -> Result<(), SutraError> {
    let required_len = params.required.len();
    let has_variadic = params.rest.is_some();
    let src_arc = to_error_source(macro_name);
    // Too few arguments
    if args_len < required_len {
        return Err(err_ctx!(
            Eval,
            format!(
                "Macro arity error: expected at least {} arguments, got {}",
                required_len, args_len
            ),
            &src_arc,
            *span,
            "Too few arguments for macro"
        ));
    }
    // Too many arguments for non-variadic macro
    if args_len > required_len && !has_variadic {
        return Err(err_ctx!(
            Eval,
            format!(
                "Macro arity error: expected exactly {} arguments, got {}",
                required_len, args_len
            ),
            &src_arc,
            *span,
            "Too many arguments for macro"
        ));
    }
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
    // Only handle Expr::Define forms
    let Expr::Define {
        name,
        params,
        body,
        span,
    } = &*expr.value
    else {
        return Ok(None);
    };
    let src_arc = to_error_source(name);
    // Check for duplicate macro names
    if !names_seen.insert(name.clone()) {
        return Err(err_ctx!(
            Validation,
            format!("Duplicate macro name '{}'", name),
            &src_arc,
            *span,
            "Duplicate macro name"
        ));
    }
    // Attempt to construct the macro template
    let template = MacroTemplate::new(params.clone(), body.clone())?;
    Ok(Some((name.clone(), template)))
}

// =============================
// Macro Definition Parsing
// =============================

/// Returns true if the given expression is a macro definition of the form (define ...).
pub fn is_macro_definition(expr: &AstNode) -> bool {
    let Expr::List(items, _) = &*expr.value else {
        return false;
    };
    if items.len() != 3 {
        return false;
    }
    let Expr::Symbol(def, _) = &*items[0].value else {
        return false;
    };
    def == "define"
}

/// Parses a macro definition AST node into a (name, MacroTemplate) pair.
pub fn parse_macro_definition(expr: &AstNode) -> Result<(String, MacroTemplate), SutraError> {
    let Expr::List(items, _) = &*expr.value else {
        return Err(err_msg!(Internal, "Not a macro definition list."));
    };
    if items.len() != 3 {
        return Err(err_msg!(Internal, "Macro definition must have 3 elements."));
    }
    let Expr::Symbol(def, _) = &*items[0].value else {
        return Err(err_msg!(Internal, "First element must be 'define'."));
    };
    if def != "define" {
        return Err(err_msg!(Internal, "First element must be 'define'."));
    }
    let Expr::ParamList(param_list) = &*items[1].value else {
        return Err(err_msg!(
            Internal,
            "Second element must be a parameter list."
        ));
    };
    let macro_name = param_list
        .required
        .first()
        .cloned()
        .ok_or_else(|| err_msg!(Internal, "Macro name missing in parameter list."))?;
    let params = ParamList {
        required: param_list.required[1..].to_vec(),
        rest: param_list.rest.clone(),
        span: param_list.span,
    };
    let template = MacroTemplate::new(params, Box::new(items[2].clone()))?;
    Ok((macro_name, template))
}

// =============================
// Tests
// =============================

#[cfg(test)]
mod tests {
    use Span;

    use super::*;

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

        assert!(check_arity(2, &params, "test", &span).is_ok());
    }

    #[test]
    fn test_check_arity_too_few() {
        let params = ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(1, &params, "test", &span).is_err());
    }

    #[test]
    fn test_check_arity_too_many_non_variadic() {
        let params = ParamList {
            required: vec!["x".to_string()],
            rest: None,
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(2, &params, "test", &span).is_err());
    }

    #[test]
    fn test_check_arity_variadic_ok() {
        let params = ParamList {
            required: vec!["x".to_string()],
            rest: Some("rest".to_string()),
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(1, &params, "test", &span).is_ok());
        assert!(check_arity(3, &params, "test", &span).is_ok());
    }

    #[test]
    fn test_check_arity_variadic_too_few() {
        let params = ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: Some("rest".to_string()),
            span: Span::default(),
        };
        let span = Span::default();

        assert!(check_arity(1, &params, "test", &span).is_err());
    }
}
