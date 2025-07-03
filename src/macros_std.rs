//! # Sutra Standard Macro Library
//!
//! This module is the sole authority on path canonicalization and provides
//! the core, author-facing macros that expand into simpler, canonical ASTs.
//!
//! ## Core Responsibility: Path Canonicalization
//!
//! The primary role of this module is to convert user-friendly path syntax
//! (e.g., `player.score` or `(player score)`) into a canonical `Expr::Path`
//! node. This is the only place in the entire engine where path syntax is parsed.

use crate::ast::{Expr, Span};
use crate::error::{SutraError, SutraErrorKind};
use crate::macros::{MacroRegistry, MacroDef, MacroTemplate};
use crate::path::Path;

// ---
// Registry
// ---

/// Registers all standard macros in the given registry.
pub fn register_std_macros(registry: &mut MacroRegistry) {
    // Core macros
    registry.register("set!", expand_set);
    registry.register("get", expand_get);
    registry.register("del!", expand_del);

    // Conditional macros
    registry.register("if", expand_if);

    // Predicate macros
    registry.register("is?", expand_is);
    registry.register("over?", expand_over);
    registry.register("under?", expand_under);
    // Assignment macros
    registry.register("add!", expand_add);
    registry.register("sub!", expand_sub);
    registry.register("inc!", expand_inc);
    registry.register("dec!", expand_dec);
    // Standard macros like cond are now loaded from macros.sutra at startup.
}

// ---
// Path Canonicalization: The Single Source of Truth
// ---

/// Converts a user-facing expression (`Symbol` or `List`) into a canonical `Path`.
/// This is the only function in the engine that understands path syntax.
fn expr_to_path(expr: &Expr) -> Result<Path, SutraError> {
    match expr {
        // Dotted symbol syntax: `player.score`
        Expr::Symbol(s, _) => Ok(Path(s.split('.').map(String::from).collect())),
        // List syntax: `(player score)`
        Expr::List(items, _) => {
            let mut segments = Vec::new();
            for item in items {
                match item {
                    Expr::Symbol(s, _) => segments.push(s.clone()),
                    Expr::String(s, _) => segments.push(s.clone()),
                    _ => {
                        return Err(SutraError {
                            kind: SutraErrorKind::Macro(
                                "Path lists can only contain symbols or strings.".to_string(),
                            ),
                            span: Some(item.span()),
                        });
                    }
                }
            }
            Ok(Path(segments))
        }
        _ => Err(SutraError {
            kind: SutraErrorKind::Macro(
                "Invalid path format: expected a symbol or a list.".to_string(),
            ),
            span: Some(expr.span()),
        }),
    }
}

/// A helper to wrap a path-like expression in a `(core/get ...)` call.
/// If the expression is a valid path, it's converted to an `Expr::Path` and
/// wrapped in `(core/get ...)`. Otherwise, it's returned as-is.
fn wrap_in_get(expr: &Expr) -> Expr {
    if let Ok(path) = expr_to_path(expr) {
        let get_symbol = Expr::Symbol("core/get".to_string(), expr.span());
        let path_expr = Expr::Path(path, expr.span());
        Expr::List(vec![get_symbol, path_expr], expr.span())
    } else {
        expr.clone()
    }
}

// ---
// Macro Helpers
// ---

/// A private helper macro to reduce boilerplate in macro expansion functions.
/// It handles the common tasks of checking that the expression is a list and
/// verifying the number of arguments.
macro_rules! define_macro_helper {
    ($expr:expr, $macro_name:expr, $expected_arity:expr, |$items:ident, $span:ident| $expansion:block) => {{
        if let Expr::List($items, $span) = $expr {
            // Arity check is `expected + 1` because the macro name itself is an item.
            if $items.len() != $expected_arity + 1 {
                Err(SutraError {
                    kind: SutraErrorKind::Macro(format!(
                        "Macro '{}' expects {} arguments, but got {}",
                        $macro_name,
                        $expected_arity,
                        $items.len() - 1
                    )),
                    span: Some($span.clone()),
                })
            } else {
                // If arity is correct, execute the provided expansion logic.
                $expansion
            }
        } else {
            Err(SutraError {
                kind: SutraErrorKind::Macro(format!(
                    "Macro '{}' can only be applied to a list.",
                    $macro_name
                )),
                span: Some($expr.span()),
            })
        }
    }};
}

// ---
// Standard Macros
// ---

/// Helper for binary predicate macros like `is?`, `over?`, etc.
fn create_binary_predicate_macro(
    expr: &Expr,
    macro_name: &str,
    atom_name: &str,
) -> Result<Expr, SutraError> {
    define_macro_helper!(expr, macro_name, 2, |items, span| {
        let atom_symbol = Expr::Symbol(atom_name.to_string(), items[0].span());
        let arg1 = wrap_in_get(&items[1]);
        let arg2 = wrap_in_get(&items[2]);
        Ok(Expr::List(vec![atom_symbol, arg1, arg2], span.clone()))
    })
}

/// Expands `(is? a b)` to `(eq? (core/get a) (core/get b))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_is;
/// let expr = Expr::List(vec![
///     Expr::Symbol("is?".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
///     Expr::Symbol("bar".to_string(), Span::default()),
/// ], Span::default());
/// let expanded = expand_is(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_is(expr: &Expr) -> Result<Expr, SutraError> {
    create_binary_predicate_macro(expr, "is?", "eq?")
}

/// Expands `(over? a b)` to `(gt? (core/get a) (core/get b))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_over;
/// let expr = Expr::List(vec![
///     Expr::Symbol("over?".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
///     Expr::Symbol("bar".to_string(), Span::default()),
/// ], Span::default());
/// let expanded = expand_over(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_over(expr: &Expr) -> Result<Expr, SutraError> {
    create_binary_predicate_macro(expr, "over?", "gt?")
}

/// Expands `(under? a b)` to `(lt? (core/get a) (core/get b))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_under;
/// let expr = Expr::List(vec![
///     Expr::Symbol("under?".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
///     Expr::Symbol("bar".to_string(), Span::default()),
/// ], Span::default());
/// let expanded = expand_under(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_under(expr: &Expr) -> Result<Expr, SutraError> {
    create_binary_predicate_macro(expr, "under?", "lt?")
}

/// Helper for assignment macros like `add!`, `sub!`, etc.
fn create_assignment_macro(
    expr: &Expr,
    macro_name: &str,
    op_symbol: &str,
) -> Result<Expr, SutraError> {
    define_macro_helper!(expr, macro_name, 2, |items, span| {
        let set_symbol = Expr::Symbol("core/set!".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = Expr::Path(expr_to_path(path_arg)?, path_arg.span());
        let value_arg = items[2].clone();

        let atom_symbol = Expr::Symbol(op_symbol.to_string(), items[0].span());
        let inner_expr = Expr::List(
            vec![atom_symbol, wrap_in_get(path_arg), value_arg],
            span.clone(),
        );

        Ok(Expr::List(
            vec![set_symbol, canonical_path, inner_expr],
            span.clone(),
        ))
    })
}

/// Expands `(add! foo 1)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_add;
/// let expr = Expr::List(vec![
///     Expr::Symbol("add!".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
///     Expr::Number(1.0, Span::default()),
/// ], Span::default());
/// let expanded = expand_add(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_add(expr: &Expr) -> Result<Expr, SutraError> {
    create_assignment_macro(expr, "add!", "+")
}

/// Expands `(sub! foo 1)` to `(core/set! (path foo) (- (core/get foo) 1))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_sub;
/// let expr = Expr::List(vec![
///     Expr::Symbol("sub!".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
///     Expr::Number(1.0, Span::default()),
/// ], Span::default());
/// let expanded = expand_sub(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_sub(expr: &Expr) -> Result<Expr, SutraError> {
    create_assignment_macro(expr, "sub!", "-")
}

/// Expands `(inc! foo)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_inc;
/// let expr = Expr::List(vec![
///     Expr::Symbol("inc!".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
/// ], Span::default());
/// let expanded = expand_inc(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_inc(expr: &Expr) -> Result<Expr, SutraError> {
    define_macro_helper!(expr, "inc!", 1, |items, span| {
        let set_symbol = Expr::Symbol("core/set!".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = Expr::Path(expr_to_path(path_arg)?, path_arg.span());
        let add_symbol = Expr::Symbol("+".to_string(), items[0].span());
        let one = Expr::Number(1.0, items[0].span());
        let inner_expr = Expr::List(vec![add_symbol, wrap_in_get(path_arg), one], span.clone());
        Ok(Expr::List(
            vec![set_symbol, canonical_path, inner_expr],
            span.clone(),
        ))
    })
}

/// Expands `(dec! foo)` to `(core/set! (path foo) (- (core/get foo) 1))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_dec;
/// let expr = Expr::List(vec![
///     Expr::Symbol("dec!".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
/// ], Span::default());
/// let expanded = expand_dec(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_dec(expr: &Expr) -> Result<Expr, SutraError> {
    define_macro_helper!(expr, "dec!", 1, |items, span| {
        let set_symbol = Expr::Symbol("core/set!".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = Expr::Path(expr_to_path(path_arg)?, path_arg.span());
        let sub_symbol = Expr::Symbol("-".to_string(), items[0].span());
        let one = Expr::Number(1.0, items[0].span());
        let inner_expr = Expr::List(vec![sub_symbol, wrap_in_get(path_arg), one], span.clone());
        Ok(Expr::List(
            vec![set_symbol, canonical_path, inner_expr],
            span.clone(),
        ))
    })
}

/// Expands `(set! foo 42)` to `(core/set! (path foo) 42)`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_set;
/// let expr = Expr::List(vec![
///     Expr::Symbol("set!".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
///     Expr::Number(42.0, Span::default()),
/// ], Span::default());
/// let expanded = expand_set(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_set(expr: &Expr) -> Result<Expr, SutraError> {
    define_macro_helper!(expr, "set!", 2, |items, span| {
        let atom_symbol = Expr::Symbol("core/set!".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = Expr::Path(expr_to_path(path_arg)?, path_arg.span());
        let value_arg = items[2].clone();
        Ok(Expr::List(
            vec![atom_symbol, canonical_path, value_arg],
            span.clone(),
        ))
    })
}

/// Expands `(get foo)` to `(core/get (path foo))`.
///
/// # Examples
///
/// ```rust
/// use sutra::ast::{Expr, Span};
/// use sutra::macros_std::expand_get;
/// let expr = Expr::List(vec![
///     Expr::Symbol("get".to_string(), Span::default()),
///     Expr::Symbol("foo".to_string(), Span::default()),
/// ], Span::default());
/// let expanded = expand_get(&expr).unwrap();
/// assert!(matches!(expanded, Expr::List(_, _)));
/// ```
pub fn expand_get(expr: &Expr) -> Result<Expr, SutraError> {
    define_macro_helper!(expr, "get", 1, |items, span| {
        let atom_symbol = Expr::Symbol("core/get".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = Expr::Path(expr_to_path(path_arg)?, path_arg.span());
        Ok(Expr::List(vec![atom_symbol, canonical_path], span.clone()))
    })
}

/// Expands `(del! <path>)` to `(core/del! (path <...>))`.
pub fn expand_del(expr: &Expr) -> Result<Expr, SutraError> {
    define_macro_helper!(expr, "del!", 1, |items, span| {
        let atom_symbol = Expr::Symbol("core/del!".to_string(), items[0].span());
        let path_arg = &items[1];
        let canonical_path = Expr::Path(expr_to_path(path_arg)?, path_arg.span());
        Ok(Expr::List(vec![atom_symbol, canonical_path], span.clone()))
    })
}

// ---
// Conditional Macros
// ---

/// Expands `(if <cond> <then> <else>)` to a canonical `Expr::If` node.
pub fn expand_if(expr: &Expr) -> Result<Expr, SutraError> {
    define_macro_helper!(expr, "if", 3, |items, span| {
        Ok(Expr::If {
            condition: Box::new(items[1].clone()),
            then_branch: Box::new(items[2].clone()),
            else_branch: Box::new(items[3].clone()),
            span: span.clone(),
        })
    })
}
