//!
//! This module provides the core evaluation engine for Sutra expressions, handling
//! the translation from AST nodes to runtime values within the context of a world state.
//!
//! ## Core Responsibility: AST → Value Translation
//!
//! The evaluation engine transforms parsed AST expressions into runtime values while
//! maintaining world state consistency and handling recursive evaluation contexts.
//!
//! ## Error Handling
//!
//! All errors in this module are reported via the unified `SutraError` type and must be constructed using the `sutra_err!` macro. See `src/diagnostics.rs` for macro arms and usage rules.
//!
//! Example:
//! ```rust
//! return Err(sutra_err!(Eval, "Arity error".to_string()));
//! ```
//!
//! All evaluation, type, and recursion errors use this system.

// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
use crate::ast::value::Value;
use crate::ast::{AstNode, Expr, WithSpan};
use crate::atoms::{AtomRegistry, OutputSink};
use crate::runtime::context::ExecutionContext;
use crate::runtime::world::World;
use crate::SutraError;
use crate::sutra_err;

// ===================================================================================================
// CORE DATA STRUCTURES: Evaluation Context
// ===================================================================================================

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
    /// Also checks for macros first before checking atoms.
    pub fn call_atom(
        &mut self,
        symbol_name: &str,
        head: &AstNode,
        args: &[AstNode],
        span: &crate::ast::Span,
    ) -> Result<(Value, World), SutraError> {
        // First, try to resolve as a macro
        if let Some((_provenance, macro_def)) = self.world.macros.lookup_macro(symbol_name) {
            // Handle macro expansion
            return match macro_def {
                crate::macros::MacroDef::Template(template) => {
                    // Create a call node for the template expansion
                    let call_node = WithSpan {
                        value: std::sync::Arc::new(Expr::List(
                            {
                                let mut items = vec![head.clone()];
                                items.extend_from_slice(args);
                                items
                            },
                            span.clone(),
                        )),
                        span: span.clone(),
                    };

                    // Expand the template macro
                    let expanded = crate::macros::expand_template(template, &call_node, 0)?;
                    // Evaluate the expanded expression
                    eval_expr(&expanded, self)
                }
                crate::macros::MacroDef::Fn(macro_fn) => {
                    // Create a call node for the function macro
                    let call_node = WithSpan {
                        value: std::sync::Arc::new(Expr::List(
                            {
                                let mut items = vec![head.clone()];
                                items.extend_from_slice(args);
                                items
                            },
                            span.clone(),
                        )),
                        span: span.clone(),
                    };

                    // Call the function macro
                    let expanded = macro_fn(&call_node)?;
                    // Evaluate the expanded expression
                    eval_expr(&expanded, self)
                }
            };
        }

        // If not a macro, try to resolve as an atom
        let Some(atom) = self.atom_registry.get(symbol_name).cloned() else {
            return Err(sutra_err!(Eval, "Undefined symbol: '{}'", symbol_name));
        };

        // Dispatch to the correct atom type.
        match atom {
            // The special form path, for atoms that control their own evaluation.
            crate::atoms::Atom::SpecialForm(special_form_fn) => special_form_fn(args, self, span),

            // The new path for atoms that need to interact with the world state.
            crate::atoms::Atom::Stateful(stateful_fn) => {
                // Convert AstNodes to Values for stateful atoms
                let mut values = Vec::new();
                for arg in args {
                    let (val, _) = eval_expr(arg, self)?;
                    values.push(val);
                }
                // Create a mutable reference to world for state modifications
                let mut world_context = self.world.clone();
                let result = {
                    let mut exec_context = ExecutionContext {
                        state: &mut world_context.state,
                        output: self.output,
                        rng: &mut world_context.prng,
                    };
                    stateful_fn(&values, &mut exec_context)?
                };
                Ok((result, world_context))
            }

            // The new path for pure functions that have no side effects.
            crate::atoms::Atom::Pure(pure_fn) => {
                // Convert AstNodes to Values for pure atoms
                let mut values = Vec::new();
                for arg in args {
                    let (val, _) = eval_expr(arg, self)?;
                    values.push(val);
                }
                let result = pure_fn(&values)?;
                Ok((result, self.world.clone()))
            }
        }
    }
}

// ===================================================================================================
// DRY UTILITIES: Common Evaluation Patterns
// ===================================================================================================

/// Wraps a value with the current world state in the standard (Value, World) result format.
fn wrap_value_with_world(value: Value, world: &World) -> Result<(Value, World), SutraError> {
    Ok((value, world.clone()))
}

/// Helper to evaluate a conditional expression and return a boolean result.
fn eval_condition_as_bool(
    condition: &AstNode,
    context: &mut EvalContext,
) -> Result<(bool, World), SutraError> {
    let (cond_val, next_world) = eval_expr(condition, context)?;

    // Guard clause: ensure condition evaluates to boolean
    let Value::Bool(b) = cond_val else {
        return Err(sutra_err!(TypeError, "Type error: if condition must be boolean".to_string()));
    };

    Ok((b, next_world))
}

/// Evaluates literal value expressions (Path, String, Number, Bool).
fn eval_literal_value(expr: &AstNode, world: &World) -> Result<(Value, World), SutraError> {
    let value = match &*expr.value {
        Expr::Path(p, _) => Value::Path(p.clone()),
        Expr::String(s, _) => Value::String(s.clone()),
        Expr::Number(n, _) => Value::Number(*n),
        Expr::Bool(b, _) => Value::Bool(*b),
        _ => return Err(sutra_err!(Internal, "eval_literal_value called with non-literal expression".to_string())),
    };
    wrap_value_with_world(value, world)
}

/// Handles evaluation of invalid expression types that cannot be evaluated at runtime.
fn eval_invalid_expr(expr: &AstNode) -> Result<(Value, World), SutraError> {
    match &*expr.value {
        Expr::ParamList(_) => Err(sutra_err!(Eval, "Cannot evaluate parameter list (ParamList AST node) at runtime".to_string())),
        Expr::Symbol(s, _span) => Err(sutra_err!(TypeError, format!("Type error: explicit (get ...) call required for symbol evaluation: {}", s).to_string())),
        Expr::Spread(_) => Err(sutra_err!(Eval, "Spread argument not allowed outside of call position (list context)".to_string())),
        _ => unreachable!("eval_invalid_expr called with valid expression type"),
    }
}

// ===================================================================================================
// PUBLIC API: Expression Evaluation Interface
// ===================================================================================================

/// Evaluates a Sutra AST node in the given world, with output and options.
///
/// # Note
/// This is a low-level, internal function. Most users should use the higher-level `eval` API.
pub fn eval_expr(expr: &AstNode, context: &mut EvalContext) -> Result<(Value, World), SutraError> {
    if context.depth > context.max_depth {
        return Err(sutra_err!(Internal, "Recursion limit exceeded".to_string()));
    }

    match &*expr.value {
        // Complex expression types with dedicated handlers
        Expr::List(items, span) => eval_list(items, span, context),
        Expr::Quote(inner, _) => eval_quote(inner, context),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => eval_if(condition, then_branch, else_branch, context),
        Expr::Define {
            name, params, body, ..
        } => {
            // If params are not empty, it's a function/macro definition.
            if !params.required.is_empty() || params.rest.is_some() {
                let mut new_world = context.world.clone();
                let template = crate::macros::MacroTemplate::new(params.clone(), body.clone())?;
                new_world.macros = new_world
                    .macros
                    .with_user_macro(name.clone(), crate::macros::MacroDef::Template(template));
                Ok((Value::Nil, new_world))
            } else {
                // It's a variable definition.
                let (value, world) = eval_expr(body, context)?;
                let new_world = world;
                let path = crate::runtime::path::Path(vec![name.clone()]);
                new_world.state.set(&path, value.clone());
                Ok((value, new_world))
            }
        }

        // Literal value types
        Expr::Path(..) | Expr::String(..) | Expr::Number(..) | Expr::Bool(..) => {
            eval_literal_value(expr, context.world)
        }

        // Invalid expression types
        Expr::ParamList(..) | Expr::Symbol(..) | Expr::Spread(..) => eval_invalid_expr(expr),
    }
}

/// Public API: evaluates an expression with the given world, output, atom registry, and max depth.
pub fn eval(
    expr: &AstNode,
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

// ===================================================================================================
// INTERNAL HELPERS: Expression-Specific Evaluation
// ===================================================================================================

// --- Core Expression Handlers ---

/// Helper for evaluating Expr::List arms.
fn eval_list(
    items: &[AstNode],
    span: &crate::ast::Span,
    context: &mut EvalContext,
) -> Result<(Value, World), SutraError> {
    if items.is_empty() {
        return wrap_value_with_world(Value::List(vec![]), context.world);
    }

    // Extract symbol name using guard clause pattern
    let head = &items[0];
    let tail = &items[1..];
    let Expr::Symbol(symbol_name, _) = &*head.value else {
        return Err(sutra_err!(Eval, "Arity error: first element must be a symbol naming a callable entity".to_string()));
    };

    // Use direct atom resolution
    let flat_tail = flatten_spread_args(tail, context)?;
    context.call_atom(symbol_name, head, &flat_tail, span)
}

/// Helper for evaluating Expr::Quote arms.
fn eval_quote(
    inner: &AstNode,
    context: &mut EvalContext,
) -> Result<(Value, World), SutraError> {
    match &*inner.value {
        Expr::Symbol(s, _) => wrap_value_with_world(Value::String(s.clone()), context.world),
        Expr::List(exprs, _) => wrap_value_with_world(eval_quoted_list(exprs)?, context.world),
        Expr::Number(n, _) => wrap_value_with_world(Value::Number(*n), context.world),
        Expr::Bool(b, _) => wrap_value_with_world(Value::Bool(*b), context.world),
        Expr::String(s, _) => wrap_value_with_world(Value::String(s.clone()), context.world),
        Expr::Path(p, _) => wrap_value_with_world(Value::Path(p.clone()), context.world),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => eval_quoted_if(condition, then_branch, else_branch, context),
        Expr::Quote(_, _) => wrap_value_with_world(Value::Nil, context.world),
        Expr::Define { .. } => Err(sutra_err!(Eval, "Cannot quote define expressions".to_string())),
        Expr::ParamList(_) => Err(sutra_err!(Eval, "Cannot evaluate parameter list (ParamList AST node) at runtime".to_string())),
        Expr::Spread(_) => Err(sutra_err!(Eval, "Spread argument not allowed inside quote".to_string())),
    }
}

/// Helper for evaluating Expr::If arms.
fn eval_if(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &AstNode,
    context: &mut EvalContext,
) -> Result<(Value, World), SutraError> {
    let (is_true, next_world) = eval_condition_as_bool(condition, context)?;
    let mut sub_context = EvalContext {
        world: &next_world,
        output: context.output,
        atom_registry: context.atom_registry,
        max_depth: context.max_depth,
        depth: context.depth + 1,
    };

    let branch = if is_true { then_branch } else { else_branch };
    eval_expr(branch, &mut sub_context)
}

// --- Quote Expression Helpers ---

/// Evaluates a single expression within a quote context.
fn eval_quoted_expr(expr: &AstNode) -> Result<Value, SutraError> {
    match &*expr.value {
        Expr::Symbol(s, _) => Ok(Value::String(s.clone())),
        Expr::Number(n, _) => Ok(Value::Number(*n)),
        Expr::Bool(b, _) => Ok(Value::Bool(*b)),
        Expr::String(s, _) => Ok(Value::String(s.clone())),
        Expr::ParamList(_) => Err(sutra_err!(Eval, "Cannot evaluate parameter list (ParamList AST node) inside quote".to_string())),
        Expr::Spread(_) => Err(sutra_err!(Eval, "Spread argument not allowed inside quote".to_string())),
        _ => Ok(Value::Nil),
    }
}

/// Evaluates a quoted list by converting each element to a value.
fn eval_quoted_list(exprs: &[AstNode]) -> Result<Value, SutraError> {
    let vals: Result<Vec<_>, SutraError> = exprs.iter().map(eval_quoted_expr).collect();
    Ok(Value::List(vals?))
}

/// Evaluates a quoted if expression.
fn eval_quoted_if(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &AstNode,
    context: &mut EvalContext,
) -> Result<(Value, World), SutraError> {
    let (is_true, next_world) = eval_condition_as_bool(condition, context)?;
    let mut sub_context = EvalContext {
        world: &next_world,
        output: context.output,
        atom_registry: context.atom_registry,
        max_depth: context.max_depth,
        depth: context.depth + 1,
    };

    let branch = if is_true { then_branch } else { else_branch };
    eval_expr(branch, &mut sub_context)
}

// --- Argument Processing Helpers ---

/// Flattens spread arguments in function call arguments.
fn flatten_spread_args(
    tail: &[AstNode],
    context: &mut EvalContext,
) -> Result<Vec<AstNode>, SutraError> {
    let mut flat_tail = Vec::new();

    for arg in tail {
        // Guard clause: handle non-spread expressions immediately
        let Expr::Spread(expr) = &*arg.value else {
            // FIX: only one deref for Arc<Expr>
            flat_tail.push(arg.clone());
            continue;
        };

        // Guard clause: evaluate spread expression
        let (val, _) = eval_expr(expr, context)?;

        // Guard clause: ensure we have a list for spreading
        let Value::List(items) = val else {
            return Err(sutra_err!(TypeError, format!("Type error: spread argument must be a list: {}", &val).to_string()));
        };

        // Process list items without nesting
        for v in items {
            flat_tail.push(WithSpan {
                value: Expr::from(v).into(), // FIX: wrap Expr in Arc via .into()
                span: arg.span.clone(),
            });
        }
    }

    Ok(flat_tail)
}
