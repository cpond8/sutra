//!
//! Parses macro definitions from source code and files, including validation
//! and duplicate checking for robust macro loading.

use std::{collections::HashSet, fs, path::Path};

use crate::prelude::*;
use crate::syntax::parser::to_source_span;
use crate::{ast::ParamList, syntax::parser, MacroTemplate};

/// Type alias for macro parsing results
type MacroParseResult = Result<Vec<(String, MacroTemplate)>, SutraError>;

// =============================
// Public API for macro loading
// =============================

/// Parses Sutra macro definitions from a source string.
/// Identifies `define` forms, validates structure and parameters, and checks for duplicates.
pub fn parse_macros_from_source(source: &str) -> MacroParseResult {
    let exprs = parser::parse(source).map_err(|e| {
        // If parser::parse already returns SutraError, this is a no-op. Otherwise, wrap as ParseMalformed.
        // Here, we assume e is convertible to string for message, and no source context is available.
        SutraError::ParseMalformed {
            construct: "macro source".to_string(),
            src: miette::NamedSource::new("macro source", source.to_string()),
            span: to_source_span(Span::default()),
            suggestion: Some(e.to_string()),
        }
    })?;
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
    let source = fs::read_to_string(&path).map_err(|e| SutraError::ResourceOperation {
        operation: "read".to_string(),
        path: path_str.to_string(),
        reason: e.to_string(),
        source: Some(Box::new(e)),
        suggestion: Some("Check that the macro file exists and is readable.".to_string()),
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
    let src = miette::NamedSource::new(macro_name.to_string(), macro_name.to_string());
    if args_len < required_len {
        return Err(SutraError::ValidationArity {
            expected: format!("at least {}", required_len),
            actual: args_len,
            src: src.clone(),
            span: to_source_span(*span),
        });
    }
    if args_len > required_len && !has_variadic {
        return Err(SutraError::ValidationArity {
            expected: format!("{}", required_len),
            actual: args_len,
            src,
            span: to_source_span(*span),
        });
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
    let src = miette::NamedSource::new(name.clone(), name.clone());
    // Check for duplicate macro names
    if !names_seen.insert(name.clone()) {
        return Err(SutraError::MacroDuplicate {
            name: name.clone(),
            src: src.clone(),
            span: to_source_span(*span),
            first_definition: to_source_span(*span),
        });
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
    let Expr::List(items, span) = &*expr.value else {
        return Err(SutraError::MacroInvalidDefinition {
            reason: "must be list expression".to_string(),
            actual_count: None,
            src: miette::NamedSource::new("macro definition", format!("{:?}", expr)),
            span: to_source_span(expr.span),
        });
    };
    if items.len() != 3 {
        return Err(SutraError::MacroInvalidDefinition {
            reason: "requires exactly 3 elements".to_string(),
            actual_count: Some(items.len()),
            src: miette::NamedSource::new("macro definition", format!("{:?}", expr)),
            span: to_source_span(*span),
        });
    }
    let Expr::Symbol(def, _) = &*items[0].value else {
        return Err(SutraError::MacroInvalidDefinition {
            reason: "must start with 'define'".to_string(),
            actual_count: Some(items.len()),
            src: miette::NamedSource::new("macro definition", format!("{:?}", expr)),
            span: to_source_span(items[0].span),
        });
    };
    if def != "define" {
        return Err(SutraError::MacroInvalidDefinition {
            reason: "first element must be 'define'".to_string(),
            actual_count: Some(items.len()),
            src: miette::NamedSource::new("macro definition", format!("{:?}", expr)),
            span: to_source_span(items[0].span),
        });
    }
    let Expr::ParamList(param_list) = &*items[1].value else {
        return Err(SutraError::MacroInvalidDefinition {
            reason: "second element must be parameter list".to_string(),
            actual_count: Some(items.len()),
            src: miette::NamedSource::new("macro definition", format!("{:?}", expr)),
            span: to_source_span(items[1].span),
        });
    };
    let macro_name =
        param_list
            .required
            .first()
            .cloned()
            .ok_or_else(|| SutraError::MacroInvalidDefinition {
                reason: "macro name missing in parameter list".to_string(),
                actual_count: Some(param_list.required.len()),
                src: miette::NamedSource::new("macro definition", format!("{:?}", expr)),
                span: to_source_span(items[1].span),
            })?;
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
