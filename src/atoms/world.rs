//!
//! This module provides all world state manipulation atom operations for the Sutra engine.
//! These atoms are the primary interface for reading and modifying game state.
//!
//! ## Atoms Provided
//!
//! - **State Operations**: `core/set!`, `core/get`, `core/del!`
//! - **State Queries**: `core/exists?`
//!
//! ## Design Principles
//!
//! - **Path-Based Access**: All operations use `Value::Path` for addressing
//! - **Immutable World**: World state is copied and modified, never mutated in place
//! - **Safe Defaults**: Missing values return `Value::Nil` rather than errors

use crate::{
    errors::{to_source_span, ErrorReporting},
    prelude::Path,
    runtime::{evaluate_ast_node, NativeFn, SpannedValue, Value},
};

// ============================================================================
// WORLD STATE OPERATIONS
// ============================================================================

/// Sets a value at a path in the world state.
/// `(core/set! <path> <value>)`
pub const ATOM_CORE_SET: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let path_sv = evaluate_ast_node(&args[0], context)?;
    let path = match &path_sv.value {
        Value::Path(p) => p,
        _ => {
            return Err(context.type_mismatch(
                "Path",
                path_sv.value.type_name(),
                to_source_span(path_sv.span),
            ))
        }
    };

    let value_sv = evaluate_ast_node(&args[1], context)?;
    context.world.borrow_mut().set(&path, value_sv.value);

    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};

/// Gets a value at a path in the world state.
/// `(core/get <path>)`
pub const ATOM_CORE_GET: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path_sv = evaluate_ast_node(&args[0], context)?;
    let path = match &path_sv.value {
        Value::Path(p) => p,
        _ => {
            return Err(context.type_mismatch(
                "Path",
                path_sv.value.type_name(),
                to_source_span(path_sv.span),
            ))
        }
    };

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
/// `(core/del! <path>)`
pub const ATOM_CORE_DEL: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path_sv = evaluate_ast_node(&args[0], context)?;
    let path = match &path_sv.value {
        Value::Path(p) => p,
        _ => {
            return Err(context.type_mismatch(
                "Path",
                path_sv.value.type_name(),
                to_source_span(path_sv.span),
            ))
        }
    };

    context.world.borrow_mut().del(&path);

    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};

/// Returns true if a path exists in the world state.
/// `(core/exists? <path>)`
pub const ATOM_EXISTS: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let path_sv = evaluate_ast_node(&args[0], context)?;
    let path = match &path_sv.value {
        Value::Path(p) => p,
        _ => {
            return Err(context.type_mismatch(
                "Path",
                path_sv.value.type_name(),
                to_source_span(path_sv.span),
            ))
        }
    };

    let exists = context.world.borrow().get(&path).is_some();

    Ok(SpannedValue {
        value: Value::Bool(exists),
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
