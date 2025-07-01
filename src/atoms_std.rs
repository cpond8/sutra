use crate::ast::Expr;
use crate::atom::AtomFn;
use crate::error::{SutraError, SutraErrorKind};
use crate::eval::EvalContext;
use crate::value::Value;

// ---
// Tier 1 Standard Atoms
// ---
//
// This module contains the standard library of irreducible atoms.
// These are the fundamental building blocks of the Sutra language.
// They are designed to be minimal, orthogonal, and fully compositional.
//
// All atoms adhere to the `AtomFn` signature:
// `fn(args: &[Expr], context: &mut EvalContext) -> Result<(Value, World), SutraError>`
//
// Atoms that do not mutate the world state simply return a clone of the world
// they received.

/// (set! <path> <value>)
///
/// Sets a value at a given path in the world.
/// The path must be a list of strings or symbols.
/// The value is evaluated before being set.
///
/// Returns: `Value::Nil`
pub const ATOM_SET: AtomFn = |args, context| {
    if args.len() != 2 {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(format!("set! expects 2 arguments, got {}", args.len())),
            span: None, // TODO: Get span from the parent expression
        });
    }

    let path_expr = &args[0];
    let value_expr = &args[1];

    // Evaluate the value expression. The world returned here is the one we should use
    // for the final `set` operation, as the evaluation might have caused its own mutations
    // (e.g., if the value is another expression with `set!`).
    let (new_value, world_after_eval) = context.eval(value_expr)?;

    // Extract the path from the path expression.
    // It should be a list of strings or symbols, optionally wrapped in a `(list ...)` call.
    let path_segments = match path_expr {
        Expr::List(items, _) => {
            if !items.is_empty() && matches!(&items[0], Expr::Symbol(s, _) if s == "list") {
                &items[1..]
            } else {
                items
            }
        }
        _ => {
            return Err(SutraError {
                kind: SutraErrorKind::Eval("set! path must be a list".to_string()),
                span: Some(path_expr.span()),
            });
        }
    };

    let path: Vec<String> = path_segments
        .iter()
        .map(|item| match item {
            Expr::Symbol(s, _) => Ok(s.clone()),
            Expr::String(s, _) => Ok(s.clone()),
            _ => Err(SutraError {
                kind: SutraErrorKind::Eval(
                    "set! path segments must be symbols or strings".to_string(),
                ),
                span: Some(item.span()),
            }),
        })
        .collect::<Result<Vec<String>, SutraError>>()?;

    let path_str: Vec<&str> = path.iter().map(|s| s.as_str()).collect();

    // Set the value in the world that resulted from evaluating the argument.
    let new_world = world_after_eval.set(&path_str, new_value);

    Ok((Value::default(), new_world))
};

/// (del! <path>)
///
/// Deletes a value at a given path in the world.
/// The path must be a list of strings or symbols.
///
/// Returns: `Value::Nil`
pub const ATOM_DEL: AtomFn = |args, context| {
    if args.len() != 1 {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(format!("del! expects 1 argument, got {}", args.len())),
            span: None, // TODO: Get span from the parent expression
        });
    }

    let path_expr = &args[0];

    // Extract the path from the path expression.
    // It should be a list of strings or symbols, optionally wrapped in a `(list ...)` call.
    let path_segments = match path_expr {
        Expr::List(items, _) => {
            if !items.is_empty() && matches!(&items[0], Expr::Symbol(s, _) if s == "list") {
                &items[1..]
            } else {
                items
            }
        }
        _ => {
            return Err(SutraError {
                kind: SutraErrorKind::Eval("del! path must be a list".to_string()),
                span: Some(path_expr.span()),
            });
        }
    };

    let path: Vec<String> = path_segments
        .iter()
        .map(|item| match item {
            Expr::Symbol(s, _) => Ok(s.clone()),
            Expr::String(s, _) => Ok(s.clone()),
            _ => Err(SutraError {
                kind: SutraErrorKind::Eval(
                    "del! path segments must be symbols or strings".to_string(),
                ),
                span: Some(item.span()),
            }),
        })
        .collect::<Result<Vec<String>, SutraError>>()?;

    let path_str: Vec<&str> = path.iter().map(|s| s.as_str()).collect();

    // Delete the value from the world. Note that we use the original world from the context,
    // as `del!` does not evaluate any arguments that could change the world.
    let new_world = context.world.del(&path_str);

    Ok((Value::default(), new_world))
};

/// (+ <arg1> <arg2> ... <argN>)
///
/// Adds any number of numeric arguments.
/// All arguments are evaluated before addition.
///
/// Returns: `Value::Number`
pub const ATOM_ADD: AtomFn = |args, context| {
    let mut sum = 0.0;
    let mut current_world = context.world.clone();

    for arg_expr in args {
        let (value, next_world) = context.eval(arg_expr)?;
        current_world = next_world; // Propagate world changes from argument evaluation

        match value {
            Value::Number(n) => sum += n,
            _ => {
                return Err(SutraError {
                    kind: SutraErrorKind::Eval(format!(
                        "+ expects numeric arguments, but got {}",
                        value.type_name()
                    )),
                    span: Some(arg_expr.span()),
                })
            }
        }
    }

    Ok((Value::Number(sum), current_world))
};

/// (eq? <arg1> <arg2>)
///
/// Checks if two values are equal.
/// Both arguments are evaluated before comparison.
///
/// Returns: `Value::Bool`
pub const ATOM_EQ: AtomFn = |args, context| {
    if args.len() != 2 {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(format!("eq? expects 2 arguments, got {}", args.len())),
            span: None, // TODO: Get span from the parent expression
        });
    }

    let (val1, world1) = context.eval(&args[0])?;
    // We must use the world from the first evaluation to evaluate the second argument.
    let mut temp_context = EvalContext {
        world: &world1,
        output: context.output,
        opts: context.opts,
        depth: context.depth,
    };
    let (val2, world2) = temp_context.eval(&args[1])?;

    Ok((Value::Bool(val1 == val2), world2))
};

/// (cond (<cond1> <expr1>) (<cond2> <expr2>) ... (<else>))
///
/// Evaluates conditions in order and executes the expression corresponding to the first true condition.
/// This is a special form: it does not evaluate all of its arguments.
///
/// The arguments are processed in pairs. For each pair, the first element (the condition)
/// is evaluated. If it returns `true`, the second element (the body) is evaluated, and
/// that result is returned.
///
/// If an odd number of arguments is provided, the last argument is treated as an `else`
/// block and is evaluated if no other conditions are met.
///
/// Returns: The value of the executed expression, or `Value::Nil` if no condition is met.
pub const ATOM_COND: AtomFn = |args, context| {
    let mut current_world = context.world.clone();

    // `chunks_exact(2)` gives us an iterator over pairs of arguments.
    // This is the cleanest way to handle the (condition, expression) structure.
    let pairs = args.chunks_exact(2);

    // If there's a single argument left over, it's our `else` clause.
    let has_else_clause = pairs.remainder().len() == 1;

    for pair in pairs {
        let cond_expr = &pair[0];
        let body_expr = &pair[1];

        // We must create a new context for each evaluation to thread the world state correctly.
        // The world can be modified by the evaluation of a condition.
        let mut cond_context = EvalContext {
            world: &current_world,
            output: context.output,
            opts: context.opts,
            depth: context.depth,
        };

        let (cond_val, next_world) = cond_context.eval(cond_expr)?;
        current_world = next_world; // Update the world state for the next iteration.

        if let Value::Bool(true) = cond_val {
            // If the condition is true, evaluate the body with the updated world and return immediately.
            let mut body_context = EvalContext {
                world: &current_world,
                output: context.output,
                opts: context.opts,
                depth: context.depth,
            };
            return body_context.eval(body_expr);
        }
    }

    // If we've gone through all the pairs and an `else` clause exists, evaluate it.
    if has_else_clause {
        let else_expr = &args[args.len() - 1];
        let mut else_context = EvalContext {
            world: &current_world,
            output: context.output,
            opts: context.opts,
            depth: context.depth,
        };
        return else_context.eval(else_expr);
    }

    // If no conditions were met and there was no `else` clause, return Nil.
    // The world returned is the one that resulted from the last failed condition check.
    Ok((Value::default(), current_world))
};

// TODO:
// - Implement remaining Tier 1 atoms:
//   - Math: `-`, `*`, `/`, `mod`
//   - Predicates: `gt?`, `lt?`, `gte?`, `lte?`, `not`
//   - List operations: `list`, `get`, `len`
// - Write unit tests for all implemented atoms.
// - Figure out how to pass the parent expression's span to atoms for better
//   error reporting on arity mismatches.

// Helper macro for binary numeric operations.
// This ensures that the world state is correctly threaded through argument evaluations
// and reduces boilerplate code for common checks.
macro_rules! eval_binary_op {
    ($args:expr, $context:expr, $op:expr, $op_name:expr) => {{
        if $args.len() != 2 {
            return Err(SutraError {
                kind: SutraErrorKind::Eval(format!(
                    "{} expects 2 arguments, got {}",
                    $op_name,
                    $args.len()
                )),
                span: None, // TODO: Get span from parent
            });
        }

        let (val1, world1) = $context.eval(&$args[0])?;
        let mut temp_context = EvalContext {
            world: &world1,
            output: $context.output,
            opts: $context.opts,
            depth: $context.depth,
        };
        let (val2, world2) = temp_context.eval(&$args[1])?;

        match (val1, val2) {
            (Value::Number(n1), Value::Number(n2)) => Ok(($op(n1, n2), world2)),
            _ => Err(SutraError {
                kind: SutraErrorKind::Eval(format!("{} expects numeric arguments", $op_name)),
                span: Some($args[0].span()), // Approximate span
            }),
        }
    }};
}

/// (- <arg1> <arg2>)
///
/// Subtracts the second numeric argument from the first.
pub const ATOM_SUB: AtomFn =
    |args, context| eval_binary_op!(args, context, |a, b| Value::Number(a - b), "-");

/// (* <arg1> <arg2> ... <argN>)
///
/// Multiplies any number of numeric arguments.
pub const ATOM_MUL: AtomFn = |args, context| {
    let mut product = 1.0;
    let mut current_world = context.world.clone();

    for arg_expr in args {
        let (value, next_world) = context.eval(arg_expr)?;
        current_world = next_world;

        match value {
            Value::Number(n) => product *= n,
            _ => {
                return Err(SutraError {
                    kind: SutraErrorKind::Eval(format!(
                        "* expects numeric arguments, but got {}",
                        value.type_name()
                    )),
                    span: Some(arg_expr.span()),
                })
            }
        }
    }

    Ok((Value::Number(product), current_world))
};

/// (/ <arg1> <arg2>)
///
/// Divides the first numeric argument by the second.
pub const ATOM_DIV: AtomFn = |args, context| {
    if args.len() != 2 {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(format!("/ expects 2 arguments, got {}", args.len())),
            span: None,
        });
    }

    let (val1, world1) = context.eval(&args[0])?;
    let mut temp_context = EvalContext {
        world: &world1,
        output: context.output,
        opts: context.opts,
        depth: context.depth,
    };
    let (val2, world2) = temp_context.eval(&args[1])?;

    match (val1, val2) {
        (Value::Number(n1), Value::Number(n2)) => {
            if n2 == 0.0 {
                return Err(SutraError {
                    kind: SutraErrorKind::Eval("Division by zero".to_string()),
                    span: Some(args[1].span()),
                });
            }
            Ok((Value::Number(n1 / n2), world2))
        }
        _ => Err(SutraError {
            kind: SutraErrorKind::Eval("/ expects numeric arguments".to_string()),
            span: Some(args[0].span()),
        }),
    }
};

/// (gt? <arg1> <arg2>)
///
/// Checks if the first numeric argument is greater than the second.
pub const ATOM_GT: AtomFn =
    |args, context| eval_binary_op!(args, context, |a, b| Value::Bool(a > b), "gt?");

/// (lt? <arg1> <arg2>)
///
/// Checks if the first numeric argument is less than the second.
pub const ATOM_LT: AtomFn =
    |args, context| eval_binary_op!(args, context, |a, b| Value::Bool(a < b), "lt?");

/// (not <arg>)
///
/// Inverts a boolean value.
pub const ATOM_NOT: AtomFn = |args, context| {
    if args.len() != 1 {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(format!("not expects 1 argument, got {}", args.len())),
            span: None,
        });
    }

    let (val, world) = context.eval(&args[0])?;

    match val {
        Value::Bool(b) => Ok((Value::Bool(!b), world)),
        _ => Err(SutraError {
            kind: SutraErrorKind::Eval("not expects a boolean argument".to_string()),
            span: Some(args[0].span()),
        }),
    }
};

/// (list <arg1> <arg2> ... <argN>)
///
/// Creates a list from its evaluated arguments.
pub const ATOM_LIST: AtomFn = |args, context| {
    let mut items = Vec::new();
    let mut current_world = context.world.clone();

    for arg_expr in args {
        let (value, next_world) = context.eval(arg_expr)?;
        current_world = next_world;
        items.push(value);
    }

    Ok((Value::List(items), current_world))
};

/// (len <list_or_string>)
///
/// Returns the length of a list or string.
pub const ATOM_LEN: AtomFn = |args, context| {
    if args.len() != 1 {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(format!("len expects 1 argument, got {}", args.len())),
            span: None,
        });
    }

    let (val, world) = context.eval(&args[0])?;

    match val {
        Value::List(items) => Ok((Value::Number(items.len() as f64), world)),
        Value::String(s) => Ok((Value::Number(s.len() as f64), world)),
        _ => Err(SutraError {
            kind: SutraErrorKind::Eval("len expects a list or string".to_string()),
            span: Some(args[0].span()),
        }),
    }
};

/// (get <path>)
///
/// Retrieves a value from a given path in the world.
/// The path must be a list of strings or symbols.
///
/// Returns: The `Value` at the path, or `Value::Nil` if not found.
pub const ATOM_GET: AtomFn = |args, context| {
    if args.len() != 1 {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(format!("get expects 1 argument, got {}", args.len())),
            span: None, // TODO: Get span from the parent expression
        });
    }

    let path_expr = &args[0];

    // Extract the path from the path expression.
    // It should be a list of strings or symbols, optionally wrapped in a `(list ...)` call.
    let path_segments = match path_expr {
        Expr::List(items, _) => {
            if !items.is_empty() && matches!(&items[0], Expr::Symbol(s, _) if s == "list") {
                &items[1..]
            } else {
                items
            }
        }
        _ => {
            return Err(SutraError {
                kind: SutraErrorKind::Eval("get path must be a list".to_string()),
                span: Some(path_expr.span()),
            });
        }
    };

    let path: Vec<String> = path_segments
        .iter()
        .map(|item| match item {
            Expr::Symbol(s, _) => Ok(s.clone()),
            Expr::String(s, _) => Ok(s.clone()),
            _ => Err(SutraError {
                kind: SutraErrorKind::Eval(
                    "get path segments must be symbols or strings".to_string(),
                ),
                span: Some(item.span()),
            }),
        })
        .collect::<Result<Vec<String>, SutraError>>()?;

    let path_str: Vec<&str> = path.iter().map(|s| s.as_str()).collect();

    // Get the value from the world.
    let value = context.world.get(&path_str).cloned().unwrap_or_default();

    // `get` is a pure function, so we return the original world unmodified.
    Ok((value, context.world.clone()))
};
