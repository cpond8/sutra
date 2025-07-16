//!
//! This module provides all collection manipulation atom operations for the Sutra engine.
//! These atoms work with lists, strings, and maps.
//!
//! ## Atoms Provided
//!
//! - **List Operations**: `list`, `len`, `has?`, `core/push!`, `core/pull!`
//! - **String Operations**: `core/str+`
//!
//! ## Design Principles
//!
//! - **Type Safety**: Clear error messages for type mismatches
//! - **Immutable Operations**: Pure functions where possible
//! - **Mutable Operations**: World-state operations for list manipulation

use crate::ast::value::Value;
// use crate::atoms::helpers::*;
use crate::atoms::{PureAtomFn, StatefulAtomFn};
use crate::sutra_err;

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
///
/// # Safety
/// Pure, does not mutate state.
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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_LEN: PureAtomFn = |args| {
    if args.len() != 1 {
        return Err(sutra_err!(Eval, "len expects 1 argument, got {}", args.len()));
    }
    match &args[0] {
        Value::List(items) => Ok(Value::Number(items.len() as f64)),
        Value::String(s) => Ok(Value::Number(s.len() as f64)),
        val => Err(sutra_err!(Eval, "len expects a List or String, found {}", val)),
    }
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
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_HAS: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(sutra_err!(Eval, "has? expects 2 arguments, got {}", args.len()));
    }
    let collection_val = &args[0];
    let search_val = &args[1];
    let found = match collection_val {
        Value::List(items) => items.contains(search_val),
        Value::Map(map) => {
            let Value::String(key) = search_val else {
                return Err(sutra_err!(Eval, "has? expects a String for Map key, found {}", search_val));
            };
            map.contains_key(&key[..])
        }
        _ => {
            return Err(sutra_err!(Eval, "has? expects a List or Map as first argument, found {}", collection_val));
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
    if args.len() != 2 {
        return Err(sutra_err!(Eval, "core/push! expects 2 arguments, got {}", args.len()));
    }
    let path = match &args[0] {
        Value::Path(p) => p,
        val => {
            return Err(sutra_err!(Eval, "core/push! expects a Path as first argument, found {}", val))
        }
    };
    let mut list_val = context
        .state
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));

    match &mut list_val {
        Value::List(items) => items.push(args[1].clone()),
        val => {
            return Err(sutra_err!(Eval, "core/push! expects a List at path, found {}", val))
        }
    }
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
    if args.len() != 1 {
        return Err(sutra_err!(Eval, "core/pull! expects 1 argument, got {}", args.len()));
    }
    let path = match &args[0] {
        Value::Path(p) => p,
        val => {
            return Err(sutra_err!(Eval, "core/pull! expects a Path as first argument, found {}", val))
        }
    };
    let mut list_val = context
        .state
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));

    let pulled_value = match &mut list_val {
        Value::List(items) => items.pop().unwrap_or(Value::Nil),
        val => {
            return Err(sutra_err!(Eval, "core/pull! expects a List at path, found {}", val))
        }
    };
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
///
/// # Safety
/// Pure, does not mutate state. This atom only accepts `Value::String` arguments.
pub const ATOM_CORE_STR_PLUS: PureAtomFn = |args| {
    if args.is_empty() {
        return Ok(Value::String(String::new()));
    }
    let mut result = String::new();
    for val in args.iter() {
        match val {
            Value::String(s) => result.push_str(&s[..]),
            _ => {
                return Err(sutra_err!(Eval, "core/str+ expects all arguments to be Strings, found {}", val));
            }
        }
    }
    Ok(Value::String(result))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all collection atoms with the given registry.
pub fn register_collection_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // List operations
    registry.register("list", crate::atoms::Atom::Pure(ATOM_LIST));
    registry.register("len", crate::atoms::Atom::Pure(ATOM_LEN));
    registry.register("has?", crate::atoms::Atom::Pure(ATOM_HAS));
    registry.register("core/push!", crate::atoms::Atom::Stateful(ATOM_CORE_PUSH));
    registry.register("core/pull!", crate::atoms::Atom::Stateful(ATOM_CORE_PULL));

    // String operations
    registry.register("core/str+", crate::atoms::Atom::Pure(ATOM_CORE_STR_PLUS));
}
