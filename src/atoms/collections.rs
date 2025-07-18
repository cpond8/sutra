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
use crate::atoms::{PureAtomFn, StatefulAtomFn};
use crate::atoms::helpers::{validate_unary_arity, validate_binary_arity, validate_even_arity};
use crate::err_msg;

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
    validate_unary_arity(args, "len")?;
    match &args[0] {
        Value::List(items) => Ok(Value::Number(items.len() as f64)),
        Value::String(s) => Ok(Value::Number(s.len() as f64)),
        _ => Err(err_msg!(
            Eval,
            "len expects a List or String, found {}",
            args[0].to_string()
        )),
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
    validate_binary_arity(args, "has?")?;
    let collection_val = &args[0];
    let search_val = &args[1];
    let found = match collection_val {
        Value::List(items) => items.contains(search_val),
        Value::Map(map) => {
            let Value::String(key) = search_val else {
                return Err(err_msg!(
                    Eval,
                    "has? expects a String for Map key, found {}",
                    search_val.to_string()
                ));
            };
            map.contains_key(&key[..])
        }
        _ => {
            return Err(err_msg!(
                Eval,
                "has? expects a List or Map as first argument, found {}",
                collection_val.to_string()
            ));
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
    validate_binary_arity(args, "core/push!")?;
    let path = match &args[0] {
        Value::Path(p) => p,
        _ => {
            return Err(err_msg!(
                Eval,
                "core/push! expects a Path as first argument, found {}",
                args[0].to_string()
            ))
        }
    };
    let mut list_val = context
        .state
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));

    match &mut list_val {
        Value::List(items) => items.push(args[1].clone()),
        _ => {
            return Err(err_msg!(
                Eval,
                "core/push! expects a List at path, found {}",
                list_val.to_string()
            ))
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
    validate_unary_arity(args, "core/pull!")?;
    let path = match &args[0] {
        Value::Path(p) => p,
        _ => {
            return Err(err_msg!(
                Eval,
                "core/pull! expects a Path as first argument, found {}",
                args[0].to_string()
            ))
        }
    };
    let mut list_val = context
        .state
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));

    let pulled_value = match &mut list_val {
        Value::List(items) => items.pop().unwrap_or(Value::Nil),
        _ => {
            return Err(err_msg!(
                Eval,
                "core/pull! expects a List at path, found {}",
                list_val.to_string()
            ))
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
                return Err(err_msg!(
                    Eval,
                    "core/str+ expects all arguments to be Strings, found {}",
                    val.to_string()
                ));
            }
        }
    }
    Ok(Value::String(result))
};

/// Returns the first element of a list.
///
/// Usage: (car <list>)
///   - <list>: List to extract the first element from
///
///   Returns: The first element of the list
///
/// Example:
///   (car (list 1 2 3)) ; => 1
///
/// # Errors
/// Returns an error if the argument is not a list or if the list is empty.
pub const ATOM_CAR: PureAtomFn = |args| {
    validate_unary_arity(args, "car")?;
    match &args[0] {
        Value::List(items) => {
            if let Some(first) = items.first() {
                Ok(first.clone())
            } else {
                Err(err_msg!(Eval, "car: empty list"))
            }
        }
        _ => Err(err_msg!(Eval, "car expects a List, found {}", args[0].to_string())),
    }
};

/// Returns the tail (all but the first element) of a list.
///
/// Usage: (cdr <list>)
///   - <list>: List to extract the tail from
///
///   Returns: List containing all elements except the first
///
/// Example:
///   (cdr (list 1 2 3)) ; => (2 3)
///
/// # Errors
/// Returns an error if the argument is not a list or if the list is empty.
pub const ATOM_CDR: PureAtomFn = |args| {
    validate_unary_arity(args, "cdr")?;
    match &args[0] {
        Value::List(items) => {
            if items.is_empty() {
                Err(err_msg!(Eval, "cdr: empty list"))
            } else {
                Ok(Value::List(items[1..].to_vec()))
            }
        }
        _ => Err(err_msg!(Eval, "cdr expects a List, found {}", args[0].to_string())),
    }
};

/// Prepends an element to a list.
///
/// Usage: (cons <element> <list>)
///   - <element>: Value to prepend
///   - <list>: List to prepend to
///
///   Returns: New list with the element prepended
///
/// Example:
///   (cons 1 (list 2 3)) ; => (1 2 3)
///
/// # Errors
/// Returns an error if the second argument is not a list.
pub const ATOM_CONS: PureAtomFn = |args| {
    validate_binary_arity(args, "cons")?;
    match &args[1] {
        Value::List(items) => {
            let mut new_list = Vec::with_capacity(items.len() + 1);
            new_list.push(args[0].clone());
            new_list.extend_from_slice(items);
            Ok(Value::List(new_list))
        }
        _ => Err(err_msg!(Eval, "cons expects second argument to be a List, found {}", args[1].to_string())),
    }
};

/// Creates a map from alternating key-value pairs.
///
/// Usage: (core/map <key1> <value1> <key2> <value2> ...)
///   - <key1>, <key2>, ...: String keys
///   - <value1>, <value2>, ...: Values to associate with keys
///
///   Returns: Map with the specified key-value pairs
///
/// Example:
///   (core/map ":span" (list 0 0) ":file" "test.sutra") ; => {:span (0 0), :file "test.sutra"}
///
/// # Errors
/// Returns an error if the number of arguments is odd or if any key is not a string.
pub const ATOM_CORE_MAP: PureAtomFn = |args| {
        validate_even_arity(args, "core/map")?;

    let mut map = im::HashMap::new();
    for chunk in args.chunks(2) {
        let key = match &chunk[0] {
            Value::String(s) => s.clone(),
            _ => return Err(err_msg!(Eval, "core/map expects string keys, found {}", chunk[0].to_string())),
        };
        let value = chunk[1].clone();
        map.insert(key, value);
    }

    Ok(Value::Map(map))
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
    registry.register("car", crate::atoms::Atom::Pure(ATOM_CAR));
    registry.register("cdr", crate::atoms::Atom::Pure(ATOM_CDR));
    registry.register("cons", crate::atoms::Atom::Pure(ATOM_CONS));

    // Map operations
    registry.register("core/map", crate::atoms::Atom::Pure(ATOM_CORE_MAP));
}
