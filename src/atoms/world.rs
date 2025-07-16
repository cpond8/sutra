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
use crate::atoms::StatefulAtomFn;
use crate::atoms::helpers::extract_path;
use crate::sutra_err;

// ============================================================================
// WORLD STATE OPERATIONS
// ============================================================================

/// Sets a value at a path in the world state.
/// `(core/set! <path> <value>)`
pub const ATOM_CORE_SET: StatefulAtomFn = |args, context| {
    if args.len() != 2 {
        return Err(sutra_err!(Eval, "core/set! expects 2 arguments, got {}", args.len()));
    }
    let path = &extract_path(&args[0])?;
    let value = args[1].clone();
    context.state.set(path, value);
    Ok(Value::Nil)
};

/// Gets a value at a path in the world state.
/// `(core/get <path>)`
pub const ATOM_CORE_GET: StatefulAtomFn = |args, context| {
    if args.len() != 1 {
        return Err(sutra_err!(Eval, "core/get expects 1 argument, got {}", args.len()));
    }
    let path = &extract_path(&args[0])?;
    let value = context.state.get(path).cloned().unwrap_or_default();
    Ok(value)
};

/// Deletes a value at a path in the world state.
/// `(core/del! <path>)`
pub const ATOM_CORE_DEL: StatefulAtomFn = |args, context| {
    if args.len() != 1 {
        return Err(sutra_err!(Eval, "core/del! expects 1 argument, got {}", args.len()));
    }
    let path = &extract_path(&args[0])?;
    context.state.del(path);
    Ok(Value::Nil)
};

/// Returns true if a path exists in the world state.
/// `(core/exists? <path>)`
pub const ATOM_EXISTS: StatefulAtomFn = |args, context| {
    if args.len() != 1 {
        return Err(sutra_err!(Eval, "core/exists? expects 1 argument, got {}", args.len()));
    }
    let path = &extract_path(&args[0])?;
    let exists = context.state.get(path).is_some();
    Ok(Value::Bool(exists))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all world state atoms with the given registry.
pub fn register_world_atoms(registry: &mut crate::atoms::AtomRegistry) {
    registry.register("core/set!", crate::atoms::Atom::Stateful(ATOM_CORE_SET));
    registry.register("core/get", crate::atoms::Atom::Stateful(ATOM_CORE_GET));
    registry.register("core/del!", crate::atoms::Atom::Stateful(ATOM_CORE_DEL));
    registry.register("core/exists?", crate::atoms::Atom::Stateful(ATOM_EXISTS));
}
