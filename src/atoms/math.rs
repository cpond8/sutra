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
use crate::atoms::PureAtomFn;
use crate::syntax::error::{SutraError, SutraErrorKind, EvalError};

// ============================================================================
// ARITHMETIC OPERATIONS
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
fn extract_number(value: &Value, index: Option<usize>, atom_name: &str) -> Result<f64, SutraError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(simple_error(&format!("{}: expected Number at position {:?}, got {:?}", atom_name, index, value))),
    }
}

/// Helper function to create arity error
fn arity_error(actual: usize, expected: usize, atom_name: &str) -> SutraError {
    simple_error(&format!("{}: expected {} arguments, got {}", atom_name, expected, actual))
}

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
pub const ATOM_ADD: PureAtomFn = |args| {
    if args.is_empty() {
        return Ok(Value::Number(0.0));
    }

    let mut result = 0.0;
    for (i, arg) in args.iter().enumerate() {
        let n = extract_number(arg, Some(i), "+")?;
        result += n;
    }
    Ok(Value::Number(result))
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
pub const ATOM_SUB: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "-"));
    }
    let a = extract_number(&args[0], Some(0), "-")?;
    let b = extract_number(&args[1], Some(1), "-")?;
    Ok(Value::Number(a - b))
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
pub const ATOM_MUL: PureAtomFn = |args| {
    if args.is_empty() {
        return Ok(Value::Number(1.0));
    }

    let mut result = 1.0;
    for (i, arg) in args.iter().enumerate() {
        let n = extract_number(arg, Some(i), "*")?;
        result *= n;
    }
    Ok(Value::Number(result))
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
pub const ATOM_DIV: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "/"));
    }
    let a = extract_number(&args[0], Some(0), "/")?;
    let b = extract_number(&args[1], Some(1), "/")?;

    if b == 0.0 {
        return Err(simple_error("/: division by zero"));
    }

    Ok(Value::Number(a / b))
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
pub const ATOM_MOD: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(args.len(), 2, "mod"));
    }
    let a = extract_number(&args[0], Some(0), "mod")?;
    let b = extract_number(&args[1], Some(1), "mod")?;

    if b == 0.0 {
        return Err(simple_error("mod: modulo by zero"));
    }

    if a.fract() != 0.0 || b.fract() != 0.0 {
        return Err(simple_error("mod: expects integer arguments"));
    }

    Ok(Value::Number((a as i64 % b as i64) as f64))
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
pub const ATOM_ABS: PureAtomFn = |args| {
    if args.len() != 1 {
        return Err(arity_error(args.len(), 1, "abs"));
    }
    let n = extract_number(&args[0], Some(0), "abs")?;
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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MIN: PureAtomFn = |args| {
    if args.is_empty() {
        return Err(simple_error("min: requires at least 1 argument"));
    }

    let mut result = f64::INFINITY;
    for (i, arg) in args.iter().enumerate() {
        let n = extract_number(arg, Some(i), "min")?;
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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MAX: PureAtomFn = |args| {
    if args.is_empty() {
        return Err(simple_error("max: requires at least 1 argument"));
    }

    let mut result = f64::NEG_INFINITY;
    for (i, arg) in args.iter().enumerate() {
        let n = extract_number(arg, Some(i), "max")?;
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
