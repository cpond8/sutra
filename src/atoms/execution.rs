//! # Execution Control
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

use crate::ast::value::Value;
use crate::ast::{Expr, WithSpan};
use crate::atoms::helpers::*;
use crate::atoms::AtomFn;
use crate::runtime::eval::eval_expr;
use crate::syntax::error::{EvalError, SutraError, SutraErrorKind};

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
pub const ATOM_DO: AtomFn = |args, context, _| {
    // The `eval_args` helper function correctly threads the world state
    // through the evaluation of each argument. We can simply use it
    // and return the value of the last expression, which is the
    // standard behavior of a `do` block.
    let (values, world) = eval_args(args, context)?;
    let last_value = values.last().cloned().unwrap_or_default();
    Ok((last_value, world))
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
pub const ATOM_ERROR: AtomFn = |args, context, parent_span| {
    let (val, _world) = eval_single_arg(args, context, parent_span, "error")?;
    let Value::String(msg) = val else {
        return Err(type_error(
            Some(parent_span.clone()),
            &args[0],
            "error",
            "a String",
            &val,
        ));
    };
    Err(SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            message: msg,
            expanded_code: format!(
                "{:?}",
                WithSpan {
                    value: Expr::List(args.to_vec(), parent_span.clone()),
                    span: parent_span.clone(),
                }
            ),
            original_code: None,
            suggestion: None,
        }),
        span: Some(parent_span.clone()),
    })
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
///
/// # Safety
/// Pure, does not mutate state. All state is explicit.
pub const ATOM_APPLY: AtomFn = |args, context, parent_span| {
    if args.len() < 2 {
        return Err(arity_error(
            Some(parent_span.clone()),
            args,
            "apply",
            "at least 2",
        ));
    }

    let func_expr = &args[0];
    let normal_args_slice = &args[1..args.len() - 1];
    let list_arg = &args[args.len() - 1];

    // Evaluate normal arguments
    let (normal_args, world) = eval_apply_normal_args(normal_args_slice, context)?;

    // Evaluate list argument
    let mut context_with_world = sub_eval_context!(context, &world);
    let (list_args, world) = eval_apply_list_arg(list_arg, &mut context_with_world, parent_span)?;

    // Build and evaluate the call expression
    let call_expr = build_apply_call_expr(func_expr, normal_args, list_args, parent_span);
    let mut sub_context = sub_eval_context!(context, &world);
    eval_expr(&call_expr, &mut sub_context)
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all execution control atoms with the given registry.
pub fn register_execution_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // Control flow
    registry.register("do", ATOM_DO);
    registry.register("error", ATOM_ERROR);

    // Higher-order functions
    registry.register("apply", ATOM_APPLY);
}
