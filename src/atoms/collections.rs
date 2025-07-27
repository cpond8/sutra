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

use crate::ast::ConsCell;
use crate::errors::ErrorReporting;
use crate::prelude::*;
use crate::{helpers, NativeEagerFn};

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
pub const ATOM_LIST: NativeEagerFn = |args, _| {
    let mut result = Value::Nil;
    for item in args.iter().rev() {
        let new_cell = ConsCell {
            car: item.clone(),
            cdr: result,
        };
        result = Value::Cons(Rc::new(new_cell));
    }
    Ok(result)
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
pub const ATOM_LEN: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "len", context)?;
    let val = args[0].clone();
    let len = match val {
        Value::Cons(_) | Value::Nil => val.try_into_iter().count(),
        Value::String(s) => s.len(),
        _ => {
            return Err(context.type_mismatch(
                "List, String, or Nil",
                val.type_name(),
                context.span_for_span(context.current_span),
            ))
        }
    };
    Ok(Value::Number(len as f64))
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
pub const ATOM_HAS: NativeEagerFn = |args, context| {
    helpers::validate_binary_arity(args, "has?", context)?;
    let collection_val = args[0].clone();
    let search_val = &args[1];
    let found = match collection_val {
        Value::Cons(_) | Value::Nil => collection_val
            .try_into_iter()
            .any(|item| &item == search_val),
        Value::Map(map) => {
            let key = helpers::validate_string_value(search_val, "map key", context)?;
            map.contains_key(key)
        }
        _ => {
            return Err(context.type_mismatch(
                "List or Map",
                collection_val.type_name(),
                context.span_for_span(context.current_span),
            ));
        }
    };
    Ok(Value::Bool(found))
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
pub const ATOM_CORE_STR_PLUS: NativeEagerFn = |args, context| {
    if args.is_empty() {
        return Ok(Value::String(String::new()));
    }
    let mut result = String::new();
    for val in args {
        let Value::String(s) = val else {
            return Err(context.type_mismatch(
                "String",
                val.type_name(),
                context.span_for_span(context.current_span),
            ));
        };
        result.push_str(&s);
    }
    Ok(Value::String(result))
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
pub const ATOM_CAR: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "car", context)?;
    let list_val = &args[0];
    match list_val {
        Value::Cons(cell) => Ok(cell.car.clone()),
        Value::Nil => Ok(Value::Nil), // car of nil is nil
        _ => Err(context.type_mismatch(
            "List or Nil",
            list_val.type_name(),
            context.span_for_span(context.current_span),
        )),
    }
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
pub const ATOM_CDR: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "cdr", context)?;
    let list_val = &args[0];
    match list_val {
        Value::Cons(cell) => Ok(cell.cdr.clone()),
        Value::Nil => Ok(Value::Nil), // cdr of nil is nil
        _ => Err(context.type_mismatch(
            "List or Nil",
            list_val.type_name(),
            context.span_for_span(context.current_span),
        )),
    }
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
pub const ATOM_CONS: NativeEagerFn = |args, context| {
    helpers::validate_binary_arity(args, "cons", context)?;

    let car = args[0].clone();
    let cdr = match &args[1] {
        Value::Cons(_) | Value::Nil => args[1].clone(),
        other => {
            // Wrap non-list cdr as a single-element list
            let cell = ConsCell {
                car: other.clone(),
                cdr: Value::Nil,
            };
            Value::Cons(Rc::new(cell))
        }
    };

    let new_cell = ConsCell { car, cdr };
    Ok(Value::Cons(Rc::new(new_cell)))
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
pub const ATOM_APPEND: NativeEagerFn = |args, context| {
    let mut new_list_items: Vec<Value> = Vec::new();
    for arg in args {
        match arg {
            Value::Cons(_) | Value::Nil => {
                new_list_items.extend(arg.clone().try_into_iter());
            }
            _ => {
                return Err(context.type_mismatch(
                    "List or Nil",
                    arg.type_name(),
                    context.span_for_span(context.current_span),
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

    Ok(result)
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
pub const ATOM_MAP: NativeEagerFn = |args, context| {
    helpers::validate_binary_arity(args, "map", context)?;
    let func = args[0].clone();
    let list_val = args[1].clone();

    // The first argument must be a callable function.
    if !matches!(func, Value::Lambda(_) | Value::NativeEagerFn(_)) {
        return Err(context.type_mismatch(
            "Lambda or NativeEagerFn",
            func.type_name(),
            context.span_for_span(context.current_span),
        ));
    }

    // The second argument must be a list.
    if !matches!(list_val, Value::Cons(_) | Value::Nil) {
        return Err(context.type_mismatch(
            "List or Nil",
            list_val.type_name(),
            context.span_for_span(context.current_span),
        ));
    }

    let mut results = Vec::new();
    for item in list_val.try_into_iter() {
        // For each item, we need to invoke the function. This requires creating a temporary
        // call expression. This is a simplification; a more robust implementation
        // would handle this more directly within the evaluator.
        let result = context.eval_call(&func, &[item])?;
        results.push(result);
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

    Ok(result_list)
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
pub const ATOM_CORE_MAP: NativeEagerFn = |args, context| {
    helpers::validate_even_arity(args, "core/map", context)?;
    let mut map = std::collections::HashMap::new();
    for chunk in args.chunks(2) {
        let key = helpers::validate_string_value(&chunk[0], "map key", context)?.to_string();
        let value = chunk[1].clone();
        map.insert(key, value);
    }
    Ok(Value::Map(map))
};
