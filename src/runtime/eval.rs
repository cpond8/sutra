//! # Sutra Runtime: Expression Evaluation Engine
//!
//! This module provides the core evaluation engine for Sutra expressions, handling
//! the translation from AST nodes to runtime values within the context of a world state.
//!
//! ## Core Responsibility: AST → Value Translation
//!
//! The evaluation engine transforms parsed AST expressions into runtime values while
//! maintaining world state consistency and handling recursive evaluation contexts.

// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
use crate::ast::value::Value;
use crate::ast::{AstNode, Expr, WithSpan};
use crate::atoms::{AtomRegistry, OutputSink};
use crate::runtime::world::World;
use crate::syntax::error::{
    eval_arity_error, eval_general_error, eval_type_error, recursion_depth_error, SutraError,
};

// ===================================================================================================
// CORE DATA STRUCTURES: Evaluation Context
// ===================================================================================================

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
        args: &[AstNode],
        span: &crate::ast::Span,
    ) -> Result<(Value, World), SutraError> {
        // Guard clause: ensure atom exists in registry
        let Some(atom_fn) = self.atom_registry.get(atom_name) else {
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
        return Err(sutra_error!(
            type,
            Some(condition.span.clone()),
            condition,
            "if",
            "Boolean",
            &cond_val
        ));
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
        _ => unreachable!("eval_literal_value called with non-literal expression"),
    };
    wrap_value_with_world(value, world)
}

/// Handles evaluation of invalid expression types that cannot be evaluated at runtime.
fn eval_invalid_expr(expr: &AstNode) -> Result<(Value, World), SutraError> {
    match &*expr.value {
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
        Expr::Spread(_) => Err(sutra_error!(
            general,
            Some(expr.span.clone()),
            expr,
            "Spread argument not allowed outside of call position (list context)"
        )),
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
pub fn eval_expr(
    expr: &AstNode,
    context: &mut EvalContext,
) -> Result<(Value, World), SutraError> {
    if context.depth > context.max_depth {
        return Err(sutra_error!(recursion, Some(expr.span.clone())));
    }

    match &*expr.value {
        // Complex expression types with dedicated handlers
        Expr::List(items, span) => eval_list(items, span, context),
        Expr::Quote(inner, _) => eval_quote(inner, context, expr),
        Expr::If { condition, then_branch, else_branch, .. } => {
            eval_if(condition, then_branch, else_branch, context)
        }

        // Literal value types
        Expr::Path(..) | Expr::String(..) | Expr::Number(..) | Expr::Bool(..) => {
            eval_literal_value(expr, context.world)
        }

        // Invalid expression types
        Expr::ParamList(..) | Expr::Symbol(..) | Expr::Spread(..) => {
            eval_invalid_expr(expr)
        }
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

    // Extract atom name using guard clause pattern
    let head = &items[0];
    let tail = &items[1..];
    let Expr::Symbol(atom_name, _) = &*head.value else { // FIX: only one deref for Arc<Expr>
        return Err(sutra_error!(
            arity,
            Some(head.span.clone()),
            items,
            "eval",
            "first element must be a symbol naming an atom"
        ));
    };

    let flat_tail = flatten_spread_args(tail, context)?;
    context.call_atom(atom_name, &flat_tail, span)
}

/// Helper for evaluating Expr::Quote arms.
fn eval_quote(
    inner: &AstNode,
    context: &mut EvalContext,
    parent_expr: &AstNode,
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
        Expr::ParamList(_) => Err(sutra_error!(
            general,
            Some(parent_expr.span.clone()),
            parent_expr,
            "Cannot evaluate parameter list (ParamList AST node) at runtime"
        )),
        Expr::Spread(_) => Err(sutra_error!(
            general,
            Some(inner.span.clone()),
            inner,
            "Spread argument not allowed inside quote"
        )),
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
        Expr::ParamList(_) => Err(sutra_error!(
            general,
            Some(expr.span.clone()),
            expr,
            "Cannot evaluate parameter list (ParamList AST node) inside quote"
        )),
        Expr::Spread(_) => Err(sutra_error!(
            general,
            Some(expr.span.clone()),
            expr,
            "Spread argument not allowed inside quote"
        )),
        _ => Ok(Value::Nil),
    }
}

/// Evaluates a quoted list by converting each element to a value.
fn eval_quoted_list(exprs: &[AstNode]) -> Result<Value, SutraError> {
    let vals: Result<Vec<_>, SutraError> = exprs
        .iter()
        .map(eval_quoted_expr)
        .collect();
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
        let Expr::Spread(expr) = &*arg.value else { // FIX: only one deref for Arc<Expr>
            flat_tail.push(arg.clone());
            continue;
        };

        // Guard clause: evaluate spread expression
        let (val, _) = eval_expr(expr, context)?;

        // Guard clause: ensure we have a list for spreading
        let Value::List(items) = val else {
            return Err(sutra_error!(
                type,
                Some(arg.span.clone()),
                arg,
                "spread",
                "List",
                &val
            ));
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
