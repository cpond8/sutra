//!
//! Parses macro definitions from source code and files, including validation
//! and duplicate checking for robust macro loading.

use std::{collections::HashSet, fs, path::Path};

use crate::prelude::*;
use crate::{
    ast::ParamList, errors, runtime::source::SourceContext, syntax::parser,
    syntax::parser::to_source_span, MacroTemplate,
};

/// Type alias for macro parsing results
type MacroParseResult = Result<Vec<(String, MacroTemplate)>, SutraError>;

// =============================
// Public API for macro loading
// =============================

/// Parses Sutra macro definitions from a source string.
/// Identifies `define` forms, validates structure and parameters, and checks for duplicates.
pub fn parse_macros_from_source(source: &str) -> MacroParseResult {
    // Filter out comment lines (lines starting with ';;')
    let filtered_source = source
        .lines()
        .filter(|line| !line.trim_start().starts_with(";;"))
        .collect::<Vec<_>>()
        .join("\n");
    let source_context = SourceContext::from_file("macro source", &filtered_source);
    let exprs = parser::parse(&filtered_source, source_context)?;
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
    let source = fs::read_to_string(&path).map_err(|e| {
        {
            let sc = SourceContext::from_file(path_str.to_string(), e.to_string());
            errors::runtime_general(
                format!("Unable to read macro file '{}'", path_str),
                "file load error",
                &sc,
                miette::SourceSpan::from((0, 0)),
            )
        }
        .with_suggestion("Check that the file exists and has correct read permissions.")
    })?;
    parse_macros_from_source(&source)
}

/// Checks the arity of macro arguments against the parameter list.
/// Provides enhanced error reporting for mismatches, including variadic parameters.
pub fn check_arity(
    args_len: usize,
    params: &ParamList,
    _macro_name: &str,
    span: &Span,
    source: &SourceContext,
) -> Result<(), SutraError> {
    let required_len = params.required.len();
    let has_variadic = params.rest.is_some();
    if args_len < required_len {
        let span = to_source_span(*span);
        return Err(errors::validation_arity(
            format!("at least {}", required_len),
            args_len,
            source,
            span,
        ));
    }
    if args_len > required_len && !has_variadic {
        let span = to_source_span(*span);
        return Err(errors::validation_arity(
            format!("{}", required_len),
            args_len,
            source,
            span,
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
    if !is_macro_definition(expr) {
        return Ok(None);
    }

    let (macro_name, template) = parse_macro_definition(expr)?;

    if !names_seen.insert(macro_name.clone()) {
        return Err({
            let sc = SourceContext::from_file(&macro_name, format!("{:?}", expr));
            errors::runtime_general(
                format!("duplicate macro definition for '{}'", macro_name),
                "duplicate macro",
                &sc,
                to_source_span(expr.span),
            )
        }
        .with_suggestion("Ensure all macro names within a file are unique."));
    }

    Ok(Some((macro_name, template)))
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
    if def != "define" {
        return false;
    }
    // The second element must be a parameter list for it to be a valid macro definition.
    matches!(&*items[1].value, Expr::ParamList(_))
}

/// Parses a macro definition AST node into a (name, MacroTemplate) pair.
pub fn parse_macro_definition(expr: &AstNode) -> Result<(String, MacroTemplate), SutraError> {
    let full_source = &format!("{:?}", expr);
    let (items, span) = if let Expr::List(items, span) = &*expr.value {
        (items, span)
    } else {
        let sc = SourceContext::from_file("macro definition", full_source);
        return Err(errors::runtime_general(
            "Macro definition must be a list expression",
            "invalid definition",
            &sc,
            to_source_span(expr.span),
        )
        .with_suggestion("Expected a form like: (define (macro-name ...) body)"));
    };

    if items.len() != 3 {
        let sc = SourceContext::from_file("macro definition", full_source);
        return Err(errors::validation_arity(
            "3".to_string(),
            items.len(),
            &sc,
            to_source_span(*span),
        )
        .with_suggestion(
            "A macro definition must have 3 parts: define keyword, parameter list, and body.",
        ));
    }

    if let Expr::Symbol(s, _) = &*items[0].value {
        if s != "define" {
            let sc = SourceContext::from_file("macro definition", full_source);
            return Err(errors::runtime_general(
                "Macro definition must start with 'define'",
                "invalid definition",
                &sc,
                to_source_span(items[0].span),
            ));
        }
    } else {
        let sc = SourceContext::from_file("macro definition", full_source);
        return Err(errors::type_mismatch(
            "symbol",
            items[0].value.type_name(),
            &sc,
            to_source_span(items[0].span),
        ));
    }

    let param_list = if let Expr::ParamList(pl) = &*items[1].value {
        pl
    } else {
        let sc = SourceContext::from_file("macro definition", full_source);
        return Err(errors::type_mismatch(
            "parameter list",
            items[1].value.type_name(),
            &sc,
            to_source_span(items[1].span),
        ));
    };

    let macro_name = param_list.required.first().cloned().ok_or_else(|| {
        let sc = SourceContext::from_file("macro definition", full_source.clone());
        errors::runtime_general(
            "Macro name missing from parameter list",
            "invalid definition",
            &sc,
            to_source_span(items[1].span),
        )
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
        assert!(error.to_string().contains("duplicate macro definition"));
    }

    #[test]
    fn test_check_arity_exact_match() {
        let params = ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: Span::default(),
        };
        let span = Span::default();
        let source = SourceContext::from_file("test", "test");

        assert!(check_arity(2, &params, "test", &span, &source).is_ok());
    }

    #[test]
    fn test_check_arity_too_few() {
        let params = ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: None,
            span: Span::default(),
        };
        let span = Span::default();
        let source = SourceContext::from_file("test", "test");

        assert!(check_arity(1, &params, "test", &span, &source).is_err());
    }

    #[test]
    fn test_check_arity_too_many_non_variadic() {
        let params = ParamList {
            required: vec!["x".to_string()],
            rest: None,
            span: Span::default(),
        };
        let span = Span::default();
        let source = SourceContext::from_file("test", "test");

        assert!(check_arity(2, &params, "test", &span, &source).is_err());
    }

    #[test]
    fn test_check_arity_variadic_ok() {
        let params = ParamList {
            required: vec!["x".to_string()],
            rest: Some("rest".to_string()),
            span: Span::default(),
        };
        let span = Span::default();
        let source = SourceContext::from_file("test", "test");

        assert!(check_arity(1, &params, "test", &span, &source).is_ok());
        assert!(check_arity(3, &params, "test", &span, &source).is_ok());
    }

    #[test]
    fn test_check_arity_variadic_too_few() {
        let params = ParamList {
            required: vec!["x".to_string(), "y".to_string()],
            rest: Some("rest".to_string()),
            span: Span::default(),
        };
        let span = Span::default();
        let source = SourceContext::from_file("test", "test");

        assert!(check_arity(1, &params, "test", &span, &source).is_err());
    }
}
