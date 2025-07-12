//! # World State Management
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

use crate::ast::value::Value;
use crate::atoms::AtomFn;
use crate::atoms::helpers::*;

// ============================================================================
// WORLD STATE OPERATIONS
// ============================================================================

/// Sets a value at a path in the world state.
///
/// Usage: (core/set! <path> <value>)
///   - <path>: Path to set (must evaluate to a Value::Path)
///   - <value>: Value to store
///
///   Returns: Nil. Mutates world state (returns new world).
///
/// Example:
///   (core/set! player.score 42)
///
/// # Safety
/// Only mutates the world at the given path.
pub const ATOM_CORE_SET: AtomFn = |args, context, parent_span| {
    eval_binary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         value: Value,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), crate::syntax::error::SutraError> {
            let new_world = world.set(&path, value);
            Ok((Value::default(), new_world))
        },
        "core/set!",
    )
};

/// Gets a value at a path in the world state.
///
/// Usage: (core/get <path>)
///   - <path>: Path to get (must evaluate to a Value::Path)
///
///   Returns: Value at path, or Nil if not found.
///
/// Example:
///   (core/get player.score)
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_CORE_GET: AtomFn = |args, context, parent_span| {
    eval_unary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), crate::syntax::error::SutraError> {
            let value = world.get(&path).cloned().unwrap_or_default();
            Ok((value, world))
        },
        "core/get",
    )
};

/// Deletes a value at a path in the world state.
///
/// Usage: (core/del! <path>)
///   - <path>: Path to delete (must evaluate to a Value::Path)
///
///   Returns: Nil. Mutates world state (returns new world).
///
/// Example:
///   (core/del! player.score)
///
/// # Safety
/// Only mutates the world at the given path.
pub const ATOM_CORE_DEL: AtomFn = |args, context, parent_span| {
    eval_unary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), crate::syntax::error::SutraError> {
            let new_world = world.del(&path);
            Ok((Value::default(), new_world))
        },
        "core/del!",
    )
};

/// Returns true if a path exists in the world state.
///
/// Usage: (core/exists? <path>)
///   - <path>: Path to check (must evaluate to a Value::Path)
///
///   Returns: Bool
///
/// Example:
///   (core/exists? player.score) ; => true if path exists, false otherwise
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_EXISTS: AtomFn = |args, context, parent_span| {
    eval_unary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), crate::syntax::error::SutraError> {
            let exists = world.get(&path).is_some();
            Ok((Value::Bool(exists), world))
        },
        "core/exists?",
    )
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all world state atoms with the given registry.
pub fn register_world_atoms(registry: &mut crate::atoms::AtomRegistry) {
    registry.register("core/set!", crate::atoms::Atom::Legacy(ATOM_CORE_SET));
    registry.register("core/get", crate::atoms::Atom::Legacy(ATOM_CORE_GET));
    registry.register("core/del!", crate::atoms::Atom::Legacy(ATOM_CORE_DEL));
    registry.register("core/exists?", crate::atoms::Atom::Legacy(ATOM_EXISTS));
}
