//! Collection operations for the Sutra language.
//!
//! This module provides atoms for working with lists, strings, and maps.
//! Includes both pure operations and stateful world operations.

use std::rc::Rc;
use std::sync::Arc;

use crate::{
    errors::{to_source_span, ErrorReporting, SutraError},
    runtime::{evaluate_ast_node, ConsCell, EvaluationContext, NativeFn, SpannedValue, Value},
    syntax::{AstNode, Span},
};

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Validates that the correct number of arguments were provided.
fn require_arity(
    args: &[AstNode],
    expected: usize,
    context: &mut EvaluationContext,
    span: Span,
) -> Result<(), SutraError> {
    if args.len() != expected {
        Err(context.arity_mismatch(&expected.to_string(), args.len(), to_source_span(span)))
    } else {
        Ok(())
    }
}

/// Validates that arguments count is even.
fn require_even_arity(
    args: &[AstNode],
    context: &mut EvaluationContext,
    span: Span,
) -> Result<(), SutraError> {
    if args.len() % 2 != 0 {
        Err(context.arity_mismatch("even", args.len(), to_source_span(span)))
    } else {
        Ok(())
    }
}

/// Evaluates a single argument.
fn eval_arg(arg: &AstNode, context: &mut EvaluationContext) -> Result<SpannedValue, SutraError> {
    evaluate_ast_node(arg, context)
}

/// Creates a type mismatch error for a SpannedValue.
fn type_error<T>(
    expected: &str,
    val: &SpannedValue,
    context: &mut EvaluationContext,
) -> Result<T, SutraError> {
    Err(context.type_mismatch(expected, val.value.type_name(), to_source_span(val.span)))
}

/// Creates a successful SpannedValue result.
fn ok_span(value: Value, span: Span) -> Result<SpannedValue, SutraError> {
    Ok(SpannedValue { value, span })
}

/// Builds a list from a vector of values.
fn build_list(items: Vec<Value>, span: Span) -> SpannedValue {
    let mut result = Value::Nil;
    for item in items.into_iter().rev() {
        result = Value::Cons(Rc::new(ConsCell {
            car: item,
            cdr: result,
        }));
    }
    SpannedValue {
        value: result,
        span,
    }
}

/// Calls a function (Lambda or NativeFn) with a single argument.
fn call_function_with_value(
    func: &Value,
    arg: Value,
    context: &mut EvaluationContext,
    call_span: &Span,
    arg_span: Span,
) -> Result<SpannedValue, SutraError> {
    match func {
        Value::Lambda(lambda) => {
            let mut new_context = context.with_new_frame();
            if lambda.params.required.len() != 1 {
                return Err(context.arity_mismatch(
                    &lambda.params.required.len().to_string(),
                    1,
                    to_source_span(arg_span),
                ));
            }
            new_context.set_var(&lambda.params.required[0], arg);
            evaluate_ast_node(&lambda.body, &mut new_context)
        }
        Value::NativeFn(native_fn) => {
            let expr =
                crate::syntax::expr_from_value_with_span(arg.clone(), arg_span).map_err(|_| {
                    context.type_mismatch(
                        "convertible to expression",
                        arg.type_name(),
                        to_source_span(arg_span),
                    )
                })?;
            let ast_node = AstNode {
                value: Arc::new(expr),
                span: arg_span,
            };
            native_fn(&[ast_node], context, call_span)
        }
        _ => Err(context.type_mismatch(
            "Lambda or NativeFn",
            func.type_name(),
            to_source_span(*call_span),
        )),
    }
}

// ============================================================================
// LIST OPERATIONS
// ============================================================================

/// Constructs a list from arguments: (list <a> <b> ...)
pub const ATOM_LIST: NativeFn = |args, context, call_span| {
    let mut result = Value::Nil;
    for item in args.iter().rev() {
        let value = eval_arg(item, context)?.value;
        result = Value::Cons(Rc::new(ConsCell {
            car: value,
            cdr: result,
        }));
    }
    ok_span(result, *call_span)
};

/// Returns the length of a list, string, or nil: (len <collection>)
pub const ATOM_LEN: NativeFn = |args, context, call_span| {
    require_arity(args, 1, context, *call_span)?;
    let val = eval_arg(&args[0], context)?;

    let len = match &val.value {
        Value::Cons(_) | Value::Nil => val.value.try_into_iter().count(),
        Value::String(s) => s.len(),
        _ => return type_error("List, String, or Nil", &val, context),
    };

    ok_span(Value::Number(len as f64), *call_span)
};

/// Returns true if a list is empty (nil): (null? <list>)
pub const ATOM_NULL: NativeFn = |args, context, call_span| {
    require_arity(args, 1, context, *call_span)?;
    let val = eval_arg(&args[0], context)?;
    ok_span(Value::Bool(matches!(val.value, Value::Nil)), *call_span)
};

/// Tests if a collection contains a value or key: (has? <collection> <value>)
pub const ATOM_HAS: NativeFn = |args, context, call_span| {
    require_arity(args, 2, context, *call_span)?;
    let collection = eval_arg(&args[0], context)?;
    let search = eval_arg(&args[1], context)?;

    let found = match &collection.value {
        Value::Cons(_) | Value::Nil => collection
            .value
            .try_into_iter()
            .any(|item| item == search.value),
        Value::Map(map) => {
            let key = match &search.value {
                Value::String(s) => s,
                _ => return type_error("String", &search, context),
            };
            map.contains_key(key)
        }
        _ => return type_error("List or Map", &collection, context),
    };

    ok_span(Value::Bool(found), *call_span)
};

// ============================================================================
// STRING OPERATIONS
// ============================================================================

/// Concatenates strings: (core/str+ <string1> <string2> ...)
pub const ATOM_CORE_STR_PLUS: NativeFn = |args, context, call_span| {
    if args.is_empty() {
        return ok_span(Value::String(String::new()), *call_span);
    }

    let mut result = String::new();
    for arg in args {
        let val = eval_arg(arg, context)?;
        let s = match &val.value {
            Value::String(s) => s.as_str(),
            _ => return type_error("String", &val, context),
        };
        result.push_str(s);
    }

    ok_span(Value::String(result), *call_span)
};

// ============================================================================
// LIST ACCESS OPERATIONS
// ============================================================================

/// Returns the first element of a list or nil: (car <list-or-nil>)
pub const ATOM_CAR: NativeFn = |args, context, call_span| {
    require_arity(args, 1, context, *call_span)?;
    let list = eval_arg(&args[0], context)?;

    let value = match &list.value {
        Value::Cons(cell) => cell.car.clone(),
        Value::Nil => Value::Nil,
        _ => return type_error("List or Nil", &list, context),
    };

    ok_span(value, *call_span)
};

/// Returns all elements except the first: (cdr <list-or-nil>)
pub const ATOM_CDR: NativeFn = |args, context, call_span| {
    require_arity(args, 1, context, *call_span)?;
    let list = eval_arg(&args[0], context)?;

    let value = match &list.value {
        Value::Cons(cell) => cell.cdr.clone(),
        Value::Nil => Value::Nil,
        _ => return type_error("List or Nil", &list, context),
    };

    ok_span(value, *call_span)
};

/// Prepends an element to a list: (cons <element> <list-or-nil>)
pub const ATOM_CONS: NativeFn = |args, context, call_span| {
    require_arity(args, 2, context, *call_span)?;
    let car = eval_arg(&args[0], context)?.value;
    let cdr_val = eval_arg(&args[1], context)?;

    let cdr = match &cdr_val.value {
        Value::Cons(_) | Value::Nil => cdr_val.value,
        other => {
            // Wrap non-list cdr as a single-element list (improper lists are not allowed)
            Value::Cons(Rc::new(ConsCell {
                car: other.clone(),
                cdr: Value::Nil,
            }))
        }
    };

    ok_span(Value::Cons(Rc::new(ConsCell { car, cdr })), *call_span)
};

/// Concatenates lists: (append <list1> <list2> ...)
pub const ATOM_APPEND: NativeFn = |args, context, call_span| {
    let mut items = Vec::new();
    for arg in args {
        let list = eval_arg(arg, context)?;
        match &list.value {
            Value::Cons(_) | Value::Nil => items.extend(list.value.try_into_iter()),
            _ => return type_error("List or Nil", &list, context),
        }
    }

    Ok(build_list(items, *call_span))
};

/// Applies a function to each element of a list: (map <function> <list>)
pub const ATOM_MAP: NativeFn = |args, context, call_span| {
    require_arity(args, 2, context, *call_span)?;
    let func = eval_arg(&args[0], context)?;
    let list = eval_arg(&args[1], context)?;

    // Validate function type
    match &func.value {
        Value::Lambda(_) | Value::NativeFn(_) => {}
        _ => return type_error("Lambda or NativeFn", &func, context),
    }

    // Validate list type
    if !matches!(list.value, Value::Cons(_) | Value::Nil) {
        return type_error("List or Nil", &list, context);
    }

    let mut results = Vec::new();
    for item in list.value.try_into_iter() {
        let result = call_function_with_value(&func.value, item, context, call_span, list.span)?;
        results.push(result.value);
    }

    Ok(build_list(results, *call_span))
};

// ============================================================================
// MAP OPERATIONS
// ============================================================================

/// Creates a map from alternating key-value pairs: (core/map <key1> <value1> <key2> <value2> ...)
pub const ATOM_CORE_MAP: NativeFn = |args, context, call_span| {
    require_even_arity(args, context, *call_span)?;

    let mut map = std::collections::HashMap::new();
    for chunk in args.chunks(2) {
        let key_val = eval_arg(&chunk[0], context)?;
        let key = match key_val.value {
            Value::String(s) => s,
            _ => return type_error("String", &key_val, context),
        };
        let value = eval_arg(&chunk[1], context)?.value;
        map.insert(key, value);
    }

    ok_span(Value::Map(map), *call_span)
};
