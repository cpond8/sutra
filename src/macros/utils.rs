// src/macros/utils.rs

//! Utility functions for building and manipulating macros.
//! This module provides common helpers to ensure consistency and reduce
//! boilerplate in macro definitions.

use crate::ast::{Expr, Span};
use crate::error::{SutraError, SutraErrorKind};

/// Single source of truth for all macro-level path canonicalization.
/// All macro logic that deals with world paths must call this function.
///
/// Transforms a path expression (e.g., a symbol `player.score` or a list
/// `(player score)`) into its canonical list-of-strings form, e.g.,
/// `(list "player" "score")`.
///
/// This is a crucial helper for macros that deal with state paths, ensuring
/// that the path argument passed to the `set!` atom is always in the correct
/// format.
///
/// # Arguments
///
/// * `path_expr` - An expression representing the path.
///
/// # Returns
///
/// A `Result` containing the canonicalized `Expr::List` on success, or a
/// `SutraError` with `SutraErrorKind::Macro` on failure.
pub fn canonicalize_path(path_expr: &Expr) -> Result<Expr, SutraError> {
    // The parser wraps everything in a `(do ...)` block. If we see that,
    // we should operate on the first real expression inside it.
    let target_expr = if let Expr::List(items, _) = path_expr {
        if items.get(0) == Some(&Expr::Symbol("do".to_string(), Span { start: 0, end: 0 }))
            && items.len() > 1
        {
            &items[1]
        } else {
            path_expr
        }
    } else {
        path_expr
    };

    let span = target_expr.span();
    match target_expr {
        // Case 1: A single symbol, e.g., `player.score` or `score`.
        Expr::Symbol(s, _) => {
            let mut parts: Vec<Expr> = s
                .split('.')
                .map(|part| Expr::String(part.to_string(), span.clone()))
                .collect();
            let mut list_elems = vec![Expr::Symbol("list".to_string(), span.clone())];
            list_elems.extend(parts);
            Ok(Expr::List(list_elems, span))
        }
        // Case 2: A list of symbols or strings, e.g., `(player score)` or `("player" "score")`.
        Expr::List(items, list_span) => {
            let mut parts = Vec::new();
            for item in items {
                match item {
                    Expr::Symbol(s, item_span) => {
                        parts.push(Expr::String(s.clone(), item_span.clone()));
                    }
                    Expr::String(s, item_span) => {
                        parts.push(Expr::String(s.clone(), item_span.clone()));
                    }
                    _ => {
                        return Err(SutraError {
                            kind: SutraErrorKind::Macro(
                                "Invalid path format: list must contain only symbols or strings."
                                    .to_string(),
                            ),
                            span: Some(item.span()),
                        });
                    }
                }
            }
            let mut list_elems = vec![Expr::Symbol("list".to_string(), list_span.clone())];
            list_elems.extend(parts);
            Ok(Expr::List(list_elems, list_span.clone()))
        }
        // Any other expression type is invalid as a path.
        _ => Err(SutraError {
            kind: SutraErrorKind::Macro(
                "Invalid path format: expected a symbol or a list.".to_string(),
            ),
            span: Some(span),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;
    use crate::parser::parse;

    fn parse_unwrap(s: &str) -> Expr {
        parse(s).unwrap()
    }

    fn s(val: &str) -> Expr {
        Expr::Symbol(val.to_string(), Span { start: 0, end: 0 })
    }

    fn str_expr(val: &str) -> Expr {
        Expr::String(val.to_string(), Span { start: 0, end: 0 })
    }

    /// Recursively strips all Span information from an expression, replacing it
    /// with a default `Span { start: 0, end: 0 }`. This is useful for tests
    /// where we only care about the structural equality of two expressions.
    fn strip_spans(expr: &mut Expr) {
        match expr {
            Expr::List(items, span) => {
                *span = Span { start: 0, end: 0 };
                for item in items {
                    strip_spans(item);
                }
            }
            Expr::Symbol(_, span)
            | Expr::String(_, span)
            | Expr::Number(_, span)
            | Expr::Bool(_, span) => {
                *span = Span { start: 0, end: 0 };
            }
        }
    }

    #[test]
    fn test_canonicalize_path_simple_symbol() {
        let input = parse_unwrap("score");
        let expected = Expr::List(
            vec![s("list"), str_expr("score")],
            Span { start: 0, end: 0 },
        );
        let mut actual = canonicalize_path(&input).unwrap();
        strip_spans(&mut actual);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_canonicalize_path_dotted_symbol() {
        let input = parse_unwrap("player.score.value");
        let expected = Expr::List(
            vec![
                s("list"),
                str_expr("player"),
                str_expr("score"),
                str_expr("value"),
            ],
            Span { start: 0, end: 0 },
        );
        let mut actual = canonicalize_path(&input).unwrap();
        strip_spans(&mut actual);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_canonicalize_path_list_of_symbols() {
        let input = parse_unwrap("(player score)");
        let expected = Expr::List(
            vec![s("list"), str_expr("player"), str_expr("score")],
            Span { start: 0, end: 0 },
        );
        let mut actual = canonicalize_path(&input).unwrap();
        strip_spans(&mut actual);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_canonicalize_path_list_of_strings() {
        let input = parse_unwrap(r#"("player" "score")"#);
        let expected = Expr::List(
            vec![s("list"), str_expr("player"), str_expr("score")],
            Span { start: 0, end: 0 },
        );
        let mut actual = canonicalize_path(&input).unwrap();
        strip_spans(&mut actual);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_canonicalize_path_invalid_list_mixed_types() {
        let input = parse_unwrap(r#"(player "score" 123)"#);
        let result = canonicalize_path(&input);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e.kind, SutraErrorKind::Macro(_)));
            if let SutraErrorKind::Macro(msg) = e.kind {
                assert!(msg.contains("list must contain only symbols or strings"));
            }
        }
    }

    #[test]
    fn test_canonicalize_path_invalid_type_number() {
        let input = parse_unwrap("123");
        let result = canonicalize_path(&input);
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(matches!(e.kind, SutraErrorKind::Macro(_)));
            if let SutraErrorKind::Macro(msg) = e.kind {
                assert!(msg.contains("expected a symbol or a list"));
            }
        }
    }
}
