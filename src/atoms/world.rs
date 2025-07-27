//! World state management atoms for the Sutra language.
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
// WORLD STATE OPERATIONS
// ============================================================================

/// Helper function to canonicalize various path expressions into a Path
fn canonicalize_path_from_ast_node(
    node: &AstNode,
    context: &mut crate::runtime::EvaluationContext,
) -> Result<Path, crate::errors::SutraError> {
    match &*node.value {
        Expr::Symbol(s, _) if s.contains('.') => Ok(Path(s.split('.').map(String::from).collect())),
        Expr::Symbol(s, _) => Ok(Path(vec![s.clone()])),
        Expr::Path(p, _) => Ok(p.clone()),
        Expr::List(items, _) => {
            let mut parts = Vec::new();
            for item in items {
                match &*item.value {
                    Expr::Symbol(s, _) | Expr::String(s, _) => parts.push(s.clone()),
                    _ => {
                        return Err(context.type_mismatch(
                            "Symbol or String",
                            item.value.type_name(),
                            to_source_span(item.span),
                        ));
                    }
                }
            }
            Ok(Path(parts))
        }
        _ => Err(context.type_mismatch(
            "Path expression",
            node.value.type_name(),
            to_source_span(node.span),
        )),
    }
}

/// Sets a value at a path in the world state.
/// `(set! <path> <value>)`
pub const ATOM_SET: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let path = canonicalize_path_from_ast_node(&args[0], context)?;
    let value_sv = evaluate_ast_node(&args[1], context)?;
    context.world.borrow_mut().set(&path, value_sv.value);

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

    let path = canonicalize_path_from_ast_node(&args[0], context)?;
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

    let path = canonicalize_path_from_ast_node(&args[0], context)?;
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

    let path = canonicalize_path_from_ast_node(&args[0], context)?;
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

    let path = canonicalize_path_from_ast_node(&args[0], context)?;
    let current = context
        .world
        .borrow()
        .get(&path)
        .cloned()
        .unwrap_or_default();

    let new_value = match current {
        Value::Number(n) => Value::Number(n + 1.0),
        _ => {
            return Err(context.type_mismatch(
                "Number",
                current.type_name(),
                to_source_span(*call_span),
            ))
        }
    };

    context.world.borrow_mut().set(&path, new_value.clone());
    Ok(SpannedValue {
        value: new_value,
        span: *call_span,
    })
};

/// Decrements a numeric value at a path.
/// `(dec! <path>)`
pub const ATOM_DEC: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path = canonicalize_path_from_ast_node(&args[0], context)?;
    let current = context
        .world
        .borrow()
        .get(&path)
        .cloned()
        .unwrap_or_default();

    let new_value = match current {
        Value::Number(n) => Value::Number(n - 1.0),
        _ => {
            return Err(context.type_mismatch(
                "Number",
                current.type_name(),
                to_source_span(*call_span),
            ))
        }
    };

    context.world.borrow_mut().set(&path, new_value.clone());
    Ok(SpannedValue {
        value: new_value,
        span: *call_span,
    })
};

/// Adds a value to a numeric value at a path.
/// `(add! <path> <value>)`
pub const ATOM_ADD: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let path = canonicalize_path_from_ast_node(&args[0], context)?;
    let current = context
        .world
        .borrow()
        .get(&path)
        .cloned()
        .unwrap_or_default();
    let addend_sv = evaluate_ast_node(&args[1], context)?;

    let new_value = match (current, addend_sv.value) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a + b),
        (Value::Number(_), other) => {
            return Err(context.type_mismatch(
                "Number",
                other.type_name(),
                to_source_span(addend_sv.span),
            ))
        }
        (other, _) => {
            return Err(context.type_mismatch(
                "Number",
                other.type_name(),
                to_source_span(*call_span),
            ))
        }
    };

    context.world.borrow_mut().set(&path, new_value.clone());
    Ok(SpannedValue {
        value: new_value,
        span: *call_span,
    })
};

/// Subtracts a value from a numeric value at a path.
/// `(sub! <path> <value>)`
pub const ATOM_SUB: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let path = canonicalize_path_from_ast_node(&args[0], context)?;
    let current = context
        .world
        .borrow()
        .get(&path)
        .cloned()
        .unwrap_or_default();
    let subtrahend_sv = evaluate_ast_node(&args[1], context)?;

    let new_value = match (current, subtrahend_sv.value) {
        (Value::Number(a), Value::Number(b)) => Value::Number(a - b),
        (Value::Number(_), other) => {
            return Err(context.type_mismatch(
                "Number",
                other.type_name(),
                to_source_span(subtrahend_sv.span),
            ))
        }
        (other, _) => {
            return Err(context.type_mismatch(
                "Number",
                other.type_name(),
                to_source_span(*call_span),
            ))
        }
    };

    context.world.borrow_mut().set(&path, new_value.clone());
    Ok(SpannedValue {
        value: new_value,
        span: *call_span,
    })
};

/// Creates a path from a string.
/// `(path <string>)`
pub const ATOM_PATH: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let string_sv = evaluate_ast_node(&args[0], context)?;
    let s = match &string_sv.value {
        Value::String(s) => s,
        _ => {
            return Err(context.type_mismatch(
                "String",
                string_sv.value.type_name(),
                to_source_span(string_sv.span),
            ))
        }
    };

    let path = Path(vec![s.clone()]);
    Ok(SpannedValue {
        value: Value::Path(path),
        span: *call_span,
    })
};
