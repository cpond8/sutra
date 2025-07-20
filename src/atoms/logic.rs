//!
//! This module provides all logic and comparison atom operations for the Sutra engine.
//! All atoms in this module are pure functions that do not mutate world state.
//!
//! ## Atoms Provided
//!
//! - **Comparison**: `eq?`, `gt?`, `lt?`, `gte?`, `lte?`
//! - **Logic**: `not`
//!
//! ## Design Principles
//!
//! - **Pure Functions**: No side effects, no world state modification
//! - **Boolean Results**: All operations return `Value::Bool`
//! - **Numeric Comparison**: Comparison operations work with `Value::Number`

use crate::prelude::*;
use crate::{atoms::PureAtomFn, helpers};

// ============================================================================
// COMPARISON OPERATIONS
// ============================================================================

/// Returns true if two values are equal.
///
/// Usage: (eq? <a> <b> ...)
///   - <a>, <b>, ...: Values to compare (at least 2 required)
///
///   Returns: Bool
///
/// Example:
///   (eq? 1 1) ; => true
///   (eq? 1 2) ; => false
///   (eq? 1 1 1) ; => true
pub const ATOM_EQ: PureAtomFn = |args| {
    helpers::validate_binary_arity(args, "eq?")?;
    for window in args.windows(2) {
        if window[0] != window[1] {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
};

/// Returns true if a > b.
///
/// Usage: (gt? <a> <b> ...)
///   - <a>, <b>, ...: Numbers (at least 2 required)
///
///   Returns: Bool
///
/// Example:
///   (gt? 3 2) ; => true
///   (gt? 3 2 1) ; => true
pub const ATOM_GT: PureAtomFn =
    |args| helpers::pure_eval_numeric_sequence_comparison(args, |a, b| a <= b, "gt?");

/// Returns true if a < b.
///
/// Usage: (lt? <a> <b> ...)
///   - <a>, <b>, ...: Numbers (at least 2 required)
///
///   Returns: Bool
///
/// Example:
///   (lt? 1 2) ; => true
///   (lt? 1 2 3) ; => true
pub const ATOM_LT: PureAtomFn =
    |args| helpers::pure_eval_numeric_sequence_comparison(args, |a, b| a >= b, "lt?");

/// Returns true if a >= b.
///
/// Usage: (gte? <a> <b> ...)
///   - <a>, <b>, ...: Numbers (at least 2 required)
///
///   Returns: Bool
///
/// Example:
///   (gte? 2 2) ; => true
///   (gte? 3 2 1) ; => true
pub const ATOM_GTE: PureAtomFn =
    |args| helpers::pure_eval_numeric_sequence_comparison(args, |a, b| a < b, "gte?");

/// Returns true if a <= b.
///
/// Usage: (lte? <a> <b> ...)
///   - <a>, <b>, ...: Numbers (at least 2 required)
///
///   Returns: Bool
///
/// Example:
///   (lte? 1 2) ; => true
///   (lte? 1 2 3) ; => true
pub const ATOM_LTE: PureAtomFn =
    |args| helpers::pure_eval_numeric_sequence_comparison(args, |a, b| a > b, "lte?");

// ============================================================================
// LOGIC OPERATIONS
// ============================================================================

/// Logical negation.
///
/// Usage: (not <a>)
///   - <a>: Boolean
///
///   Returns: Bool
///
/// Example:
///   (not true) ; => false
pub const ATOM_NOT: PureAtomFn =
    |args| helpers::pure_eval_unary_typed_op::<bool, _>(args, |b| Value::Bool(!b), "not");

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all logic and comparison atoms with the given registry.
pub fn register_logic_atoms(registry: &mut AtomRegistry) {
    // Comparison operations
    registry.register_pure("eq?", ATOM_EQ);
    registry.register_pure("gt?", ATOM_GT);
    registry.register_pure("lt?", ATOM_LT);
    registry.register_pure("gte?", ATOM_GTE);
    registry.register_pure("lte?", ATOM_LTE);

    // Comparison aliases
    registry.register_pure("=", ATOM_EQ);
    registry.register_pure(">", ATOM_GT);
    registry.register_pure("<", ATOM_LT);
    registry.register_pure(">=", ATOM_GTE);
    registry.register_pure("<=", ATOM_LTE);
    registry.register_pure("is?", ATOM_EQ);
    registry.register_pure("over?", ATOM_GT);
    registry.register_pure("under?", ATOM_LT);
    registry.register_pure("at-least?", ATOM_GTE);
    registry.register_pure("at-most?", ATOM_LTE);

    // Logic operations
    registry.register_pure("not", ATOM_NOT);
}
