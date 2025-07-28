//! Mathematical operations for the Sutra language.
//!
//! This module provides arithmetic operations that work with numeric values.
//! All operations are pure functions that do not modify world state.
//!
//! ## Atoms Provided
//!
//! - **Basic Arithmetic**: `+`, `-`, `*`, `/`, `mod`
//! - **Mathematical Functions**: `abs`, `min`, `max`

use crate::{
    errors::{to_source_span, ErrorReporting, SutraError},
    runtime::{evaluate_ast_node, EvaluationContext, NativeFn, SpannedValue, Value},
    syntax::{AstNode, Span},
};

// ============================================================================
// COMMON HELPERS
// ============================================================================

/// Evaluates arguments and ensures they are all numbers.
fn evaluate_numeric_args(
    args: &[AstNode],
    context: &mut EvaluationContext,
) -> Result<Vec<(f64, Span)>, SutraError> {
    let mut numbers = Vec::with_capacity(args.len());
    for arg_node in args {
        let spanned_arg = evaluate_ast_node(arg_node, context)?;
        match spanned_arg.value {
            Value::Number(n) => numbers.push((n, spanned_arg.span)),
            _ => {
                return Err(context.type_mismatch(
                    "Number",
                    spanned_arg.value.type_name(),
                    to_source_span(spanned_arg.span),
                ));
            }
        }
    }
    Ok(numbers)
}

/// Checks arity requirements and returns appropriate error if not met.
fn check_arity(
    actual: usize,
    expected: &str,
    operation: &str,
    call_span: Span,
    context: &EvaluationContext,
) -> Result<(), SutraError> {
    match expected {
        "at least 1" => {
            if actual >= 1 {
                return Ok(());
            }
        }
        "at least 2" => {
            if actual >= 2 {
                return Ok(());
            }
        }
        "exactly 1" => {
            if actual == 1 {
                return Ok(());
            }
        }
        "exactly 2" => {
            if actual == 2 {
                return Ok(());
            }
        }
        _ => return Err(context.arity_mismatch(expected, actual, to_source_span(call_span))),
    };

    Err(context.arity_mismatch(
        &format!("{} for '{}'", expected, operation),
        actual,
        to_source_span(call_span),
    ))
}

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
pub const ATOM_ADD: NativeFn = |args, context, call_span| {
    check_arity(args.len(), "at least 2", "+", *call_span, context)?;
    let numbers = evaluate_numeric_args(args, context)?;
    let sum: f64 = numbers.iter().map(|(n, _)| n).sum();
    Ok(SpannedValue {
        value: Value::Number(sum),
        span: *call_span,
    })
};

/// Subtracts numbers.
///
/// Usage: (- <a> <b> ...)
///   - <a>: Number (minuend)
///   - <b>, ...: Numbers (subtrahends)
///
///   Returns: Number (difference)
///
/// Single argument returns negation: (- <a>) => -a
/// Multiple arguments: (- <a> <b> <c>) => a - b - c
///
/// Example:
///   (- 5 2) ; => 3
///   (- 10) ; => -10
pub const ATOM_SUB: NativeFn = |args, context, call_span| {
    check_arity(args.len(), "at least 1", "-", *call_span, context)?;
    let numbers = evaluate_numeric_args(args, context)?;

    let first = numbers[0].0;
    if numbers.len() == 1 {
        return Ok(SpannedValue {
            value: Value::Number(-first),
            span: *call_span,
        });
    }

    let result = numbers.iter().skip(1).fold(first, |acc, (n, _)| acc - n);
    Ok(SpannedValue {
        value: Value::Number(result),
        span: *call_span,
    })
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
pub const ATOM_MUL: NativeFn = |args, context, call_span| {
    check_arity(args.len(), "at least 2", "*", *call_span, context)?;
    let numbers = evaluate_numeric_args(args, context)?;
    let product: f64 = numbers.iter().map(|(n, _)| n).product();
    Ok(SpannedValue {
        value: Value::Number(product),
        span: *call_span,
    })
};

/// Divides numbers.
///
/// Usage: (/ <a> <b> ...)
///   - <a>: Number (dividend)
///   - <b>, ...: Numbers (divisors)
///
///   Returns: Number (quotient)
///
/// Single argument returns reciprocal: (/ <a>) => 1/a
/// Multiple arguments: (/ <a> <b> <c>) => a / b / c
///
/// Example:
///   (/ 6 2) ; => 3
///   (/ 4) ; => 0.25
///
/// Note: Errors on division by zero.
pub const ATOM_DIV: NativeFn = |args, context, call_span| {
    check_arity(args.len(), "at least 1", "/", *call_span, context)?;
    let numbers = evaluate_numeric_args(args, context)?;

    let first = numbers[0].0;
    if numbers.len() == 1 {
        if first == 0.0 {
            return Err(context.invalid_operation(
                "division",
                "zero",
                to_source_span(numbers[0].1),
            ));
        }
        return Ok(SpannedValue {
            value: Value::Number(1.0 / first),
            span: *call_span,
        });
    }

    let mut result = first;
    for (n, span) in numbers.iter().skip(1) {
        if *n == 0.0 {
            return Err(context.invalid_operation("division", "zero", to_source_span(*span)));
        }
        result /= n;
    }

    Ok(SpannedValue {
        value: Value::Number(result),
        span: *call_span,
    })
};

/// Modulo operation.
///
/// Usage: (mod <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Number (a % b)
///
/// Example:
///   (mod 5 2) ; => 1
///
/// Note: Errors on division by zero.
pub const ATOM_MOD: NativeFn = |args, context, call_span| {
    check_arity(args.len(), "exactly 2", "mod", *call_span, context)?;
    let numbers = evaluate_numeric_args(args, context)?;

    let (a, _) = numbers[0];
    let (b, b_span) = numbers[1];

    if b == 0.0 {
        return Err(context.invalid_operation("modulo", "zero", to_source_span(b_span)));
    }

    Ok(SpannedValue {
        value: Value::Number(a % b),
        span: *call_span,
    })
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
pub const ATOM_ABS: NativeFn = |args, context, call_span| {
    check_arity(args.len(), "exactly 1", "abs", *call_span, context)?;
    let numbers = evaluate_numeric_args(args, context)?;
    let n = numbers[0].0;
    Ok(SpannedValue {
        value: Value::Number(n.abs()),
        span: *call_span,
    })
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
pub const ATOM_MIN: NativeFn = |args, context, call_span| {
    check_arity(args.len(), "at least 1", "min", *call_span, context)?;
    let numbers = evaluate_numeric_args(args, context)?;
    let min = numbers
        .iter()
        .map(|(n, _)| *n)
        .fold(f64::INFINITY, f64::min);
    Ok(SpannedValue {
        value: Value::Number(min),
        span: *call_span,
    })
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
pub const ATOM_MAX: NativeFn = |args, context, call_span| {
    check_arity(args.len(), "at least 1", "max", *call_span, context)?;
    let numbers = evaluate_numeric_args(args, context)?;
    let max = numbers
        .iter()
        .map(|(n, _)| *n)
        .fold(f64::NEG_INFINITY, f64::max);
    Ok(SpannedValue {
        value: Value::Number(max),
        span: *call_span,
    })
};
