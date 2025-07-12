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
use crate::atoms::PureAtomFn;
use crate::syntax::error::{EvalError, SutraError, SutraErrorKind};

// ============================================================================
// ERROR HELPERS
// ============================================================================

/// Helper function to create a simple error for pure atoms
fn simple_error(message: &str) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            message: message.to_string(),
            expanded_code: String::new(),
            original_code: None,
            suggestion: None,
        }),
        span: None,
    }
}

/// Helper function to extract a number from a Value
fn extract_number(value: &Value, index: usize, atom_name: &str) -> Result<f64, SutraError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(simple_error(&format!(
            "{}: expected Number at position {}, got {:?}",
            atom_name, index, value
        ))),
    }
}

/// Helper function to create arity error
fn arity_error(actual: usize, expected: usize, atom_name: &str) -> SutraError {
    simple_error(&format!(
        "{}: expected {} arguments, got {}",
        atom_name, expected, actual
    ))
}

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
pub const ATOM_EQ: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "eq?"));
    }
    Ok(Value::Bool(args[0] == args[1]))
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
pub const ATOM_GT: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "gt?"));
    }
    let a = extract_number(&args[0], 0, "gt?")?;
    let b = extract_number(&args[1], 1, "gt?")?;
    Ok(Value::Bool(a > b))
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
pub const ATOM_LT: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "lt?"));
    }
    let a = extract_number(&args[0], 0, "lt?")?;
    let b = extract_number(&args[1], 1, "lt?")?;
    Ok(Value::Bool(a < b))
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
pub const ATOM_GTE: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "gte?"));
    }
    let a = extract_number(&args[0], 0, "gte?")?;
    let b = extract_number(&args[1], 1, "gte?")?;
    Ok(Value::Bool(a >= b))
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
pub const ATOM_LTE: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "lte?"));
    }
    let a = extract_number(&args[0], 0, "lte?")?;
    let b = extract_number(&args[1], 1, "lte?")?;
    Ok(Value::Bool(a <= b))
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
pub const ATOM_NOT: PureAtomFn = |args| {
    if args.len() != 1 {
        return Err(arity_error(args.len(), 1, "not"));
    }
    match &args[0] {
        Value::Bool(b) => Ok(Value::Bool(!b)),
        _ => Err(simple_error(&format!(
            "not: expected Bool at position 0, got {:?}",
            args[0]
        ))),
    }
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all logic and comparison atoms with the given registry.
pub fn register_logic_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // Comparison operations
    registry.register("eq?", crate::atoms::Atom::Pure(ATOM_EQ));
    registry.register("gt?", crate::atoms::Atom::Pure(ATOM_GT));
    registry.register("lt?", crate::atoms::Atom::Pure(ATOM_LT));
    registry.register("gte?", crate::atoms::Atom::Pure(ATOM_GTE));
    registry.register("lte?", crate::atoms::Atom::Pure(ATOM_LTE));

    // Logic operations
    registry.register("not", crate::atoms::Atom::Pure(ATOM_NOT));
}
