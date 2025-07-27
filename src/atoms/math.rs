// This module provides all mathematical atom operations for the Sutra engine.
// All atoms in this module are pure functions that do not mutate world state.

use crate::{
    ast::{
        spanned_value::SpannedValue,
        value::{NativeFn, Value},
    },
    engine::evaluate_ast_node,
    errors::{to_source_span, ErrorReporting},
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
pub const ATOM_ADD: NativeFn = |args, context, call_span| {
    if args.len() < 2 {
        return Err(context.arity_mismatch(
            "at least 2 for '+'",
            args.len(),
            to_source_span(*call_span),
        ));
    }
    let mut sum = 0.0;
    for arg_node in args {
        let spanned_arg = evaluate_ast_node(arg_node, context)?;
        let n = match spanned_arg.value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", spanned_arg.value.type_name(), to_source_span(spanned_arg.span)));
            }
        };
        sum += n;
    }
    Ok(SpannedValue {
        value: Value::Number(sum),
        span: *call_span,
    })
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
pub const ATOM_SUB: NativeFn = |args, context, call_span| {
    if args.is_empty() {
        return Err(context.arity_mismatch(
            "at least 1 for '-'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let mut evaluated_args = Vec::new();
    for arg_node in args {
        evaluated_args.push(evaluate_ast_node(arg_node, context)?);
    }

    let first = match evaluated_args[0].value {
        Value::Number(n) => n,
        _ => {
            return Err(context.type_mismatch("Number", evaluated_args[0].value.type_name(), to_source_span(evaluated_args[0].span)));
        }
    };

    if args.len() == 1 {
        return Ok(SpannedValue {
            value: Value::Number(-first),
            span: *call_span,
        });
    }

    let mut result = first;
    for spanned_arg in evaluated_args.iter().skip(1) {
        let n = match spanned_arg.value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", spanned_arg.value.type_name(), to_source_span(spanned_arg.span)));
            }
        };
        result -= n;
    }
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
    if args.len() < 2 {
        return Err(context.arity_mismatch(
            "at least 2 for '*'",
            args.len(),
            to_source_span(*call_span),
        ));
    }
    let mut product = 1.0;
    for arg_node in args {
        let spanned_arg = evaluate_ast_node(arg_node, context)?;
        let n = match spanned_arg.value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", spanned_arg.value.type_name(), to_source_span(spanned_arg.span)));
            }
        };
        product *= n;
    }
    Ok(SpannedValue {
        value: Value::Number(product),
        span: *call_span,
    })
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
pub const ATOM_DIV: NativeFn = |args, context, call_span| {
    if args.is_empty() {
        return Err(context.arity_mismatch(
            "at least 1 for '/'",
            args.len(),
            to_source_span(*call_span),
        ));
    }

    let mut evaluated_args = Vec::new();
    for arg_node in args {
        evaluated_args.push(evaluate_ast_node(arg_node, context)?);
    }

    let first_span = evaluated_args[0].span;
    let first = match evaluated_args[0].value {
        Value::Number(n) => n,
        _ => return Err(context.type_mismatch("Number", evaluated_args[0].value.type_name(), to_source_span(first_span))),
    };

    if args.len() == 1 {
        if first == 0.0 {
            return Err(context.invalid_operation("division", "zero", to_source_span(first_span)));
        }
        return Ok(SpannedValue {
            value: Value::Number(1.0 / first),
            span: *call_span,
        });
    }

    let mut result = first;
    for spanned_arg in evaluated_args.iter().skip(1) {
        let n = match spanned_arg.value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", spanned_arg.value.type_name(), to_source_span(spanned_arg.span)));
            }
        };
        if n == 0.0 {
             return Err(context.invalid_operation("division", "zero", to_source_span(spanned_arg.span)));
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
///   - <a>, <b>: Integers
///
///   Returns: Number (a % b)
///
/// Example:
///   (mod 5 2) ; => 1
///
/// Note: Errors on division by zero or non-integer input.
pub const ATOM_MOD: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2 for 'mod'", args.len(), to_source_span(*call_span)));
    }

    let mut eval_args = Vec::new();
    for arg in args {
        eval_args.push(evaluate_ast_node(arg, context)?);
    }

    let a = match eval_args[0].value {
        Value::Number(n) => n,
        _ => return Err(context.type_mismatch("Number", eval_args[0].value.type_name(), to_source_span(eval_args[0].span)))
    };
    let b = match eval_args[1].value {
        Value::Number(n) => n,
        _ => return Err(context.type_mismatch("Number", eval_args[1].value.type_name(), to_source_span(eval_args[1].span)))
    };

    if b == 0.0 {
        return Err(context.invalid_operation("modulo", "zero", to_source_span(eval_args[1].span)));
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
    if args.len() != 1 {
        return Err(context.arity_mismatch("1 for 'abs'", args.len(), to_source_span(*call_span)));
    }
    let spanned_arg = evaluate_ast_node(&args[0], context)?;
    let n = match spanned_arg.value {
        Value::Number(n) => n,
        _ => {
            return Err(context.type_mismatch("Number", spanned_arg.value.type_name(), to_source_span(spanned_arg.span)));
        }
    };
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
    if args.is_empty() {
        return Err(context.arity_mismatch("at least 1 for 'min'", args.len(), to_source_span(*call_span)));
    }

    let mut min = f64::INFINITY;
    for arg_node in args {
        let spanned_arg = evaluate_ast_node(arg_node, context)?;
        let n = match spanned_arg.value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", spanned_arg.value.type_name(), to_source_span(spanned_arg.span)));
            }
        };
        min = min.min(n);
    }
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
    if args.is_empty() {
        return Err(context.arity_mismatch("at least 1 for 'max'", args.len(), to_source_span(*call_span)));
    }
    let mut max = f64::NEG_INFINITY;
    for arg_node in args {
        let spanned_arg = evaluate_ast_node(arg_node, context)?;
        let n = match spanned_arg.value {
            Value::Number(n) => n,
            _ => {
                return Err(context.type_mismatch("Number", spanned_arg.value.type_name(), to_source_span(spanned_arg.span)));
            }
        };
        max = max.max(n);
    }
    Ok(SpannedValue {
        value: Value::Number(max),
        span: *call_span,
    })
};
