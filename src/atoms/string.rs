//! # String Manipulation Atoms
//!
//! This module provides atoms for creating and manipulating strings.
//!
//! ## Atoms Provided
//!
//! - **`str`**: Converts any value to its string representation.
//! - **`str+`**: Concatenates multiple values into a single string.

use crate::{
    atoms::{
        helpers::{pure_eval_string_concat, validate_unary_arity},
        Atom, AtomRegistry, PureAtomFn,
    },
    Value,
};

// ============================================================================
// STRING OPERATIONS
// ============================================================================

/// Converts any value to its string representation.
///
/// Usage: (str <value>)
///   - <value>: Any value
///
/// Returns: A new String value.
pub const ATOM_STR: PureAtomFn = |args| {
    validate_unary_arity(args, "str")?;
    Ok(Value::String(args[0].to_string()))
};

/// Concatenates multiple values into a single string.
///
/// Usage: (str+ <value1> <value2> ...)
///   - <value...>: Zero or more values to concatenate.
///
/// Returns: A new String value. If no arguments are provided, returns an empty string.
pub const ATOM_STR_PLUS: PureAtomFn = |args| pure_eval_string_concat(args, "str+");

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all string manipulation atoms with the given registry.
pub fn register_string_atoms(registry: &mut AtomRegistry) {
    registry.register("str", Atom::Pure(ATOM_STR));
    registry.register("str+", Atom::Pure(ATOM_STR_PLUS));
}
