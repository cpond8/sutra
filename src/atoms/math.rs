// This module provides all mathematical atom operations for the Sutra engine.
// All atoms in this module are pure functions that do not mutate world state.

use crate::prelude::*;
use crate::{
    atoms::EagerAtomFn,
    errors,
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
pub const ATOM_ADD: EagerAtomFn = |args, context| {
    helpers::validate_min_arity(args, 2, "+")?;
    let mut sum = 0.0;
    for arg in args {
        let n: f64 = arg.extract(Some(context))?;
        sum += n;
    }
    Ok((Value::Number(sum), context.world.clone()))
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
pub const ATOM_SUB: EagerAtomFn = |args, context| {
    helpers::validate_min_arity(args, 1, "-")?;
    let first: f64 = args[0].extract(Some(context))?;
    if args.len() == 1 {
        return Ok((Value::Number(-first), context.world.clone()));
    }
    let mut result = first;
    for arg in args.iter().skip(1) {
        let n: f64 = arg.extract(Some(context))?;
        result -= n;
    }
    Ok((Value::Number(result), context.world.clone()))
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
pub const ATOM_MUL: EagerAtomFn = |args, context| {
    helpers::validate_min_arity(args, 2, "*")?;
    let mut product = 1.0;
    for arg in args {
        let n: f64 = arg.extract(Some(context))?;
        product *= n;
    }
    Ok((Value::Number(product), context.world.clone()))
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
pub const ATOM_DIV: EagerAtomFn = |args, context| {
    helpers::validate_min_arity(args, 1, "/")?;
    let first: f64 = args[0].extract(Some(context))?;
    if args.len() == 1 {
        if first == 0.0 {
            return Err(errors::runtime_general(
                "division by zero",
                context.current_file(),
                context.current_source(),
                // It is not currently possible to propagate a proper span to `extract`.
                // This is a known limitation that should be addressed in a future refactoring.
                context.span_for_span(Span::default()),
            ));
        }
        return Ok((Value::Number(1.0 / first), context.world.clone()));
    }
    let mut result = first;
    for arg in args.iter().skip(1) {
        let n: f64 = arg.extract(Some(context))?;
        if n == 0.0 {
            return Err(errors::runtime_general(
                "division by zero",
                context.current_file(),
                context.current_source(),
                context.span_for_span(Span::default()),
            ));
        }
        result /= n;
    }
    Ok((Value::Number(result), context.world.clone()))
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
pub const ATOM_MOD: EagerAtomFn = |args, context| {
    helpers::validate_binary_arity(args, "mod")?;
    let a: f64 = args[0].extract(Some(context))?;
    let b: f64 = args[1].extract(Some(context))?;
    if b == 0.0 {
        return Err(errors::runtime_general(
            "modulo by zero",
            context.current_file(),
            context.current_source(),
            context.span_for_span(Span::default()),
        ));
    }
    Ok((Value::Number(a % b), context.world.clone()))
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
pub const ATOM_ABS: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "abs")?;
    let n: f64 = args[0].extract(Some(context))?;
    Ok((Value::Number(n.abs()), context.world.clone()))
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
pub const ATOM_MIN: EagerAtomFn = |args, context| {
    helpers::validate_min_arity(args, 1, "min")?;
    let mut min = f64::INFINITY;
    for arg in args {
        let n: f64 = arg.extract(Some(context))?;
        min = min.min(n);
    }
    Ok((Value::Number(min), context.world.clone()))
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
pub const ATOM_MAX: EagerAtomFn = |args, context| {
    helpers::validate_min_arity(args, 1, "max")?;
    let mut max = f64::NEG_INFINITY;
    for arg in args {
        let n: f64 = arg.extract(Some(context))?;
        max = max.max(n);
    }
    Ok((Value::Number(max), context.world.clone()))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all mathematical atoms with the given registry.
pub fn register_math_atoms(registry: &mut AtomRegistry) {
    // Arithmetic operations
    registry.register_eager("+", ATOM_ADD);
    registry.register_eager("-", ATOM_SUB);
    registry.register_eager("*", ATOM_MUL);
    registry.register_eager("/", ATOM_DIV);
    registry.register_eager("mod", ATOM_MOD);

    // Math functions
    registry.register_eager("abs", ATOM_ABS);
    registry.register_eager("min", ATOM_MIN);
    registry.register_eager("max", ATOM_MAX);
}
