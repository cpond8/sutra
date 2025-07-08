// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
use crate::ast::{Expr, WithSpan};
use crate::atoms::{AtomRegistry, OutputSink};
use crate::syntax::error::{
    eval_arity_error, eval_general_error, eval_type_error, recursion_depth_error, SutraError,
};
use crate::ast::value::Value;
use crate::runtime::world::World;

pub struct EvalOptions {
    pub max_depth: usize,
    pub atom_registry: AtomRegistry,
}

/// The context for a single evaluation, passed to atoms.
pub struct EvalContext<'a, 'o> {
    pub world: &'a World,
    pub output: &'o mut dyn OutputSink,
    pub opts: &'a EvalOptions,
    pub depth: usize,
}

impl EvalContext<'_, '_> {
    /// A helper method to recursively call the evaluator with the context's current world.
    pub fn eval(&mut self, expr: &WithSpan<Expr>) -> Result<(Value, World), SutraError> {
        eval_expr(expr, self.world, self.output, self.opts, self.depth + 1)
    }

    /// A helper method to recursively call the evaluator with a new world state.
    pub fn eval_in(
        &mut self,
        world: &World,
        expr: &WithSpan<Expr>,
    ) -> Result<(Value, World), SutraError> {
        eval_expr(expr, world, self.output, self.opts, self.depth + 1)
    }
}

pub fn eval(
    expr: &WithSpan<Expr>,
    world: &World,
    output: &mut dyn OutputSink,
    opts: &EvalOptions,
) -> Result<(Value, World), SutraError> {
    eval_expr(expr, world, output, opts, 0)
}

/// Evaluates a Sutra AST node in the given world, with output and options.
///
/// # Note
/// This is a low-level, internal function. Most users should use the higher-level `eval` API.
pub fn eval_expr(
    expr: &WithSpan<Expr>,
    world: &World,
    output: &mut dyn OutputSink,
    opts: &EvalOptions,
    depth: usize,
) -> Result<(Value, World), SutraError> {
    if depth > opts.max_depth {
        return Err(recursion_depth_error(Some(expr.span.clone())));
    }

    match &expr.value {
        Expr::List(items, span) => {
            if items.is_empty() {
                // An empty list evaluates to an empty list value.
                return Ok((Value::List(vec![]), world.clone()));
            }

            // The head of the list must be a symbol corresponding to an atom.
            // The macro expansion phase is responsible for ensuring this.
            let head = &items[0];
            let tail = &items[1..];

            let atom_name = if let Expr::Symbol(s, _) = &head.value {
                s
            } else {
                return Err(eval_arity_error(
                    Some(head.span.clone()),
                    items,
                    "eval",
                    "first element must be a symbol naming an atom",
                ));
            };

            let atom_fn = if let Some(f) = opts.atom_registry.get(atom_name) {
                f
            } else {
                return Err(eval_type_error(
                    Some(head.span.clone()),
                    head,
                    "eval",
                    "atom",
                    &Value::String(atom_name.clone()),
                ));
            };

            let mut context = EvalContext {
                world,
                output,
                opts,
                depth,
            };

            atom_fn(tail, &mut context, span)
        }
        Expr::Quote(inner, _) => {
            // Evaluate to the quoted value as a Value variant
            match &inner.value {
                Expr::Symbol(s, _) => Ok((Value::String(s.clone()), world.clone())),
                Expr::List(exprs, _) => {
                    let vals: Result<Vec<_>, SutraError> = exprs
                        .iter()
                        .map(|e| match &e.value {
                            Expr::Symbol(s, _) => Ok(Value::String(s.clone())),
                            Expr::Number(n, _) => Ok(Value::Number(*n)),
                            Expr::Bool(b, _) => Ok(Value::Bool(*b)),
                            Expr::String(s, _) => Ok(Value::String(s.clone())),
                            Expr::ParamList(_) => Err(eval_general_error(
                                Some(inner.span.clone()),
                                inner,
                                "Cannot evaluate parameter list (ParamList AST node) inside quote",
                            )),
                            _ => Ok(Value::Nil),
                        })
                        .collect();
                    Ok((Value::List(vals?), world.clone()))
                }
                Expr::Number(n, _) => Ok((Value::Number(*n), world.clone())),
                Expr::Bool(b, _) => Ok((Value::Bool(*b), world.clone())),
                Expr::String(s, _) => Ok((Value::String(s.clone()), world.clone())),
                Expr::Path(p, _) => Ok((Value::Path(p.clone()), world.clone())),
                Expr::If {
                    condition,
                    then_branch,
                    else_branch,
                    ..
                } => {
                    let (cond_val, next_world) =
                        eval_expr(condition, world, output, opts, depth + 1)?;
                    if let Value::Bool(b) = cond_val {
                        if b {
                            eval_expr(then_branch, &next_world, output, opts, depth + 1)
                        } else {
                            eval_expr(else_branch, &next_world, output, opts, depth + 1)
                        }
                    } else {
                        Err(eval_type_error(
                            Some(condition.span.clone()),
                            condition,
                            "if",
                            "Boolean",
                            &cond_val,
                        ))
                    }
                }
                Expr::Quote(_, _) => Ok((Value::Nil, world.clone())),
                Expr::ParamList(_) => Err(eval_general_error(
                    Some(expr.span.clone()),
                    expr,
                    "Cannot evaluate parameter list (ParamList AST node) at runtime",
                )),
            }
        }
        Expr::ParamList(_) => Err(eval_general_error(
            Some(expr.span.clone()),
            expr,
            "Cannot evaluate parameter list (ParamList AST node) at runtime",
        )),
        Expr::Symbol(s, span) => Err(eval_type_error(
            Some(span.clone()),
            expr,
            "eval",
            "explicit (get ...) call",
            &Value::String(s.clone()),
        )),
        Expr::Path(p, _) => Ok((Value::Path(p.clone()), world.clone())),
        Expr::String(s, _) => Ok((Value::String(s.clone()), world.clone())),
        Expr::Number(n, _) => Ok((Value::Number(*n), world.clone())),
        Expr::Bool(b, _) => Ok((Value::Bool(*b), world.clone())),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let (cond_val, next_world) = eval_expr(condition, world, output, opts, depth + 1)?;
            if let Value::Bool(b) = cond_val {
                if b {
                    eval_expr(then_branch, &next_world, output, opts, depth + 1)
                } else {
                    eval_expr(else_branch, &next_world, output, opts, depth + 1)
                }
            } else {
                Err(eval_type_error(
                    Some(condition.span.clone()),
                    condition,
                    "if",
                    "Boolean",
                    &cond_val,
                ))
            }
        }
    }
}
