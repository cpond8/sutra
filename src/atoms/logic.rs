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
use crate::atoms::helpers::extract_number;
use crate::sutra_err;

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
    if args.len() < 2 {
        return Ok(Value::Bool(true));
    }
    for window in args.windows(2) {
        if window[0] != window[1] {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
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
    if args.len() < 2 {
        return Ok(Value::Bool(true));
    }
    for i in 0..args.len() - 1 {
        let a = extract_number(&args[i])?;
        let b = extract_number(&args[i + 1])?;
        if a <= b {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
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
    if args.len() < 2 {
        return Ok(Value::Bool(true));
    }
    for i in 0..args.len() - 1 {
        let a = extract_number(&args[i])?;
        let b = extract_number(&args[i + 1])?;
        if a >= b {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
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
    if args.len() < 2 {
        return Ok(Value::Bool(true));
    }
    for i in 0..args.len() - 1 {
        let a = extract_number(&args[i])?;
        let b = extract_number(&args[i + 1])?;
        if a < b {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
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
    if args.len() < 2 {
        return Ok(Value::Bool(true));
    }
    for i in 0..args.len() - 1 {
        let a = extract_number(&args[i])?;
        let b = extract_number(&args[i + 1])?;
        if a > b {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
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
        return Err(sutra_err!(Eval, "not expects 1 argument, got {}", args.len()));
    }
    match &args[0] {
        Value::Bool(b) => Ok(Value::Bool(!b)),
        val => Err(sutra_err!(Eval, "not expects a Bool, found {}", val)),
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
