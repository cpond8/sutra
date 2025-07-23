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
use crate::helpers;
use crate::atoms::AtomResult;
use crate::helpers::ExtractValue;

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
pub const ATOM_EQ: NativeEagerFn = |args: &[Value], _| {
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
pub const ATOM_GT: NativeEagerFn = |args: &[Value], context: &mut EvaluationContext| -> AtomResult {
    helpers::validate_sequence_arity(args, "gt?")?;
    for i in 0..args.len() - 1 {
        let a: f64 = args[i].extract(Some(context))?;
        let b: f64 = args[i + 1].extract(Some(context))?;
        if a <= b {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
};

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
pub const ATOM_LT: NativeEagerFn = |args: &[Value], context: &mut EvaluationContext| -> AtomResult {
    helpers::validate_sequence_arity(args, "lt?")?;
    for i in 0..args.len() - 1 {
        let a: f64 = args[i].extract(Some(context))?;
        let b: f64 = args[i + 1].extract(Some(context))?;
        if a >= b {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
};

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
pub const ATOM_GTE: NativeEagerFn = |args: &[Value], context: &mut EvaluationContext| -> AtomResult {
    helpers::validate_sequence_arity(args, "gte?")?;
    for i in 0..args.len() - 1 {
        let a: f64 = args[i].extract(Some(context))?;
        let b: f64 = args[i + 1].extract(Some(context))?;
        if a < b {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
};

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
pub const ATOM_LTE: NativeEagerFn = |args: &[Value], context: &mut EvaluationContext| -> AtomResult {
    helpers::validate_sequence_arity(args, "lte?")?;
    for i in 0..args.len() - 1 {
        let a: f64 = args[i].extract(Some(context))?;
        let b: f64 = args[i + 1].extract(Some(context))?;
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
pub const ATOM_NOT: NativeEagerFn = |args: &[Value], context: &mut EvaluationContext| -> AtomResult {
    helpers::validate_unary_arity(args, "not")?;
    let b: bool = args[0].extract(Some(context))?;
    Ok(Value::Bool(!b))
};
