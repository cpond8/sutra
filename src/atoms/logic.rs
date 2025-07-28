//! Logic and comparison operations for the Sutra language.
//!
//! This module provides boolean logic and value comparison operations.
//! All operations are pure functions that return boolean results.
//!
//! ## Atoms Provided
//!
//! - **Equality**: `eq?` (aliases: `=`, `is?`)
//! - **Comparison**: `gt?` (`>`), `lt?` (`<`), `gte?` (`>=`), `lte?` (`<=`)
//! - **Aliases**: `over?`, `under?`, `at-least?`, `at-most?`
//! - **Logic**: `not`
//!
//! ## Design Notes
//!
//! Comparison operations work primarily with numeric values and return boolean results.
//! Multiple aliases are provided for convenience and readability.

use crate::{
    errors::{to_source_span, ErrorReporting, SutraError},
    prelude::*,
    runtime::{evaluate_ast_node, NativeFn, SpannedResult, SpannedValue, Value},
    syntax::AstNode,
};

// ============================================================================
// CORE HELPERS
// ============================================================================

/// Evaluates arguments and ensures minimum arity
fn evaluate_args_with_min_arity(
    args: &[AstNode],
    context: &mut EvaluationContext,
    min_arity: usize,
    fn_name: &str,
    call_span: Span,
) -> Result<Vec<SpannedValue>, SutraError> {
    if args.len() < min_arity {
        return Err(context.arity_mismatch(
            &format!("at least {} for '{}'", min_arity, fn_name),
            args.len(),
            to_source_span(call_span),
        ));
    }

    args.iter()
        .map(|arg| evaluate_ast_node(arg, context))
        .collect()
}

/// Extracts numbers from spanned values, returning type errors for non-numbers
fn extract_numbers(
    values: &[SpannedValue],
    context: &mut EvaluationContext,
) -> Result<Vec<f64>, SutraError> {
    values
        .iter()
        .map(|v| match &v.value {
            Value::Number(n) => Ok(*n),
            _ => Err(context.type_mismatch("Number", v.value.type_name(), to_source_span(v.span))),
        })
        .collect()
}

/// Generic chain comparison - applies comparison function across consecutive pairs
fn chain_compare<F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    call_span: &Span,
    fn_name: &str,
    compare_fn: F,
) -> SpannedResult
where
    F: Fn(f64, f64) -> bool,
{
    let values = evaluate_args_with_min_arity(args, context, 2, fn_name, *call_span)?;
    let numbers = extract_numbers(&values, context)?;

    let result = numbers.windows(2).all(|pair| compare_fn(pair[0], pair[1]));

    Ok(SpannedValue {
        value: Value::Bool(result),
        span: *call_span,
    })
}

// ============================================================================
// EXPORTED ATOMS
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
pub const ATOM_EQ: NativeFn = |args, context, call_span| {
    let values = evaluate_args_with_min_arity(args, context, 2, "eq?", *call_span)?;

    let result = values.windows(2).all(|pair| pair[0].value == pair[1].value);

    Ok(SpannedValue {
        value: Value::Bool(result),
        span: *call_span,
    })
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
pub const ATOM_GT: NativeFn =
    |args, context, call_span| chain_compare(args, context, call_span, "gt?", |a, b| a > b);

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
pub const ATOM_LT: NativeFn =
    |args, context, call_span| chain_compare(args, context, call_span, "lt?", |a, b| a < b);

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
pub const ATOM_GTE: NativeFn =
    |args, context, call_span| chain_compare(args, context, call_span, "gte?", |a, b| a >= b);

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
pub const ATOM_LTE: NativeFn =
    |args, context, call_span| chain_compare(args, context, call_span, "lte?", |a, b| a <= b);

/// Logical negation.
///
/// Usage: (not <a>)
///   - <a>: Boolean
///
///   Returns: Bool
///
/// Example:
///   (not true) ; => false
pub const ATOM_NOT: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch(
            "exactly 1 for 'not'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let value = evaluate_ast_node(&args[0], context)?;

    match value.value {
        Value::Bool(b) => Ok(SpannedValue {
            value: Value::Bool(!b),
            span: *call_span,
        }),
        _ => {
            Err(context.type_mismatch("Bool", value.value.type_name(), to_source_span(value.span)))
        }
    }
};
