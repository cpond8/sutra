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
use crate::atoms::PureAtomFn;
use crate::atoms::helpers::extract_number;
use crate::err_ctx;
use crate::err_msg;

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
pub const ATOM_ADD: PureAtomFn = |args| {
    let mut sum = 0.0;
    for arg in args.iter() {
        sum += extract_number(arg)?;
    }
    Ok(Value::Number(sum))
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
pub const ATOM_SUB: PureAtomFn = |args| {
    if args.is_empty() {
        return Err(err_ctx!(Eval, "- expects at least 1 argument, got {}", "-", "Arity error"));
    }

    let first = extract_number(&args[0])?;
    if args.len() == 1 {
        return Ok(Value::Number(-first));
    }

    let mut result = first;
    for arg in args.iter().skip(1) {
        result -= extract_number(arg)?;
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
pub const ATOM_MUL: PureAtomFn = |args| {
    let mut product = 1.0;
    for arg in args.iter() {
        product *= extract_number(arg)?;
    }
    Ok(Value::Number(product))
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
/// Note: Errors on division by zero.
pub const ATOM_DIV: PureAtomFn = |args| {
    if args.is_empty() {
        return Err(err_ctx!(Eval, "/ expects at least 1 argument, got {}", "/", "Arity error"));
    }

    let first = extract_number(&args[0])?;
    if args.len() == 1 {
        if first == 0.0 {
            return Err(err_msg!(Eval, "division by zero"));
        }
        return Ok(Value::Number(1.0 / first));
    }

    let mut result = first;
    for arg in args.iter().skip(1) {
        let n = extract_number(arg)?;
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
    if args.len() != 2 {
        return Err(err_ctx!(Eval, "mod expects 2 arguments, got {}", "mod", "Arity error"));
    }
    let a = extract_number(&args[0])?;
    let b = extract_number(&args[1])?;

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
pub const ATOM_ABS: PureAtomFn = |args| {
    if args.len() != 1 {
        return Err(err_ctx!(Eval, "abs expects 1 argument, got {}", "abs", "Arity error"));
    }
    let n = extract_number(&args[0])?;
    Ok(Value::Number(n.abs()))
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
pub const ATOM_MIN: PureAtomFn = |args| {
    if args.is_empty() {
        return Err(err_ctx!(Eval, "min expects at least 1 argument, got {}", "min", "Arity error"));
    }

    let mut result = f64::INFINITY;
    for arg in args.iter() {
        let n = extract_number(arg)?;
        result = result.min(n);
    }
    Ok(Value::Number(result))
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
    if args.is_empty() {
        return Err(err_ctx!(Eval, "max expects at least 1 argument, got {}", "max", "Arity error"));
    }

    let mut result = f64::NEG_INFINITY;
    for arg in args.iter() {
        let n = extract_number(arg)?;
        result = result.max(n);
    }
    Ok(Value::Number(result))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all mathematical atoms with the given registry.
pub fn register_math_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // Arithmetic operations
    registry.register("+", crate::atoms::Atom::Pure(ATOM_ADD));
    registry.register("-", crate::atoms::Atom::Pure(ATOM_SUB));
    registry.register("*", crate::atoms::Atom::Pure(ATOM_MUL));
    registry.register("/", crate::atoms::Atom::Pure(ATOM_DIV));
    registry.register("mod", crate::atoms::Atom::Pure(ATOM_MOD));

    // Math functions
    registry.register("abs", crate::atoms::Atom::Pure(ATOM_ABS));
    registry.register("min", crate::atoms::Atom::Pure(ATOM_MIN));
    registry.register("max", crate::atoms::Atom::Pure(ATOM_MAX));
}
