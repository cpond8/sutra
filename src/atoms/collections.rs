//!
//! This module provides all collection manipulation atom operations for the Sutra engine.
//! These atoms work with lists, strings, and maps.
//!
//! ## Atoms Provided
//!
//! ### List Operations
//! - `list` - Constructs a list from arguments
//! - `len` - Returns the length of a list or string
//! - `has?` - Tests if a collection contains a value or key
//! - `core/push!` - Appends a value to a list at a path
//! - `core/pull!` - Removes and returns the last element from a list
//! - `car` - Returns the first element of a list
//! - `cdr` - Returns the tail of a list
//! - `cons` - Prepends an element to a list
//!
//! ### String Operations
//! - `core/str+` - Concatenates two or more strings
//!
//! ### Map Operations
//! - `core/map` - Creates a map from alternating key-value pairs
//!
//! ## Design Principles
//!
//! - **Type Safety**: Clear error messages for type mismatches
//! - **Immutable Operations**: Pure functions where possible
//! - **Mutable Operations**: World-state operations for list manipulation
//! - **Performance**: Minimal cloning, efficient string operations

use crate::{
    atoms::{
        helpers::{
            validate_binary_arity, validate_even_arity, validate_list_value,
            validate_list_value_mut, validate_path_arg, validate_string_value,
            validate_unary_arity,
        },
        AtomRegistry, PureAtomFn, StatefulAtomFn,
    },
    err_msg, Value,
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
    validate_unary_arity(args, "len")?;

    let val = &args[0];
    let len = match val {
        Value::List(items) => items.len(),
        Value::String(s) => s.len(),
        _ => {
            return Err(err_msg!(
                Eval,
                "len expects a List or String, found {}",
                val.to_string()
            ));
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
    validate_binary_arity(args, "has?")?;

    let collection_val = &args[0];
    let search_val = &args[1];

    let found = match collection_val {
        Value::List(items) => items.contains(search_val),
        Value::Map(map) => {
            let key = validate_string_value(search_val, "map key")?;
            map.contains_key(key)
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

    let path = validate_path_arg(args, "core/push!")?;

    // Get existing list or create new one
    let mut list_val = context
        .state
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));

    // Validate and modify list
    let items = validate_list_value_mut(&mut list_val, "core/push!")?;
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
    validate_unary_arity(args, "core/pull!")?;

    let path = validate_path_arg(args, "core/pull!")?;

    // Get existing list or create new one
    let mut list_val = context
        .state
        .get(path)
        .cloned()
        .unwrap_or(Value::List(vec![]));

    // Validate and modify list
    let items = validate_list_value_mut(&mut list_val, "core/pull!")?;
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
    // Handle empty args case explicitly
    if args.is_empty() {
        return Ok(Value::String(String::new()));
    }

    // Pre-calculate total capacity for better performance
    let total_capacity: usize = args
        .iter()
        .filter_map(|v| {
            if let Value::String(s) = v {
                Some(s.len())
            } else {
                None
            }
        })
        .sum();

    let mut result = String::with_capacity(total_capacity);

    for val in args.iter() {
        let Value::String(s) = val else {
            return Err(err_msg!(
                Eval,
                "core/str+ expects all arguments to be Strings, found {}",
                val.to_string()
            ));
        };
        result.push_str(s);
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

    let Value::List(items) = &args[0] else {
        return Err(err_msg!(
            Eval,
            "car expects a List, found {}",
            args[0].to_string()
        ));
    };

    let first = items
        .first()
        .ok_or_else(|| err_msg!(Eval, "car: empty list"))?;
    Ok(first.clone())
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

    let Value::List(items) = &args[0] else {
        return Err(err_msg!(
            Eval,
            "cdr expects a List, found {}",
            args[0].to_string()
        ));
    };

    if items.is_empty() {
        return Err(err_msg!(Eval, "cdr: empty list"));
    }

    Ok(Value::List(items[1..].to_vec()))
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

    let items = validate_list_value(&args[1], "cons second argument")?;

    // Pre-allocate vector for better performance
    let mut new_list = Vec::with_capacity(items.len() + 1);
    new_list.push(args[0].clone());
    new_list.extend_from_slice(items);

    Ok(Value::List(new_list))
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
        let key = validate_string_value(&chunk[0], "core/map key")?.to_string();
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
    registry.register_stateful("core/push!", ATOM_CORE_PUSH);
    registry.register_stateful("core/pull!", ATOM_CORE_PULL);

    // String operations
    registry.register_pure("core/str+", ATOM_CORE_STR_PLUS);
    registry.register_pure("car", ATOM_CAR);
    registry.register_pure("cdr", ATOM_CDR);
    registry.register_pure("cons", ATOM_CONS);

    // Map operations
    registry.register_pure("core/map", ATOM_CORE_MAP);
}
