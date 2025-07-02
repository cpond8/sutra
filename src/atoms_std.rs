use crate::ast::Expr;
use crate::atom::AtomFn;
use crate::error::{EvalError, SutraError, SutraErrorKind};
use crate::eval::EvalContext;
use crate::value::Value;

/// ---
/// Error macro: all evaluation errors route through here.
/// ---
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
                expanded_code: Expr::List($args.to_vec(), $span.clone()).pretty(),
                original_code: None,
                suggestion: None,
            }),
            span: Some($span.clone()),
        }
    };
    (type, $expr:expr, $name:expr, $expected:expr, $actual:expr) => {
        SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                message: format!(
                    "`{}` expects {}, got {}",
                    $name,
                    $expected,
                    $actual.type_name()
                ),
                expanded_code: $expr.pretty(),
                original_code: None,
                suggestion: Some(format!(
                    "All `{}` arguments must evaluate to {}.",
                    $name, $expected
                )),
            }),
            span: Some($expr.span()),
        }
    };
    (general, $expr:expr, $msg:expr) => {
        SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                message: $msg.to_string(),
                expanded_code: $expr.pretty(),
                original_code: None,
                suggestion: None,
            }),
            span: Some($expr.span()),
        }
    };
}

/// Extracts a canonical path: must be (list sym|str ...).
fn extract_path(expr: &Expr, op_name: &str) -> Result<Vec<String>, SutraError> {
    match expr {
        Expr::List(items, _) if !items.is_empty() => {
            if let Expr::Symbol(s, _) = &items[0] {
                if s == "list" {
                    items[1..]
                        .iter()
                        .map(|e| match e {
                            Expr::Symbol(s, _) | Expr::String(s, _) => Ok(s.clone()),
                            _ => Err(eval_err!(
                                general,
                                e,
                                &format!("`{}` path segments must be symbols or strings", op_name)
                            )),
                        })
                        .collect()
                } else {
                    Err(eval_err!(
                        general,
                        expr,
                        &format!(
                            "`{}` expects a symbol or a list of symbols/strings as path",
                            op_name
                        )
                    ))
                }
            } else {
                Err(eval_err!(
                    general,
                    expr,
                    &format!(
                        "`{}` expects a symbol or a list of symbols/strings as path",
                        op_name
                    )
                ))
            }
        }
        _ => Err(eval_err!(
            general,
            expr,
            &format!(
                "`{}` expects a symbol or a list of symbols/strings as path",
                op_name
            )
        )),
    }
}

/// Evaluates a sequence of arguments, returning Vec<Value> and final world.
fn eval_args<'a>(
    args: &'a [Expr],
    context: &mut EvalContext<'_, '_>,
) -> Result<(Vec<Value>, crate::world::World), SutraError> {
    let mut values = Vec::with_capacity(args.len());
    let mut world = context.world.clone();
    for arg in args {
        let (val, next_world) = context.eval(arg)?;
        values.push(val);
        world = next_world;
    }
    Ok((values, world))
}

/// Standard binary numeric op with arity/type checks.
macro_rules! eval_binary_op {
    ($args:expr, $context:expr, $parent_span:expr, $op:expr, $name:expr) => {{
        if $args.len() != 2 {
            return Err(eval_err!(arity, $parent_span, $args, $name, 2));
        }
        let (val1, world1) = $context.eval(&$args[0])?;
        let mut ctx2 = EvalContext {
            world: &world1,
            output: $context.output,
            opts: $context.opts,
            depth: $context.depth,
        };
        let (val2, world2) = ctx2.eval(&$args[1])?;
        match (&val1, &val2) {
            (Value::Number(n1), Value::Number(n2)) => Ok(($op(*n1, *n2), world2)),
            _ => Err(eval_err!(
                general,
                &Expr::List($args.to_vec(), $parent_span.clone()),
                &format!(
                    "`{}` expects two Numbers, got {} and {}",
                    $name,
                    val1.type_name(),
                    val2.type_name()
                )
            )),
        }
    }};
}

// --- ATOMS ---

/// (set! <path> <value>)
pub const ATOM_SET: AtomFn = |args, context, parent_span| {
    if args.len() != 2 {
        return Err(eval_err!(arity, parent_span, args, "set!", 2));
    }
    let path = extract_path(&args[0], "set!")?;
    let (val, world) = context.eval(&args[1])?;
    let path_str: Vec<&str> = path.iter().map(String::as_str).collect();
    let new_world = world.set(&path_str, val);
    Ok((Value::default(), new_world))
};

/// (del! <path>)
pub const ATOM_DEL: AtomFn = |args, context, parent_span| {
    if args.len() != 1 {
        return Err(eval_err!(arity, parent_span, args, "del!", 1));
    }
    let path = extract_path(&args[0], "del!")?;
    let path_str: Vec<&str> = path.iter().map(String::as_str).collect();
    let new_world = context.world.del(&path_str);
    Ok((Value::default(), new_world))
};

/// (+ <args...>)
pub const ATOM_ADD: AtomFn = |args, context, _| {
    let (values, world) = eval_args(args, context)?;
    let mut sum = 0.0;
    for v in &values {
        match v {
            Value::Number(n) => sum += n,
            _ => {
                return Err(eval_err!(
                    type,
                    &args[values.iter().position(|x| x == v).unwrap()],
                    "+",
                    "a Number",
                    v
                ))
            }
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
    let mut ctx2 = EvalContext {
        world: &w1,
        output: context.output,
        opts: context.opts,
        depth: context.depth,
    };
    let (v2, w2) = ctx2.eval(&args[1])?;
    Ok((Value::Bool(v1 == v2), w2))
};

/// (cond (<cond> <expr>) ... [<else>])
pub const ATOM_COND: AtomFn = |args, context, _| {
    let mut world = context.world.clone();
    let mut pairs = args.chunks_exact(2);
    let else_clause = pairs.remainder().get(0);
    for pair in pairs.by_ref() {
        let mut cond_ctx = EvalContext {
            world: &world,
            output: context.output,
            opts: context.opts,
            depth: context.depth,
        };
        let (cond_val, w) = cond_ctx.eval(&pair[0])?;
        world = w;
        if matches!(cond_val, Value::Bool(true)) {
            let mut body_ctx = EvalContext {
                world: &world,
                output: context.output,
                opts: context.opts,
                depth: context.depth,
            };
            return body_ctx.eval(&pair[1]);
        }
    }
    if let Some(else_expr) = else_clause {
        let mut else_ctx = EvalContext {
            world: &world,
            output: context.output,
            opts: context.opts,
            depth: context.depth,
        };
        else_ctx.eval(else_expr)
    } else {
        Ok((Value::default(), world))
    }
};

/// (- <a> <b>)
pub const ATOM_SUB: AtomFn = |args, context, parent_span| {
    eval_binary_op!(args, context, parent_span, |a, b| Value::Number(a - b), "-")
};

/// (* <args...>)
pub const ATOM_MUL: AtomFn = |args, context, _| {
    let (values, world) = eval_args(args, context)?;
    let mut product = 1.0;
    for v in &values {
        match v {
            Value::Number(n) => product *= n,
            _ => {
                return Err(eval_err!(
                    type,
                    &args[values.iter().position(|x| x == v).unwrap()],
                    "*",
                    "a Number",
                    v
                ))
            }
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
    let mut ctx2 = EvalContext {
        world: &w1,
        output: context.output,
        opts: context.opts,
        depth: context.depth,
    };
    let (v2, w2) = ctx2.eval(&args[1])?;
    match (v1, v2) {
        (Value::Number(n1), Value::Number(n2)) if n2 != 0.0 => Ok((Value::Number(n1 / n2), w2)),
        (Value::Number(_), Value::Number(n2)) if n2 == 0.0 => {
            Err(eval_err!(general, &args[1], "Division by zero"))
        }
        (a, b) => Err(eval_err!(
            general,
            &Expr::List(args.to_vec(), parent_span.clone()),
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
        _ => Err(eval_err!(type, &args[0], "len", "a List or String", val)),
    }
};

/// (get <path> | <collection> <key>)
pub const ATOM_GET: AtomFn = |args, context, parent_span| {
    match args.len() {
        1 => {
            // World-path lookup (canonical symbol or list ...)
            let path = match &args[0] {
                Expr::Symbol(s, _) => s.split('.').map(str::to_string).collect(),
                Expr::List(items, _) if items.is_empty() => {
                    // Special case: (get (list)) returns Nil
                    return Ok((Value::Nil, context.world.clone()));
                }
                _ => extract_path(&args[0], "get")?,
            };
            if path.is_empty() {
                // If the extracted path is empty, return Nil
                return Ok((Value::Nil, context.world.clone()));
            }
            let path_str: Vec<&str> = path.iter().map(String::as_str).collect();
            let value = context.world.get(&path_str).cloned().unwrap_or(Value::Nil);
            Ok((value, context.world.clone()))
        }
        2 => {
            let (collection, _) = context.eval(&args[0])?;
            let (key, _) = context.eval(&args[1])?;
            match &collection {
                Value::List(list) => {
                    if let Value::Number(idx) = key {
                        let idx = idx.trunc() as usize;
                        Ok((
                            list.get(idx).cloned().unwrap_or(Value::Nil),
                            context.world.clone(),
                        ))
                    } else {
                        Err(eval_err!(
                            general,
                            &args[1],
                            "`get` list index must be a number"
                        ))
                    }
                }
                Value::Map(map) => {
                    let k = match key {
                        Value::String(ref s) => s.as_str(),
                        _ => {
                            return Err(eval_err!(
                                general,
                                &args[1],
                                "`get` map key must be a string"
                            ))
                        }
                    };
                    Ok((
                        map.get(k).cloned().unwrap_or(Value::Nil),
                        context.world.clone(),
                    ))
                }
                Value::String(ref s) => {
                    if let Value::Number(idx) = key {
                        let idx = idx.trunc() as usize;
                        Ok((
                            s.chars()
                                .nth(idx)
                                .map(|ch| Value::String(ch.to_string()))
                                .unwrap_or(Value::Nil),
                            context.world.clone(),
                        ))
                    } else {
                        Err(eval_err!(
                            general,
                            &args[1],
                            "`get` string index must be a number"
                        ))
                    }
                }
                _ => Err(eval_err!(
                    general,
                    &args[0],
                    "`get` expects list, map, or string as first arg when given two args"
                )),
            }
        }
        _ => Err(eval_err!(arity, parent_span, args, "get", 1)),
    }
};

/// (do <exprs...>)
pub const ATOM_DO: AtomFn = |args, context, _| {
    let mut val = Value::default();
    let mut world = context.world.clone();
    for expr in args {
        let (v, w) = context.eval(expr)?;
        val = v;
        world = w;
    }
    Ok((val, world))
};

/// (mod <a> <b>)
pub const ATOM_MOD: AtomFn = |args, context, parent_span| {
    if args.len() != 2 {
        return Err(eval_err!(arity, parent_span, args, "mod", 2));
    }
    let (v1, w1) = context.eval(&args[0])?;
    let mut ctx2 = EvalContext {
        world: &w1,
        output: context.output,
        opts: context.opts,
        depth: context.depth,
    };
    let (v2, w2) = ctx2.eval(&args[1])?;
    match (v1, v2) {
        (Value::Number(n1), Value::Number(n2)) => {
            if n2 == 0.0 {
                return Err(eval_err!(general, &args[1], "Modulo by zero"));
            }
            if n1.fract() != 0.0 || n2.fract() != 0.0 {
                return Err(eval_err!(
                    type,
                    &Expr::List(args.to_vec(), parent_span.clone()),
                    "mod",
                    "two Integers",
                    Value::Number(n1)
                ));
            }
            Ok((Value::Number((n1 as i64 % n2 as i64) as f64), w2))
        }
        (a, b) => Err(eval_err!(
            general,
            &Expr::List(args.to_vec(), parent_span.clone()),
            &format!(
                "`mod` expects two Integers, got {} and {}",
                a.type_name(),
                b.type_name()
            )
        )),
    }
};
