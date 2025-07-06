use crate::ast::{Expr, WithSpan};
use crate::atom::{AtomRegistry, OutputSink};
use crate::error::{EvalError, SutraError, SutraErrorKind};
use crate::value::Value;
use crate::world::World;

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
    pub fn eval_in(&mut self, world: &World, expr: &WithSpan<Expr>) -> Result<(Value, World), SutraError> {
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

fn eval_expr(
    expr: &WithSpan<Expr>,
    world: &World,
    output: &mut dyn OutputSink,
    opts: &EvalOptions,
    depth: usize,
) -> Result<(Value, World), SutraError> {
    if depth > opts.max_depth {
        return Err(SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                message: "Recursion depth limit exceeded.".to_string(),
                expanded_code: expr.value.pretty(),
                original_code: None,
                suggestion: None,
            }),
            span: Some(expr.span.clone()),
        });
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
                return Err(SutraError {
                    kind: SutraErrorKind::Eval(EvalError {
                        message: "The first element of a list to be evaluated must be a symbol naming an atom.".to_string(),
                        expanded_code: expr.value.pretty(),
                        original_code: None,
                        suggestion: None,
                    }),
                    span: Some(head.span.clone()),
                });
            };

            let atom_fn = if let Some(f) = opts.atom_registry.get(atom_name) {
                f
            } else {
                return Err(SutraError {
                    kind: SutraErrorKind::Eval(EvalError {
                        message: format!("Atom '{}' not found.", atom_name),
                        expanded_code: expr.value.pretty(),
                        original_code: None,
                        suggestion: None,
                    }),
                    span: Some(head.span.clone()),
                });
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
                    let vals: Result<Vec<_>, SutraError> = exprs.iter().map(|e| match &e.value {
                        Expr::Symbol(s, _) => Ok(Value::String(s.clone())),
                        Expr::Number(n, _) => Ok(Value::Number(*n)),
                        Expr::Bool(b, _) => Ok(Value::Bool(*b)),
                        Expr::String(s, _) => Ok(Value::String(s.clone())),
                        Expr::ParamList(_) => Err(SutraError {
                            kind: SutraErrorKind::Eval(EvalError {
                                message: "Cannot evaluate parameter list (ParamList AST node) inside quote".to_string(),
                                expanded_code: inner.value.pretty(),
                                original_code: None,
                                suggestion: None,
                            }),
                            span: Some(inner.span.clone()),
                        }),
                        _ => Ok(Value::Nil),
                    }).collect();
                    Ok((Value::List(vals?), world.clone()))
                }
                Expr::Number(n, _) => Ok((Value::Number(*n), world.clone())),
                Expr::Bool(b, _) => Ok((Value::Bool(*b), world.clone())),
                Expr::String(s, _) => Ok((Value::String(s.clone()), world.clone())),
                Expr::Path(p, _) => Ok((Value::Path(p.clone()), world.clone())),
                Expr::If { .. } => Ok((Value::Nil, world.clone())),
                Expr::Quote(_, _) => Ok((Value::Nil, world.clone())),
                Expr::ParamList(_) => {
                    Err(SutraError {
                        kind: SutraErrorKind::Eval(EvalError {
                            message: "Cannot evaluate parameter list (ParamList AST node) at runtime".to_string(),
                            expanded_code: expr.value.pretty(),
                            original_code: None,
                            suggestion: None,
                        }),
                        span: Some(expr.span.clone()),
                    })
                }
            }
        }
        Expr::ParamList(_) => {
            Err(SutraError {
                kind: SutraErrorKind::Eval(EvalError {
                    message: "Cannot evaluate parameter list (ParamList AST node) at runtime".to_string(),
                    expanded_code: expr.value.pretty(),
                    original_code: None,
                    suggestion: None,
                }),
                span: Some(expr.span.clone()),
            })
        }
        Expr::Symbol(s, span) => Err(SutraError {
            kind: SutraErrorKind::Eval(EvalError {
                message: format!(
                    "Unexpected bare symbol '{}' found during evaluation. All value lookups must be explicit `(get ...)` calls.",
                    s
                ),
                expanded_code: expr.value.pretty(),
                original_code: None,
                suggestion: Some("Did you mean to use `(get ...)`?".to_string()),
            }),
            span: Some(span.clone()),
        }),
        Expr::Path(path, _) => Ok((Value::Path(path.clone()), world.clone())),
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
                Err(SutraError {
                    kind: SutraErrorKind::Eval(EvalError {
                        message: "The condition for an `if` expression must evaluate to a Boolean."
                            .to_string(),
                        expanded_code: condition.value.pretty(),
                        original_code: None,
                        suggestion: None,
                    }),
                    span: Some(condition.span.clone()),
                })
            }
        }
    }
}
