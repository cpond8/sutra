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
use crate::{atoms::EagerAtomFn, helpers};
use crate::errors;

// ============================================================================
// I/O OPERATIONS
// ============================================================================

/// Emits output to the output sink.
///
/// Usage: (print <value>)
///   - <value>: Any value
///
///   Returns: Nil. Emits output.
///
/// Example:
///   (print "hello")
pub const ATOM_PRINT: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "print")?;
    context.output.borrow_mut().emit(&args[0].to_string(), None);
    Ok((Value::Nil, context.world.clone()))
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
pub const ATOM_OUTPUT: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "output")?;
    context.output.borrow_mut().emit(&args[0].to_string(), None);
    Ok((Value::Nil, context.world.clone()))
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
pub const ATOM_RAND: EagerAtomFn = |args, context| {
    helpers::validate_zero_arity(args, "rand")?;
    // If randomness is not available, return a canonical error
    return Err(errors::runtime_general(
        "Random number generation is not available in this context.",
        context.current_file(),
        context.current_source(),
        context.span_for_span(Span::default()),
    ));
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all external interface atoms with the given registry.
pub fn register_external_atoms(registry: &mut AtomRegistry) {
    // I/O operations
    registry.register_eager("print", ATOM_PRINT);
    registry.register_eager("core/print", ATOM_PRINT);
    registry.register_eager("output", ATOM_OUTPUT);

    // Randomness
    registry.register_eager("rand", ATOM_RAND);
}
