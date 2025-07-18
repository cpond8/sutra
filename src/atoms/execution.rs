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

use crate::{
    atoms::{
        helpers::{
            build_apply_call_expr, eval_apply_list_arg, eval_apply_normal_args, eval_single_arg,
            sub_eval_context, validate_special_form_min_arity,
        },
        SpecialFormAtomFn,
    },
    err_msg,
    runtime::eval::evaluate_ast_node,
    AtomRegistry, Value,
};

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
pub const ATOM_DO: SpecialFormAtomFn = |args, context, _parent_span| {
    let mut last_value = Value::default();
    let mut world = context.world.clone();

    for arg in args {
        // Create a new evaluation context for the sub-expression, threading the world.
        let mut sub_context = sub_eval_context!(context, &world);
        // Evaluate the expression. `evaluate_ast_node` will return the result and the *new* world state.
        let (val, new_world) = evaluate_ast_node(arg, &mut sub_context)?;
        last_value = val;
        // Update our tracked world state for the next iteration.
        world = new_world;
    }

    // The final world state is returned along with the last value.
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
pub const ATOM_ERROR: SpecialFormAtomFn = |args, context, _parent_span| {
    let (val, _world) = eval_single_arg(args, context)?;
    let Value::String(msg) = val else {
        return Err(err_msg!(TypeError, "error expects a String argument"));
    };
    Err(err_msg!(Eval, msg))
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
pub const ATOM_APPLY: SpecialFormAtomFn = |args, context, parent_span| {
    validate_special_form_min_arity(args, 2, "apply")?;

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
    evaluate_ast_node(&call_expr, &mut sub_context)
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all execution control atoms with the given registry.
pub fn register_execution_atoms(registry: &mut AtomRegistry) {
    // Control flow
    registry.register("do", crate::atoms::Atom::SpecialForm(ATOM_DO));
    registry.register("error", crate::atoms::Atom::SpecialForm(ATOM_ERROR));

    // Higher-order functions
    registry.register("apply", crate::atoms::Atom::SpecialForm(ATOM_APPLY));
}
