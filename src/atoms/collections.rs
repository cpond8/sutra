//! # Collection Operations
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
use crate::atoms::helpers::*;
use crate::atoms::AtomFn;
use crate::syntax::error::{EvalError, SutraError, SutraErrorKind};

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
pub const ATOM_LIST: AtomFn = |args, context, _| {
    let (items, world) = eval_args(args, context)?;
    Ok((Value::List(items), world))
};

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
pub const ATOM_LEN: AtomFn = |args, context, parent_span| {
    if args.len() != 1 {
        return Err(arity_error(Some(parent_span.clone()), args, "len", 1));
    }
    let mut sub_context = sub_eval_context!(context, context.world);
    let (val, world) = crate::runtime::eval::eval_expr(&args[0], &mut sub_context)?;
    match val {
        Value::List(ref items) => Ok((Value::Number(items.len() as f64), world)),
        Value::String(ref s) => Ok((Value::Number(s.len() as f64), world)),
        _ => Err(type_error(
            Some(parent_span.clone()),
            &args[0],
            "len",
            "a List or String",
            &val,
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
pub const ATOM_HAS: AtomFn = |args, context, parent_span| {
    if args.len() != 2 {
        return Err(arity_error(Some(parent_span.clone()), args, "has?", 2));
    }

    let (collection_val, search_val, world) = eval_binary_args(args, context, parent_span, "has?")?;

    let found = match collection_val {
        Value::List(ref items) => items.contains(&search_val),
        Value::Map(ref map) => {
            // For maps, check if the search value exists as a key
            let Value::String(key) = search_val else {
                return Err(type_error(
                    Some(parent_span.clone()),
                    &args[1],
                    "has?",
                    "a String (for Map keys)",
                    &search_val,
                ));
            };
            map.contains_key(&key)
        }
        _ => {
            return Err(type_error(
                Some(parent_span.clone()),
                &args[0],
                "has?",
                "a List or Map",
                &collection_val,
            ));
        }
    };

    Ok((Value::Bool(found), world))
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
/// Mutates the world at the given path. Creates empty list if path doesn't exist.
pub const ATOM_CORE_PUSH: AtomFn = |args, context, parent_span| {
    eval_binary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         value: Value,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let mut current = world.get(&path).cloned().unwrap_or(Value::List(vec![]));

            let Value::List(ref mut items) = current else {
                return Err(SutraError {
                    kind: SutraErrorKind::Eval(EvalError {
                        message: format!("Cannot push to non-list value at path '{}'", path),
                        expanded_code: format!("(core/push! {} {:?})", path, value),
                        original_code: None,
                        suggestion: Some(
                            "Ensure the path contains a list before pushing".to_string(),
                        ),
                    }),
                    span: Some(parent_span.clone()),
                });
            };

            items.push(value);
            let new_world = world.set(&path, current);
            Ok((Value::default(), new_world))
        },
        "core/push!",
    )
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
/// Mutates the world at the given path. Creates empty list if path doesn't exist.
pub const ATOM_CORE_PULL: AtomFn = |args, context, parent_span| {
    eval_unary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let mut current = world.get(&path).cloned().unwrap_or(Value::List(vec![]));

            let Value::List(ref mut items) = current else {
                return Err(SutraError {
                    kind: SutraErrorKind::Eval(EvalError {
                        message: format!("Cannot pull from non-list value at path '{}'", path),
                        expanded_code: format!("(core/pull! {})", path),
                        original_code: None,
                        suggestion: Some(
                            "Ensure the path contains a list before pulling".to_string(),
                        ),
                    }),
                    span: Some(parent_span.clone()),
                });
            };

            let pulled_value = items.pop().unwrap_or_default();
            let new_world = world.set(&path, current);
            Ok((pulled_value, new_world))
        },
        "core/pull!",
    )
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
pub const ATOM_CORE_STR_PLUS: AtomFn = |args, context, parent_span| {
    // Handle zero arguments: return empty string
    if args.is_empty() {
        return Ok((Value::String(String::new()), context.world.clone()));
    }

    // Evaluate all arguments in the current context.
    let (values, world) = eval_args(args, context)?;
    // Collect string slices, error if any argument is not a string.
    let mut result = String::new();
    for (i, val) in values.iter().enumerate() {
        match val {
            Value::String(s) => result.push_str(s),
            _ => {
                return Err(type_error(
                    Some(parent_span.clone()),
                    &args[i],
                    "core/str+",
                    "a String",
                    val,
                ));
            }
        }
    }
    Ok((Value::String(result), world))
};

// ============================================================================
// REGISTRATION FUNCTION
// ============================================================================

/// Registers all collection atoms with the given registry.
pub fn register_collection_atoms(registry: &mut crate::atoms::AtomRegistry) {
    // List operations
    registry.register("list", crate::atoms::Atom::Legacy(ATOM_LIST));
    registry.register("len", crate::atoms::Atom::Legacy(ATOM_LEN));
    registry.register("has?", crate::atoms::Atom::Legacy(ATOM_HAS));
    registry.register("core/push!", crate::atoms::Atom::Legacy(ATOM_CORE_PUSH));
    registry.register("core/pull!", crate::atoms::Atom::Legacy(ATOM_CORE_PULL));

    // String operations
    registry.register("core/str+", crate::atoms::Atom::Legacy(ATOM_CORE_STR_PLUS));
}
