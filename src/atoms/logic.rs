//! # Logic and Comparison Operations
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

use crate::ast::value::Value;
use crate::atoms::helpers::*;
use crate::atoms::AtomFn;

// ============================================================================
// COMPARISON OPERATIONS
// ============================================================================

/// Returns true if two values are equal.
///
/// Usage: (eq? <a> <b>)
///   - <a>, <b>: Values to compare
///
///   Returns: Bool
///
/// Example:
///   (eq? 1 1) ; => true
///   (eq? 1 2) ; => false
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_EQ: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a == b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "eq?",
    )
};

/// Returns true if a > b.
///
/// Usage: (gt? <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Bool
///
/// Example:
///   (gt? 3 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_GT: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a > b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "gt?",
    )
};

/// Returns true if a < b.
///
/// Usage: (lt? <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Bool
///
/// Example:
///   (lt? 1 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_LT: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a < b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "lt?",
    )
};

/// Returns true if a >= b.
///
/// Usage: (gte? <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Bool
///
/// Example:
///   (gte? 2 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_GTE: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a >= b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "gte?",
    )
};

/// Returns true if a <= b.
///
/// Usage: (lte? <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Bool
///
/// Example:
///   (lte? 1 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_LTE: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a <= b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "lte?",
    )
};

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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_NOT: AtomFn = |args, context, parent_span| {
    eval_unary_bool_op(args, context, parent_span, |b: bool| Value::Bool(!b), "not")
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all logic and comparison atoms with the given registry.
pub fn register_logic_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // Comparison operations
    registry.register("eq?", ATOM_EQ);
    registry.register("gt?", ATOM_GT);
    registry.register("lt?", ATOM_LT);
    registry.register("gte?", ATOM_GTE);
    registry.register("lte?", ATOM_LTE);

    // Logic operations
    registry.register("not", ATOM_NOT);
}
