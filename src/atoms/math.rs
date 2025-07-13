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

/// Helper function to extract a number from a Value
fn extract_number(value: &Value, _index: Option<usize>, atom_name: &str) -> Result<f64, SutraError> {
    match value {
        Value::Number(n) => Ok(*n),
        _ => Err(SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                kind: crate::syntax::error::EvalErrorKind::Type {
                    func_name: atom_name.to_string(),
                    expected: "Number".to_string(),
                    found: value.clone(),
                },
                expanded_code: String::new(),
                original_code: None,
            }),
            span: None,
        }),
    }
}

/// Helper function to create arity error
fn arity_error(actual: usize, expected: usize, atom_name: &str) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            kind: crate::syntax::error::EvalErrorKind::Arity {
                func_name: atom_name.to_string(),
                expected: expected.to_string(),
                actual,
            },
            expanded_code: String::new(),
            original_code: None,
        }),
        span: None,
    }
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
pub const ATOM_ADD: PureAtomFn = |args| {
    let mut sum = 0.0;
    for (i, arg) in args.iter().enumerate() {
        sum += extract_number(arg, Some(i), "+")?;
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
        return Err(arity_error(args.len(), 1, "-"));
    }

    let first = extract_number(&args[0], Some(0), "-")?;
    if args.len() == 1 {
        return Ok(Value::Number(-first));
    }

    let mut result = first;
    for (i, arg) in args.iter().enumerate().skip(1) {
        result -= extract_number(arg, Some(i), "-")?;
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
    for (i, arg) in args.iter().enumerate() {
        product *= extract_number(arg, Some(i), "*")?;
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
        return Err(arity_error(args.len(), 1, "/"));
    }

    let first = extract_number(&args[0], Some(0), "/")?;
    if args.len() == 1 {
        if first == 0.0 {
            return Err(SutraError {
                kind: SutraErrorKind::Eval(EvalError {
                    kind: crate::syntax::error::EvalErrorKind::DivisionByZero,
                    expanded_code: String::new(),
                    original_code: None,
                }),
                span: None,
            });
        }
        return Ok(Value::Number(1.0 / first));
    }

    let mut result = first;
    for (i, arg) in args.iter().enumerate().skip(1) {
        let n = extract_number(arg, Some(i), "/")?;
        if n == 0.0 {
            return Err(SutraError {
                kind: SutraErrorKind::Eval(EvalError {
                    kind: crate::syntax::error::EvalErrorKind::DivisionByZero,
                    expanded_code: String::new(),
                    original_code: None,
                }),
                span: None,
            });
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
        return Err(arity_error(args.len(), 2, "mod"));
    }
    let a = extract_number(&args[0], Some(0), "mod")?;
    let b = extract_number(&args[1], Some(1), "mod")?;

    if b == 0.0 {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                kind: crate::syntax::error::EvalErrorKind::General(
                    "mod: modulo by zero".to_string(),
                ),
                expanded_code: String::new(),
                original_code: None,
            }),
            span: None,
        });
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
pub const ATOM_MIN: PureAtomFn = |args| {
    if args.is_empty() {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                kind: crate::syntax::error::EvalErrorKind::Arity {
                    func_name: "min".to_string(),
                    expected: "at least 1".to_string(),
                    actual: 0,
                },
                expanded_code: String::new(),
                original_code: None,
            }),
            span: None,
        });
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
pub const ATOM_MAX: PureAtomFn = |args| {
    if args.is_empty() {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                kind: crate::syntax::error::EvalErrorKind::Arity {
                    func_name: "max".to_string(),
                    expected: "at least 1".to_string(),
                    actual: 0,
                },
                expanded_code: String::new(),
                original_code: None,
            }),
            span: None,
        });
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
