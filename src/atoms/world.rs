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
    atoms::{AtomRegistry, EagerAtomFn},
    errors,
    helpers::{self, ExtractValue},
};

// ============================================================================
// WORLD STATE OPERATIONS
// ============================================================================

/// Sets a value at a path in the world state.
/// `(core/set! <path> <value>)`
pub const ATOM_CORE_SET: EagerAtomFn = |args, context| {
    helpers::validate_binary_arity(args, "core/set!")?;
    let path = &args[0].extract(Some(context))?;
    let value = args[1].clone();
    let new_world = context.world.set(path, value);
    Ok((Value::Nil, new_world))
};

/// Gets a value at a path in the world state.
/// `(core/get <path>)`
pub const ATOM_CORE_GET: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "core/get")?;
    let path = &args[0].extract(Some(context))?;
    let value = context.world.get(path).cloned().unwrap_or_default();
    Ok((value, context.world.clone()))
};

/// Deletes a value at a path in the world state.
/// `(core/del! <path>)`
pub const ATOM_CORE_DEL: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "core/del!")?;
    let path = &args[0].extract(Some(context))?;
    let new_world = context.world.del(path);
    Ok((Value::Nil, new_world))
};

/// Returns true if a path exists in the world state.
/// `(core/exists? <path>)`
pub const ATOM_EXISTS: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "core/exists?")?;
    let path = &args[0].extract(Some(context))?;
    let exists = context.world.get(path).is_some();
    Ok((Value::Bool(exists), context.world.clone()))
};

/// Creates a path from a string.
/// `(path <string>)`
pub const ATOM_PATH: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "path")?;
    match &args[0] {
        Value::String(s) => {
            let path = Path(vec![s.clone()]);
            Ok((Value::Path(path), context.world.clone()))
        }
        _ => Err(errors::type_mismatch(
            "String",
            args[0].type_name(),
            context.current_file(),
            context.current_source(),
            // Since this is eager, we don't have a specific node span.
            // Using the parent span from the context would be ideal if available,
            // otherwise a default span is the fallback.
            context.span_for_span(Span::default()),
        )),
    }
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all world state atoms with the given registry.
pub fn register_world_atoms(registry: &mut AtomRegistry) {
    registry.register_eager("core/set!", ATOM_CORE_SET);
    registry.register_eager("core/get", ATOM_CORE_GET);
    registry.register_eager("core/del!", ATOM_CORE_DEL);
    registry.register_eager("core/exists?", ATOM_EXISTS);
    registry.register_eager("path", ATOM_PATH);
}
