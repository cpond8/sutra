//!
//! This module provides execution control atom operations for the Sutra engine.
//! These atoms control program flow and higher-order function application.
//!
//! ## Atoms Provided
//!
//! - **Control Flow**: `do`, `error`
//! - **Higher-Order**: `apply`
//!
//! ## Design Principles
//!
//! - **Flow Control**: Sequential execution and error handling
//! - **Meta-Programming**: Function application with argument flattening
//! - **State Threading**: Proper world state propagation through execution

use crate::prelude::*;
use crate::{helpers, runtime::eval, NativeLazyFn};
use crate::errors;

// ============================================================================
// CONTROL FLOW OPERATIONS
// ============================================================================

/// Sequentially evaluates expressions, returning the last value.
///
/// Usage: (do <expr1> <expr2> ...)
///   - <expr1>, <expr2>, ...: Expressions to evaluate in sequence
///
///   Returns: Value of last expression
///
/// Example:
///   (do (core/set! x 1) (core/get x)) ; => 1
///
/// # Safety
/// May mutate world if inner expressions do.
pub const ATOM_DO: NativeLazyFn = |args, context, _parent_span| {
    let mut last_value = Value::default();

    for arg in args {
        // Create a new evaluation context for the sub-expression.
        let mut sub_context = helpers::sub_eval_context!(context);
        // Evaluate the expression. The world is mutated via the shared context.
        last_value = eval::evaluate_ast_node(arg, &mut sub_context)?;
    }

    // Return the value of the last expression.
    Ok(last_value)
};

/// Raises an error with a message.
///
/// Usage: (error <message>)
///   - <message>: String
///
///   Returns: Error (never returns normally)
///
/// Example:
///   (error "fail!")
///
/// # Safety
/// Always errors. Does not mutate state.
pub const ATOM_ERROR: NativeLazyFn = |args, context, _parent_span| {
    let val = helpers::eval_single_arg(args, context)?;
    let Value::String(msg) = val else {
        return Err(errors::type_mismatch(
            "String",
            val.type_name(),
            context.current_file(),
            context.current_source(),
            context.span_for_span(Span::default()),
        ));
    };
    Err(errors::runtime_general(
        msg,
        context.current_file(),
        context.current_source(),
        context.span_for_span(Span::default()),
    ))
};

// ============================================================================
// HIGHER-ORDER OPERATIONS
// ============================================================================

/// Calls a function, macro, or atom with arguments, flattening the final list argument.
///
/// Usage: (apply <function> <arg1> <arg2> ... <list>)
///   - <function>: Function to call
///   - <arg1>, <arg2>, ...: Normal arguments
///   - <list>: List of additional arguments to flatten
///
///   Returns: Result of function call
///
/// Example:
///   (apply + 1 2 (list 3 4)) ; => 10
///   (apply core/str+ (list "a" "b" "c")) ; => "abc"
pub const ATOM_APPLY: NativeLazyFn = |args, context, parent_span| {
    helpers::validate_special_form_min_arity(args, 2, "apply")?;

    let func_expr = &args[0];
    let normal_args_slice = &args[1..args.len() - 1];
    let list_arg = &args[args.len() - 1];

    // Evaluate normal arguments
    let normal_args = helpers::eval_apply_normal_args(normal_args_slice, context)?;

    // Evaluate list argument
    let list_args = helpers::eval_apply_list_arg(list_arg, context, parent_span)?;

    // Build and evaluate the call expression in a new sub-context for recursion depth.
    let call_expr = helpers::build_apply_call_expr(func_expr, normal_args, list_args, parent_span);
    let mut sub_context = helpers::sub_eval_context!(context);
    eval::evaluate_ast_node(&call_expr, &mut sub_context)
};
