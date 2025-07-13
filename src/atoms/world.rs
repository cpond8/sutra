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
use crate::atoms::StatefulAtomFn;
use crate::runtime::path::Path;
use crate::syntax::error::{EvalError, SutraError, SutraErrorKind};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn simple_error(message: &str) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            kind: crate::syntax::error::EvalErrorKind::General(message.to_string()),
            expanded_code: String::new(),
            original_code: None,
        }),
        span: None,
    }
}

fn arity_error(actual: usize, expected: usize, atom_name: &str) -> SutraError {
    simple_error(&format!(
        "{}: expected {} arguments, got {}",
        atom_name, expected, actual
    ))
}

fn extract_path<'a>(
    value: &'a Value,
    atom_name: &str,
    arg_position: usize,
) -> Result<&'a Path, SutraError> {
    match value {
        Value::Path(p) => Ok(p),
        _ => Err(simple_error(&format!(
            "{}: expected a Path at argument {}, but found type {}",
            atom_name,
            arg_position + 1,
            value.type_name()
        ))),
    }
}

// ============================================================================
// WORLD STATE OPERATIONS
// ============================================================================

/// Sets a value at a path in the world state.
/// `(core/set! <path> <value>)`
pub const ATOM_CORE_SET: StatefulAtomFn = |args, context| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "core/set!"));
    }
    let path = extract_path(&args[0], "core/set!", 0)?;
    let value = args[1].clone();
    context.state.set(path, value);
    Ok(Value::Nil)
};

/// Gets a value at a path in the world state.
/// `(core/get <path>)`
pub const ATOM_CORE_GET: StatefulAtomFn = |args, context| {
    if args.len() != 1 {
        return Err(arity_error(args.len(), 1, "core/get"));
    }
    let path = extract_path(&args[0], "core/get", 0)?;
    let value = context.state.get(path).cloned().unwrap_or_default();
    Ok(value)
};

/// Deletes a value at a path in the world state.
/// `(core/del! <path>)`
pub const ATOM_CORE_DEL: StatefulAtomFn = |args, context| {
    if args.len() != 1 {
        return Err(arity_error(args.len(), 1, "core/del!"));
    }
    let path = extract_path(&args[0], "core/del!", 0)?;
    context.state.del(path);
    Ok(Value::Nil)
};

/// Returns true if a path exists in the world state.
/// `(core/exists? <path>)`
pub const ATOM_EXISTS: StatefulAtomFn = |args, context| {
    if args.len() != 1 {
        return Err(arity_error(args.len(), 1, "core/exists?"));
    }
    let path = extract_path(&args[0], "core/exists?", 0)?;
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
