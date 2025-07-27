//!
//! This module provides all collection atom operations for the Sutra engine.
//! Collections include lists, maps, and strings with both pure and stateful operations.
//!
//! ## Atoms Provided
//!
//! - **List Operations**: `list`, `len`, `has?`, `car`, `cdr`, `cons`
//! - **Mutable List Operations**: `core/push!`, `core/pull!`
//! - **String Operations**: `core/str+`
//! - **Map Operations**: `core/map`
//!
//! ## Design Principles
//!
//! - **Type Safety**: Clear error messages for type mismatches
//! - **Immutable Operations**: Pure functions where possible
//! - **Mutable Operations**: World-state operations for list manipulation
//! - **Performance**: Minimal cloning, efficient string operations

use std::rc::Rc;
use std::sync::Arc;

use crate::{
    ast::{
        spanned_value::SpannedValue,
        value::{NativeFn, Value},
        AstNode, ConsCell,
    },
    engine::evaluate_ast_node,
    errors::{to_source_span, ErrorReporting},
};

// ============================================================================
// LIST OPERATIONS
// ============================================================================

/// Constructs a list from arguments.
///
/// Usage: (list <a> <b> ...)
///   - <a>, <b>, ...: Values to include in the list
///
///   Returns: List containing all arguments
///
/// Example:
///   (list 1 2 3) ; => (1 2 3)
pub const ATOM_LIST: NativeFn = |args, context, call_span| {
    let mut result = Value::Nil;
    for item in args.iter().rev() {
        let value = evaluate_ast_node(item, context)?.value;
        let new_cell = ConsCell {
            car: value,
            cdr: result,
        };
        result = Value::Cons(Rc::new(new_cell));
    }
    Ok(SpannedValue {
        value: result,
        span: *call_span,
    })
};

/// Returns the length of a list, string, or nil.
///
/// Usage: (len <list-or-string-or-nil>)
///   - <list-or-string-or-nil>: List, String, or Nil to measure
///
///   Returns: Number (length)
///
/// Example:
///   (len (list 1 2 3)) ; => 3
///   (len "abc") ; => 3
///   (len nil) ; => 0
pub const ATOM_LEN: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let val_sv = evaluate_ast_node(&args[0], context)?;
    let len = match &val_sv.value {
        Value::Cons(_) | Value::Nil => val_sv.value.try_into_iter().count(),
        Value::String(s) => s.len(),
        _ => {
            return Err(context.type_mismatch(
                "List, String, or Nil",
                val_sv.value.type_name(),
                to_source_span(val_sv.span),
            ))
        }
    };
    Ok(SpannedValue {
        value: Value::Number(len as f64),
        span: *call_span,
    })
};

/// Tests if a collection contains a value or key.
///
/// Usage: (has? <collection> <value>)
///   - <collection>: List or Map to search in
///   - <value>: Value to search for (element in List, key in Map)
///
///   Returns: Bool (true if found, false otherwise)
///
/// Example:
///   (has? (list 1 2 3) 2) ; => true
///   (has? {"key" "value"} "key") ; => true
///   (has? (list 1 2 3) 4) ; => false
pub const ATOM_HAS: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let collection_sv = evaluate_ast_node(&args[0], context)?;
    let search_sv = evaluate_ast_node(&args[1], context)?;

    let found = match &collection_sv.value {
        Value::Cons(_) | Value::Nil => collection_sv
            .value
            .try_into_iter()
            .any(|item| item == search_sv.value),
        Value::Map(map) => {
            let key = match &search_sv.value {
                Value::String(s) => s,
                _ => {
                    return Err(context.type_mismatch(
                        "String",
                        search_sv.value.type_name(),
                        to_source_span(search_sv.span),
                    ))
                }
            };
            map.contains_key(key)
        }
        _ => {
            return Err(context.type_mismatch(
                "List or Map",
                collection_sv.value.type_name(),
                to_source_span(collection_sv.span),
            ));
        }
    };
    Ok(SpannedValue {
        value: Value::Bool(found),
        span: *call_span,
    })
};

// `ATOM_CORE_PUSH` and `ATOM_CORE_PULL` have been removed as they are fundamentally
// incompatible with the new immutable, `Rc`-based list implementation. A separate,
// mutable list type may be introduced in the future if required.

// ============================================================================
// STRING OPERATIONS
// ============================================================================

/// Concatenates two or more strings into a single string.
///
/// Usage: (core/str+ <string1> <string2> ...)
///   - <string1>, <string2>, ...: Strings to concatenate
///
///   Returns: String (concatenated result)
///
/// Example:
///   (core/str+ "foo" "bar" "baz") ; => "foobarbaz"
pub const ATOM_CORE_STR_PLUS: NativeFn = |args, context, call_span| {
    if args.is_empty() {
        return Ok(SpannedValue {
            value: Value::String(String::new()),
            span: *call_span,
        });
    }
    let mut result = String::new();
    for arg in args {
        let val_sv = evaluate_ast_node(arg, context)?;
        let s = match &val_sv.value {
            Value::String(s) => s,
            _ => {
                return Err(context.type_mismatch(
                    "String",
                    val_sv.value.type_name(),
                    to_source_span(val_sv.span),
                ))
            }
        };
        result.push_str(&s);
    }
    Ok(SpannedValue {
        value: Value::String(result),
        span: *call_span,
    })
};

// ============================================================================
// LIST ACCESS OPERATIONS
// ============================================================================

/// Returns the first element of a list or nil.
///
/// Usage: (car <list-or-nil>)
///   - <list-or-nil>: List or Nil to get first element from
///
///   Returns: First element, or Nil if list is empty or input is nil
///
/// Example:
///   (car (list 1 2 3)) ; => 1
///   (car (list)) ; => nil
///   (car nil) ; => nil
pub const ATOM_CAR: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let list_sv = evaluate_ast_node(&args[0], context)?;

    let value = match &list_sv.value {
        Value::Cons(cell) => cell.car.clone(),
        Value::Nil => Value::Nil, // car of nil is nil
        _ => {
            return Err(context.type_mismatch(
                "List or Nil",
                list_sv.value.type_name(),
                to_source_span(list_sv.span),
            ))
        }
    };
    Ok(SpannedValue {
        value,
        span: *call_span,
    })
};

/// Returns all elements of a list except the first, or nil for nil.
///
/// Usage: (cdr <list-or-nil>)
///   - <list-or-nil>: List or Nil to get rest of elements from
///
///   Returns: List of remaining elements, empty list if original is empty, or nil for nil input
///
/// Example:
///   (cdr (list 1 2 3)) ; => (2 3)
///   (cdr (list 1)) ; => ()
///   (cdr (list)) ; => ()
///   (cdr nil) ; => nil
pub const ATOM_CDR: NativeFn = |args, context, call_span| {
    if args.len() != 1 {
        return Err(context.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }

    let list_sv = evaluate_ast_node(&args[0], context)?;

    let value = match &list_sv.value {
        Value::Cons(cell) => cell.cdr.clone(),
        Value::Nil => Value::Nil, // cdr of nil is nil
        _ => {
            return Err(context.type_mismatch(
                "List or Nil",
                list_sv.value.type_name(),
                to_source_span(list_sv.span),
            ))
        }
    };

    Ok(SpannedValue {
        value,
        span: *call_span,
    })
};

/// Constructs a new list by prepending an element to an existing list or nil.
///
/// Usage: (cons <element> <list-or-nil>)
///   - <element>: Value to prepend
///   - <list-or-nil>: List or Nil to prepend to
///
///   Returns: New list with element prepended
///
/// Example:
///   (cons 1 (list 2 3)) ; => (1 2 3)
///   (cons 1 (list)) ; => (1)
///   (cons 1 nil) ; => (1)
pub const ATOM_CONS: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }

    let car_sv = evaluate_ast_node(&args[0], context)?;
    let cdr_sv = evaluate_ast_node(&args[1], context)?;

    let cdr = match &cdr_sv.value {
        Value::Cons(_) | Value::Nil => cdr_sv.value,
        other => {
            // Wrap non-list cdr as a single-element list
            let cell = ConsCell {
                car: other.clone(),
                cdr: Value::Nil,
            };
            Value::Cons(Rc::new(cell))
        }
    };

    let new_cell = ConsCell {
        car: car_sv.value,
        cdr,
    };
    Ok(SpannedValue {
        value: Value::Cons(Rc::new(new_cell)),
        span: *call_span,
    })
};

/// Appends two or more lists.
///
/// Usage: (append <list1> <list2> ...)
///   - <list1>, <list2>, ...: Lists or nil to append
///
///   Returns: New list containing all elements from the input lists
///
/// Example:
///   (append (list 1 2) (list 3 4))   ; => (1 2 3 4)
///   (append (list 1) nil (list 2))   ; => (1 2)
pub const ATOM_APPEND: NativeFn = |args, context, call_span| {
    let mut new_list_items: Vec<Value> = Vec::new();
    for arg in args {
        let list_sv = evaluate_ast_node(arg, context)?;
        match &list_sv.value {
            Value::Cons(_) | Value::Nil => {
                new_list_items.extend(list_sv.value.try_into_iter());
            }
            _ => {
                return Err(context.type_mismatch(
                    "List or Nil",
                    list_sv.value.type_name(),
                    to_source_span(list_sv.span),
                ));
            }
        }
    }

    // Build the new list from the collected items
    let mut result = Value::Nil;
    for item in new_list_items.into_iter().rev() {
        let new_cell = ConsCell {
            car: item,
            cdr: result,
        };
        result = Value::Cons(Rc::new(new_cell));
    }

    Ok(SpannedValue {
        value: result,
        span: *call_span,
    })
};

/// Applies a function to each element of a list, returning a new list of the results.
///
/// Usage: (map <function> <list>)
///   - <function>: The function to apply to each element
///   - <list>: The list to iterate over
///
///   Returns: A new list containing the results of applying the function to each element.
///
/// Example:
///   (map (lambda (x) (* x 2)) (list 1 2 3))  ; => (2 4 6)
pub const ATOM_MAP: NativeFn = |args, context, call_span| {
    if args.len() != 2 {
        return Err(context.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }
    let func_sv = evaluate_ast_node(&args[0], context)?;
    let list_sv = evaluate_ast_node(&args[1], context)?;

    // The first argument must be a callable function.
    let func = match func_sv.value {
        Value::Lambda(_) | Value::NativeFn(_) => func_sv.value,
        _ => {
            return Err(context.type_mismatch(
                "Lambda or NativeFn",
                func_sv.value.type_name(),
                to_source_span(func_sv.span),
            ));
        }
    };

    // The second argument must be a list.
    if !matches!(list_sv.value, Value::Cons(_) | Value::Nil) {
        return Err(context.type_mismatch(
            "List or Nil",
            list_sv.value.type_name(),
            to_source_span(list_sv.span),
        ));
    }

    let mut results = Vec::new();
    for item in list_sv.value.try_into_iter() {
        let result = match &func {
            Value::Lambda(lambda) => {
                let mut new_context = context.with_new_frame();
                if lambda.params.required.len() != 1 {
                    return Err(context.arity_mismatch(
                        &lambda.params.required.len().to_string(),
                        1,
                        to_source_span(list_sv.span),
                    ));
                }
                new_context.set_var(
                    &lambda.params.required[0],
                    item.clone(),
                );
                evaluate_ast_node(&lambda.body, &mut new_context)?
            }
            Value::NativeFn(native_fn) => {
                let expr = crate::ast::expr_from_value_with_span(item.clone(), list_sv.span)
                    .map_err(|_| context.type_mismatch(
                        "convertible to expression",
                        item.type_name(),
                        to_source_span(list_sv.span),
                    ))?;
                let ast_node = AstNode {
                    value: Arc::new(expr),
                    span: list_sv.span,
                };
                native_fn(&[ast_node], context, call_span)?
            }
            _ => {
                return Err(context.type_mismatch(
                    "Lambda or NativeFn",
                    func.type_name(),
                    to_source_span(func_sv.span),
                ))
            }
        };

        results.push(result.value);
    }

    // Build the new list from the results
    let mut result_list = Value::Nil;
    for item in results.into_iter().rev() {
        let new_cell = ConsCell {
            car: item,
            cdr: result_list,
        };
        result_list = Value::Cons(Rc::new(new_cell));
    }

    Ok(SpannedValue {
        value: result_list,
        span: *call_span,
    })
};

// ============================================================================
// MAP OPERATIONS
// ============================================================================

/// Creates a map from alternating key-value pairs.
///
/// Usage: (core/map <key1> <value1> <key2> <value2> ...)
///   - <key1>, <value1>, ...: Alternating keys and values
///
///   Returns: Map with the key-value pairs
///
/// Example:
///   (core/map "a" 1 "b" 2) ; => {"a" 1 "b" 2}
pub const ATOM_CORE_MAP: NativeFn = |args, context, call_span| {
    if args.len() % 2 != 0 {
        return Err(context.arity_mismatch("even", args.len(), to_source_span(*call_span)));
    }
    let mut map = std::collections::HashMap::new();
    for chunk in args.chunks(2) {
        let key_sv = evaluate_ast_node(&chunk[0], context)?;
        let key = match key_sv.value {
            Value::String(s) => s,
            _ => {
                return Err(context.type_mismatch(
                    "String",
                    key_sv.value.type_name(),
                    to_source_span(key_sv.span),
                ))
            }
        };
        let value_sv = evaluate_ast_node(&chunk[1], context)?;
        map.insert(key, value_sv.value);
    }
    Ok(SpannedValue {
        value: Value::Map(map),
        span: *call_span,
    })
};
