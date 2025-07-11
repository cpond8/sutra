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

use crate::ast::{Expr, WithSpan};
use crate::macros::MacroRegistry;
use crate::runtime::path::Path;
use crate::syntax::error::validation_error;
use crate::syntax::error::SutraError;

// ===================================================================================================
// REGISTRY: Standard Macro Registration
// ===================================================================================================

/// Registers all standard macros in the given registry.
///
/// Return values are ignored since these are built-in macros that shouldn't conflict.
#[allow(unused_must_use)]
pub fn register_std_macros(registry: &mut MacroRegistry) {
    // Core path operations (alphabetical)
    registry.register("del!", expand_del);
    registry.register("exists?", expand_exists);
    registry.register("get", expand_get);
    registry.register("set!", expand_set);

    // Control flow
    registry.register("if", expand_if);

    // Predicates (building on core/get - alphabetical)
    registry.register("is?", expand_is);
    registry.register("over?", expand_over);
    registry.register("under?", expand_under);

    // Compound assignments (building on core/get and core/set! - alphabetical)
    registry.register("add!", expand_add);
    registry.register("dec!", expand_dec);
    registry.register("inc!", expand_inc);
    registry.register("sub!", expand_sub);

    // I/O utilities
    registry.register("print", expand_print);

    // Standard macros like cond are now loaded from macros.sutra at startup.
}

// ===================================================================================================
// PATH CANONICALIZATION: The Single Source of Truth
// ===================================================================================================

/// Converts a user-facing expression (`Symbol` or `List`) into a canonical `Path`.
/// This is the only function in the engine that understands path syntax.
fn expr_to_path(expr: &WithSpan<Expr>) -> Result<Path, SutraError> {
    match &expr.value {
        // Dotted symbol syntax: `player.score`
        Expr::Symbol(s, _) => Ok(Path(s.split('.').map(String::from).collect())),
        // List syntax: `(player score)`
        Expr::List(items, _) => {
            let segments: Result<Vec<_>, _> = items
                .iter()
                .map(|item| match &item.value {
                    Expr::Symbol(s, _) | Expr::String(s, _) => Ok(s.clone()),
                    _ => Err(validation_error(
                        "Path lists can only contain symbols or strings.",
                        Some(item.span.clone()),
                    )),
                })
                .collect();
            Ok(Path(segments?))
        }
        _ => Err(validation_error(
            "Invalid path format: expected a symbol or a list.",
            Some(expr.span.clone()),
        )),
    }
}

/// A helper to wrap a path-like expression in a `(core/get ...)` call.
/// If the expression is a valid path, it's converted to an `Expr::Path` and
/// wrapped in `(core/get ...)`. Otherwise, it's returned as-is.
fn wrap_in_get(expr: &WithSpan<Expr>) -> WithSpan<Expr> {
    if let Ok(path) = expr_to_path(expr) {
        let get_symbol = WithSpan {
            value: Expr::Symbol("core/get".to_string(), expr.span.clone()),
            span: expr.span.clone(),
        };
        let path_expr = WithSpan {
            value: Expr::Path(path, expr.span.clone()),
            span: expr.span.clone(),
        };
        WithSpan {
            value: Expr::List(vec![get_symbol, path_expr], expr.span.clone()),
            span: expr.span.clone(),
        }
    } else {
        expr.clone()
    }
}

// ===================================================================================================
// INTERNAL HELPERS: Macro Construction Utilities
// ===================================================================================================

// -----------------------------------------------
// Validation Helpers
// -----------------------------------------------

/// Validates that the given expression is a list with the expected number of arguments.
/// Returns the items and span if valid, or a SutraError otherwise.
fn expect_list_with_n_args<'a>(
    expr: &'a WithSpan<Expr>,
    n: usize,
    macro_name: &str,
) -> Result<(&'a [WithSpan<Expr>], &'a crate::ast::Span), SutraError> {
    match &expr.value {
        Expr::List(items, span) if items.len() == n => Ok((items, span)),
        Expr::List(items, span) => Err(validation_error(
            format!(
                "Macro '{}' expects {} arguments, but got {}.",
                macro_name,
                n - 1,
                items.len() - 1
            ),
            Some(span.clone()),
        )),
        _ => Err(validation_error(
            format!("Macro '{}' can only be applied to a list.", macro_name),
            Some(expr.span.clone()),
        )),
    }
}

// -----------------------------------------------
// AST Construction Helpers
// -----------------------------------------------

/// Creates a `WithSpan<Expr>` containing a symbol with the given name and span.
fn create_symbol(name: &str, span: &crate::ast::Span) -> WithSpan<Expr> {
    WithSpan {
        value: Expr::Symbol(name.to_string(), span.clone()),
        span: span.clone(),
    }
}

/// Creates a `WithSpan<Expr>` containing a number literal with the given value and span.
fn create_number(value: f64, span: &crate::ast::Span) -> WithSpan<Expr> {
    WithSpan {
        value: Expr::Number(value, span.clone()),
        span: span.clone(),
    }
}

/// Converts a path argument to a canonical `Expr::Path` node.
fn create_canonical_path(path_arg: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    Ok(WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()),
        span: path_arg.span.clone(),
    })
}

// -----------------------------------------------
// Macro Pattern Generators
// -----------------------------------------------

/// Helper for unary core path operations like `get`, `del!`, `exists?`.
fn create_unary_core_macro(
    expr: &WithSpan<Expr>,
    macro_name: &str,
    core_name: &str,
) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 2, macro_name)?;
    let atom_symbol = create_symbol(core_name, &items[0].span);
    let canonical_path = create_canonical_path(&items[1])?;
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, canonical_path], span.clone()),
        span: span.clone(),
    })
}

/// Helper for binary core path operations like `set!`.
fn create_binary_core_macro(
    expr: &WithSpan<Expr>,
    macro_name: &str,
    core_name: &str,
) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 3, macro_name)?;
    let atom_symbol = create_symbol(core_name, &items[0].span);
    let canonical_path = create_canonical_path(&items[1])?;
    let value_arg = items[2].clone();
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, canonical_path, value_arg], span.clone()),
        span: span.clone(),
    })
}

/// Helper for binary predicate macros like `is?`, `over?`, etc.
fn create_binary_predicate_macro(
    expr: &WithSpan<Expr>,
    macro_name: &str,
    atom_name: &str,
) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 3, macro_name)?;
    let atom_symbol = create_symbol(atom_name, &items[0].span);
    let arg1 = wrap_in_get(&items[1]);
    let arg2 = wrap_in_get(&items[2]);
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, arg1, arg2], span.clone()),
        span: span.clone(),
    })
}

/// Helper for assignment macros like `add!`, `sub!`, etc.
fn create_assignment_macro(
    expr: &WithSpan<Expr>,
    macro_name: &str,
    op_symbol: &str,
) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 3, macro_name)?;
    let set_symbol = create_symbol("core/set!", &items[0].span);
    let canonical_path = create_canonical_path(&items[1])?;
    let value_arg = items[2].clone();
    let atom_symbol = create_symbol(op_symbol, &items[0].span);
    let inner_expr = WithSpan {
        value: Expr::List(
            vec![atom_symbol, wrap_in_get(&items[1]), value_arg],
            span.clone(),
        ),
        span: span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![set_symbol, canonical_path, inner_expr], span.clone()),
        span: span.clone(),
    })
}

/// Helper for unary increment/decrement macros like `inc!`, `dec!`.
fn create_unary_assignment_macro(
    expr: &WithSpan<Expr>,
    macro_name: &str,
    op_symbol: &str,
) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 2, macro_name)?;
    let set_symbol = create_symbol("core/set!", &items[0].span);
    let canonical_path = create_canonical_path(&items[1])?;
    let op_symbol_expr = create_symbol(op_symbol, &items[0].span);
    let one = create_number(1.0, &items[0].span);
    let inner_expr = WithSpan {
        value: Expr::List(vec![op_symbol_expr, wrap_in_get(&items[1]), one], span.clone()),
        span: span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![set_symbol, canonical_path, inner_expr], span.clone()),
        span: span.clone(),
    })
}

// ===================================================================================================
// PUBLIC API: Standard Macro Implementations
// ===================================================================================================

// -----------------------------------------------
// Core Path Operations
// -----------------------------------------------

/// Expands `(set! foo bar)` to `(core/set! (path foo) bar)`.
pub fn expand_set(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_core_macro(expr, "set!", "core/set!")
}

/// Expands `(get foo)` to `(core/get (path foo))`.
pub fn expand_get(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_unary_core_macro(expr, "get", "core/get")
}

/// Expands `(del! foo)` to `(core/del! (path foo))`.
pub fn expand_del(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_unary_core_macro(expr, "del!", "core/del!")
}

/// Expands `(exists? foo)` to `(core/exists? (path foo))`.
pub fn expand_exists(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_unary_core_macro(expr, "exists?", "core/exists?")
}

// -----------------------------------------------
// Predicate Operations
// -----------------------------------------------

/// Expands `(is? a b)` to `(eq? (core/get a) (core/get b))`.
pub fn expand_is(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "is?", "eq?")
}

/// Expands `(over? a b)` to `(gt? (core/get a) (core/get b))`.
pub fn expand_over(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "over?", "gt?")
}

/// Expands `(under? a b)` to `(lt? (core/get a) (core/get b))`.
pub fn expand_under(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_binary_predicate_macro(expr, "under?", "lt?")
}

// -----------------------------------------------
// Assignment Operations
// -----------------------------------------------

/// Expands `(add! foo 1)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
pub fn expand_add(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_assignment_macro(expr, "add!", "+")
}

/// Expands `(sub! foo 1)` to `(core/set! (path foo) (- (core/get foo) 1))`.
pub fn expand_sub(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_assignment_macro(expr, "sub!", "-")
}

/// Expands `(inc! foo)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
pub fn expand_inc(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_unary_assignment_macro(expr, "inc!", "+")
}

/// Expands `(dec! foo)` to `(core/set! (path foo) (- (core/get foo) 1))`.
pub fn expand_dec(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    create_unary_assignment_macro(expr, "dec!", "-")
}

// -----------------------------------------------
// Control Flow
// -----------------------------------------------

/// Expands `(if cond then else)` to a canonical conditional form.
///
/// # Example
/// ```
/// (if (eq? x 1) (print "yes") (print "no"))
/// ```
pub fn expand_if(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 4, "if")?;
    Ok(WithSpan {
        value: Expr::If {
            condition: Box::new(items[1].clone()),
            then_branch: Box::new(items[2].clone()),
            else_branch: Box::new(items[3].clone()),
            span: span.clone(),
        },
        span: span.clone(),
    })
}

// -----------------------------------------------
// I/O Operations
// -----------------------------------------------

/// Expands `(print x)` to `(core/print x)`.
pub fn expand_print(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let (items, span) = expect_list_with_n_args(expr, 2, "print")?;
    let atom_symbol = create_symbol("core/print", &items[0].span);
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, items[1].clone()], span.clone()),
        span: span.clone(),
    })
}
