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
    atoms::{AtomRegistry, StatefulAtomFn},
    helpers::{self, ExtractValue},
    syntax::parser::to_source_span,
};
use miette::NamedSource;

// ============================================================================
// WORLD STATE OPERATIONS
// ============================================================================

/// Sets a value at a path in the world state.
/// `(core/set! <path> <value>)`
pub const ATOM_CORE_SET: StatefulAtomFn = |args, context| {
    helpers::validate_binary_arity(args, "core/set!")?;
    let path = &args[0].extract()?;
    let value = args[1].clone();
    context.state.set(path, value);
    Ok(Value::Nil)
};

/// Gets a value at a path in the world state.
/// `(core/get <path>)`
pub const ATOM_CORE_GET: StatefulAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "core/get")?;
    let path = &args[0].extract()?;
    let value = context.state.get(path).cloned().unwrap_or_default();
    Ok(value)
};

/// Deletes a value at a path in the world state.
/// `(core/del! <path>)`
pub const ATOM_CORE_DEL: StatefulAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "core/del!")?;
    let path = &args[0].extract()?;
    context.state.del(path);
    Ok(Value::Nil)
};

/// Returns true if a path exists in the world state.
/// `(core/exists? <path>)`
pub const ATOM_EXISTS: StatefulAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "core/exists?")?;
    let path = &args[0].extract()?;
    let exists = context.state.get(path).is_some();
    Ok(Value::Bool(exists))
};

/// Creates a path from a string.
/// `(path <string>)`
pub const ATOM_PATH: StatefulAtomFn = |args, _context| {
    helpers::validate_unary_arity(args, "path")?;
    match &args[0] {
        Value::String(s) => {
            let path = Path(vec![s.clone()]);
            Ok(Value::Path(path))
        }
        _ => Err(SutraError::TypeMismatch {
            expected: "String".to_string(),
            actual: args[0].type_name().to_string(),
            src: NamedSource::new("atoms/world.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
        }),
    }
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all world state atoms with the given registry.
pub fn register_world_atoms(registry: &mut AtomRegistry) {
    registry.register_stateful("core/set!", ATOM_CORE_SET);
    registry.register_stateful("core/get", ATOM_CORE_GET);
    registry.register_stateful("core/del!", ATOM_CORE_DEL);
    registry.register_stateful("core/exists?", ATOM_EXISTS);
    registry.register_stateful("path", ATOM_PATH);
}
