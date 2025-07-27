// This module provides all mathematical atom operations for the Sutra engine.
// All atoms in this module are pure functions that do not mutate world state.

use crate::prelude::*;
use crate::{
    errors::{ErrorReporting, ErrorKind},
    helpers::{self, ExtractValue},
    NativeEagerFn,
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
pub const ATOM_ADD: NativeEagerFn = |args, context| {
    helpers::validate_min_arity(args, 2, "+", context)?;
    let mut sum = 0.0;
    for arg in args {
        let n: f64 = arg.extract(context)?;
        sum += n;
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
pub const ATOM_SUB: NativeEagerFn = |args, context| {
    helpers::validate_min_arity(args, 1, "-", context)?;
    let first: f64 = args[0].extract(context)?;
    if args.len() == 1 {
        return Ok(Value::Number(-first));
    }
    let mut result = first;
    for arg in args.iter().skip(1) {
        let n: f64 = arg.extract(context)?;
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
pub const ATOM_MUL: NativeEagerFn = |args, context| {
    helpers::validate_min_arity(args, 2, "*", context)?;
    let mut product = 1.0;
    for arg in args {
        let n: f64 = arg.extract(context)?;
        product *= n;
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
pub const ATOM_DIV: NativeEagerFn = |args, context| {
    helpers::validate_min_arity(args, 1, "/", context)?;
    let first: f64 = args[0].extract(context)?;
    if args.len() == 1 {
        if first == 0.0 {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "division".to_string(),
                    operand_type: "zero".to_string(),
                },
                context.span_for_span(context.current_span),
            ));
        }
        return Ok(Value::Number(1.0 / first));
    }
    let mut result = first;
    for arg in args.iter().skip(1) {
        let n: f64 = arg.extract(context)?;
        if n == 0.0 {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "division".to_string(),
                    operand_type: "zero".to_string(),
                },
                context.span_for_span(context.current_span),
            ));
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
pub const ATOM_MOD: NativeEagerFn = |args, context| {
    helpers::validate_binary_arity(args, "mod", context)?;
    let a: f64 = args[0].extract(context)?;
    let b: f64 = args[1].extract(context)?;
    if b == 0.0 {
        return Err(context.report(
            ErrorKind::InvalidOperation {
                operation: "modulo".to_string(),
                operand_type: "zero".to_string(),
            },
            context.span_for_span(context.current_span),
        ));
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
pub const ATOM_ABS: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "abs", context)?;
    let n: f64 = args[0].extract(context)?;
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
pub const ATOM_MIN: NativeEagerFn = |args, context| {
    helpers::validate_min_arity(args, 1, "min", context)?;
    let mut min = f64::INFINITY;
    for arg in args {
        let n: f64 = arg.extract(context)?;
        min = min.min(n);
    }
    Ok(Value::Number(min))
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
pub const ATOM_MAX: NativeEagerFn = |args, context| {
    helpers::validate_min_arity(args, 1, "max", context)?;
    let mut max = f64::NEG_INFINITY;
    for arg in args {
        let n: f64 = arg.extract(context)?;
        max = max.max(n);
    }
    Ok(Value::Number(max))
};
