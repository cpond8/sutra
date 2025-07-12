//! # Mathematical Operations
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

use crate::ast::value::Value;
use crate::atoms::helpers::*;
use crate::atoms::AtomFn;

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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_ADD: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op(args, context, parent_span, 0.0, |a, b| a + b, "+")
};

/// Subtracts two numbers.
///
/// Usage: (- <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Number (a - b)
///
/// Example:
///   (- 5 2) ; => 3
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_SUB: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Number(a - b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "-",
    )
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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MUL: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op(args, context, parent_span, 1.0, |a, b| a * b, "*")
};

/// Divides two numbers.
///
/// Usage: (/ <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Number (a / b)
///
/// Example:
///   (/ 6 2) ; => 3
///
/// # Safety
/// Pure, does not mutate state. Errors on division by zero.
pub const ATOM_DIV: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Number(a / b),
        Some(|_a, b| {
            if b == 0.0 {
                Err("Division by zero")
            } else {
                Ok(())
            }
        }),
        "/",
    )
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
/// # Safety
/// Pure, does not mutate state. Errors on division by zero or non-integer input.
pub const ATOM_MOD: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Number((a as i64 % b as i64) as f64),
        Some(|a: f64, b: f64| -> Result<(), &'static str> {
            if b == 0.0 {
                return Err("Modulo by zero");
            }
            if a.fract() != 0.0 || b.fract() != 0.0 {
                return Err("Modulo expects integers");
            }
            Ok(())
        }),
        "mod",
    )
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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_ABS: AtomFn = |args, context, parent_span| {
    let (val, world) = eval_single_arg(args, context, parent_span, "abs")?;
    let n = extract_number(&val, args, parent_span, "abs")?;
    Ok((Value::Number(n.abs()), world))
};

/// Minimum of multiple numbers.
///
/// Usage: (min <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (minimum value)
///
/// Example:
///   (min 3 1 4) ; => 1
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MIN: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op(args, context, parent_span, f64::INFINITY, f64::min, "min")
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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MAX: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op(
        args,
        context,
        parent_span,
        f64::NEG_INFINITY,
        f64::max,
        "max",
    )
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all mathematical atoms with the given registry.
pub fn register_math_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // Arithmetic operations
    registry.register("+", ATOM_ADD);
    registry.register("-", ATOM_SUB);
    registry.register("*", ATOM_MUL);
    registry.register("/", ATOM_DIV);
    registry.register("mod", ATOM_MOD);

    // Math functions
    registry.register("abs", ATOM_ABS);
    registry.register("min", ATOM_MIN);
    registry.register("max", ATOM_MAX);
}
