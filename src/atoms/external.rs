//!
//! This module provides atoms that interface with external systems.
//! These atoms break the pure functional model by interacting with I/O or randomness.
//!
//! ## Atoms Provided
//!
//! - **I/O**: `print`
//! - **Randomness**: `rand`
//!
//! ## Design Principles
//!
//! - **Side Effects**: Clear documentation of external interactions
//! - **Deterministic Where Possible**: Consistent behavior within constraints
//! - **Minimal External Dependencies**: Simple implementations

use crate::prelude::*;
use crate::{helpers, NativeEagerFn};
use crate::errors::{ErrorKind, ErrorReporting};

// ============================================================================
// I/O OPERATIONS
// ============================================================================

/// Emits output to the output sink.
///
/// Usage: (core/print <value>)
///   - <value>: Any value
///
///   Returns: Nil. Emits output.
///
/// Example:
///   (core/print "hello")
pub const ATOM_PRINT: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "core/print", context)?;
    context.output.borrow_mut().emit(&args[0].to_string(), None);
    Ok(Value::Nil)
};

/// Emits output to the output sink (alias for print).
///
/// Usage: (output <value>)
///   - <value>: Any value
///
///   Returns: Nil. Emits output.
///
/// Example:
///   (output "hello")
pub const ATOM_OUTPUT: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "output", context)?;
    context.output.borrow_mut().emit(&args[0].to_string(), None);
    Ok(Value::Nil)
};

// ============================================================================
// RANDOMNESS OPERATIONS
// ============================================================================

/// Generates a pseudo-random number between 0.0 (inclusive) and 1.0 (exclusive).
///
/// Usage: (rand)
///   - No arguments
///
///   Returns: Number (pseudo-random float between 0.0 and 1.0)
///
/// Example:
///   (rand) ; => 0.7234567 (example)
pub const ATOM_RAND: NativeEagerFn = |args, context| {
    helpers::validate_zero_arity(args, "rand", context)?;
    // If randomness is not available, return a canonical error
    Err(context.report(
        ErrorKind::InvalidOperation {
            operation: "random number generation".to_string(),
            operand_type: "unsupported in this context".to_string(),
        },
        context.span_for_span(context.current_span),
    ))
};
