//! World state management atoms - simplified and unified
//!
//! This module provides atoms for reading and modifying the persistent world state.
//! The world state is a hierarchical key-value store accessible via dot-notation paths.
//!
//! ## Atoms Provided
//!
//! - **State Operations**: `set!`, `get`, `del!`, `path`
//! - **State Queries**: `exists?`
//! - **Arithmetic Updates**: `inc!`, `dec!`, `add!`, `sub!`
//!
//! ## Design Notes
//!
//! All operations use `Path` objects for addressing state locations.
//! Missing values return `nil` rather than errors for graceful handling.

use crate::{
    errors::{to_source_span, ErrorReporting},
    prelude::Path,
    runtime::{evaluate_ast_node, NativeFn, SpannedValue, Value},
    syntax::{AstNode, Expr},
};

// ============================================================================
// CORE UTILITIES
// ============================================================================

/// Convert AST node to Path, handling only the patterns actually used
fn resolve_path(
    node: &AstNode,
    context: &mut crate::runtime::EvaluationContext,
) -> Result<Path, crate::errors::SutraError> {
    match &*node.value {
        Expr::Symbol(s, _) if s.contains('.') => Ok(Path(s.split('.').map(String::from).collect())),
        Expr::Symbol(s, _) => Ok(Path(vec![s.clone()])),
        Expr::Path(p, _) => Ok(p.clone()),
        _ => Err(context.type_mismatch(
            "Path expression",
            node.value.type_name(),
            to_source_span(node.span),
        )),
    }
}

/// Unified arithmetic operations on world state
#[derive(Clone, Copy)]
enum ArithmeticOp {
    Add(f64),
    Subtract(f64),
    Increment,
    Decrement,
}

fn apply_arithmetic(
    path: &Path,
    op: ArithmeticOp,
    context: &mut crate::runtime::EvaluationContext,
    call_span: crate::Span,
) -> Result<SpannedValue, crate::errors::SutraError> {
    let current = context
        .world
        .borrow()
        .get(path)
        .cloned()
        .unwrap_or_default();

    let new_value = match (current, op) {
        (Value::Number(n), ArithmeticOp::Add(x)) => Value::Number(n + x),
        (Value::Number(n), ArithmeticOp::Subtract(x)) => Value::Number(n - x),
        (Value::Number(n), ArithmeticOp::Increment) => Value::Number(n + 1.0),
        (Value::Number(n), ArithmeticOp::Decrement) => Value::Number(n - 1.0),
        (other, _) => {
            return Err(context.type_mismatch(
                "Number",
                other.type_name(),
                to_source_span(call_span),
            ))
        }
    };

    context.world.borrow_mut().set(path, new_value.clone());
    Ok(SpannedValue {
        value: new_value,
        span: call_span,
    })
}

// ============================================================================
// WORLD STATE ATOMS
// ============================================================================

/// Sets a value at a path in the world state.
/// `(set! <path> <value>)`
pub const ATOM_SET: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let path = resolve_path(&args[0], context)?;
    let value = evaluate_ast_node(&args[1], context)?.value;
    context.world.borrow_mut().set(&path, value);

    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};

/// Gets a value at a path in the world state.
/// `(get <path>)`
pub const ATOM_GET: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path = resolve_path(&args[0], context)?;
    let value = context
        .world
        .borrow()
        .get(&path)
        .cloned()
        .unwrap_or_default();

    Ok(SpannedValue {
        value,
        span: *call_span,
    })
};

/// Deletes a value at a path in the world state.
/// `(del! <path>)`
pub const ATOM_DEL: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path = resolve_path(&args[0], context)?;
    context.world.borrow_mut().del(&path);

    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};

/// Returns true if a path exists in the world state.
/// `(exists? <path>)`
pub const ATOM_EXISTS: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path = resolve_path(&args[0], context)?;
    let exists = context.world.borrow().get(&path).is_some();

    Ok(SpannedValue {
        value: Value::Bool(exists),
        span: *call_span,
    })
};

/// Increments a numeric value at a path.
/// `(inc! <path>)`
pub const ATOM_INC: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path = resolve_path(&args[0], context)?;
    apply_arithmetic(&path, ArithmeticOp::Increment, context, *call_span)
};

/// Decrements a numeric value at a path.
/// `(dec! <path>)`
pub const ATOM_DEC: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path = resolve_path(&args[0], context)?;
    apply_arithmetic(&path, ArithmeticOp::Decrement, context, *call_span)
};

/// Adds a value to a numeric value at a path.
/// `(add! <path> <value>)`
pub const ATOM_ADD: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let path = resolve_path(&args[0], context)?;
    let operand = evaluate_ast_node(&args[1], context)?;

    match operand.value {
        Value::Number(n) => apply_arithmetic(&path, ArithmeticOp::Add(n), context, *call_span),
        _ => Err(context.type_mismatch(
            "Number",
            operand.value.type_name(),
            to_source_span(operand.span),
        )),
    }
};

/// Subtracts a value from a numeric value at a path.
/// `(sub! <path> <value>)`
pub const ATOM_SUB: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let path = resolve_path(&args[0], context)?;
    let operand = evaluate_ast_node(&args[1], context)?;

    match operand.value {
        Value::Number(n) => apply_arithmetic(&path, ArithmeticOp::Subtract(n), context, *call_span),
        _ => Err(context.type_mismatch(
            "Number",
            operand.value.type_name(),
            to_source_span(operand.span),
        )),
    }
};

/// Creates a path from multiple strings.
/// `(path <string> <string> ...)`
pub const ATOM_PATH: NativeFn = |args, context, call_span| {
    if args.is_empty() {
        return Err(context.arity_mismatch("1+", args.len(), to_source_span(*call_span)));
    }

    let mut parts = Vec::new();
    for arg in args {
        let value = evaluate_ast_node(arg, context)?;
        match value.value {
            Value::String(s) => parts.push(s),
            _ => {
                return Err(context.type_mismatch(
                    "String",
                    value.value.type_name(),
                    to_source_span(value.span),
                ))
            }
        }
    }

    Ok(SpannedValue {
        value: Value::Path(Path(parts)),
        span: *call_span,
    })
};
