//! # Sutra Standard Atom Library
//!
//! This module provides the core, primitive operations of the engine.
//!
//! ## Atom Contracts
//!
//! - **Canonical Arguments**: Atoms assume their arguments are canonical and valid.
//!   For example, `set!` expects its first argument to evaluate to a `Value::Path`.
//!   It does no parsing or transformation itself.
//! - **State Propagation**: Atoms that modify state (like `set!`) must accept a
//!   `World` and return a new, modified `World`.
//! - **Clarity over Complexity**: Each atom has a single, clear responsibility.
//!   Complex operations are built by composing atoms, not by creating complex atoms.

use crate::ast::value::Value;
use crate::ast::AstNode;
use crate::ast::{Expr, WithSpan};
use crate::atoms::{AtomFn, AtomRegistry};
use crate::runtime::eval::{eval_expr, EvalContext};
use crate::syntax::error::{eval_arity_error, eval_general_error, eval_type_error};
use crate::syntax::error::{EvalError, SutraError, SutraErrorKind};

// ============================================================================
// CORE DATA STRUCTURES AND TYPE ALIASES
// ============================================================================

/// Convenient type alias for atom return values - modern Rust idiom
pub type AtomResult = Result<(Value, crate::runtime::world::World), SutraError>;

// ============================================================================
// PUBLIC API IMPLEMENTATION - STANDARD ATOMS
// ============================================================================

// ----------------------------------------------------------------------------
// Core atoms: World state manipulation
// ----------------------------------------------------------------------------

/// Sets a value at a path in the world state.
///
/// Usage: (core/set! <path> <value>)
///   - <path>: Path to set (must evaluate to a Value::Path)
///   - <value>: Value to store
///
///   Returns: Nil. Mutates world state (returns new world).
///
/// Example:
///   (core/set! player.score 42)
///
/// # Safety
/// Only mutates the world at the given path.
pub const ATOM_CORE_SET: AtomFn = |args, context, parent_span| {
    eval_binary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         value: Value,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let new_world = world.set(&path, value);
            Ok((Value::default(), new_world))
        },
        "core/set!",
    )
};

/// Gets a value at a path in the world state.
///
/// Usage: (core/get <path>)
///   - <path>: Path to get (must evaluate to a Value::Path)
///
///   Returns: Value at path, or Nil if not found.
///
/// Example:
///   (core/get player.score)
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_CORE_GET: AtomFn = |args, context, parent_span| {
    eval_unary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let value = world.get(&path).cloned().unwrap_or_default();
            Ok((value, world))
        },
        "core/get",
    )
};

/// Deletes a value at a path in the world state.
///
/// Usage: (core/del! <path>)
///   - <path>: Path to delete (must evaluate to a Value::Path)
///
///   Returns: Nil. Mutates world state (returns new world).
///
/// Example:
///   (core/del! player.score)
///
/// # Safety
/// Only mutates the world at the given path.
pub const ATOM_CORE_DEL: AtomFn = |args, context, parent_span| {
    eval_unary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let new_world = world.del(&path);
            Ok((Value::default(), new_world))
        },
        "core/del!",
    )
};

// ----------------------------------------------------------------------------
// Arithmetic atoms: Basic mathematical operations
// ----------------------------------------------------------------------------

/// Adds numbers.
///
/// Usage: (+ <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (sum)
///
/// Example:
///   (+ 1 2 3) ; => 6
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_ADD: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op(args, context, parent_span, 0.0, |a, b| a + b, "+")
};

/// Subtracts two numbers.
///
/// Usage: (- <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Number (a - b)
///
/// Example:
///   (- 5 2) ; => 3
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_SUB: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Number(a - b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "-",
    )
};

/// Multiplies numbers.
///
/// Usage: (* <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (product)
///
/// Example:
///   (* 2 3 4) ; => 24
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MUL: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op(args, context, parent_span, 1.0, |a, b| a * b, "*")
};

/// Divides two numbers.
///
/// Usage: (/ <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Number (a / b)
///
/// Example:
///   (/ 6 2) ; => 3
///
/// # Safety
/// Pure, does not mutate state. Errors on division by zero.
pub const ATOM_DIV: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Number(a / b),
        Some(|_a, b| {
            if b == 0.0 {
                Err("Division by zero")
            } else {
                Ok(())
            }
        }),
        "/",
    )
};

/// Modulo operation.
///
/// Usage: (mod <a> <b>)
///   - <a>, <b>: Integers
///
///   Returns: Number (a % b)
///
/// Example:
///   (mod 5 2) ; => 1
///
/// # Safety
/// Pure, does not mutate state. Errors on division by zero or non-integer input.
pub const ATOM_MOD: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Number((a as i64 % b as i64) as f64),
        Some(|a: f64, b: f64| -> Result<(), &'static str> {
            if b == 0.0 {
                return Err("Modulo by zero");
            }
            if a.fract() != 0.0 || b.fract() != 0.0 {
                return Err("Modulo expects integers");
            }
            Ok(())
        }),
        "mod",
    )
};

/// Absolute value of a number.
///
/// Usage: (abs <n>)
///   - <n>: Number
///
///   Returns: Number (absolute value)
///
/// Example:
///   (abs -5) ; => 5
///   (abs 3.14) ; => 3.14
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_ABS: AtomFn = |args, context, parent_span| {
    let (val, world) = eval_single_arg(args, context, parent_span, "abs")?;
    let n = extract_number(&val, args, parent_span, "abs")?;
    Ok((Value::Number(n.abs()), world))
};

/// Minimum of multiple numbers.
///
/// Usage: (min <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (minimum value)
///
/// Example:
///   (min 3 1 4) ; => 1
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MIN: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op(args, context, parent_span, f64::INFINITY, f64::min, "min")
};

/// Maximum of multiple numbers.
///
/// Usage: (max <a> <b> ...)
///   - <a>, <b>, ...: Numbers
///
///   Returns: Number (maximum value)
///
/// Example:
///   (max 3 1 4) ; => 4
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MAX: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op(
        args,
        context,
        parent_span,
        f64::NEG_INFINITY,
        f64::max,
        "max",
    )
};

// ----------------------------------------------------------------------------
// Comparison atoms: Relational and equality operations
// ----------------------------------------------------------------------------

/// Returns true if two values are equal.
///
/// Usage: (eq? <a> <b>)
///   - <a>, <b>: Values to compare
///
///   Returns: Bool
///
/// Example:
///   (eq? 1 1) ; => true
///   (eq? 1 2) ; => false
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_EQ: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a == b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "eq?",
    )
};

/// Returns true if a > b.
///
/// Usage: (gt? <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Bool
///
/// Example:
///   (gt? 3 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_GT: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a > b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "gt?",
    )
};

/// Returns true if a < b.
///
/// Usage: (lt? <a> <b>)
///   - <a>, <b: Numbers
///
///   Returns: Bool
///
/// Example:
///   (lt? 1 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_LT: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a < b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "lt?",
    )
};

/// Returns true if a >= b.
///
/// Usage: (gte? <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Bool
///
/// Example:
///   (gte? 2 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_GTE: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a >= b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "gte?",
    )
};

/// Returns true if a <= b.
///
/// Usage: (lte? <a> <b>)
///   - <a>, <b>: Numbers
///
///   Returns: Bool
///
/// Example:
///   (lte? 1 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_LTE: AtomFn = |args, context, parent_span| {
    eval_binary_numeric_op(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a <= b),
        None::<fn(f64, f64) -> Result<(), &'static str>>,
        "lte?",
    )
};

/// Returns true if a path exists in the world state.
///
/// Usage: (core/exists? <path>)
///   - <path>: Path to check (must evaluate to a Value::Path)
///
///   Returns: Bool
///
/// Example:
///   (core/exists? player.score) ; => true if path exists, false otherwise
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_EXISTS: AtomFn = |args, context, parent_span| {
    eval_unary_path_op(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let exists = world.get(&path).is_some();
            Ok((Value::Bool(exists), world))
        },
        "core/exists?",
    )
};

// ----------------------------------------------------------------------------
// Logic atoms: Boolean operations
// ----------------------------------------------------------------------------

/// Logical negation.
///
/// Usage: (not <a>)
///   - <a>: Boolean
///
///   Returns: Bool
///
/// Example:
///   (not true) ; => false
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_NOT: AtomFn = |args, context, parent_span| {
    eval_unary_bool_op(args, context, parent_span, |b: bool| Value::Bool(!b), "not")
};

// ----------------------------------------------------------------------------
// Collection and text atoms: List and string operations
// ----------------------------------------------------------------------------

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
    let (val, world) = eval_expr(&args[0], &mut sub_context)?;
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
                    "a String when searching in a Map",
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

/// Generates a pseudo-random number between 0.0 (inclusive) and 1.0 (exclusive).
///
/// Usage: (rand)
///   - No arguments
///
///   Returns: Number (pseudo-random float between 0.0 and 1.0)
///
/// Example:
///   (rand) ; => 0.7234567 (example)
///
/// # Safety
/// Pure random generation, does not mutate world state.
/// Uses a simple pseudo-random generator based on system time.
pub const ATOM_RAND: AtomFn = |args, context, parent_span| {
    if !args.is_empty() {
        return Err(arity_error(Some(parent_span.clone()), args, "rand", 0));
    }

    // Generate pseudo-random number using system time as seed
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let nanos = duration.as_nanos();

    let mut hasher = DefaultHasher::new();
    nanos.hash(&mut hasher);
    let hash = hasher.finish();

    // Convert to 0.0..1.0 range
    let random_value = (hash as f64) / (u64::MAX as f64);
    Ok((Value::Number(random_value), context.world.clone()))
};

/// Concatenates two or more strings into a single string.
///
/// # Example (Sutra script)
///
/// ```sutra
/// (str+ "foo" "bar" "baz") ; => "foobarbaz"
/// ```
///
/// This atom is not directly callable as a Rust function; it is invoked by the Sutra engine when evaluating `(core/str+ ...)` forms.
///
/// # Safety
/// This atom only accepts `Value::String` arguments. If any argument is not a string, a type error is returned.
///
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

/// Calls a function, macro, or atom with arguments, flattening the final list argument.
///
/// # Usage
/// ```sutra
/// (apply + 1 2 '(3 4)) ; => 10
/// (apply str+ '("a" "b" "c")) ; => "abc"
/// ```
///
/// # Errors
/// Returns an error if the function is not callable or the last argument is not a list.
///
/// # Safety
/// Pure, does not mutate state. All state is explicit.
pub const ATOM_APPLY: AtomFn = |args, context, parent_span| {
    if args.len() < 2 {
        return Err(arity_error(
            Some(parent_span.clone()),
            args,
            "apply",
            "at least 2",
        ));
    }

    let func_expr = &args[0];
    let normal_args_slice = &args[1..args.len() - 1];
    let list_arg = &args[args.len() - 1];

    // Evaluate normal arguments
    let (normal_args, world) = eval_apply_normal_args(normal_args_slice, context)?;

    // Evaluate list argument
    let mut context_with_world = sub_eval_context!(context, &world);
    let (list_args, world) = eval_apply_list_arg(list_arg, &mut context_with_world, parent_span)?;

    // Build and evaluate the call expression
    let call_expr = build_apply_call_expr(func_expr, normal_args, list_args, parent_span);
    let mut sub_context = sub_eval_context!(context, &world);
    eval_expr(&call_expr, &mut sub_context)
};

// ----------------------------------------------------------------------------
// Control flow atoms: Program structure and flow control
// ----------------------------------------------------------------------------

/// Sequentially evaluates expressions, returning the last value.
///
/// Usage: (do <expr1> <expr2> ...)
///   - <expr1>, <expr2>, ...: Expressions to evaluate in sequence
///
///   Returns: Value of last expression
///
/// Example:
///   (do (core/set! x 1) (core/get x)) ; => 1
///
/// # Safety
/// May mutate world if inner expressions do.
pub const ATOM_DO: AtomFn = |args, context, _| {
    // The `eval_args` helper function correctly threads the world state
    // through the evaluation of each argument. We can simply use it
    // and return the value of the last expression, which is the
    // standard behavior of a `do` block.
    let (values, world) = eval_args(args, context)?;
    let last_value = values.last().cloned().unwrap_or_default();
    Ok((last_value, world))
};

/// Raises an error with a message.
///
/// Usage: (error <message>)
///   - <message>: String
///
///   Returns: Error (never returns normally)
///
/// Example:
///   (error "fail!")
///
/// # Safety
/// Always errors. Does not mutate state.
pub const ATOM_ERROR: AtomFn = |args, context, parent_span| {
    eval_unary_value_op(
        args,
        context,
        parent_span,
        |msg_val: Value,
         _world: crate::runtime::world::World,
         parent_span: &crate::ast::Span,
         _context: &mut EvalContext<'_, '_>|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let Value::String(msg) = msg_val else {
                return Err(type_error(
                    Some(parent_span.clone()),
                    &args[0],
                    "error",
                    "a String",
                    &msg_val,
                ));
            };
            Err(SutraError {
                kind: SutraErrorKind::Eval(EvalError {
                    message: msg,
                    expanded_code: format!(
                        "{:?}",
                        WithSpan {
                            value: Expr::List(args.to_vec(), parent_span.clone()),
                            span: parent_span.clone(),
                        }
                    ), // FIX: use format! to get String, not .into()
                    original_code: None,
                    suggestion: None,
                }),
                span: Some(parent_span.clone()),
            })
        },
        "error",
    )
};

// ----------------------------------------------------------------------------
// I/O atoms: Input/output operations
// ----------------------------------------------------------------------------

/// Emits output to the output sink.
///
/// Usage: (print <value>)
///   - <value>: Any value
///
///   Returns: Nil. Emits output.
///
/// Example:
///   (print "hello")
///
/// # Safety
/// Emits output, does not mutate world state.
pub const ATOM_PRINT: AtomFn = |args, context, parent_span| {
    eval_unary_value_op(
        args,
        context,
        parent_span,
        |val: Value,
         world: crate::runtime::world::World,
         parent_span: &crate::ast::Span,
         context: &mut EvalContext<'_, '_>|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            context.output.emit(&val.to_string(), Some(parent_span));
            Ok((Value::Nil, world)) // Return Nil so the engine does not print again
        },
        "print",
    )
};

// --- FIX: Arc migration issues ---
// 1. Wrap Expr in .into() where AstNode expects Arc<Expr>.
// 2. Update Vec<WithSpan<Expr>> to Vec<AstNode> (WithSpan<Arc<Expr>>).
// 3. Update pattern matches and dereferences to work with Arc<Expr>.
// 4. Update function signatures and calls to use AstNode.
// 5. Add comments for non-obvious changes.
// ============================================================================
// INFRASTRUCTURE/TRAITS - TYPE EXTRACTION AND ARGUMENT EVALUATION
// ============================================================================

/// Generic type extraction trait for eliminating DRY violations in extract_* functions
trait ExtractValue<T> {
    fn extract(
        &self,
        args: &[AstNode],
        arg_index: usize,
        parent_span: &crate::ast::Span,
        name: &str,
        expected_type: &str,
    ) -> Result<T, SutraError>;
}

impl ExtractValue<f64> for Value {
    fn extract(
        &self,
        args: &[AstNode],
        arg_index: usize,
        parent_span: &crate::ast::Span,
        name: &str,
        expected_type: &str,
    ) -> Result<f64, SutraError> {
        match self {
            Value::Number(n) => Ok(*n),
            _ => Err(type_error(
                Some(parent_span.clone()),
                &args[arg_index],
                name,
                expected_type,
                self,
            )),
        }
    }
}

impl ExtractValue<bool> for Value {
    fn extract(
        &self,
        args: &[AstNode],
        arg_index: usize,
        parent_span: &crate::ast::Span,
        name: &str,
        expected_type: &str,
    ) -> Result<bool, SutraError> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(type_error(
                Some(parent_span.clone()),
                &args[arg_index],
                name,
                expected_type,
                self,
            )),
        }
    }
}

impl ExtractValue<crate::runtime::path::Path> for Value {
    fn extract(
        &self,
        args: &[AstNode],
        arg_index: usize,
        parent_span: &crate::ast::Span,
        name: &str,
        expected_type: &str,
    ) -> Result<crate::runtime::path::Path, SutraError> {
        match self {
            Value::Path(path) => Ok(path.clone()),
            _ => Err(type_error(
                Some(parent_span.clone()),
                &args[arg_index],
                name,
                expected_type,
                self,
            )),
        }
    }
}

// ============================================================================
// INTERNAL HELPERS - EVALUATION PATTERNS AND UTILITIES
// ============================================================================

// ----------------------------------------------------------------------------
// Error construction helpers
// ----------------------------------------------------------------------------

/// Creates an arity error for atoms with consistent messaging
pub fn arity_error(
    span: Option<crate::ast::Span>,
    args: &[AstNode],
    name: &str,
    expected: impl ToString,
) -> SutraError {
    eval_arity_error(span, args, name, expected)
}

/// Creates a type error for atoms with consistent messaging
pub fn type_error(
    span: Option<crate::ast::Span>,
    arg: &AstNode,
    name: &str,
    expected: &str,
    found: &Value,
) -> SutraError {
    eval_type_error(span, arg, name, expected, found)
}

/// Creates a validation error for atoms with consistent messaging
pub fn validation_error(
    span: Option<crate::ast::Span>,
    arg: &AstNode,
    message: &str,
) -> SutraError {
    eval_general_error(span, arg, message)
}

/// Macro to create a sub-evaluation context with a new world state.
/// This centralizes the repetitive context construction pattern used throughout atoms.
///
/// # Usage
/// ```ignore
/// let mut sub_context = sub_eval_context!(parent_context, &new_world);
/// let (result, world) = eval_expr(&args[0], &mut sub_context)?;
/// ```
#[macro_export]
macro_rules! sub_eval_context {
    ($parent:expr, $world:expr) => {
        $crate::runtime::eval::EvalContext {
            world: $world,
            output: $parent.output,
            atom_registry: $parent.atom_registry,
            max_depth: $parent.max_depth,
            depth: $parent.depth,
        }
    };
}

// Also provide the local version for use within this module
use sub_eval_context;

// ----------------------------------------------------------------------------
// Basic argument evaluation helpers
// ----------------------------------------------------------------------------

/// Evaluates all arguments in sequence, threading world state through each evaluation.
fn eval_args(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
) -> Result<(Vec<Value>, crate::runtime::world::World), SutraError> {
    args.iter().try_fold(
        (Vec::with_capacity(args.len()), context.world.clone()),
        |(mut values, world), arg| {
            let mut sub_context = sub_eval_context!(context, &world);
            let (val, next_world) = eval_expr(arg, &mut sub_context)?;
            values.push(val);
            Ok((values, next_world))
        },
    )
}

/// Generic argument evaluation with compile-time arity checking
fn eval_n_args<const N: usize>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<([Value; N], crate::runtime::world::World), SutraError> {
    if args.len() != N {
        return Err(arity_error(Some(parent_span.clone()), args, name, N));
    }

    let mut values = Vec::with_capacity(N);
    let mut world = context.world.clone();

    for arg in args.iter().take(N) {
        let mut sub_context = sub_eval_context!(context, &world);
        let (val, next_world) = eval_expr(arg, &mut sub_context)?;
        values.push(val);
        world = next_world;
    }

    // Convert Vec to array - this is safe because we checked length
    let values_array: [Value; N] = values
        .try_into()
        .map_err(|_| arity_error(Some(parent_span.clone()), args, name, N))?;

    Ok((values_array, world))
}

/// Evaluates a single argument and returns the value and world
fn eval_single_arg(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError> {
    let ([val], world) = eval_n_args::<1>(args, context, parent_span, name)?;
    Ok((val, world))
}

/// Evaluates two arguments and returns both values and the final world
fn eval_binary_args(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<(Value, Value, crate::runtime::world::World), SutraError> {
    let ([val1, val2], world) = eval_n_args::<2>(args, context, parent_span, name)?;
    Ok((val1, val2, world))
}

// ----------------------------------------------------------------------------
// Type extraction helpers
// ----------------------------------------------------------------------------

/// Extracts two numbers from values with type checking using the trait
fn extract_numbers(
    val1: &Value,
    val2: &Value,
    args: &[AstNode],
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<(f64, f64), SutraError> {
    let n1 = val1.extract(args, 0, parent_span, name, "a Number")?;
    let n2 = val2.extract(args, 1, parent_span, name, "a Number")?;
    Ok((n1, n2))
}

/// Extracts a single number from a value with type checking using the trait
fn extract_number(
    val: &Value,
    args: &[AstNode],
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<f64, SutraError> {
    val.extract(args, 0, parent_span, name, "a Number")
}

/// Extracts a boolean from a value with type checking using the trait
fn extract_bool(
    val: &Value,
    args: &[AstNode],
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<bool, SutraError> {
    val.extract(args, 0, parent_span, name, "a Boolean")
}

/// Extracts a path from a value with type checking using the trait
fn extract_path(
    val: &Value,
    args: &[AstNode],
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<crate::runtime::path::Path, SutraError> {
    val.extract(args, 0, parent_span, name, "a Path")
}

// ----------------------------------------------------------------------------
// Operation evaluation templates
// ----------------------------------------------------------------------------

/// Evaluates a binary numeric operation atomically, with optional validation.
/// Handles arity, type checking, and error construction.
fn eval_binary_numeric_op<F, V>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    validator: Option<V>,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(f64, f64) -> Value,
    V: Fn(f64, f64) -> Result<(), &'static str>,
{
    let (val1, val2, world) = eval_binary_args(args, context, parent_span, name)?;
    let (n1, n2) = extract_numbers(&val1, &val2, args, parent_span, name)?;

    if let Some(validate) = validator {
        validate(n1, n2)
            .map_err(|msg| validation_error(Some(parent_span.clone()), &args[1], msg))?;
    }

    Ok((op(n1, n2), world))
}

/// Evaluates an n-ary numeric operation (e.g., sum, product).
/// Handles arity, type checking, and error construction.
fn eval_nary_numeric_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    init: f64,
    fold: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(f64, f64) -> f64,
{
    if args.len() < 2 {
        return Err(arity_error(
            Some(parent_span.clone()),
            args,
            name,
            "at least 2",
        ));
    }

    let (values, world) = eval_args(args, context)?;
    let mut acc = init;

    for (i, v) in values.iter().enumerate() {
        let n = extract_number(v, args, parent_span, name)
            .map_err(|_| type_error(Some(parent_span.clone()), &args[i], name, "a Number", v))?;
        acc = fold(acc, n);
    }

    Ok((Value::Number(acc), world))
}

/// Evaluates a unary boolean operation.
/// Handles arity, type checking, and error construction.
fn eval_unary_bool_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(bool) -> Value,
{
    let (val, world) = eval_single_arg(args, context, parent_span, name)?;
    let b = extract_bool(&val, args, parent_span, name)?;
    Ok((op(b), world))
}

/// Evaluates a unary path operation (get, del).
/// Handles arity, type checking, and error construction.
fn eval_unary_path_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        crate::runtime::path::Path,
        crate::runtime::world::World,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (val, world) = eval_single_arg(args, context, parent_span, name)?;
    let path = extract_path(&val, args, parent_span, name)?;
    op(path, world)
}

/// Evaluates a binary path operation (set).
/// Handles arity, type checking, and error construction.
fn eval_binary_path_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        crate::runtime::path::Path,
        Value,
        crate::runtime::world::World,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (path_val, value, world) = eval_binary_args(args, context, parent_span, name)?;
    let path = extract_path(&path_val, args, parent_span, name)?;
    op(path, value, world)
}

/// Evaluates a unary operation that takes any value.
/// Handles arity and error construction.
fn eval_unary_value_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        Value,
        crate::runtime::world::World,
        &crate::ast::Span,
        &mut EvalContext<'_, '_>,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (val, world) = eval_single_arg(args, context, parent_span, name)?;
    op(val, world, parent_span, context)
}

// ----------------------------------------------------------------------------
// Apply atom helpers
// ----------------------------------------------------------------------------

/// Evaluates normal arguments for apply (all except the last argument).
/// Returns the evaluated arguments as expressions and the final world state.
fn eval_apply_normal_args(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
) -> Result<(Vec<AstNode>, crate::runtime::world::World), SutraError> {
    let mut evald_args = Vec::with_capacity(args.len());
    let mut world = context.world.clone();
    for arg in args {
        let mut sub_context = sub_eval_context!(context, &world);
        let (val, next_world) = eval_expr(arg, &mut sub_context)?;
        evald_args.push(WithSpan {
            value: Expr::from(val).into(), // FIX: wrap Expr in Arc via .into()
            span: arg.span.clone(),
        });
        world = next_world;
    }
    Ok((evald_args, world))
}

/// Evaluates the list argument for apply (the last argument).
/// Returns the list items as expressions and the final world state.
fn eval_apply_list_arg(
    arg: &AstNode,
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
) -> Result<(Vec<AstNode>, crate::runtime::world::World), SutraError> {
    let mut sub_context = sub_eval_context!(context, context.world);
    let (list_val, world) = eval_expr(arg, &mut sub_context)?;
    let Value::List(items) = list_val else {
        return Err(type_error(
            Some(parent_span.clone()),
            arg,
            "apply",
            "a List as the last argument",
            &list_val,
        ));
    };
    let list_items = items
        .into_iter()
        .map(|v| WithSpan {
            value: Expr::from(v).into(), // FIX: wrap Expr in Arc via .into()
            span: parent_span.clone(),
        })
        .collect();
    Ok((list_items, world))
}

/// Builds the call expression for apply by combining function, normal args, and list args.
fn build_apply_call_expr(
    func_expr: &AstNode,
    normal_args: Vec<AstNode>,
    list_args: Vec<AstNode>,
    parent_span: &crate::ast::Span,
) -> AstNode {
    let mut call_items = Vec::with_capacity(1 + normal_args.len() + list_args.len());
    call_items.push(func_expr.clone());
    call_items.extend(normal_args);
    call_items.extend(list_args);
    WithSpan {
        value: Expr::List(call_items, parent_span.clone()).into(), // FIX: wrap Expr in Arc via .into()
        span: parent_span.clone(),
    }
}

// ============================================================================
// MODULE EXPORTS - REGISTRATION FUNCTIONS
// ============================================================================

#[cfg(any(test, feature = "test-atom", debug_assertions))]
/// Registers all standard atoms in the given registry.
pub fn register_std_atoms(registry: &mut AtomRegistry) {
    // Core atoms
    registry.register("core/set!", ATOM_CORE_SET);
    registry.register("core/get", ATOM_CORE_GET);
    registry.register("core/del!", ATOM_CORE_DEL);

    // Arithmetic atoms
    registry.register("+", ATOM_ADD);
    registry.register("-", ATOM_SUB);
    registry.register("*", ATOM_MUL);
    registry.register("/", ATOM_DIV);
    registry.register("mod", ATOM_MOD);
    registry.register("abs", ATOM_ABS);
    registry.register("min", ATOM_MIN);
    registry.register("max", ATOM_MAX);

    // Comparison atoms
    registry.register("eq?", ATOM_EQ);
    registry.register("gt?", ATOM_GT);
    registry.register("lt?", ATOM_LT);
    registry.register("gte?", ATOM_GTE);
    registry.register("lte?", ATOM_LTE);
    registry.register("core/exists?", ATOM_EXISTS);

    // Logic atoms
    registry.register("not", ATOM_NOT);

    // List and string atoms
    registry.register("list", ATOM_LIST);
    registry.register("len", ATOM_LEN);
    registry.register("has?", ATOM_HAS);
    registry.register("core/push!", ATOM_CORE_PUSH);
    registry.register("core/pull!", ATOM_CORE_PULL);
    registry.register("core/str+", ATOM_CORE_STR_PLUS);
    registry.register("apply", ATOM_APPLY);

    // Control flow atoms
    registry.register("do", ATOM_DO);
    registry.register("error", ATOM_ERROR);

    // Random number generation
    registry.register("rand", ATOM_RAND);

    // I/O atoms
    registry.register("print", ATOM_PRINT);
    registry.register("core/print", ATOM_PRINT);
}
