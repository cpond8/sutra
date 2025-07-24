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
pub const ATOM_LIST: NativeEagerFn = |args, _| Ok(Value::List(args.to_vec()));

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
    let val = &args[0];
    let len = match val {
        Value::List(items) => items.len(),
        Value::String(s) => s.len(),
        Value::Nil => 0, // nil has length 0 (treated as empty list)
        _ => {
            return Err(context.create_type_mismatch_error(
                "List, String, or Nil",
                val.type_name(),
                context.span_for_span(Span::default()),
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
    let collection_val = &args[0];
    let search_val = &args[1];
    let found = match collection_val {
        Value::List(items) => items.contains(search_val),
        Value::Map(map) => {
            let key = helpers::validate_string_value(search_val, "map key", context)?;
            map.contains_key(key)
        }
        _ => {
            return Err(context.create_type_mismatch_error(
                "List or Map",
                collection_val.type_name(),
                context.span_for_span(Span::default()),
            ))
        }
    };
    Ok(Value::Bool(found))
};

/// Appends a value to a list at a path in the world state.
///
/// Usage: (core/push! <path> <value>)
///   - <path>: Path to the list (must evaluate to a Value::Path)
///   - <value>: Value to append to the list
///
///   Returns: Nil. Mutates world state (returns new world).
///
/// Example:
///   (core/push! items 42)  ; Appends 42 to the list at 'items'
///
/// # Safety
/// Mutates the world at the given path. **Creates a new empty list if the path doesn't exist.**
pub const ATOM_CORE_PUSH: NativeEagerFn = |args, context| {
    helpers::validate_binary_arity(args, "core/push!", context)?;
    let path = helpers::validate_path_arg(args, "core/push!", context)?;
    let value_to_push = args[1].clone();

    let mut world = context.world.borrow_mut();
    if world.state.get(path).is_none() {
        world.state.set(path, Value::List(vec![]));
    }
    let list_val = world.state.get_mut(path).unwrap();

    let items = helpers::validate_list_value_mut(list_val, "core/push!", context)?;
    items.push(value_to_push);

    Ok(Value::Nil)
};

/// Removes and returns the last element from a list at a path in the world state.
///
/// Usage: (core/pull! <path>)
///   - <path>: Path to the list (must evaluate to a Value::Path)
///
///   Returns: The removed element, or Nil if list is empty or doesn't exist.
///
/// Example:
///   (core/pull! items)  ; Removes and returns last element from 'items'
///
/// # Safety
/// Mutates the world at the given path. **Creates a new empty list if the path doesn't exist.**
pub const ATOM_CORE_PULL: NativeEagerFn = |args, context| {
    helpers::validate_unary_arity(args, "core/pull!", context)?;
    let path = helpers::validate_path_arg(args, "core/pull!", context)?;

    let mut world = context.world.borrow_mut();
    let list_val = world.state.get_mut(path);

    if let Some(list_val) = list_val {
        let items = helpers::validate_list_value_mut(list_val, "core/pull!", context)?;
        let pulled_value = items.pop().unwrap_or(Value::Nil);
        Ok(pulled_value)
    } else {
        Ok(Value::Nil)
    }
};

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
            return Err(context.create_type_mismatch_error(
                "String",
                val.type_name(),
                context.span_for_span(Span::default()),
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
        Value::List(items) => {
            let Some(first) = items.first() else {
                return Ok(Value::Nil);
            };
            Ok(first.clone())
        }
        Value::Nil => Ok(Value::Nil), // car of nil is nil
        _ => Err(context.create_type_mismatch_error(
            "List or Nil",
            list_val.type_name(),
            context.span_for_span(Span::default()),
        )
        .with_suggestion("Expected a List or Nil for car")),
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
        Value::List(items) => {
            if items.is_empty() {
                Ok(Value::List(vec![]))
            } else {
                Ok(Value::List(items[1..].to_vec()))
            }
        }
        Value::Nil => Ok(Value::Nil), // cdr of nil is nil
        _ => Err(context.create_type_mismatch_error(
            "List or Nil",
            list_val.type_name(),
            context.span_for_span(Span::default()),
        )
        .with_suggestion("Expected a List or Nil for cdr")),
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
    let element = &args[0];
    let list_val = &args[1];
    match list_val {
        Value::List(items) => {
            let mut new_list = Vec::with_capacity(items.len() + 1);
            new_list.push(element.clone());
            new_list.extend(items.iter().cloned());
            Ok(Value::List(new_list))
        }
        Value::Nil => {
            // cons with nil creates a single-element list
            Ok(Value::List(vec![element.clone()]))
        }
        _ => Err(context.create_type_mismatch_error(
            "List or Nil",
            list_val.type_name(),
            context.span_for_span(Span::default()),
        )
        .with_suggestion("Expected a List or Nil for cons")),
    }
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
    let mut new_list = Vec::new();
    for arg in args {
        match arg {
            Value::List(items) => {
                new_list.extend(items.iter().cloned());
            }
            Value::Nil => {
                // Treat nil as an empty list, so do nothing
            }
            _ => {
                return Err(context.create_type_mismatch_error(
                    "List or Nil",
                    arg.type_name(),
                    context.span_for_span(Span::default()),
                )
                .with_suggestion("`append` only works on lists or nil"));
            }
        }
    }
    Ok(Value::List(new_list))
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
    let func = &args[0];
    let list_val = &args[1];

    let items = match list_val {
        Value::List(items) => items,
        Value::Nil => return Ok(Value::List(vec![])), // map over nil is an empty list
        _ => {
            return Err(context.create_type_mismatch_error(
                "List or Nil",
                list_val.type_name(),
                context.span_for_span(Span::default()),
            ));
        }
    };

    let mut results = Vec::new();
    for item in items {
        let result = match func {
            Value::Lambda(lambda) => {
                crate::atoms::special_forms::call_lambda(lambda, &[item.clone()], context)?
            }
            Value::NativeEagerFn(native_fn) => native_fn(&[item.clone()], context)?,
            _ => {
                return Err(context.create_type_mismatch_error(
                    "Lambda or NativeEagerFn",
                    func.type_name(),
                    context.span_for_span(Span::default()),
                ).with_suggestion("The first argument to `map` must be a callable function."));
            }
        };
        results.push(result);
    }

    Ok(Value::List(results))
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
