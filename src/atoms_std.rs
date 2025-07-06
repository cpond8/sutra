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

use crate::ast::{Expr, WithSpan};
use crate::atom::{AtomFn, AtomRegistry};
use crate::error::{EvalError, SutraError, SutraErrorKind};
use crate::eval::EvalContext;
use crate::value::Value;

// ---
// Registry
// ---

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
}

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

macro_rules! eval_err {
    (arity, $span:expr, $args:expr, $name:expr, $expected:expr) => {
        SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                message: format!(
                    "`{}` expects {} arguments, got {}",
                    $name,
                    $expected,
                    $args.len()
                ),
                expanded_code: WithSpan {
                    value: Expr::List($args.to_vec(), $span.clone()),
                    span: $span.clone(),
                }
                .value
                .pretty(),
                original_code: None,
                suggestion: None,
            }),
            span: Some($span.clone()),
        }
    };
    (type, $expr:expr, $name:expr, $expected:expr, $actual:expr) => {{
        let span = $expr.span.clone();
        let pretty = match &$expr.value {
            Expr::List(items, span) => WithSpan {
                value: Expr::List(items.to_vec(), span.clone()),
                span: span.clone(),
            }
            .value
            .pretty(),
            _ => $expr.value.pretty(),
        };
        SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                message: format!(
                    "`{}` expects {}, got {}",
                    $name,
                    $expected,
                    $actual.type_name()
                ),
                expanded_code: pretty,
                original_code: None,
                suggestion: None,
            }),
            span: Some(span),
        }
    }};
    (general, $expr:expr, $msg:expr) => {
        SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                message: $msg.to_string(),
                expanded_code: $expr.value.pretty(),
                original_code: None,
                suggestion: None,
            }),
            span: Some($expr.span.clone()),
        }
    };
}

fn eval_args(
    args: &[WithSpan<Expr>],
    context: &mut EvalContext<'_, '_>,
) -> Result<(Vec<Value>, crate::world::World), SutraError> {
    // Use try_fold to create a functional pipeline that threads the world
    // state through the evaluation of each argument. This is the canonical
    // pattern for safe, sequential evaluation in Sutra.
    args.iter().try_fold(
        (Vec::with_capacity(args.len()), context.world.clone()),
        |(mut values, world), arg| {
            // This is the critical state propagation step. The `world` from the
            // previous evaluation is passed into the evaluation of the next argument.
            let (val, next_world) = context.eval_in(&world, arg)?;
            values.push(val);
            Ok((values, next_world))
        },
    )
}

macro_rules! eval_binary_op {
    ($args:expr, $context:expr, $parent_span:expr, $op:expr, $name:expr) => {{
        if $args.len() != 2 {
            return Err(eval_err!(arity, $parent_span, $args, $name, 2));
        }
        let (val1, world1) = $context.eval(&$args[0])?;
        let (val2, world2) = $context.eval_in(&world1, &$args[1])?;
        match (&val1, &val2) {
            (Value::Number(n1), Value::Number(n2)) => Ok(($op(*n1, *n2), world2)),
            _ => Err(eval_err!(type, &$args[0], $name, "two Numbers", &val1)),
        }
    }};
}

// ---
// Core Atoms
// ---

/// (core/set! <path> <value>)
pub const ATOM_CORE_SET: AtomFn = |args, context, parent_span| {
    if args.len() != 2 {
        return Err(eval_err!(arity, parent_span, args, "core/set!", 2));
    }
    let (path_val, world1) = context.eval(&args[0])?;
    let (value, world2) = context.eval_in(&world1, &args[1])?;

    if let Value::Path(path) = path_val {
        let new_world = world2.set(&path, value);
        Ok((Value::default(), new_world))
    } else {
        Err(eval_err!(type, &args[0], "core/set!", "a Path", &path_val))
    }
};

/// (core/get <path>)
pub const ATOM_CORE_GET: AtomFn = |args, context, parent_span| {
    if args.len() != 1 {
        return Err(eval_err!(arity, parent_span, args, "core/get", 1));
    }
    let (path_val, world) = context.eval(&args[0])?;
    if let Value::Path(path) = path_val {
        let value = world.get(&path).cloned().unwrap_or_default();
        Ok((value, world))
    } else {
        Err(eval_err!(type, &args[0], "core/get", "a Path", &path_val))
    }
};

/// (core/del! <path>)
pub const ATOM_CORE_DEL: AtomFn = |args, context, parent_span| {
    if args.len() != 1 {
        return Err(eval_err!(arity, parent_span, args, "core/del!", 1));
    }
    let (path_val, world) = context.eval(&args[0])?;
    if let Value::Path(path) = path_val {
        let new_world = world.del(&path);
        Ok((Value::default(), new_world))
    } else {
        Err(eval_err!(type, &args[0], "core/del!", "a Path", &path_val))
    }
};

/// (+ <args...>)
pub const ATOM_ADD: AtomFn = |args, context, parent_span| {
    if args.len() < 2 {
        return Err(eval_err!(arity, parent_span, args, "+", "at least 2"));
    }
    let (values, world) = eval_args(args, context)?;
    let mut sum = 0.0;
    for v in &values {
        if let Value::Number(n) = v {
            sum += n;
        } else {
            return Err(eval_err!(
                type,
                &WithSpan {
                    value: Expr::List(args.to_vec(), parent_span.clone()),
                    span: parent_span.clone()
                },
                "+",
                "a Number",
                v
            ));
        }
    }
    Ok((Value::Number(sum), world))
};

/// (eq? <a> <b>)
pub const ATOM_EQ: AtomFn = |args, context, parent_span| {
    if args.len() != 2 {
        return Err(eval_err!(arity, parent_span, args, "eq?", 2));
    }
    let (v1, w1) = context.eval(&args[0])?;
    let (v2, w2) = context.eval_in(&w1, &args[1])?;
    Ok((Value::Bool(v1 == v2), w2))
};

/// (do <exprs...>)
pub const ATOM_DO: AtomFn = |args, context, _| {
    // The `eval_args` helper function correctly threads the world state
    // through the evaluation of each argument. We can simply use it
    // and return the value of the last expression, which is the
    // standard behavior of a `do` block.
    let (values, world) = eval_args(args, context)?;
    let last_value = values.last().cloned().unwrap_or_default();
    Ok((last_value, world))
};

/// (- <a> <b>)
pub const ATOM_SUB: AtomFn = |args, context, parent_span| {
    eval_binary_op!(args, context, parent_span, |a, b| Value::Number(a - b), "-")
};

/// (* <args...>)
pub const ATOM_MUL: AtomFn = |args, context, parent_span| {
    let (values, world) = eval_args(args, context)?;
    let mut product = 1.0;
    for v in &values {
        if let Value::Number(n) = v {
            product *= n;
        } else {
            return Err(eval_err!(
                type,
                &WithSpan {
                    value: Expr::List(args.to_vec(), parent_span.clone()),
                    span: parent_span.clone()
                },
                "*",
                "a Number",
                v
            ));
        }
    }
    Ok((Value::Number(product), world))
};

/// (/ <a> <b>)
pub const ATOM_DIV: AtomFn = |args, context, parent_span| {
    if args.len() != 2 {
        return Err(eval_err!(arity, parent_span, args, "/", 2));
    }
    let (v1, w1) = context.eval(&args[0])?;
    let (v2, w2) = context.eval_in(&w1, &args[1])?;
    match (v1, v2) {
        (Value::Number(n1), Value::Number(n2)) if n2 != 0.0 => Ok((Value::Number(n1 / n2), w2)),
        (Value::Number(_), Value::Number(n2)) if n2 == 0.0 => {
            Err(eval_err!(general, &args[1], "Division by zero"))
        }
        (a, b) => Err(eval_err!(
            general,
            &WithSpan {
                value: Expr::List(args.to_vec(), parent_span.clone()),
                span: parent_span.clone()
            },
            &format!(
                "`/` expects two Numbers, got {} and {}",
                a.type_name(),
                b.type_name()
            )
        )),
    }
};

/// (gt? <a> <b>)
pub const ATOM_GT: AtomFn = |args, context, parent_span| {
    eval_binary_op!(args, context, parent_span, |a, b| Value::Bool(a > b), "gt?")
};

/// (lt? <a> <b>)
pub const ATOM_LT: AtomFn = |args, context, parent_span| {
    eval_binary_op!(args, context, parent_span, |a, b| Value::Bool(a < b), "lt?")
};

/// (gte? <a> <b>)
pub const ATOM_GTE: AtomFn = |args, context, parent_span| {
    eval_binary_op!(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a >= b),
        "gte?"
    )
};

/// (lte? <a> <b>)
pub const ATOM_LTE: AtomFn = |args, context, parent_span| {
    eval_binary_op!(
        args,
        context,
        parent_span,
        |a, b| Value::Bool(a <= b),
        "lte?"
    )
};

/// (not <a>)
pub const ATOM_NOT: AtomFn = |args, context, parent_span| {
    if args.len() != 1 {
        return Err(eval_err!(arity, parent_span, args, "not", 1));
    }
    let (v, world) = context.eval(&args[0])?;
    match v {
        Value::Bool(b) => Ok((Value::Bool(!b), world)),
        _ => Err(eval_err!(type, &args[0], "not", "a Boolean", v)),
    }
};

/// (list <args...>)
pub const ATOM_LIST: AtomFn = |args, context, _| {
    let (items, world) = eval_args(args, context)?;
    Ok((Value::List(items), world))
};

/// (len <list-or-string>)
pub const ATOM_LEN: AtomFn = |args, context, parent_span| {
    if args.len() != 1 {
        return Err(eval_err!(arity, parent_span, args, "len", 1));
    }
    let (val, world) = context.eval(&args[0])?;
    match val {
        Value::List(ref items) => Ok((Value::Number(items.len() as f64), world)),
        Value::String(ref s) => Ok((Value::Number(s.len() as f64), world)),
        _ => Err(eval_err!(type, &args[0], "len", "a List or String", &val)),
    }
};

/// (mod <a> <b>)
pub const ATOM_MOD: AtomFn = |args, context, parent_span| {
    if args.len() != 2 {
        return Err(eval_err!(arity, parent_span, args, "mod", 2));
    }
    let (v1, w1) = context.eval(&args[0])?;
    let (v2, w2) = context.eval_in(&w1, &args[1])?;
    match (v1, v2) {
        (Value::Number(n1), Value::Number(n2)) => {
            if n2 == 0.0 {
                return Err(eval_err!(general, &args[1], "Modulo by zero"));
            }
            if n1.fract() != 0.0 || n2.fract() != 0.0 {
                return Err(eval_err!(
                    type,
                    &WithSpan {
                        value: Expr::List(args.to_vec(), parent_span.clone()),
                        span: parent_span.clone()
                    },
                    "mod",
                    "two Integers",
                    &Value::Number(n1)
                ));
            }
            Ok((Value::Number((n1 as i64 % n2 as i64) as f64), w2))
        }
        (a, b) => Err(eval_err!(
            general,
            &WithSpan {
                value: Expr::List(args.to_vec(), parent_span.clone()),
                span: parent_span.clone()
            },
            &format!(
                "`mod` expects two Integers, got {} and {}",
                a.type_name(),
                b.type_name()
            )
        )),
    }
};

/// (error <message>)
pub const ATOM_ERROR: AtomFn = |args, context, parent_span| {
    if args.len() != 1 {
        return Err(eval_err!(arity, parent_span, args, "error", 1));
    }
    let (msg_val, _world) = context.eval(&args[0])?;
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
        Err(eval_err!(type, &args[0], "error", "a String", msg_val))
    }
};
