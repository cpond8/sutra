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
use crate::{
    atoms::EagerAtomFn,
    helpers,
};
use crate::errors;

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
pub const ATOM_LIST: EagerAtomFn = |args, context| Ok((Value::List(args.to_vec()), context.world.clone()));

/// Returns the length of a list or string.
///
/// Usage: (len <list-or-string>)
///   - <list-or-string>: List or String to measure
///
///   Returns: Number (length)
///
/// Example:
///   (len (list 1 2 3)) ; => 3
///   (len "abc") ; => 3
pub const ATOM_LEN: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "len")?;
    let val = &args[0];
    let len = match val {
        Value::List(items) => items.len(),
        Value::String(s) => s.len(),
        _ => return Err(errors::type_mismatch(
            "List or String",
            val.type_name(),
            context.current_file(),
            context.current_source(),
            context.span_for_span(Span::default()),
        )),
    };
    Ok((Value::Number(len as f64), context.world.clone()))
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
pub const ATOM_HAS: EagerAtomFn = |args, context| {
    helpers::validate_binary_arity(args, "has?")?;
    let collection_val = &args[0];
    let search_val = &args[1];
    let found = match collection_val {
        Value::List(items) => items.contains(search_val),
        Value::Map(map) => {
            let key = helpers::validate_string_value(search_val, "map key")?;
            map.contains_key(key)
        }
        _ => return Err(errors::type_mismatch(
            "List or Map",
            collection_val.type_name(),
            context.current_file(),
            context.current_source(),
            context.span_for_span(Span::default()),
        )),
    };
    Ok((Value::Bool(found), context.world.clone()))
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
pub const ATOM_CORE_PUSH: EagerAtomFn = |args, context| {
    helpers::validate_binary_arity(args, "core/push!")?;
    let path = helpers::validate_path_arg(args, "core/push!")?;
    let mut list_val = context
        .world
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));
    let items = helpers::validate_list_value_mut(&mut list_val, "core/push!")?;
    items.push(args[1].clone());
    let new_world = context.world.set(path, list_val);
    Ok((Value::Nil, new_world))
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
pub const ATOM_CORE_PULL: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "core/pull!")?;
    let path = helpers::validate_path_arg(args, "core/pull!")?;
    let mut list_val = context
        .world
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));
    let items = helpers::validate_list_value_mut(&mut list_val, "core/pull!")?;
    let pulled_value = items.pop().unwrap_or(Value::Nil);
    let new_world = context.world.set(path, list_val);
    Ok((pulled_value, new_world))
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
pub const ATOM_CORE_STR_PLUS: EagerAtomFn = |args, context| {
    if args.is_empty() {
        return Ok((Value::String(String::new()), context.world.clone()));
    }
    let mut result = String::new();
    for val in args {
        let Value::String(s) = val else {
            return Err(errors::type_mismatch(
                "String",
                val.type_name(),
                context.current_file(),
                context.current_source(),
                context.span_for_span(Span::default()),
            ));
        };
        result.push_str(&s);
    }
    Ok((Value::String(result), context.world.clone()))
};

// ============================================================================
// LIST ACCESS OPERATIONS
// ============================================================================

/// Returns the first element of a list.
///
/// Usage: (car <list>)
///   - <list>: List to get first element from
///
///   Returns: First element, or Nil if list is empty
///
/// Example:
///   (car (list 1 2 3)) ; => 1
///   (car (list)) ; => Nil
pub const ATOM_CAR: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "car")?;
    let list_val = &args[0];
    let items = helpers::validate_list_value(list_val, "car")?;
    let Some(first) = items.first() else {
        return Ok((Value::Nil, context.world.clone()));
    };
    Ok((first.clone(), context.world.clone()))
};

/// Returns all elements of a list except the first.
///
/// Usage: (cdr <list>)
///   - <list>: List to get rest of elements from
///
///   Returns: List of remaining elements, or empty list if original is empty
///
/// Example:
///   (cdr (list 1 2 3)) ; => (2 3)
///   (cdr (list 1)) ; => ()
///   (cdr (list)) ; => ()
pub const ATOM_CDR: EagerAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "cdr")?;
    let list_val = &args[0];
    let items = helpers::validate_list_value(list_val, "cdr")?;
    if items.is_empty() {
        return Ok((Value::List(vec![]), context.world.clone()));
    }
    Ok((Value::List(items[1..].to_vec()), context.world.clone()))
};

/// Constructs a new list by prepending an element to an existing list.
///
/// Usage: (cons <element> <list>)
///   - <element>: Value to prepend
///   - <list>: List to prepend to
///
///   Returns: New list with element prepended
///
/// Example:
///   (cons 1 (list 2 3)) ; => (1 2 3)
///   (cons 1 (list)) ; => (1)
pub const ATOM_CONS: EagerAtomFn = |args, context| {
    helpers::validate_binary_arity(args, "cons")?;
    let element = &args[0];
    let list_val = &args[1];
    let items = helpers::validate_list_value(list_val, "cons")?;
    let mut new_list = Vec::with_capacity(items.len() + 1);
    new_list.push(element.clone());
    new_list.extend(items.iter().cloned());
    Ok((Value::List(new_list), context.world.clone()))
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
pub const ATOM_CORE_MAP: EagerAtomFn = |args, context| {
    helpers::validate_even_arity(args, "core/map")?;
    let mut map = im::HashMap::new();
    for chunk in args.chunks(2) {
        let key = helpers::validate_string_value(&chunk[0], "map key")?.to_string();
        let value = chunk[1].clone();
        map.insert(key, value);
    }
    Ok((Value::Map(map), context.world.clone()))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all collection atoms with the given registry.
pub fn register_collection_atoms(registry: &mut AtomRegistry) {
    // List operations
    registry.register_eager("list", ATOM_LIST);
    registry.register_eager("len", ATOM_LEN);
    registry.register_eager("has?", ATOM_HAS);
    registry.register_eager("car", ATOM_CAR);
    registry.register_eager("cdr", ATOM_CDR);
    registry.register_eager("cons", ATOM_CONS);
    // Mutable list operations
    registry.register_eager("core/push!", ATOM_CORE_PUSH);
    registry.register_eager("core/pull!", ATOM_CORE_PULL);
    // String operations
    registry.register_eager("core/str+", ATOM_CORE_STR_PLUS);
    // Map operations
    registry.register_eager("core/map", ATOM_CORE_MAP);
}
