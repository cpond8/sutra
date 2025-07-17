//!
//! This module is the sole authority on path canonicalization and provides
//! the core, author-facing macros that expand into simpler, canonical ASTs.
//!
//! ## Core Responsibility: Path Canonicalization
//!
//! The primary role of this module is to convert user-friendly path syntax
//! (e.g., `player.score` or `(player score)`) into a canonical `Expr::Path`
//! node. This is the only place in the entire engine where path syntax is parsed.

use crate::ast::{AstNode, Expr, WithSpan};
use crate::macros::MacroRegistry;
use crate::runtime::path::Path;
use crate::SutraError;
use crate::err_msg;

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
fn expr_to_path(expr: &AstNode) -> Result<Path, SutraError> {
    // Match on the inner expression by dereferencing the Arc
    match &*expr.value {
        // Dotted symbol syntax: `player.score`
        Expr::Symbol(s, _) => Ok(Path(s.split('.').map(String::from).collect())),
        // List syntax: `(path player score)`
        Expr::List(items, _) => {
            let parts = items
                .iter()
                .map(|item| match &*item.value {
                    Expr::Symbol(s, _) | Expr::String(s, _) => Ok(s.clone()),
                    _ => Err(err_msg!(Validation, "Path elements must be symbols or strings")),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Path(parts))
        }
        _ => Err(err_msg!(Validation, "Expression cannot be converted to a path")),
    }
}

/// Wraps an expression in a `(get ...)` call.
fn wrap_in_get(expr: &AstNode) -> AstNode {
    let get_symbol = create_symbol("get", &expr.span);
    let path_expr = expr.clone();
    WithSpan {
        value: Expr::List(vec![get_symbol, path_expr], expr.span).into(),
        span: expr.span,
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
fn expect_args(
    n: usize,
    expr: &AstNode,
) -> Result<(&[AstNode], &crate::ast::Span), SutraError> {
    match &*expr.value {
        Expr::List(items, span) if items.len() == n => Ok((items, span)),
        Expr::List(items, _span) => {
            let msg = format!("Expected {} arguments, but got {}", n, items.len());
            Err(err_msg!(Validation, msg))
        },
        _ => Err(err_msg!(Validation, "Expected a list form for this macro")),
    }
}

// -----------------------------------------------
// AST Construction Helpers
// -----------------------------------------------

/// Creates a `AstNode` containing a symbol with the given name and span.
fn create_symbol(name: &str, span: &crate::ast::Span) -> AstNode {
    WithSpan {
        value: Expr::Symbol(name.to_string(), *span).into(),
        span: *span,
    }
}

/// Creates a `AstNode` containing a number literal with the given value and span.
fn create_number(value: f64, span: &crate::ast::Span) -> AstNode {
    WithSpan {
        value: Expr::Number(value, *span).into(),
        span: *span,
    }
}

/// Converts a path argument to a canonical `Expr::Path` node.
fn create_canonical_path(path_arg: &AstNode) -> Result<AstNode, SutraError> {
    Ok(WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span).into(),
        span: path_arg.span,
    })
}

// -----------------------------------------------
// Macro Pattern Generators
// -----------------------------------------------

/// Flexible helper for path operations that lets atoms handle arity validation.
/// Requires at least min_args total arguments (including macro name).
/// Converts the first argument to a canonical path if present.
fn create_flexible_path_op(expr: &AstNode, op_name: &str, min_args: usize) -> Result<AstNode, SutraError> {
    match &*expr.value {
        Expr::List(items, span) if items.len() >= min_args => {
            let atom_symbol = create_symbol(op_name, span);
            let mut new_items = vec![atom_symbol];

            // Convert first argument to canonical path if present
            if items.len() > 1 {
                let canonical_path = create_canonical_path(&items[1])?;
                new_items.push(canonical_path);
                // Add any additional arguments
                if items.len() > 2 {
                    new_items.extend_from_slice(&items[2..]);
                }
            }

            Ok(WithSpan {
                value: Expr::List(new_items, *span).into(),
                span: *span,
            })
        }
        Expr::List(items, _) => {
            let expected = if min_args > 1 { min_args - 1 } else { 0 };
            let got = if items.len() > 0 { items.len() - 1 } else { 0 };
            let msg = format!("{} requires at least {} argument(s), got {}", op_name, expected, got);
            Err(err_msg!(Validation, msg))
        }
        _ => Err(err_msg!(Validation, "Expected a list form for this macro")),
    }
}

/// Helper for unary core path operations like `get`, `del!`, `exists?`.
fn create_unary_op(expr: &AstNode, op_name: &str) -> Result<AstNode, SutraError> {
    create_flexible_path_op(expr, op_name, 1) // Allow 0+ arguments, let atom validate
}

/// Helper for binary core path operations like `set!`.
fn create_binary_op(expr: &AstNode, op_name: &str) -> Result<AstNode, SutraError> {
    create_flexible_path_op(expr, op_name, 2) // Allow 1+ arguments, let atom validate
}
/// Flexible helper for assignment macros like `add!`, `sub!`, etc.
fn create_assignment_macro(expr: &AstNode, op_symbol: &str) -> Result<AstNode, SutraError> {
    match &*expr.value {
        Expr::List(items, span) if items.len() >= 3 => {
            let set_symbol = create_symbol("core/set!", &items[0].span);
            let canonical_path = create_canonical_path(&items[1])?;
            let value_arg = items[2].clone();
            let atom_symbol = create_symbol(op_symbol, &items[0].span);
            let inner_expr = WithSpan {
                value: Expr::List(
                    vec![atom_symbol, wrap_in_get(&items[1]), value_arg],
                    *span,
                )
                .into(),
                span: *span,
            };
            Ok(WithSpan {
                value: Expr::List(vec![set_symbol, canonical_path, inner_expr], *span).into(),
                span: *span,
            })
        }
        Expr::List(items, _) => {
            let got = if items.len() > 0 { items.len() - 1 } else { 0 };
            let msg = format!("{} requires 2 arguments (path and value), got {}", op_symbol, got);
            Err(err_msg!(Validation, msg))
        }
        _ => Err(err_msg!(Validation, "Expected a list form for this macro")),
    }
}

/// Flexible helper for unary increment/decrement macros like `inc!`, `dec!`.
fn create_unary_assignment_macro(expr: &AstNode, op_symbol: &str) -> Result<AstNode, SutraError> {
    match &*expr.value {
        Expr::List(items, span) if items.len() >= 2 => {
            let set_symbol = create_symbol("core/set!", &items[0].span);
            let canonical_path = create_canonical_path(&items[1])?;
            let op_symbol_expr = create_symbol(op_symbol, &items[0].span);
            let one = create_number(1.0, &items[0].span);
            let inner_expr = WithSpan {
                value: Expr::List(
                    vec![op_symbol_expr, wrap_in_get(&items[1]), one],
                    *span,
                )
                .into(),
                span: *span,
            };
            Ok(WithSpan {
                value: Expr::List(vec![set_symbol, canonical_path, inner_expr], *span).into(),
                span: *span,
            })
        }
        Expr::List(items, _) => {
            let got = if items.len() > 0 { items.len() - 1 } else { 0 };
            let msg = format!("{} requires 1 argument (path), got {}", op_symbol, got);
            Err(err_msg!(Validation, msg))
        }
        _ => Err(err_msg!(Validation, "Expected a list form for this macro")),
    }
}

// ===================================================================================================
// PUBLIC API: Standard Macro Implementations
// ===================================================================================================

// -----------------------------------------------
// Core Path Operations
// -----------------------------------------------

/// Expands `(set! foo bar)` to `(core/set! (path foo) bar)`.
pub fn expand_set(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_binary_op(expr, "core/set!")
}

/// Expands `(get foo)` to `(core/get (path foo))`.
pub fn expand_get(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_unary_op(expr, "core/get")
}

/// Expands `(del! foo)` to `(core/del! (path foo))`.
pub fn expand_del(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_unary_op(expr, "core/del!")
}

/// Expands `(exists? foo)` to `(core/exists? (path foo))`.
pub fn expand_exists(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_unary_op(expr, "core/exists?")
}

// --- ARITHMETIC ---
pub fn expand_add(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_assignment_macro(expr, "add!")
}

/// Expands `(sub! foo 1)` to `(core/set! (path foo) (- (core/get foo) 1))`.
pub fn expand_sub(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_assignment_macro(expr, "sub!")
}

/// Expands `(inc! foo)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
pub fn expand_inc(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_unary_assignment_macro(expr, "inc!")
}

/// Expands `(dec! foo)` to `(core/set! (path foo) (- (core/get foo) 1))`.
pub fn expand_dec(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_unary_assignment_macro(expr, "dec!")
}

// -----------------------------------------------
// Control Flow
// -----------------------------------------------

/// Expands `(if cond then else)` to a canonical conditional form.
///
/// (if (eq? x 1) (print "yes") (print "no"))
pub fn expand_if(expr: &AstNode) -> Result<AstNode, SutraError> {
    let (items, span) = expect_args(4, expr)?;
    Ok(WithSpan {
        value: Expr::If {
            condition: Box::new(items[1].clone()),
            then_branch: Box::new(items[2].clone()),
            else_branch: Box::new(items[3].clone()),
            span: *span,
        }
        .into(),
        span: *span,
    })
}

// -----------------------------------------------
// I/O Operations
// -----------------------------------------------

/// Expands `(print ...)` to `(core/print ...)`, letting the atom handle arity validation.
pub fn expand_print(expr: &AstNode) -> Result<AstNode, SutraError> {
    match &*expr.value {
        Expr::List(items, span) if items.len() >= 1 => {
            let atom_symbol = create_symbol("core/print", span);
            let mut new_items = vec![atom_symbol];
            // Add all arguments after the macro name
            new_items.extend_from_slice(&items[1..]);
            Ok(WithSpan {
                value: Expr::List(new_items, *span).into(),
                span: *span,
            })
        }
        _ => Err(err_msg!(Validation, "Expected a list form for this macro")),
    }
}
