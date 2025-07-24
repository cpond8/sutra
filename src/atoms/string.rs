//! # String Manipulation Atoms
//!
//! This module provides atoms for creating and manipulating strings.
//!
//! ## Atoms Provided
//!
//! - **`str`**: Converts any value to its string representation.
//! - **`str+`**: Concatenates multiple values into a single string.

use crate::atoms::AtomResult;
use crate::helpers;
use crate::prelude::*;

// ============================================================================
// STRING OPERATIONS
// ============================================================================

/// Converts any value to its string representation.
///
/// Usage: (str <value>)
///   - <value>: Any value
///
/// Returns: A new String value.
pub const ATOM_STR: NativeEagerFn =
    |args: &[Value], context: &mut EvaluationContext| -> AtomResult {
        helpers::validate_unary_arity(args, "str", context)?;
        Ok(Value::String(args[0].to_string()))
    };

/// Concatenates multiple values into a single string.
///
/// Usage: (str+ <value1> <value2> ...)
///   - <value...>: Zero or more values to concatenate.
///
/// Returns: A new String value. If no arguments are provided, returns an empty string.
pub const ATOM_STR_PLUS: NativeEagerFn = |args: &[Value], _| -> AtomResult {
    let mut result = String::new();
    for arg in args {
        result.push_str(&arg.to_string());
    }
    Ok(Value::String(result))
};
