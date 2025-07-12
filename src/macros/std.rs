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

use crate::ast::{AstNode, Expr, WithSpan};
use crate::macros::MacroRegistry;
use crate::runtime::path::Path;
use crate::syntax::error::{validation_error, SutraError};

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
                    _ => Err(validation_error(
                        "Path elements must be symbols or strings",
                        Some(item.span.clone()),
                    )),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Path(parts))
        }
        _ => Err(validation_error(
            "Expression cannot be converted to a path",
            Some(expr.span.clone()),
        )),
    }
}

/// Wraps an expression in a `(get ...)` call.
fn wrap_in_get(expr: &AstNode) -> AstNode {
    let get_symbol = create_symbol("get", &expr.span);
    let path_expr = expr.clone();
    WithSpan {
        value: Expr::List(vec![get_symbol, path_expr], expr.span.clone()).into(),
        span: expr.span.clone(),
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
fn expect_args<'a>(
    n: usize,
    expr: &'a AstNode,
) -> Result<(&'a [AstNode], &'a crate::ast::Span), SutraError> {
    match &*expr.value {
        Expr::List(items, span) if items.len() == n => Ok((items, span)),
        Expr::List(items, span) => Err(validation_error(
            format!("Expected {} arguments, but got {}", n, items.len()),
            Some(span.clone()),
        )),
        _ => Err(validation_error(
            "Expected a list form for this macro",
            Some(expr.span.clone()),
        )),
    }
}

// -----------------------------------------------
// AST Construction Helpers
// -----------------------------------------------

/// Creates a `AstNode` containing a symbol with the given name and span.
fn create_symbol(name: &str, span: &crate::ast::Span) -> AstNode {
    WithSpan {
        value: Expr::Symbol(name.to_string(), span.clone()).into(),
        span: span.clone(),
    }
}

/// Creates a `AstNode` containing a number literal with the given value and span.
fn create_number(value: f64, span: &crate::ast::Span) -> AstNode {
    WithSpan {
        value: Expr::Number(value, span.clone()).into(),
        span: span.clone(),
    }
}

/// Converts a path argument to a canonical `Expr::Path` node.
fn create_canonical_path(path_arg: &AstNode) -> Result<AstNode, SutraError> {
    Ok(WithSpan {
        value: Expr::Path(expr_to_path(path_arg)?, path_arg.span.clone()).into(),
        span: path_arg.span.clone(),
    })
}

// -----------------------------------------------
// Macro Pattern Generators
// -----------------------------------------------

/// Helper for unary core path operations like `get`, `del!`, `exists?`.
fn create_unary_op(expr: &AstNode, op_name: &str) -> Result<AstNode, SutraError> {
    let (items, span) = expect_args(2, expr)?;
    let atom_symbol = create_symbol(op_name, span);
    let canonical_path = create_canonical_path(&items[1])?;
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, canonical_path], span.clone()).into(),
        span: span.clone(),
    })
}

/// Helper for binary core path operations like `set!`.
fn create_binary_op(expr: &AstNode, op_name: &str) -> Result<AstNode, SutraError> {
    let (items, span) = expect_args(3, expr)?;
    let atom_symbol = create_symbol(op_name, span);
    let canonical_path = create_canonical_path(&items[1])?;
    let value_arg = items[2].clone();
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, canonical_path, value_arg], span.clone()).into(),
        span: span.clone(),
    })
}

/// Helper for assignment macros like `add!`, `sub!`, etc.
fn create_assignment_macro(expr: &AstNode, op_symbol: &str) -> Result<AstNode, SutraError> {
    let (items, span) = expect_args(3, expr)?;
    let set_symbol = create_symbol("core/set!", &items[0].span);
    let canonical_path = create_canonical_path(&items[1])?;
    let value_arg = items[2].clone();
    let atom_symbol = create_symbol(op_symbol, &items[0].span);
    let inner_expr = WithSpan {
        value: Expr::List(
            vec![atom_symbol, wrap_in_get(&items[1]), value_arg],
            span.clone(),
        )
        .into(),
        span: span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![set_symbol, canonical_path, inner_expr], span.clone()).into(),
        span: span.clone(),
    })
}

/// Helper for unary increment/decrement macros like `inc!`, `dec!`.
fn create_unary_assignment_macro(expr: &AstNode, op_symbol: &str) -> Result<AstNode, SutraError> {
    let (items, span) = expect_args(2, expr)?;
    let set_symbol = create_symbol("core/set!", &items[0].span);
    let canonical_path = create_canonical_path(&items[1])?;
    let op_symbol_expr = create_symbol(op_symbol, &items[0].span);
    let one = create_number(1.0, &items[0].span);
    let inner_expr = WithSpan {
        value: Expr::List(
            vec![op_symbol_expr, wrap_in_get(&items[1]), one],
            span.clone(),
        )
        .into(),
        span: span.clone(),
    };
    Ok(WithSpan {
        value: Expr::List(vec![set_symbol, canonical_path, inner_expr], span.clone()).into(),
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

// --- LOGICAL ---
pub fn expand_is(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_binary_op(expr, "core/is?")
}
pub fn expand_over(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_unary_op(expr, "core/over?")
}
pub fn expand_under(expr: &AstNode) -> Result<AstNode, SutraError> {
    create_unary_op(expr, "core/under?")
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
            span: span.clone(),
        }
        .into(),
        span: span.clone(),
    })
}

// -----------------------------------------------
// I/O Operations
// -----------------------------------------------

/// Expands `(print x)` to `(core/print x)`.
pub fn expand_print(expr: &AstNode) -> Result<AstNode, SutraError> {
    let (items, span) = expect_args(2, expr)?;
    let atom_symbol = create_symbol("core/print", span); // FIX: no exclamation mark
    Ok(WithSpan {
        value: Expr::List(vec![atom_symbol, items[1].clone()], span.clone()).into(),
        span: span.clone(),
    })
}
