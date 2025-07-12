//! # External Interface
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
use crate::atoms::helpers::*;
use crate::atoms::AtomFn;
use crate::runtime::eval::EvalContext;

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
pub const ATOM_PRINT: AtomFn = |args, context, parent_span| {
    eval_unary_value_op(
        args,
        context,
        parent_span,
        |val: Value,
         world: crate::runtime::world::World,
         parent_span: &crate::ast::Span,
         context: &mut EvalContext<'_, '_>|
         -> Result<(Value, crate::runtime::world::World), crate::syntax::error::SutraError> {
            context.output.emit(&val.to_string(), Some(parent_span));
            Ok((Value::Nil, world)) // Return Nil so the engine does not print again
        },
        "print",
    )
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
pub const ATOM_RAND: AtomFn = |args, context, parent_span| {
    if !args.is_empty() {
        return Err(arity_error(Some(parent_span.clone()), args, "rand", 0));
    }

    // Generate pseudo-random number using system time as seed
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = duration.as_nanos();

    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    let hash = hasher.finish();

    // Convert to 0.0..1.0 range
    let random_value = (hash as f64) / (u64::MAX as f64);

    Ok((Value::Number(random_value), context.world.clone()))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all external interface atoms with the given registry.
pub fn register_external_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // I/O operations
    registry.register("print", crate::atoms::Atom::Legacy(ATOM_PRINT));
    registry.register("core/print", crate::atoms::Atom::Legacy(ATOM_PRINT));

    // Randomness
    registry.register("rand", crate::atoms::Atom::Legacy(ATOM_RAND));
}
