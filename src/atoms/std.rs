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
use crate::ast::{Expr, WithSpan};
use crate::atoms::{AtomFn, AtomRegistry};
use crate::runtime::eval::{eval_expr, EvalContext};
use crate::syntax::error::{eval_arity_error, eval_general_error, eval_type_error};
use crate::syntax::error::{EvalError, SutraError, SutraErrorKind};

// Add macro for error construction at the top of the file if not already in scope
macro_rules! sutra_error {
    (arity, $span:expr, $args:expr, $func:expr, $expected:expr) => {
        eval_arity_error($span, $args, $func, $expected)
    };
    (type, $span:expr, $arg:expr, $func:expr, $expected:expr, $found:expr) => {
        eval_type_error($span, $arg, $func, $expected, $found)
    };
    (general, $span:expr, $arg:expr, $msg:expr) => {
        eval_general_error($span, $arg, $msg)
    };
    (recursion, $span:expr) => {
        recursion_depth_error($span)
    };
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

// ---
// Registry
// ---

// ATOM_CORE_SET, ATOM_CORE_GET, ATOM_CORE_DEL, ATOM_ADD, ATOM_SUB, ATOM_MUL, ATOM_DIV, ATOM_MOD, ATOM_EQ, ATOM_GT, ATOM_LT, ATOM_GTE, ATOM_LTE, ATOM_NOT, ATOM_DO, ATOM_LIST, ATOM_LEN, ATOM_ERROR, ATOM_PRINT

/*
NOTE: Direct doctests for atoms (e.g., ATOM_ADD) are not feasible here because they require
internal context types (EvalContext, EvalOptions, World) that are not public or do not implement
required traits for doctesting. The previous doctest example was removed because it cannot compile
outside the engine crate. Atom functions are best tested via integration or unit tests
where the full engine context is available. See tests/ for examples.
*/

// ---
// Error Handling & Helpers
// ---

// Update eval_args to use eval_expr with &mut context and manually update world.
fn eval_args(
    args: &[WithSpan<Expr>],
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

/// Macro for binary numeric or predicate operations.
/// Usage: eval_binary_op!(args, context, parent_span, |a, b| ..., "atom_name", "expected_type")
macro_rules! eval_binary_op {
    ($args:expr, $context:expr, $parent_span:expr, $op:expr, $name:expr, $expected:expr) => {{
        if $args.len() != 2 {
            return Err(sutra_error!(
                arity,
                Some($parent_span.clone()),
                $args,
                $name,
                2
            ));
        }
        let mut sub_context1 = sub_eval_context!($context, $context.world);
        let (val1, world1) = eval_expr(&$args[0], &mut sub_context1)?;
        let mut sub_context2 = sub_eval_context!($context, &world1);
        let (val2, world2) = eval_expr(&$args[1], &mut sub_context2)?;
        match (&val1, &val2) {
            (Value::Number(n1), Value::Number(n2)) => Ok(($op(*n1, *n2), world2)),
            _ => Err(sutra_error!(
                type,
                Some($parent_span.clone()),
                &$args[0],
                $name,
                $expected,
                &val1
            )),
        }
    }};
}

/// Macro for binary numeric operations with custom validation (e.g., division by zero check).
/// Usage: eval_binary_op_with_validation!(args, context, parent_span, |a, b| ..., "atom_name", |a, b| validation_result)
macro_rules! eval_binary_op_with_validation {
    ($args:expr, $context:expr, $parent_span:expr, $op:expr, $name:expr, $validator:expr) => {{
        if $args.len() != 2 {
            return Err(sutra_error!(
                arity,
                Some($parent_span.clone()),
                $args,
                $name,
                2
            ));
        }
        let mut sub_context1 = sub_eval_context!($context, $context.world);
        let (val1, world1) = eval_expr(&$args[0], &mut sub_context1)?;
        let mut sub_context2 = sub_eval_context!($context, &world1);
        let (val2, world2) = eval_expr(&$args[1], &mut sub_context2)?;
        match (&val1, &val2) {
            (Value::Number(n1), Value::Number(n2)) => {
                // Apply custom validation
                if let Err(err_msg) = $validator(*n1, *n2) {
                    return Err(sutra_error!(
                        general,
                        Some($parent_span.clone()),
                        &$args[1],
                        err_msg
                    ));
                }
                Ok(($op(*n1, *n2), world2))
            }
            _ => Err(sutra_error!(
                general,
                Some($parent_span.clone()),
                &$args[0],
                format!(
                    "`{}` expects two Numbers, got {} and {}",
                    $name,
                    val1.type_name(),
                    val2.type_name()
                )
            )),
        }
    }};
}

/// Macro for n-ary numeric operations (sum, product, etc.)
/// Usage: eval_nary_numeric_op!(args, context, parent_span, init, |acc, v| ..., "atom_name")
macro_rules! eval_nary_numeric_op {
    ($args:expr, $context:expr, $parent_span:expr, $init:expr, $fold:expr, $name:expr) => {{
        if $args.len() < 2 {
            return Err(sutra_error!(
                arity,
                Some($parent_span.clone()),
                $args,
                $name,
                "at least 2"
            ));
        }
        let (values, world) = eval_args($args, $context)?;
        let mut acc = $init;
        for (i, v) in values.iter().enumerate() {
            if let Value::Number(n) = v {
                acc = $fold(acc, *n);
            } else {
                return Err(sutra_error!(
                    type,
                    Some($parent_span.clone()),
                    &$args[i],
                    $name,
                    "a Number",
                    v
                ));
            }
        }
        Ok((Value::Number(acc), world))
    }};
}

/// Macro for boolean unary operations.
/// Usage: eval_unary_bool_op!(args, context, parent_span, |b| ..., "atom_name")
macro_rules! eval_unary_bool_op {
    ($args:expr, $context:expr, $parent_span:expr, $op:expr, $name:expr) => {{
        if $args.len() != 1 {
            return Err(sutra_error!(
                arity,
                Some($parent_span.clone()),
                $args,
                $name,
                1
            ));
        }
        let mut sub_context = sub_eval_context!($context, $context.world);
        let (val, world) = eval_expr(&$args[0], &mut sub_context)?;
        match val {
            Value::Bool(b) => {
                let result = $op(b);
                Ok((result, world))
            }
            _ => Err(sutra_error!(
                type,
                Some($parent_span.clone()),
                &$args[0],
                $name,
                "a Boolean",
                &val
            )),
        }
    }};
}

/// Macro for unary path operations (get, del).
/// Usage: eval_unary_path_op!(args, context, parent_span, |path, world| ..., "atom_name")
macro_rules! eval_unary_path_op {
    ($args:expr, $context:expr, $parent_span:expr, $op:expr, $name:expr) => {{
        if $args.len() != 1 {
            return Err(sutra_error!(
                arity,
                Some($parent_span.clone()),
                $args,
                $name,
                1
            ));
        }
        let mut sub_context = sub_eval_context!($context, $context.world);
        let (path_val, world) = eval_expr(&$args[0], &mut sub_context)?;
        if let Value::Path(path) = path_val {
            $op(path, world)
        } else {
            Err(sutra_error!(
                type,
                Some($parent_span.clone()),
                &$args[0],
                $name,
                "a Path",
                &path_val
            ))
        }
    }};
}

/// Macro for binary path operations (set).
/// Usage: eval_binary_path_op!(args, context, parent_span, |path, value, world| ..., "atom_name")
macro_rules! eval_binary_path_op {
    ($args:expr, $context:expr, $parent_span:expr, $op:expr, $name:expr) => {{
        if $args.len() != 2 {
            return Err(sutra_error!(
                arity,
                Some($parent_span.clone()),
                $args,
                $name,
                2
            ));
        }
        let mut sub_context1 = sub_eval_context!($context, $context.world);
        let (path_val, world1) = eval_expr(&$args[0], &mut sub_context1)?;
        let mut sub_context2 = sub_eval_context!($context, &world1);
        let (value, world2) = eval_expr(&$args[1], &mut sub_context2)?;

        if let Value::Path(path) = path_val {
            $op(path, value, world2)
        } else {
            Err(sutra_error!(
                type,
                Some($parent_span.clone()),
                &$args[0],
                $name,
                "a Path",
                &path_val
            ))
        }
    }};
}

/// Macro for unary operations that take any value.
/// Usage: eval_unary_value_op!(args, context, parent_span, |val, world| ..., "atom_name")
macro_rules! eval_unary_value_op {
    ($args:expr, $context:expr, $parent_span:expr, $op:expr, $name:expr) => {{
        if $args.len() != 1 {
            return Err(sutra_error!(
                arity,
                Some($parent_span.clone()),
                $args,
                $name,
                1
            ));
        }
        let mut sub_context = sub_eval_context!($context, $context.world);
        let (val, world) = eval_expr(&$args[0], &mut sub_context)?;
        $op(val, world, $parent_span, $context)
    }};
}

// After macro definitions, move all ATOM_* constants here:
// ATOM_CORE_SET, ATOM_CORE_GET, ATOM_CORE_DEL, ATOM_ADD, ATOM_SUB, ATOM_MUL, ATOM_DIV, ATOM_MOD, ATOM_EQ, ATOM_GT, ATOM_LT, ATOM_GTE, ATOM_LTE, ATOM_NOT, ATOM_DO, ATOM_LIST, ATOM_LEN, ATOM_ERROR, ATOM_PRINT

/// Sets a value at a path in the world state.
///
/// Usage: (core/set! <path> <value>)
/// - <path>: Path to set (must evaluate to a Value::Path)
/// - <value>: Value to store
/// Returns: Nil. Mutates world state (returns new world).
///
/// Example:
///   (core/set! player.score 42)
///
/// # Safety
/// Only mutates the world at the given path.
pub const ATOM_CORE_SET: AtomFn = |args, context, parent_span| {
    eval_binary_path_op!(
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
        "core/set!"
    )
};

/// Gets a value at a path in the world state.
///
/// Usage: (core/get <path>)
/// - <path>: Path to get (must evaluate to a Value::Path)
/// Returns: Value at path, or Nil if not found.
///
/// Example:
///   (core/get player.score)
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_CORE_GET: AtomFn = |args, context, parent_span| {
    eval_unary_path_op!(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let value = world.get(&path).cloned().unwrap_or_default();
            Ok((value, world))
        },
        "core/get"
    )
};

/// Deletes a value at a path in the world state.
///
/// Usage: (core/del! <path>)
/// - <path>: Path to delete (must evaluate to a Value::Path)
/// Returns: Nil. Mutates world state (returns new world).
///
/// Example:
///   (core/del! player.score)
///
/// # Safety
/// Only mutates the world at the given path.
pub const ATOM_CORE_DEL: AtomFn = |args, context, parent_span| {
    eval_unary_path_op!(
        args,
        context,
        parent_span,
        |path: crate::runtime::path::Path,
         world: crate::runtime::world::World|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            let new_world = world.del(&path);
            Ok((Value::default(), new_world))
        },
        "core/del!"
    )
};

/// Returns true if two values are equal.
///
/// Usage: (eq? <a> <b>)
/// - <a>, <b>: Values to compare
/// Returns: Bool
///
/// Example:
///   (eq? 1 1) ; => true
///   (eq? 1 2) ; => false
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_EQ: AtomFn = |args, context, parent_span| {
    eval_binary_op!(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a == b),
        "eq?",
        "two Numbers"
    )
};

/// Sequentially evaluates expressions, returning the last value.
///
/// Usage: (do <expr1> <expr2> ...)
/// - <expr1>, <expr2>, ...: Expressions to evaluate in sequence
/// Returns: Value of last expression
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

/// Adds numbers.
///
/// Usage: (+ <a> <b> ...)
/// - <a>, <b>, ...: Numbers
/// Returns: Number (sum)
///
/// Example:
///   (+ 1 2 3) ; => 6
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_ADD: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op!(args, context, parent_span, 0.0, |a, b| a + b, "+")
};

/// Subtracts two numbers.
///
/// Usage: (- <a> <b>)
/// - <a>, <b>: Numbers
/// Returns: Number (a - b)
///
/// Example:
///   (- 5 2) ; => 3
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_SUB: AtomFn = |args, context, parent_span| {
    eval_binary_op!(
        args,
        context,
        parent_span,
        |a, b| Value::Number(a - b),
        "-",
        "two Numbers"
    )
};

/// Multiplies numbers.
///
/// Usage: (* <a> <b> ...)
/// - <a>, <b>, ...: Numbers
/// Returns: Number (product)
///
/// Example:
///   (* 2 3 4) ; => 24
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_MUL: AtomFn = |args, context, parent_span| {
    eval_nary_numeric_op!(args, context, parent_span, 1.0, |a, b| a * b, "*")
};

/// Divides two numbers.
///
/// Usage: (/ <a> <b>)
/// - <a>, <b>: Numbers
/// Returns: Number (a / b)
///
/// Example:
///   (/ 6 2) ; => 3
///
/// # Safety
/// Pure, does not mutate state. Errors on division by zero.
pub const ATOM_DIV: AtomFn = |args, context, parent_span| {
    eval_binary_op_with_validation!(
        args,
        context,
        parent_span,
        |a, b| Value::Number(a / b),
        "/",
        |_a, b| {
            if b == 0.0 {
                Err("Division by zero")
            } else {
                Ok(())
            }
        }
    )
};

/// Returns true if a > b.
///
/// Usage: (gt? <a> <b>)
/// - <a>, <b>: Numbers
/// Returns: Bool
///
/// Example:
///   (gt? 3 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_GT: AtomFn = |args, context, parent_span| {
    eval_binary_op!(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a > b),
        "gt?",
        "two Numbers"
    )
};

/// Returns true if a < b.
///
/// Usage: (lt? <a> <b>)
/// - <a>, <b>: Numbers
/// Returns: Bool
///
/// Example:
///   (lt? 1 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_LT: AtomFn = |args, context, parent_span| {
    eval_binary_op!(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a < b),
        "lt?",
        "two Numbers"
    )
};

/// Returns true if a >= b.
///
/// Usage: (gte? <a> <b>)
/// - <a>, <b>: Numbers
/// Returns: Bool
///
/// Example:
///   (gte? 2 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_GTE: AtomFn = |args, context, parent_span| {
    eval_binary_op!(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a >= b),
        "gte?",
        "two Numbers"
    )
};

/// Returns true if a <= b.
///
/// Usage: (lte? <a> <b>)
/// - <a>, <b>: Numbers
/// Returns: Bool
///
/// Example:
///   (lte? 1 2) ; => true
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_LTE: AtomFn = |args, context, parent_span| {
    eval_binary_op!(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a <= b),
        "lte?",
        "two Numbers"
    )
};

/// Logical negation.
///
/// Usage: (not <a>)
/// - <a>: Boolean
/// Returns: Bool
///
/// Example:
///   (not true) ; => false
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_NOT: AtomFn = |args, context, parent_span| {
    eval_unary_bool_op!(args, context, parent_span, |b: bool| Value::Bool(!b), "not")
};

/// Constructs a list from arguments.
///
/// Usage: (list <a> <b> ...)
/// - <a>, <b>, ...: Values to include in the list
/// Returns: List containing all arguments
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
/// - <list-or-string>: List or String to measure
/// Returns: Number (length)
///
/// Example:
///   (len (list 1 2 3)) ; => 3
///   (len "abc") ; => 3
///
/// # Safety
/// Pure, does not mutate state.
pub const ATOM_LEN: AtomFn = |args, context, parent_span| {
    if args.len() != 1 {
        return Err(sutra_error!(
            arity,
            Some(parent_span.clone()),
            args,
            "len",
            1
        ));
    }
    let mut sub_context = sub_eval_context!(context, context.world);
    let (val, world) = eval_expr(&args[0], &mut sub_context)?;
    match val {
        Value::List(ref items) => Ok((Value::Number(items.len() as f64), world)),
        Value::String(ref s) => Ok((Value::Number(s.len() as f64), world)),
        _ => Err(sutra_error!(
            type,
            Some(parent_span.clone()),
            &args[0],
            "len",
            "a List or String",
            &val
        )),
    }
};

/// Modulo operation.
///
/// Usage: (mod <a> <b>)
/// - <a>, <b>: Integers
/// Returns: Number (a % b)
///
/// Example:
///   (mod 5 2) ; => 1
///
/// # Safety
/// Pure, does not mutate state. Errors on division by zero or non-integer input.
pub const ATOM_MOD: AtomFn = |args, context, parent_span| {
    eval_binary_op_with_validation!(
        args,
        context,
        parent_span,
        |a, b| Value::Number((a as i64 % b as i64) as f64),
        "mod",
        |a: f64, b: f64| -> Result<(), &'static str> {
            if b == 0.0 {
                Err("Modulo by zero")
            } else if a.fract() != 0.0 || b.fract() != 0.0 {
                Err("Modulo expects integers")
            } else {
                Ok(())
            }
        }
    )
};

/// Raises an error with a message.
///
/// Usage: (error <message>)
/// - <message>: String
/// Returns: Error (never returns normally)
///
/// Example:
///   (error "fail!")
///
/// # Safety
/// Always errors. Does not mutate state.
pub const ATOM_ERROR: AtomFn = |args, context, parent_span| {
    eval_unary_value_op!(
        args,
        context,
        parent_span,
        |msg_val: Value,
         _world: crate::runtime::world::World,
         parent_span: &crate::ast::Span,
         _context: &mut EvalContext<'_, '_>|
         -> Result<(Value, crate::runtime::world::World), SutraError> {
            if let Value::String(msg) = msg_val {
                Err(SutraError {
                    kind: SutraErrorKind::Eval(EvalError {
                        message: msg,
                        expanded_code: WithSpan {
                            value: Expr::List(args.to_vec(), parent_span.clone()),
                            span: parent_span.clone(),
                        }
                        .value
                        .pretty(),
                        original_code: None,
                        suggestion: None,
                    }),
                    span: Some(parent_span.clone()),
                })
            } else {
                Err(sutra_error!(
                    type,
                    Some(parent_span.clone()),
                    &args[0],
                    "error",
                    "a String",
                    &msg_val
                ))
            }
        },
        "error"
    )
};

/// Emits output to the output sink.
///
/// Usage: (print <value>)
/// - <value>: Any value
/// Returns: Nil. Emits output.
///
/// Example:
///   (print "hello")
///
/// # Safety
/// Emits output, does not mutate world state.
pub const ATOM_PRINT: AtomFn = |args, context, parent_span| {
    eval_unary_value_op!(
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
        "print"
    )
};

#[cfg(any(test, feature = "test-atom", debug_assertions))]

/// Registers all standard atoms in the given registry.
pub fn register_std_atoms(registry: &mut AtomRegistry) {
    registry.register("core/set!", ATOM_CORE_SET);
    registry.register("core/get", ATOM_CORE_GET);
    registry.register("core/del!", ATOM_CORE_DEL);
    registry.register("+", ATOM_ADD);
    registry.register("-", ATOM_SUB);
    registry.register("*", ATOM_MUL);
    registry.register("/", ATOM_DIV);
    registry.register("mod", ATOM_MOD);
    registry.register("eq?", ATOM_EQ);
    registry.register("gt?", ATOM_GT);
    registry.register("lt?", ATOM_LT);
    registry.register("gte?", ATOM_GTE);
    registry.register("lte?", ATOM_LTE);
    registry.register("not", ATOM_NOT);
    registry.register("do", ATOM_DO);
    registry.register("list", ATOM_LIST);
    registry.register("len", ATOM_LEN);
    registry.register("error", ATOM_ERROR);
    registry.register("print", ATOM_PRINT);
    registry.register("core/print", ATOM_PRINT);
}
