// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
use crate::ast::value::Value;
use crate::ast::{Expr, WithSpan};
use crate::atoms::{AtomRegistry, OutputSink};
use crate::runtime::world::World;
use crate::syntax::error::{
    eval_arity_error, eval_general_error, eval_type_error, recursion_depth_error, SutraError,
};

// Move macro for error construction to the very top of the file
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

/// The context for a single evaluation, passed to atoms and all evaluation functions.
pub struct EvalContext<'a, 'o> {
    pub world: &'a World,
    pub output: &'o mut dyn OutputSink,
    pub atom_registry: &'a AtomRegistry,
    pub max_depth: usize,
    pub depth: usize,
}

impl EvalContext<'_, '_> {
    /// Helper to increment depth for recursive calls.
    pub fn next_depth(&self) -> usize {
        self.depth + 1
    }

    /// Looks up and invokes an atom by name, handling errors for missing atoms.
    pub fn call_atom(
        &mut self,
        atom_name: &str,
        args: &[WithSpan<Expr>],
        span: &crate::ast::Span,
    ) -> Result<(Value, World), SutraError> {
        let atom_fn = if let Some(f) = self.atom_registry.get(atom_name) {
            f
        } else {
            return Err(sutra_error!(
                type,
                Some(span.clone()),
                &args[0],
                "eval",
                "atom",
                &Value::String(atom_name.to_string())
            ));
        };
        atom_fn(args, self, span)
    }
}

/// Evaluates a Sutra AST node in the given world, with output and options.
///
/// # Note
/// This is a low-level, internal function. Most users should use the higher-level `eval` API.
pub fn eval_expr(
    expr: &WithSpan<Expr>,
    context: &mut EvalContext,
) -> Result<(Value, World), SutraError> {
    if context.depth > context.max_depth {
        return Err(sutra_error!(recursion, Some(expr.span.clone())));
    }
    match &expr.value {
        Expr::List(items, span) => eval_list(items, span, context),
        Expr::Quote(inner, _) => eval_quote(inner, context, expr),
        Expr::ParamList(_) => Err(sutra_error!(
            general,
            Some(expr.span.clone()),
            expr,
            "Cannot evaluate parameter list (ParamList AST node) at runtime"
        )),
        Expr::Symbol(s, span) => Err(sutra_error!(
            type,
            Some(span.clone()),
            expr,
            "eval",
            "explicit (get ...) call",
            &Value::String(s.clone())
        )),
        Expr::Path(p, _) => Ok((Value::Path(p.clone()), context.world.clone())),
        Expr::String(s, _) => Ok((Value::String(s.clone()), context.world.clone())),
        Expr::Number(n, _) => Ok((Value::Number(*n), context.world.clone())),
        Expr::Bool(b, _) => Ok((Value::Bool(*b), context.world.clone())),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => eval_if(condition, then_branch, else_branch, context),
    }
}

/// Public API: evaluates an expression with the given world, output, atom registry, and max depth.
pub fn eval(
    expr: &WithSpan<Expr>,
    world: &World,
    output: &mut dyn OutputSink,
    atom_registry: &AtomRegistry,
    max_depth: usize,
) -> Result<(Value, World), SutraError> {
    let mut context = EvalContext {
        world,
        output,
        atom_registry,
        max_depth,
        depth: 0,
    };
    eval_expr(expr, &mut context)
}

/// Helper for evaluating Expr::List arms.
fn eval_list(
    items: &[WithSpan<Expr>],
    span: &crate::ast::Span,
    context: &mut EvalContext,
) -> Result<(Value, World), SutraError> {
    if items.is_empty() {
        // An empty list evaluates to an empty list value.
        return Ok((Value::List(vec![]), context.world.clone()));
    }
    // The head of the list must be a symbol corresponding to an atom.
    // The macro expansion phase is responsible for ensuring this.
    let head = &items[0];
    let tail = &items[1..];
    let atom_name = if let Expr::Symbol(s, _) = &head.value {
        s
    } else {
        return Err(sutra_error!(
            arity,
            Some(head.span.clone()),
            items,
            "eval",
            "first element must be a symbol naming an atom"
        ));
    };
    context.call_atom(atom_name, tail, span)
}

/// Helper for evaluating Expr::Quote arms.
fn eval_quote(
    inner: &WithSpan<Expr>,
    context: &mut EvalContext,
    parent_expr: &WithSpan<Expr>,
) -> Result<(Value, World), SutraError> {
    match &inner.value {
        Expr::Symbol(s, _) => Ok((Value::String(s.clone()), context.world.clone())),
        Expr::List(exprs, _) => {
            let vals: Result<Vec<_>, SutraError> = exprs
                .iter()
                .map(|e| match &e.value {
                    Expr::Symbol(s, _) => Ok(Value::String(s.clone())),
                    Expr::Number(n, _) => Ok(Value::Number(*n)),
                    Expr::Bool(b, _) => Ok(Value::Bool(*b)),
                    Expr::String(s, _) => Ok(Value::String(s.clone())),
                    Expr::ParamList(_) => Err(sutra_error!(
                        general,
                        Some(inner.span.clone()),
                        inner,
                        "Cannot evaluate parameter list (ParamList AST node) inside quote"
                    )),
                    _ => Ok(Value::Nil),
                })
                .collect();
            Ok((Value::List(vals?), context.world.clone()))
        }
        Expr::Number(n, _) => Ok((Value::Number(*n), context.world.clone())),
        Expr::Bool(b, _) => Ok((Value::Bool(*b), context.world.clone())),
        Expr::String(s, _) => Ok((Value::String(s.clone()), context.world.clone())),
        Expr::Path(p, _) => Ok((Value::Path(p.clone()), context.world.clone())),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let mut sub_context = EvalContext {
                world: context.world,
                output: context.output,
                atom_registry: context.atom_registry,
                max_depth: context.max_depth,
                depth: context.depth + 1,
            };
            let (cond_val, next_world) = eval_expr(condition, &mut sub_context)?;
            if let Value::Bool(b) = cond_val {
                if b {
                    eval_expr(
                        then_branch,
                        &mut EvalContext {
                            world: &next_world,
                            ..sub_context
                        },
                    )
                } else {
                    eval_expr(
                        else_branch,
                        &mut EvalContext {
                            world: &next_world,
                            ..sub_context
                        },
                    )
                }
            } else {
                Err(sutra_error!(
                    type,
                    Some(condition.span.clone()),
                    condition,
                    "if",
                    "Boolean",
                    &cond_val
                ))
            }
        }
        Expr::Quote(_, _) => Ok((Value::Nil, context.world.clone())),
        Expr::ParamList(_) => Err(sutra_error!(
            general,
            Some(parent_expr.span.clone()),
            parent_expr,
            "Cannot evaluate parameter list (ParamList AST node) at runtime"
        )),
    }
}

/// Helper for evaluating Expr::If arms.
fn eval_if(
    condition: &WithSpan<Expr>,
    then_branch: &WithSpan<Expr>,
    else_branch: &WithSpan<Expr>,
    context: &mut EvalContext,
) -> Result<(Value, World), SutraError> {
    let mut sub_context = EvalContext {
        world: context.world,
        output: context.output,
        atom_registry: context.atom_registry,
        max_depth: context.max_depth,
        depth: context.depth + 1,
    };
    let (cond_val, next_world) = eval_expr(condition, &mut sub_context)?;
    if let Value::Bool(b) = cond_val {
        if b {
            eval_expr(
                then_branch,
                &mut EvalContext {
                    world: &next_world,
                    ..sub_context
                },
            )
        } else {
            eval_expr(
                else_branch,
                &mut EvalContext {
                    world: &next_world,
                    ..sub_context
                },
            )
        }
    } else {
        Err(sutra_error!(
            type,
            Some(condition.span.clone()),
            condition,
            "if",
            "Boolean",
            &cond_val
        ))
    }
}
