//! # Sutra Standard Macro Library
//!
//! This module provides the core, author-facing macros that expand into
//! simpler atom-level expressions. This is where the ergonomic "surface syntax"
//! of the language is defined.
//!
//! ## Auto-Get Principle
//!
//! A key responsibility of these macros is to handle the "auto-get" feature.
//! They inspect their arguments and, if an argument is a bare symbol, they
//! wrap it in an explicit `(get ...)` form. This ensures that by the time
//! the AST reaches the evaluator, all world-state lookups are explicit.

use crate::ast::Expr;
use crate::error::{SutraError, SutraErrorKind};
use crate::macros::utils::canonicalize_path;

// ---
// Helper Functions
// ---

/// A helper to wrap a path expression in a `(get ...)` list.
/// It uses `canonicalize_path` to ensure the path is in the correct format.
/// If the expression is not a valid path, it returns the original expression.
fn wrap_in_get(expr: &Expr) -> Expr {
    if let Ok(path) = canonicalize_path(expr) {
        let get_symbol = Expr::Symbol("get".to_string(), expr.span());
        Expr::List(vec![get_symbol, path], expr.span())
    } else {
        // If it's not a valid path (e.g., a literal number), return as is.
        expr.clone()
    }
}

/// A helper for creating binary predicate macros like `is?`, `over?`, etc.
/// It ensures the macro has two arguments and expands it to the target atom.
fn create_binary_predicate_macro(
    expr: &Expr,
    macro_name: &str,
    atom_name: &str,
) -> Result<Expr, SutraError> {
    if let Expr::List(items, span) = expr {
        if items.len() != 3 {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro '{}' expects 2 arguments, but got {}",
                    macro_name,
                    items.len() - 1
                )),
                span: Some(span.clone()),
            });
        }

        let atom_symbol = Expr::Symbol(atom_name.to_string(), items[0].span());
        let arg1 = wrap_in_get(&items[1]);
        let arg2 = wrap_in_get(&items[2]);

        Ok(Expr::List(vec![atom_symbol, arg1, arg2], span.clone()))
    } else {
        Err(SutraError {
            kind: SutraErrorKind::Macro(format!(
                "Macro '{}' can only be applied to a list.",
                macro_name
            )),
            span: Some(expr.span()),
        })
    }
}

// ---
// Standard Macros
// ---

/// Expands `(is? a b)` to `(eq? (get a) (get b))`.
pub fn expand_is(expr: &Expr) -> Result<Expr, SutraError> {
    create_binary_predicate_macro(expr, "is?", "eq?")
}

/// Expands `(over? a b)` to `(gt? (get a) (get b))`.
pub fn expand_over(expr: &Expr) -> Result<Expr, SutraError> {
    create_binary_predicate_macro(expr, "over?", "gt?")
}

/// Expands `(under? a b)` to `(lt? (get a) (get b))`.
pub fn expand_under(expr: &Expr) -> Result<Expr, SutraError> {
    create_binary_predicate_macro(expr, "under?", "lt?")
}

/// A helper for creating assignment macros like `add!`, `sub!`, etc.
/// It expands `(macro! path value)` to `(set! (list "path") (atom (get (list "path")) value))`.
fn create_assignment_macro(
    expr: &Expr,
    macro_name: &str,
    atom_name: &str,
) -> Result<Expr, SutraError> {
    if let Expr::List(items, span) = expr {
        if items.len() != 3 {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro '{}' expects 2 arguments, but got {}",
                    macro_name,
                    items.len() - 1
                )),
                span: Some(span.clone()),
            });
        }

        let set_symbol = Expr::Symbol("set!".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = canonicalize_path(path_arg)?;
        let value_arg = items[2].clone();

        // Create the inner expression: (atom (get path) value)
        let atom_symbol = Expr::Symbol(atom_name.to_string(), items[0].span());
        let inner_expr = Expr::List(
            vec![
                atom_symbol,
                wrap_in_get(path_arg), // Get the current value at the path
                value_arg,             // The value to add/subtract
            ],
            span.clone(), // Approximate span
        );

        // Create the final expression: (set! <canonical-path> inner_expr)
        Ok(Expr::List(
            vec![set_symbol, canonical_path, inner_expr],
            span.clone(),
        ))
    } else {
        Err(SutraError {
            kind: SutraErrorKind::Macro(format!(
                "Macro '{}' can only be applied to a list.",
                macro_name
            )),
            span: Some(expr.span()),
        })
    }
}

/// Expands `(add! path value)` to `(set! path (+ (get path) value))`.
///
/// All path arguments are canonicalized at macro-expansion time.
/// Atom contracts assume canonical input; this is strictly enforced and tested.
pub fn expand_add(expr: &Expr) -> Result<Expr, SutraError> {
    create_assignment_macro(expr, "add!", "+")
}

/// Expands `(sub! path value)` to `(set! path (- (get path) value))`.
///
/// All path arguments are canonicalized at macro-expansion time.
/// Atom contracts assume canonical input; this is strictly enforced and tested.
pub fn expand_sub(expr: &Expr) -> Result<Expr, SutraError> {
    create_assignment_macro(expr, "sub!", "-")
}

/// Expands `(inc! path)` to `(set! (list "path") (+ (get (list "path")) 1))`.
///
/// All path arguments are canonicalized at macro-expansion time.
/// Atom contracts assume canonical input; this is strictly enforced and tested.
pub fn expand_inc(expr: &Expr) -> Result<Expr, SutraError> {
    if let Expr::List(items, span) = expr {
        if items.len() != 2 {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro 'inc!' expects 1 argument, but got {}",
                    items.len() - 1
                )),
                span: Some(span.clone()),
            });
        }

        let set_symbol = Expr::Symbol("set!".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = canonicalize_path(path_arg)?;

        // Create the inner expression: (+ (get path) 1)
        let add_symbol = Expr::Symbol("+".to_string(), items[0].span());
        let one = Expr::Number(1.0, items[0].span());
        let inner_expr = Expr::List(vec![add_symbol, wrap_in_get(path_arg), one], span.clone());

        Ok(Expr::List(
            vec![set_symbol, canonical_path, inner_expr],
            span.clone(),
        ))
    } else {
        Err(SutraError {
            kind: SutraErrorKind::Macro("Macro 'inc!' can only be applied to a list.".to_string()),
            span: Some(expr.span()),
        })
    }
}

/// Expands `(dec! path)` to `(set! (list "path") (- (get (list "path")) 1))`.
///
/// All path arguments are canonicalized at macro-expansion time.
/// Atom contracts assume canonical input; this is strictly enforced and tested.
pub fn expand_dec(expr: &Expr) -> Result<Expr, SutraError> {
    if let Expr::List(items, span) = expr {
        if items.len() != 2 {
            return Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro 'dec!' expects 1 argument, but got {}",
                    items.len() - 1
                )),
                span: Some(span.clone()),
            });
        }

        let set_symbol = Expr::Symbol("set!".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = canonicalize_path(path_arg)?;

        // Create the inner expression: (- (get path) 1)
        let sub_symbol = Expr::Symbol("-".to_string(), items[0].span());
        let one = Expr::Number(1.0, items[0].span());
        let inner_expr = Expr::List(vec![sub_symbol, wrap_in_get(path_arg), one], span.clone());

        Ok(Expr::List(
            vec![set_symbol, canonical_path, inner_expr],
            span.clone(),
        ))
    } else {
        Err(SutraError {
            kind: SutraErrorKind::Macro("Macro 'dec!' can only be applied to a list.".to_string()),
            span: Some(expr.span()),
        })
    }
}
