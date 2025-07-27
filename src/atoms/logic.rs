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

use crate::{
    ast::{
        spanned_value::{SpannedResult, SpannedValue},
        value::{NativeFn, Value},
        AstNode,
    },
    engine::evaluate_ast_node,
    errors::{to_source_span, ErrorReporting},
    prelude::*,
};

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
pub const ATOM_EQ: NativeFn = |args, context, call_span| {
    if args.len() < 2 {
        return Err(context.arity_mismatch(
            "at least 2 for 'eq?'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(evaluate_ast_node(arg, context)?);
    }

    for window in evaluated_args.windows(2) {
        if window[0].value != window[1].value {
            return Ok(SpannedValue {
                value: Value::Bool(false),
                span: *call_span,
            });
        }
    }

    Ok(SpannedValue {
        value: Value::Bool(true),
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
pub const ATOM_GT: NativeFn = |args: &[AstNode], context: &mut EvaluationContext, call_span: &Span| -> SpannedResult {
    if args.len() < 2 {
        return Err(context.arity_mismatch(
            "at least 2 for 'gt?'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(evaluate_ast_node(arg, context)?);
    }

    for window in evaluated_args.windows(2) {
        let a = match &window[0].value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", window[0].value.type_name(), to_source_span(window[0].span)));
            }
        };
        let b = match &window[1].value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", window[1].value.type_name(), to_source_span(window[1].span)));
            }
        };

        if a <= b {
            return Ok(SpannedValue {
                value: Value::Bool(false),
                span: *call_span,
            });
        }
    }

    Ok(SpannedValue {
        value: Value::Bool(true),
        span: *call_span,
    })
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
pub const ATOM_LT: NativeFn = |args: &[AstNode], context: &mut EvaluationContext, call_span: &Span| -> SpannedResult {
    if args.len() < 2 {
        return Err(context.arity_mismatch(
            "at least 2 for 'lt?'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(evaluate_ast_node(arg, context)?);
    }

    for window in evaluated_args.windows(2) {
        let a = match &window[0].value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", window[0].value.type_name(), to_source_span(window[0].span)));
            }
        };
        let b = match &window[1].value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", window[1].value.type_name(), to_source_span(window[1].span)));
            }
        };

        if a >= b {
            return Ok(SpannedValue {
                value: Value::Bool(false),
                span: *call_span,
            });
        }
    }

    Ok(SpannedValue {
        value: Value::Bool(true),
        span: *call_span,
    })
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
pub const ATOM_GTE: NativeFn = |args: &[AstNode], context: &mut EvaluationContext, call_span: &Span| -> SpannedResult {
    if args.len() < 2 {
        return Err(context.arity_mismatch(
            "at least 2 for 'gte?'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(evaluate_ast_node(arg, context)?);
    }

    for window in evaluated_args.windows(2) {
        let a = match &window[0].value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", window[0].value.type_name(), to_source_span(window[0].span)));
            }
        };
        let b = match &window[1].value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", window[1].value.type_name(), to_source_span(window[1].span)));
            }
        };

        if a < b {
            return Ok(SpannedValue {
                value: Value::Bool(false),
                span: *call_span,
            });
        }
    }

    Ok(SpannedValue {
        value: Value::Bool(true),
        span: *call_span,
    })
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
pub const ATOM_LTE: NativeFn = |args: &[AstNode], context: &mut EvaluationContext, call_span: &Span| -> SpannedResult {
    if args.len() < 2 {
        return Err(context.arity_mismatch(
            "at least 2 for 'lte?'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let mut evaluated_args = Vec::new();
    for arg in args {
        evaluated_args.push(evaluate_ast_node(arg, context)?);
    }

    for window in evaluated_args.windows(2) {
        let a = match &window[0].value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", window[0].value.type_name(), to_source_span(window[0].span)));
            }
        };
        let b = match &window[1].value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", window[1].value.type_name(), to_source_span(window[1].span)));
            }
        };

        if a > b {
            return Ok(SpannedValue {
                value: Value::Bool(false),
                span: *call_span,
            });
        }
    }

    Ok(SpannedValue {
        value: Value::Bool(true),
        span: *call_span,
    })
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
pub const ATOM_NOT: NativeFn = |args: &[AstNode], context: &mut EvaluationContext, call_span: &Span| -> SpannedResult {
    if args.len() != 1 {
        return Err(context.arity_mismatch(
            "exactly 1 for 'not'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let spanned_arg = evaluate_ast_node(&args[0], context)?;
    let b = match spanned_arg.value {
        Value::Bool(b) => b,
        _ => {
            return Err(context.type_mismatch("Bool", spanned_arg.value.type_name(), to_source_span(spanned_arg.span)));
        }
    };

    Ok(SpannedValue {
        value: Value::Bool(!b),
        span: *call_span,
    })
};
