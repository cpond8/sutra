//! Execution control atoms for the Sutra language.
//!
//! This module provides atoms for controlling program flow and higher-order
//! function operations.
//!
//! ## Atoms Provided
//!
//! - **Control Flow**: `do`, `error`
//! - **Higher-Order Functions**: `apply`, `for-each`
//!
//! ## Design Notes
//!
//! These atoms control evaluation order and can manipulate the execution context.
//! They properly thread world state through sequential operations.

use crate::{
    atoms::special_forms::call_lambda,
    errors::{to_source_span, ErrorKind, ErrorReporting},
    runtime::{evaluate_ast_node, NativeFn, SpannedValue, Value},
    syntax::Expr,
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
pub const ATOM_DO: NativeFn = |args, context, call_span| {
    let mut last_result = SpannedValue {
        value: Value::Nil,
        span: *call_span,
    };

    for arg in args {
        last_result = evaluate_ast_node(arg, context)?;
    }

    Ok(last_result)
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
pub const ATOM_ERROR: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let spanned_value = evaluate_ast_node(&args[0], context)?;
    let msg = match spanned_value.value {
        Value::String(s) => s,
        _ => {
            return Err(context.type_mismatch(
                "String",
                spanned_value.value.type_name(),
                to_source_span(spanned_value.span),
            ));
        }
    };
    Err(context.report(
        ErrorKind::InvalidOperation {
            operation: "user error".to_string(),
            operand_type: msg,
        },
        to_source_span(spanned_value.span),
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
pub const ATOM_APPLY: NativeFn = |args, context, call_span| {
    // 1. Validate arity
    if args.len() < 2 {
        return Err(context.arity_mismatch("at least 2", args.len(), to_source_span(*call_span)));
    }

    // 2. Evaluate the callable
    let callable_sv = evaluate_ast_node(&args[0], context)?;

    // 3. Eagerly evaluate all normal arguments
    let mut final_args = Vec::new();
    for arg_node in &args[1..args.len() - 1] {
        let spanned_value = evaluate_ast_node(arg_node, context)?;
        final_args.push(spanned_value.value);
    }

    // 4. Evaluate the final argument, which must be a list
    let list_sv = evaluate_ast_node(args.last().unwrap(), context)?;
    let list_span = list_sv.span;

    // 5. Unpack the list argument
    let mut current = list_sv.value;
    loop {
        match current {
            Value::Cons(cell) => {
                final_args.push(cell.car.clone());
                current = cell.cdr.clone();
            }
            Value::Nil => break,
            _ => {
                return Err(context.type_mismatch(
                    "proper List",
                    current.type_name(),
                    to_source_span(list_span),
                ));
            }
        }
    }

    // 6. Dispatch the call
    match callable_sv.value {
        Value::Lambda(lambda) => {
            // We have a lambda, so we can call it.
            call_lambda(&lambda, &final_args, context, call_span)
        }
        Value::NativeFn(_) => {
            // This is a design limitation. `apply` cannot be used with native functions
            // as they expect AST nodes, not evaluated values.
            Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "apply".to_string(),
                    operand_type: "cannot apply a native function (atom)".to_string(),
                },
                to_source_span(callable_sv.span),
            ))
        }
        _ => Err(context.type_mismatch(
            "Callable (Lambda or NativeFn)",
            callable_sv.value.type_name(),
            to_source_span(callable_sv.span),
        )),
    }
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
pub const ATOM_FOR_EACH: NativeFn = |args, context, call_span| {
    if args.len() < 3 {
        return Err(context.arity_mismatch("at least 3", args.len(), to_source_span(*call_span)));
    }

    // First argument must be the variable symbol.
    let var_name = match &*args[0].value {
        Expr::Symbol(s, _) => s.clone(),
        _ => {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "for-each definition".to_string(),
                    operand_type: "first argument must be a symbol".to_string(),
                },
                to_source_span(args[0].span),
            ));
        }
    };

    // Second argument is the collection to iterate over.
    let collection_sv = evaluate_ast_node(&args[1], context)?;
    let body_exprs = &args[2..];

    let mut current = collection_sv.value;
    let collection_type = current.type_name();
    loop {
        match current {
            Value::Cons(cell) => {
                let mut sub_context = context.with_new_frame();
                sub_context.set_var(&var_name, cell.car.clone());

                // Execute the body expressions.
                for expr in body_exprs {
                    evaluate_ast_node(expr, &mut sub_context)?;
                }
                current = cell.cdr.clone();
            }
            Value::Nil => break,
            _ => {
                return Err(context.type_mismatch(
                    "List",
                    collection_type,
                    to_source_span(collection_sv.span),
                ));
            }
        }
    }

    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
};
