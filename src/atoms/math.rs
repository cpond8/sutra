//!
//! This module provides all mathematical atom operations for the Sutra engine.
//! All atoms in this module are pure functions that do not mutate world state.
//!
//! ## Atoms Provided
//!
//! - **Arithmetic**: `+`, `-`, `*`, `/`, `mod`
//! - **Math Functions**: `abs`, `min`, `max`
//!
//! ## Design Principles
//!
//! - **Pure Functions**: No side effects, no world state modification
//! - **Numeric Focus**: All operations work with `Value::Number` (f64)
//! - **Error Handling**: Proper validation for edge cases (division by zero, etc.)

use crate::prelude::*;
use crate::{
    atoms::PureAtomFn,
    helpers::{self, ExtractValue},
};

// ============================================================================
// ARITHMETIC OPERATIONS
// ============================================================================

/// Adds numbers.
///
/// Usage: (+ <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (sum)
///
/// Example:
///   (+ 1 2 3) ; => 6
pub const ATOM_ADD: PureAtomFn =
    |args| helpers::pure_eval_nary_numeric_op_custom(args, 0.0, |acc, n| acc + n, "+");

/// Subtracts two numbers.
///
/// Usage: (- <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Number (a - b)
///
/// Example:
///   (- 5 2) ; => 3
pub const ATOM_SUB: PureAtomFn = |args| {
    helpers::validate_min_arity(args, 1, "-")?;

    let first: f64 = args[0].extract()?;
    if args.len() == 1 {
        return Ok(Value::Number(-first));
    }

    let mut result = first;
    for arg in args.iter().skip(1) {
        let n: f64 = arg.extract()?;
        result -= n;
    }
    Ok(Value::Number(result))
};

/// Multiplies numbers.
///
/// Usage: (* <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (product)
///
/// Example:
///   (* 2 3 4) ; => 24
pub const ATOM_MUL: PureAtomFn =
    |args| helpers::pure_eval_nary_numeric_op_custom(args, 1.0, |acc, n| acc * n, "*");

/// Divides two numbers.
///
/// Usage: (/ <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Number (a / b)
///
/// Example:
///   (/ 6 2) ; => 3
/// Note: Errors on division by zero.
pub const ATOM_DIV: PureAtomFn = |args| {
    helpers::validate_min_arity(args, 1, "/")?;

    let first: f64 = args[0].extract()?;
    if args.len() == 1 {
        if first == 0.0 {
            return Err(err_msg!(Eval, "division by zero"));
        }
        return Ok(Value::Number(1.0 / first));
    }

    let mut result = first;
    for arg in args.iter().skip(1) {
        let n: f64 = arg.extract()?;
        if n == 0.0 {
            return Err(err_msg!(Eval, "division by zero"));
        }
        result /= n;
    }
    Ok(Value::Number(result))
};

/// Modulo operation.
///
/// Usage: (mod <a> <b>)
///   - <a>, <b>: Integers
///
///   Returns: Number (a % b)
///
/// Example:
///   (mod 5 2) ; => 1
///
/// Note: Errors on division by zero or non-integer input.
pub const ATOM_MOD: PureAtomFn = |args| {
    helpers::validate_binary_arity(args, "mod")?;
    let a: f64 = args[0].extract()?;
    let b: f64 = args[1].extract()?;

    if b == 0.0 {
        return Err(err_msg!(Eval, "modulo by zero"));
    }

    Ok(Value::Number(a % b))
};

// ============================================================================
// MATH FUNCTIONS
// ============================================================================

/// Absolute value of a number.
///
/// Usage: (abs <n>)
///   - <n>: Number
///
///   Returns: Number (absolute value)
///
/// Example:
///   (abs -5) ; => 5
///   (abs 3.14) ; => 3.14
pub const ATOM_ABS: PureAtomFn =
    |args| helpers::pure_eval_unary_typed_op::<f64, _>(args, |n| Value::Number(n.abs()), "abs");

/// Minimum of multiple numbers.
///
/// Usage: (min <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (minimum value)
///
/// Example:
///   (min 3 1 4) ; => 1
pub const ATOM_MIN: PureAtomFn = |args| {
    helpers::pure_eval_nary_numeric_op_custom(args, f64::INFINITY, |acc, n| acc.min(n), "min")
};

/// Maximum of multiple numbers.
///
/// Usage: (max <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (maximum value)
///
/// Example:
///   (max 3 1 4) ; => 4
pub const ATOM_MAX: PureAtomFn = |args| {
    helpers::pure_eval_nary_numeric_op_custom(args, f64::NEG_INFINITY, |acc, n| acc.max(n), "max")
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all mathematical atoms with the given registry.
pub fn register_math_atoms(registry: &mut AtomRegistry) {
    // Arithmetic operations
    registry.register_pure("+", ATOM_ADD);
    registry.register_pure("-", ATOM_SUB);
    registry.register_pure("*", ATOM_MUL);
    registry.register_pure("/", ATOM_DIV);
    registry.register_pure("mod", ATOM_MOD);

    // Math functions
    registry.register_pure("abs", ATOM_ABS);
    registry.register_pure("min", ATOM_MIN);
    registry.register_pure("max", ATOM_MAX);
}
