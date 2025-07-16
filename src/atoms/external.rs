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

use crate::ast::value::Value;
use crate::atoms::StatefulAtomFn;
use crate::err_ctx;

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
///
/// # Safety
/// Emits output, does not mutate world state.
pub const ATOM_PRINT: StatefulAtomFn = |args, context| {
    if args.len() != 1 {
        return Err(err_ctx!(Eval, "print expects 1 argument, got {}", "print", "Arity error"));
    }

    context.output.emit(&args[0].to_string(), None);
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
///
/// # Safety
/// Pure random generation, does not mutate world state.
/// Uses a simple pseudo-random generator based on system time.
pub const ATOM_RAND: StatefulAtomFn = |args, context| {
    if !args.is_empty() {
        return Err(err_ctx!(Eval, "rand expects 0 arguments, got {}", "rand", "Arity error"));
    }

    let n = context.rng.next_u32();
    let random_value = (n as f64) / (u32::MAX as f64);
    Ok(Value::Number(random_value))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all external interface atoms with the given registry.
pub fn register_external_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // I/O operations
    registry.register("print", crate::atoms::Atom::Stateful(ATOM_PRINT));
    registry.register("core/print", crate::atoms::Atom::Stateful(ATOM_PRINT));

    // Randomness
    registry.register("rand", crate::atoms::Atom::Stateful(ATOM_RAND));
}
