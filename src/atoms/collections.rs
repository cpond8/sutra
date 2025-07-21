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
    atoms::{PureAtomFn, StatefulAtomFn},
    helpers,
};
use crate::syntax::parser::to_source_span;
use miette::NamedSource;

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
pub const ATOM_LIST: PureAtomFn = |args| Ok(Value::List(args.to_vec()));

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
pub const ATOM_LEN: PureAtomFn = |args| {
    helpers::validate_unary_arity(args, "len")?;

    let val = &args[0];
    let len = match val {
        Value::List(items) => items.len(),
        Value::String(s) => s.len(),
        _ => {
            return Err(SutraError::RuntimeGeneral {
                message: format!("len expects a List or String, found {}", val.to_string()),
                src: NamedSource::new("atoms/collections.rs".to_string(), "".to_string()),
                span: to_source_span(Span::default()),
                suggestion: None,
            });
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
pub const ATOM_HAS: PureAtomFn = |args| {
    helpers::validate_binary_arity(args, "has?")?;

    let collection_val = &args[0];
    let search_val = &args[1];

    let found = match collection_val {
        Value::List(items) => items.contains(search_val),
        Value::Map(map) => {
            let key = helpers::validate_string_value(search_val, "map key")?;
            map.contains_key(key)
        }
        _ => {
            return Err(SutraError::RuntimeGeneral {
                message: format!("has? expects a List or Map as first argument, found {}", collection_val.to_string()),
                src: NamedSource::new("atoms/collections.rs".to_string(), "".to_string()),
                span: to_source_span(Span::default()),
                suggestion: None,
            });
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
pub const ATOM_CORE_PUSH: StatefulAtomFn = |args, context| {
    helpers::validate_binary_arity(args, "core/push!")?;

    let path = helpers::validate_path_arg(args, "core/push!")?;

    // Get existing list or create new one
    let mut list_val = context
        .state
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));

    // Validate and modify list
    let items = helpers::validate_list_value_mut(&mut list_val, "core/push!")?;
    items.push(args[1].clone());

    context.state.set(path, list_val);
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
pub const ATOM_CORE_PULL: StatefulAtomFn = |args, context| {
    helpers::validate_unary_arity(args, "core/pull!")?;

    let path = helpers::validate_path_arg(args, "core/pull!")?;

    // Get existing list or create new one
    let mut list_val = context
        .state
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));

    // Validate and modify list
    let items = helpers::validate_list_value_mut(&mut list_val, "core/pull!")?;
    let pulled_value = items.pop().unwrap_or(Value::Nil);

    context.state.set(path, list_val);
    Ok(pulled_value)
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
pub const ATOM_CORE_STR_PLUS: PureAtomFn = |args| {
    if args.is_empty() {
        return Ok(Value::String(String::new()));
    }

    let mut result = String::new();

    for val in args {
        let Value::String(s) = val else {
            return Err(SutraError::RuntimeGeneral {
                message: format!("core/str+ expects all arguments to be Strings, found {}", val.to_string()),
                src: NamedSource::new("atoms/collections.rs".to_string(), "".to_string()),
                span: to_source_span(Span::default()),
                suggestion: None,
            });
        };
        result.push_str(s);
    }

    Ok(Value::String(result))
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
pub const ATOM_CAR: PureAtomFn = |args| {
    helpers::validate_unary_arity(args, "car")?;

    let list_val = &args[0];
    let items = helpers::validate_list_value(list_val, "car")?;

    let first = items
        .first()
        .ok_or_else(|| SutraError::RuntimeGeneral {
            message: "car: empty list".to_string(),
            src: NamedSource::new("atoms/collections.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        })?;

    Ok(first.clone())
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
pub const ATOM_CDR: PureAtomFn = |args| {
    helpers::validate_unary_arity(args, "cdr")?;

    let list_val = &args[0];
    let items = helpers::validate_list_value(list_val, "cdr")?;

    if items.is_empty() {
        return Err(SutraError::RuntimeGeneral {
            message: "cdr: empty list".to_string(),
            src: NamedSource::new("atoms/collections.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
    }

    let rest = items[1..].to_vec();
    Ok(Value::List(rest))
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
pub const ATOM_CONS: PureAtomFn = |args| {
    helpers::validate_binary_arity(args, "cons")?;

    let element = &args[0];
    let list_val = &args[1];
    let items = helpers::validate_list_value(list_val, "cons")?;

    let mut new_list = vec![element.clone()];
    new_list.extend(items.iter().cloned());

    Ok(Value::List(new_list))
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
pub const ATOM_CORE_MAP: PureAtomFn = |args| {
    helpers::validate_even_arity(args, "core/map")?;

    let mut map = im::HashMap::new();
    for chunk in args.chunks(2) {
        let key = helpers::validate_string_value(&chunk[0], "map key")?.to_string();
        let value = chunk[1].clone();
        map.insert(key, value);
    }

    Ok(Value::Map(map))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all collection atoms with the given registry.
pub fn register_collection_atoms(registry: &mut AtomRegistry) {
    // List operations
    registry.register_pure("list", ATOM_LIST);
    registry.register_pure("len", ATOM_LEN);
    registry.register_pure("has?", ATOM_HAS);
    registry.register_pure("car", ATOM_CAR);
    registry.register_pure("cdr", ATOM_CDR);
    registry.register_pure("cons", ATOM_CONS);

    // Mutable list operations
    registry.register_stateful("core/push!", ATOM_CORE_PUSH);
    registry.register_stateful("core/pull!", ATOM_CORE_PULL);

    // String operations
    registry.register_pure("core/str+", ATOM_CORE_STR_PLUS);

    // Map operations
    registry.register_pure("core/map", ATOM_CORE_MAP);
}
