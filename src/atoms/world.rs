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

use crate::prelude::*;
use crate::{
    errors,
    helpers::{self, ExtractValue},
    NativeEagerFn,
};

// ============================================================================
// WORLD STATE OPERATIONS
// ============================================================================

/// Sets a value at a path in the world state.
/// `(core/set! <path> <value>)`
pub const ATOM_CORE_SET: NativeEagerFn = |args, context| {
    helpers::validate_binary_arity(args, "core/set!", context)?;
    let path = helpers::validate_path_arg(&args[0], context.current_span, "core/set!", context)?;
    let value = args[1].clone();
    context.world.borrow_mut().set(path, value);
    Ok(Value::Nil)
};

/// Gets a value at a path in the world state.
/// `(core/get <path>)`
pub const ATOM_CORE_GET: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "core/get", context)?;
    let path = helpers::validate_path_arg(&args[0], context.current_span, "core/get", context)?;
    let value = context
        .world
        .borrow()
        .get(path)
        .cloned()
        .unwrap_or_default();
    Ok(value)
};

/// Deletes a value at a path in the world state.
/// `(core/del! <path>)`
pub const ATOM_CORE_DEL: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "core/del!", context)?;
    let path = helpers::validate_path_arg(&args[0], context.current_span, "core/del!", context)?;
    context.world.borrow_mut().del(path);
    Ok(Value::Nil)
};

/// Returns true if a path exists in the world state.
/// `(core/exists? <path>)`
pub const ATOM_EXISTS: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "core/exists?", context)?;
    let path = helpers::validate_path_arg(&args[0], context.current_span, "core/exists?", context)?;
    let exists = context.world.borrow().get(path).is_some();
    Ok(Value::Bool(exists))
};

/// Creates a path from a string.
/// `(path <string>)`
pub const ATOM_PATH: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "path", context)?;
    match &args[0] {
        Value::String(s) => {
            let path = Path(vec![s.clone()]);
            Ok(Value::Path(path))
        }
        _ => Err(errors::type_mismatch(
            "String",
            args[0].type_name(),
            &context.source,
            context.span_for_span(context.current_span),
        )),
    }
};
