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
use crate::{
    ast,
    errors,
    helpers,
    runtime::{eval, world},
    NativeLazyFn,
};
use std::sync::Arc;

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
    // 1. Validate arity
    helpers::validate_special_form_min_arity(args, 2, "apply", context)?;

    // 2. Extract function argument (do not evaluate)
    let func_expr = &args[0];

    // 3. Evaluate all arguments except the last
    let normal_args = helpers::eval_apply_normal_args(&args[1..args.len() - 1], context)?;

    // 4. Evaluate the last argument and extract list elements
    let list_val = eval::evaluate_ast_node(&args[args.len() - 1], context)?;
    let mut list_args = Vec::new();
    let mut current = &list_val;
    loop {
        match current {
            Value::Cons(boxed) => {
                let expr = ast::expr_from_value_with_span(boxed.car.clone(), *parent_span)
                    .map_err(|msg| errors::runtime_general(msg, "value-to-ast", &context.source, context.span_for_span(*parent_span)))?;
                list_args.push(Spanned {
                    value: Arc::new(expr),
                    span: *parent_span,
                });
                current = &boxed.cdr;
            }
            Value::Nil => break,
            _ => {
                return Err(errors::type_mismatch(
                    "proper List",
                    current.type_name(),
                    &context.source,
                    context.span_for_span(*parent_span),
                ).with_suggestion("The last argument to 'apply' must be a proper list (ending in nil)."));
            }
        }
    }

    // 5. Build the call expression
    let mut call_items = Vec::with_capacity(1 + normal_args.len() + list_args.len());
    call_items.push(func_expr.clone());
    call_items.extend(normal_args);
    call_items.extend(list_args);

    let call_expr = Spanned {
        value: Arc::new(Expr::List(call_items, *parent_span)),
        span: *parent_span,
    };

    // 6. Evaluate the call expression
    let mut sub_context = helpers::sub_eval_context!(context);
    crate::runtime::eval::evaluate_ast_node(&call_expr, &mut sub_context)
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
    let body_exprs = &args[2..];

    // Ensure we have a list to iterate over.
    if !matches!(&collection_val, &Value::Cons(_) | &Value::Nil) {
        return Err(errors::type_mismatch(
            "List or Nil",
            collection_val.type_name(),
            &context.source,
            context.span_for_node(&args[1]),
        ));
    }

    let path = world::Path(vec![var_name]);

    for item_val in collection_val.try_into_iter() {
        let mut sub_context = helpers::sub_eval_context!(context);
        // Set the loop variable in the sub-context's world.
        sub_context.world.borrow_mut().set(&path, item_val);

        // Execute the body expressions.
        for expr in body_exprs {
            eval::evaluate_ast_node(expr, &mut sub_context)?;
        }
    }

    Ok(Value::Nil)
};
