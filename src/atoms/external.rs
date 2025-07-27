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

use crate::{
    ast::spanned_value::SpannedValue,
    engine::evaluate_ast_node,
    errors::{to_source_span, ErrorReporting},
    prelude::*,
};

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
pub const ATOM_PRINT: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }
    let spanned_val = evaluate_ast_node(&args[0], context)?;
    context
        .output
        .borrow_mut()
        .emit(&spanned_val.value.to_string(), Some(&spanned_val.span));
    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
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
pub const ATOM_OUTPUT: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }
    let spanned_val = evaluate_ast_node(&args[0], context)?;
    context
        .output
        .borrow_mut()
        .emit(&spanned_val.value.to_string(), Some(&spanned_val.span));
    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
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
pub const ATOM_RAND: NativeFn = |args, context, call_span| {
    if !args.is_empty() {
        return Err(context.arity_mismatch("0", args.len(), to_source_span(*call_span)));
    }
    let rand_val = context.world.borrow_mut().next_u32();
    let result = (rand_val as f64) / (u32::MAX as f64);
    Ok(SpannedValue {
        value: Value::Number(result),
        span: *call_span,
    })
};
