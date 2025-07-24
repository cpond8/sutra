//!
//! This module is the sole authority on path canonicalization and provides
//! the core, author-facing macros that expand into simpler, canonical ASTs.
//!
//! ## Core Responsibility: Path Canonicalization
//!
//! The primary role of this module is to convert user-friendly path syntax
//! (e.g., `player.score` or `(player score)`) into a canonical `Expr::Path`
//! node. This is the only place in the entire engine where path syntax is parsed.

use crate::prelude::*;
use crate::{
    macros::MacroExpansionResult as macro_result, runtime::source::SourceContext, syntax::parser,
};

use crate::errors;

// ===================================================================================================
// SYSTEM OVERVIEW
// ===================================================================================================
//
// This module follows a clear logical flow:
// 1. Foundation: Type aliases, constants, and core utilities
// 2. Core Logic: Path canonicalization (the main responsibility)
// 3. Construction Helpers: AST building utilities
// 4. Pattern Generators: Macro expansion patterns
// 5. Public API: Macro implementations (the main interface)
// 6. Integration: Registration (how it all comes together)

// ===================================================================================================
// FOUNDATION: Type Aliases and Constants
// ===================================================================================================

/// Type alias for core operation names to ensure consistency
type CoreOpName = &'static str;

/// Core operation names for consistent usage throughout the module
const CORE_SET: CoreOpName = "core/set!";
const CORE_GET: CoreOpName = "core/get";

// ===================================================================================================
// CORE LOGIC: Path Canonicalization (The Single Source of Truth)
// ===================================================================================================

/// Converts a user-facing expression (`Symbol`, `List`, or `Path`) into a canonical `Path`.
/// This is the only function in the engine that understands path syntax.
fn expr_to_path(expr: &AstNode, source: &SourceContext) -> Result<Path, SutraError> {
    let value = &*expr.value;

    // Handle dotted symbol syntax: `player.score` or plain symbol: `player`
    if let Expr::Symbol(s, _) = value {
        return Ok(Path(s.split('.').map(String::from).collect()));
    }

    // Handle already parsed path: `player.health` (from parser)
    if let Expr::Path(path, _) = value {
        return Ok(path.clone());
    }

    // Handle list syntax: `(path player score)`
    if let Expr::List(items, _) = value {
        let mut parts = Vec::new();

        for item in items {
            let item_value = &*item.value;
            match item_value {
                Expr::Symbol(s, _) | Expr::String(s, _) => parts.push(s.clone()),
                _ => {
                    return Err(errors::runtime_general(
                        "Path elements must be symbols or strings".to_string(),
                        "path conversion",
                        source,
                        parser::to_source_span(expr.span),
                    ));
                }
            }
        }

        return Ok(Path(parts));
    }

    // Fallback for unsupported expression types
    {
        Err(errors::runtime_general(
            "Expression cannot be converted to a path".to_string(),
            "path conversion",
            source,
            parser::to_source_span(expr.span),
        ))
    }
}

/// Converts a path argument to a canonical `Expr::Path` node.
fn create_canonical_path(path_arg: &AstNode, source: &SourceContext) -> macro_result {
    Ok(Spanned {
        value: Expr::Path(expr_to_path(path_arg, source)?, path_arg.span).into(),
        span: path_arg.span,
    })
}

/// Wraps an expression in a `(core/get ...)` call with proper path conversion.
fn wrap_in_get(expr: &AstNode, source: &SourceContext) -> AstNode {
    let get_symbol = create_symbol(CORE_GET, &expr.span);
    // Convert the expression to a canonical path, but handle errors gracefully
    let path_expr = match create_canonical_path(expr, source) {
        Ok(canonical_path) => canonical_path,
        Err(_) => expr.clone(), // Fall back to original expression if path conversion fails
    };
    create_ast_list(vec![get_symbol, path_expr], expr.span)
}

// ===================================================================================================
// CONSTRUCTION HELPERS: AST Building Utilities
// ===================================================================================================

/// Creates an AST list node with consistent span handling.
/// Reduces repetitive Spanned construction patterns throughout the module.
fn create_ast_list(items: Vec<AstNode>, span: Span) -> AstNode {
    Spanned {
        value: Expr::List(items, span).into(),
        span,
    }
}

/// Creates a `AstNode` containing a symbol with the given name and span.
fn create_symbol(name: &str, span: &Span) -> AstNode {
    Spanned {
        value: Expr::Symbol(name.to_string(), *span).into(),
        span: *span,
    }
}

/// Creates a `AstNode` containing a number literal with the given value and span.
fn create_number(value: f64, span: &Span) -> AstNode {
    Spanned {
        value: Expr::Number(value, *span).into(),
        span: *span,
    }
}

/// Creates a validation error with consistent formatting for macro arity mismatches.
fn create_arity_error(op_name: &str, expected: usize, got: usize) -> SutraError {
    {
        let sc = SourceContext::from_file("macro", op_name);
        let span = parser::to_source_span(Span::default());
        errors::validation_arity(format!("at least {}", expected), got, &sc, span)
    }
}

// ===================================================================================================
// PATTERN GENERATORS: Macro Expansion Patterns
// ===================================================================================================

/// Flexible helper for path operations that lets atoms handle arity validation.
/// Requires at least min_args total arguments (including macro name).
/// Converts the first argument to a canonical path if present.
fn create_flexible_path_op(
    expr: &AstNode,
    op_name: &str,
    min_args: usize,
    source: &SourceContext,
) -> macro_result {
    let Expr::List(items, span) = &*expr.value else {
        let sc = SourceContext::from_file(op_name, format!("{:?}", expr));
        return Err(errors::runtime_general(
            "Expected list form for macro".to_string(),
            "macro expansion",
            &sc,
            parser::to_source_span(expr.span),
        ));
    };

    if items.len() < min_args {
        return Err(create_arity_error(
            op_name,
            min_args - 1,
            items.len().saturating_sub(1),
        ));
    }

    let mut new_items = vec![create_symbol(op_name, span)];
    if items.len() > 1 {
        new_items.push(create_canonical_path(&items[1], source)?);
        new_items.extend_from_slice(&items[2..]);
    }

    Ok(create_ast_list(new_items, *span))
}

/// Helper for unary core path operations like `get`, `del!`, `exists?`.
fn create_unary_op(expr: &AstNode, op_name: &str, source: &SourceContext) -> macro_result {
    create_flexible_path_op(expr, op_name, 1, source) // Allow 0+ arguments, let atom validate
}

/// Helper for binary core path operations like `set!`.
fn create_binary_op(expr: &AstNode, op_name: &str, source: &SourceContext) -> macro_result {
    create_flexible_path_op(expr, op_name, 2, source) // Allow 1+ arguments, let atom validate
}

/// Flexible helper for assignment macros like `add!`, `sub!`, etc.
fn create_assignment_macro(
    expr: &AstNode,
    op_symbol: &str,
    source: &SourceContext,
) -> macro_result {
    let Expr::List(items, span) = &*expr.value else {
        let sc = SourceContext::from_file("create_assignment_macro", format!("{:?}", expr));
        return Err(errors::runtime_general(
            "Expected list form for macro".to_string(),
            "macro expansion",
            &sc,
            parser::to_source_span(expr.span),
        ));
    };

    if items.len() < 3 {
        return Err(create_arity_error(
            op_symbol,
            2,
            items.len().saturating_sub(1),
        ));
    }

    let inner_expr = create_ast_list(
        vec![
            create_symbol(op_symbol, &items[0].span),
            wrap_in_get(&items[1], source),
            items[2].clone(),
        ],
        *span,
    );

    Ok(create_ast_list(
        vec![
            create_symbol(CORE_SET, &items[0].span),
            create_canonical_path(&items[1], source)?,
            inner_expr,
        ],
        *span,
    ))
}

/// Flexible helper for unary increment/decrement macros like `inc!`, `dec!`.
fn create_unary_assignment_macro(
    expr: &AstNode,
    op_symbol: &str,
    source: &SourceContext,
) -> macro_result {
    let Expr::List(items, span) = &*expr.value else {
        let sc = SourceContext::from_file("create_unary_assignment_macro", format!("{:?}", expr));
        return Err(errors::runtime_general(
            "Expected list form for macro".to_string(),
            "macro expansion",
            &sc,
            parser::to_source_span(expr.span),
        ));
    };

    if items.len() < 2 {
        return Err(create_arity_error(
            op_symbol,
            1,
            items.len().saturating_sub(1),
        ));
    }

    let atom_name = match op_symbol {
        "inc!" => "+",
        "dec!" => "-",
        _ => op_symbol,
    };

    let inner_expr = create_ast_list(
        vec![
            create_symbol(atom_name, &items[0].span),
            wrap_in_get(&items[1], source),
            create_number(1.0, &items[0].span),
        ],
        *span,
    );

    Ok(create_ast_list(
        vec![
            create_symbol(CORE_SET, &items[0].span),
            create_canonical_path(&items[1], source)?,
            inner_expr,
        ],
        *span,
    ))
}

// ===================================================================================================
// PUBLIC API: Standard Macro Implementations
// ===================================================================================================

// -----------------------------------------------
// Core Path Operations
// -----------------------------------------------

/// Expands `(set! foo bar)` to `(core/set! (path foo) bar)`.
pub fn expand_set(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_binary_op(expr, "core/set!", source)
}

/// Expands `(get foo)` to `(core/get (path foo))`.
pub fn expand_get(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_unary_op(expr, "core/get", source)
}

/// Expands `(del! foo)` to `(core/del! (path foo))`.
pub fn expand_del(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_unary_op(expr, "core/del!", source)
}

/// Expands `(exists? foo)` to `(core/exists? (path foo))`.
pub fn expand_exists(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_unary_op(expr, "core/exists?", source)
}

// -----------------------------------------------
// Arithmetic Assignment Operations
// -----------------------------------------------

/// Expands `(add! foo 1)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
pub fn expand_add(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_assignment_macro(expr, "add!", source)
}

/// Expands `(sub! foo 1)` to `(core/set! (path foo) (- (core/get foo) 1))`.
pub fn expand_sub(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_assignment_macro(expr, "sub!", source)
}

/// Expands `(inc! foo)` to `(core/set! (path foo) (+ (core/get foo) 1))`.
pub fn expand_inc(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_unary_assignment_macro(expr, "inc!", source)
}

/// Expands `(dec! foo)` to `(core/set! (path foo) (- (core/get foo) 1))`.
pub fn expand_dec(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_unary_assignment_macro(expr, "dec!", source)
}

// -----------------------------------------------
// I/O Operations
// -----------------------------------------------

/// Expands `(print ...)` to `(core/print ...)`, letting the atom handle arity validation.
pub fn expand_print(expr: &AstNode, source: &SourceContext) -> macro_result {
    create_unary_op(expr, "core/print", source)
}

// ===================================================================================================
// INTEGRATION: Registration and System Assembly
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

    // Control flow - if is implemented as a special form, not a macro

    // Compound assignments (building on core/get and core/set! - alphabetical)
    registry.register("add!", expand_add);
    registry.register("dec!", expand_dec);
    registry.register("inc!", expand_inc);
    registry.register("sub!", expand_sub);

    // I/O utilities
    registry.register("print", expand_print);

    // Standard macros like cond are now loaded from std_macros.sutra at startup.
}
