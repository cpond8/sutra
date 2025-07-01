use crate::ast::Expr;
use crate::atom::{AtomRegistry, OutputSink};
use crate::error::{SutraError, SutraErrorKind};
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

impl<'a, 'o> EvalContext<'a, 'o> {
    /// A helper method to recursively call the evaluator.
    /// This is the primary way atoms should evaluate their arguments.
    pub fn eval(&mut self, expr: &Expr) -> Result<(Value, World), SutraError> {
        eval_expr(expr, self.world, self.output, self.opts, self.depth + 1)
    }
}

pub fn eval(
    expr: &Expr,
    world: &World,
    output: &mut dyn OutputSink,
    opts: &EvalOptions,
) -> Result<(Value, World), SutraError> {
    eval_expr(expr, world, output, opts, 0)
}

fn eval_expr(
    expr: &Expr,
    world: &World,
    output: &mut dyn OutputSink,
    opts: &EvalOptions,
    depth: usize,
) -> Result<(Value, World), SutraError> {
    if depth > opts.max_depth {
        return Err(SutraError {
            kind: SutraErrorKind::Eval("Recursion depth limit exceeded.".to_string()),
            span: Some(expr.span()),
        });
    }

    match expr {
        Expr::List(items, _span) => {
            if items.is_empty() {
                return Ok((Value::List(vec![]), world.clone()));
            }

            let head = &items[0];
            let tail = &items[1..];

            let atom_name = match head {
                Expr::Symbol(s, _) => s,
                _ => {
                    return Err(SutraError {
                        kind: SutraErrorKind::Eval(
                            "The first element of a list must be a symbol naming an atom."
                                .to_string(),
                        ),
                        span: Some(head.span()),
                    })
                }
            };

            let atom_fn = match opts.atom_registry.get(atom_name) {
                Some(f) => f,
                None => {
                    return Err(SutraError {
                        kind: SutraErrorKind::Eval(format!("Atom '{}' not found.", atom_name)),
                        span: Some(head.span()),
                    })
                }
            };

            let mut context = EvalContext {
                world,
                output,
                opts,
                depth,
            };

            atom_fn(tail, &mut context)
        }
        // Literals evaluate to themselves, with the crucial exception of symbols.
        Expr::Symbol(s, _) => {
            // ---
            // TODO: Lexical Scoping and the "Auto-Get" Fallback
            //
            // The current implementation provides the "auto-get" feature by assuming
            // any unresolved symbol is a path to be looked up in the global `World`.
            // This is correct for the current specification.
            //
            // However, when a feature like `let` bindings is introduced to create
            // lexical scopes, this logic will need to be extended. The symbol
            // resolution order should be:
            //
            // 1. Check for the symbol in the current lexical scope (and its parents).
            // 2. If not found, *then* fall back to looking it up in the `World` as a path.
            //
            // This will likely involve adding a lexical `Environment` to the
            // `EvalContext` (e.g., as a stack of HashMaps representing scopes)
            // and checking it here before the world lookup.
            // ---
            let path: Vec<&str> = s.split('.').collect();
            let value = world.get(&path).cloned().unwrap_or_default();
            Ok((value, world.clone()))
        }
        Expr::String(s, _) => Ok((Value::String(s.clone()), world.clone())),
        Expr::Number(n, _) => Ok((Value::Number(*n), world.clone())),
        Expr::Bool(b, _) => Ok((Value::Bool(*b), world.clone())),
    }
}
