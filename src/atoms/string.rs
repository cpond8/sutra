//! # String Manipulation Atoms
//!
//! This module provides atoms for creating and manipulating strings.
//!
//! ## Atoms Provided
//!
//! - **`str`**: Converts any value to its string representation.
//! - **`str+`**: Concatenates multiple values into a single string.

use crate::{
    errors::{to_source_span, ErrorReporting},
    runtime::{evaluate_ast_node, NativeFn, SpannedValue, Value},
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
pub const ATOM_STR: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let val_sv = evaluate_ast_node(&args[0], context)?;

    Ok(SpannedValue {
        value: Value::String(val_sv.value.to_string()),
        span: *call_span,
    })
};

/// Concatenates multiple values into a single string.
///
/// Usage: (str+ <value1> <value2> ...)
///   - <value...>: Zero or more values to concatenate.
///
/// Returns: A new String value. If no arguments are provided, returns an empty string.
pub const ATOM_STR_PLUS: NativeFn = |args, context, call_span| {
    let mut result = String::new();
    for arg in args {
        let val_sv = evaluate_ast_node(arg, context)?;
        result.push_str(&val_sv.value.to_string());
    }
    Ok(SpannedValue {
        value: Value::String(result),
        span: *call_span,
    })
};
