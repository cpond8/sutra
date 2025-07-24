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

use crate::errors;
use crate::prelude::*;
use crate::{helpers, runtime::eval, NativeLazyFn};

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

    for arg in args.iter() {
        last_value = eval::evaluate_ast_node(arg, context)?;
    }

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
            &context.source,
            context.span_for_span(Span::default()),
        ));
    };
    Err(errors::runtime_general(
        msg,
        "user error",
        &context.source,
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
    helpers::validate_special_form_min_arity(args, 2, "apply", context)?;

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

/// Executes a body of code for each element in a collection.
///
/// Usage: (for-each <var> <collection> <body>...)
///   - <var>: A symbol that will be bound to each element of the collection.
///   - <collection>: The list to iterate over.
///   - <body>...: One or more expressions to execute for each element.
///
/// Returns: Nil.
///
/// Example:
///   (for-each item (list 1 2 3) (print item))
pub const ATOM_FOR_EACH: NativeLazyFn = |args, context, _parent_span| {
    helpers::validate_special_form_min_arity(args, 3, "for-each", context)?;

    // First argument must be the variable symbol.
    let var_name = match &*args[0].value {
        Expr::Symbol(s, _) => s.clone(),
        _ => {
            return Err(errors::runtime_general(
                "for-each: first argument must be a symbol",
                "invalid definition",
                &context.source,
                context.span_for_node(&args[0]),
            ));
        }
    };

    // Second argument is the collection to iterate over.
    let collection_val = eval::evaluate_ast_node(&args[1], context)?;
    let items = match collection_val {
        Value::List(items) => items,
        Value::Nil => vec![], // for-each on nil does nothing.
        _ => {
            return Err(errors::type_mismatch(
                "List or Nil",
                collection_val.type_name(),
                &context.source,
                context.span_for_node(&args[1]),
            ));
        }
    };

    // The rest of the arguments form the body.
    let body_exprs = &args[2..];

    // Iterate over the collection.
    for item in items {
        // For each item, create a new lexical scope with the variable bound to the item.
        let mut loop_context = context.clone_with_new_lexical_frame();
        loop_context.set_lexical_var(&var_name, item);

        // Execute the body expressions within the new scope.
        for expr in body_exprs {
            eval::evaluate_ast_node(expr, &mut loop_context)?;
        }
    }

    Ok(Value::Nil)
};
