//! # String Manipulation Atoms
//!
//! This module provides atoms for creating and manipulating strings.
//!
//! ## Atoms Provided
//!
//! - **`str`**: Converts any value to its string representation.
//! - **`str+`**: Concatenates multiple values into a single string.

use crate::ast::value::Value;
use crate::atoms::PureAtomFn;
use crate::err_ctx;

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
    if args.len() != 1 {
        return Err(err_ctx!(Eval, "str expects 1 argument, got {}", "str", "Arity error"));
    }
    Ok(Value::String(args[0].to_string()))
};

/// Concatenates multiple values into a single string.
///
/// Usage: (str+ <value1> <value2> ...)
///   - <value...>: Zero or more values to concatenate.
///
/// Returns: A new String value. If no arguments are provided, returns an empty string.
pub const ATOM_STR_PLUS: PureAtomFn = |args| {
    let mut result = String::new();
    for arg in args {
        result.push_str(&arg.to_string());
    }
    Ok(Value::String(result))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all string manipulation atoms with the given registry.
pub fn register_string_atoms(registry: &mut crate::atoms::AtomRegistry) {
    registry.register("str", crate::atoms::Atom::Pure(ATOM_STR));
    registry.register("str+", crate::atoms::Atom::Pure(ATOM_STR_PLUS));
}