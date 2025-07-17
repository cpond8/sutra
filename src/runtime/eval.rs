//! This module provides the core evaluation engine for Sutra expressions, handling
//! the translation from AST nodes to runtime values within the context of a world state.
//!
//! ## Core Responsibility: AST → Value Translation
//!
//! The evaluation engine transforms parsed AST expressions into runtime values while
//! maintaining world state consistency and handling recursive evaluation contexts.
//!
//! ## CRITICAL: Atom Calling Conventions
//!
//! This module correctly dispatches to atoms based on their registered `Atom`
//! variant (`Pure`, `Stateful`, or `SpecialForm`). `Pure` and `Stateful` atoms
//! have their arguments eagerly evaluated, while `SpecialForm` atoms receive
//! unevaluated `AstNode`s to manage their own evaluation.
//!
//! **It is a critical safety violation to misclassify an atom.** See the
//! documentation in `src/atoms/mod.rs` for a detailed explanation of the
//! dual-convention architecture.
//!
//! ## Error Handling
//!
//! All errors in this module are reported via the unified `SutraError` type and must be constructed using the `err_msg!` or `err_ctx!` macro. See `src/diagnostics.rs` for macro arms and usage rules.
//!
//! Example:
//! ```rust
//! use sutra::err_msg;
//! let err = err_msg!(Eval, "Arity error");
//! assert!(matches!(err, sutra::SutraError::Eval { .. }));
//! ```
//!
//! All evaluation, type, and recursion errors use this system.

// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
use crate::ast::value::Value;
use crate::ast::{AstNode, Expr, Spanned};
use crate::atoms::{AtomRegistry, OutputSink};
use crate::diagnostics::SutraError;
use crate::runtime::context::AtomExecutionContext;
use crate::runtime::world::World;
use crate::err_ctx;
use crate::err_src;
use miette::NamedSource;
use std::sync::Arc;
use crate::macros::expand_macro_call;

// ===================================================================================================
// CORE DATA STRUCTURES: Evaluation Context
// ===================================================================================================

/// The context for a single evaluation, passed to atoms and all evaluation functions.
pub struct EvaluationContext<'a, 'o> {
    pub world: &'a World,
    pub output: &'o mut dyn OutputSink,
    pub atom_registry: &'a AtomRegistry,
    pub source: Arc<NamedSource<String>>,
    pub max_depth: usize,
    pub depth: usize,
}

impl EvaluationContext<'_, '_> {
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
        if let Some((_, macro_def)) = self.world.macros.lookup_macro(symbol_name) {
            // Handle macro expansion
            // Create the full call AST node
            let call_node = Spanned {
                value: std::sync::Arc::new(Expr::List(
                    {
                        let mut items = vec![head.clone()];
                        items.extend_from_slice(args);
                        items
                    },
                    *span,
                )),
                span: *span,
            };

            // Expand macro using the centralized expand_macro function
            let expanded = expand_macro_call(macro_def, &call_node, &self.world.macros, self.depth)?;

            // Evaluate the expanded expression using the standard evaluation function
            return evaluate_ast_node(&expanded, self);
        }

        // If not a macro, try to resolve as an atom
        let Some(atom) = self.atom_registry.get(symbol_name).cloned() else {
            return Err(crate::err_src!(
                Eval,
                format!("Undefined symbol: '{}'", symbol_name),
                &self.source,
                head.span
            ));
        };
        // --- SAFETY VALIDATION (DEBUG ONLY) ---
        // This assertion prevents special form atoms, which have a unique calling
        // convention (lazy evaluation), from being misclassified as Pure or Stateful
        // atoms, which would cause runtime failures.
        #[cfg(debug_assertions)]
        {
            const SPECIAL_FORM_NAMES: &[&str] = &[
                "do",
                "error",
                "apply",
                "assert",
                "assert-eq",
                // Test-only special forms
                "test/echo",
                "test/borrow_stress",
                "register-test!",
            ];

            if SPECIAL_FORM_NAMES.contains(&symbol_name) {
                assert!(
                    matches!(atom, crate::atoms::Atom::SpecialForm(_)),
                    "CRITICAL: Atom '{}' is a special form and MUST be registered as Atom::SpecialForm.",
                    symbol_name
                );
            }
        }


        // Dispatch to the correct atom type.
        match atom {
            // The special form path, for atoms that control their own evaluation.
            crate::atoms::Atom::SpecialForm(special_form_fn) => special_form_fn(args, self, span),

            // Eagerly evaluated atoms (Pure and Stateful)
            crate::atoms::Atom::Stateful(stateful_fn) => {
                let (values, world_after_args) = evaluate_eager_args(args, self)?;
                let mut world_context = world_after_args;
                let result = {
                    let mut exec_context = AtomExecutionContext {
                        state: &mut world_context.state,
                        output: self.output,
                        rng: &mut world_context.prng,
                    };
                    stateful_fn(&values, &mut exec_context)?
                };
                Ok((result, world_context))
            }

            crate::atoms::Atom::Pure(pure_fn) => {
                let (values, world_after_args) = evaluate_eager_args(args, self)?;
                let result = pure_fn(&values)?;
                Ok((result, world_after_args))
            }
        }
    }
}

// ===================================================================================================
// DRY UTILITIES: Common Evaluation Patterns
// ===================================================================================================

/// Evaluates arguments for eager atoms (Pure, Stateful), threading world state.
fn evaluate_eager_args(
    args: &[AstNode],
    context: &mut EvaluationContext,
) -> Result<(Vec<Value>, World), SutraError> {
    let mut values = Vec::with_capacity(args.len());
    let mut current_world = context.world.clone();

    for arg in args {
        let (val, next_world) = {
            let next_depth = context.next_depth();
            let mut arg_context = EvaluationContext {
                world: &current_world,
                output: &mut *context.output,
                atom_registry: context.atom_registry,
                source: context.source.clone(),
                max_depth: context.max_depth,
                depth: next_depth,
            };
            evaluate_ast_node(arg, &mut arg_context)?
        };
        values.push(val);
        current_world = next_world;
    }

    Ok((values, current_world))
}

/// Wraps a value with the current world state in the standard (Value, World) result format.
fn wrap_value_with_world_state(value: Value, world: &World) -> Result<(Value, World), SutraError> {
    Ok((value, world.clone()))
}

/// Helper to evaluate a conditional expression and return a boolean result.
fn evaluate_condition_as_bool(
    condition: &AstNode,
    context: &mut EvaluationContext,
) -> Result<(bool, World), SutraError> {
    let (cond_val, next_world) = evaluate_ast_node(condition, context)?;

    // Guard clause: ensure condition evaluates to boolean
    let Value::Bool(b) = cond_val else {
        return Err(err_src!(
            TypeError,
            "if condition must be boolean",
            &context.source,
            condition.span
        ));
    };

    Ok((b, next_world))
}

/// Evaluates literal value expressions (Path, String, Number, Bool).
fn evaluate_literal_value(
    expr: &AstNode,
    context: &mut EvaluationContext,
) -> Result<(Value, World), SutraError> {
    let value = match &*expr.value {
        Expr::Path(p, _) => Value::Path(p.clone()),
        Expr::String(s, _) => Value::String(s.clone()),
        Expr::Number(n, _) => Value::Number(*n),
        Expr::Bool(b, _) => Value::Bool(*b),
        _ => {
            return Err(err_src!(
                Internal,
                "eval_literal_value called with non-literal expression",
                &context.source,
                expr.span
            ))
        }
    };
    wrap_value_with_world_state(value, context.world)
}

/// Handles evaluation of invalid expression types that cannot be evaluated at runtime.
fn evaluate_invalid_expr(
    expr: &AstNode,
    context: &mut EvaluationContext,
) -> Result<(Value, World), SutraError> {
    let (msg, span) = match &*expr.value {
        Expr::ParamList(_) => (
            "Cannot evaluate parameter list (ParamList AST node) at runtime",
            expr.span,
        ),
        Expr::Symbol(s, span) => {
            // Attempt to resolve the symbol as a path in the world state.
            // This allows for dynamic access to world variables.
            let path = crate::runtime::path::Path(vec![s.clone()]);
            if let Some(value) = context.world.state.get(&path) {
                return wrap_value_with_world_state(value.clone(), context.world);
            }

            // If the symbol is not found in the world state, it's an undefined symbol.
            (
                "explicit (get ...) call required for symbol evaluation",
                *span,
            )
        }
        Expr::Spread(_) => (
            "Spread argument not allowed outside of call position (list context)",
            expr.span,
        ),
        _ => unreachable!("eval_invalid_expr called with valid expression type"),
    };
    Err(err_src!(
        Eval,
        msg,
        &context.source,
        span
    ))
}

// ===================================================================================================
// PUBLIC API: Expression Evaluation Interface
// ===================================================================================================

/// Evaluates a Sutra AST node in the given world, with output and options.
///
/// # Note
/// This is a low-level, internal function. Most users should use the higher-level `eval` API.
pub fn evaluate_ast_node(expr: &AstNode, context: &mut EvaluationContext) -> Result<(Value, World), SutraError> {
    if context.depth > context.max_depth {
        return Err(err_src!(
            Internal,
            "Recursion limit exceeded",
            &context.source,
            expr.span
        ));
    }

    match &*expr.value {
        // Complex expression types with dedicated handlers
        Expr::List(items, span) => evaluate_list(items, span, context),
        Expr::Quote(inner, _) => evaluate_quote(inner, context),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => evaluate_if(condition, then_branch, else_branch, context),
        Expr::Define {
            name, params, body, ..
        } => {
            // If params are not empty, it's a function/macro definition.
            if !params.required.is_empty() || params.rest.is_some() {
                let mut new_world = context.world.clone();
                let template = crate::macros::MacroTemplate::new(params.clone(), body.clone())?;
                new_world.macros = new_world
                    .macros
                    .with_user_macro(name.clone(), crate::macros::MacroDefinition::Template(template));
                Ok((Value::Nil, new_world))
            } else {
                // It's a variable definition.
                let (value, world) = evaluate_ast_node(body, context)?;
                let new_world = world;
                let path = crate::runtime::path::Path(vec![name.clone()]);
                new_world.state.set(&path, value.clone());
                Ok((value, new_world))
            }
        }

        // Literal value types
        Expr::Path(..) | Expr::String(..) | Expr::Number(..) | Expr::Bool(..) => {
            evaluate_literal_value(expr, context)
        }

        // Invalid expression types
        Expr::ParamList(..) | Expr::Symbol(..) | Expr::Spread(..) => evaluate_invalid_expr(expr, context),
    }
}

/// Public API: evaluates an expression with the given world, output, atom registry, and max depth.
pub fn evaluate(
    expr: &AstNode,
    world: &World,
    output: &mut dyn OutputSink,
    atom_registry: &AtomRegistry,
    source: Arc<NamedSource<String>>,
    max_depth: usize,
) -> Result<(Value, World), SutraError> {
    let mut context = EvaluationContext {
        world,
        output,
        atom_registry,
        source,
        max_depth,
        depth: 0,
    };
    evaluate_ast_node(expr, &mut context)
}

// ===================================================================================================
// INTERNAL HELPERS: Expression-Specific Evaluation
// ===================================================================================================

// --- Core Expression Handlers ---

/// Helper for evaluating Expr::List arms.
fn evaluate_list(
    items: &[AstNode],
    span: &crate::ast::Span,
    context: &mut EvaluationContext,
) -> Result<(Value, World), SutraError> {
    if items.is_empty() {
        return wrap_value_with_world_state(Value::List(vec![]), context.world);
    }

    // Extract symbol name using guard clause pattern
    let head = &items[0];
    let tail = &items[1..];
    let Expr::Symbol(symbol_name, _) = &*head.value else {
        return Err(err_src!(
            Eval,
            "first element must be a symbol naming a callable entity",
            &context.source,
            head.span
        ));
    };

    // Use direct atom resolution
    let flat_tail = flatten_spread_args(tail, context)?;
    context.call_atom(symbol_name, head, &flat_tail, span)
}

/// Helper for evaluating Expr::Quote arms.
fn evaluate_quote(
    inner: &AstNode,
    context: &mut EvaluationContext,
) -> Result<(Value, World), SutraError> {
    match &*inner.value {
        Expr::Symbol(s, _) => wrap_value_with_world_state(Value::String(s.clone()), context.world),
        Expr::List(exprs, _) => wrap_value_with_world_state(evaluate_quoted_list(exprs, context)?, context.world),
        Expr::Number(n, _) => wrap_value_with_world_state(Value::Number(*n), context.world),
        Expr::Bool(b, _) => wrap_value_with_world_state(Value::Bool(*b), context.world),
        Expr::String(s, _) => wrap_value_with_world_state(Value::String(s.clone()), context.world),
        Expr::Path(p, _) => wrap_value_with_world_state(Value::Path(p.clone()), context.world),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => evaluate_quoted_if(condition, then_branch, else_branch, context),
        Expr::Quote(_, _) => wrap_value_with_world_state(Value::Nil, context.world),
        Expr::Define { .. } => Err(err_ctx!(
            Eval,
            "Cannot quote define expressions",
            context.source.as_ref().name(),
            inner.span
        )),
        Expr::ParamList(_) => Err(err_src!(
            Eval,
            "Cannot evaluate parameter list (ParamList AST node) at runtime",
            &context.source,
            inner.span
        )),
        Expr::Spread(_) => Err(err_src!(
            Eval,
            "Spread argument not allowed inside quote",
            &context.source,
            inner.span
        )),
    }
}

/// Helper for evaluating Expr::If arms.
fn evaluate_if(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &AstNode,
    context: &mut EvaluationContext,
) -> Result<(Value, World), SutraError> {
    let (is_true, next_world) = evaluate_condition_as_bool(condition, context)?;
    let mut sub_context = EvaluationContext {
        world: &next_world,
        output: context.output,
        atom_registry: context.atom_registry,
        source: context.source.clone(),
        max_depth: context.max_depth,
        depth: context.depth + 1,
    };

    let branch = if is_true { then_branch } else { else_branch };
    evaluate_ast_node(branch, &mut sub_context)
}

// --- Quote Expression Helpers ---

/// Evaluates a single expression within a quote context.
fn evaluate_quoted_expr(expr: &AstNode, context: &EvaluationContext) -> Result<Value, SutraError> {
    match &*expr.value {
        Expr::Symbol(s, _) => Ok(Value::String(s.clone())),
        Expr::Number(n, _) => Ok(Value::Number(*n)),
        Expr::Bool(b, _) => Ok(Value::Bool(*b)),
        Expr::String(s, _) => Ok(Value::String(s.clone())),
        Expr::ParamList(_) => Err(err_src!(
            Eval,
            "Cannot evaluate parameter list (ParamList AST node) inside quote",
            &context.source,
            expr.span
        )),
        Expr::Spread(_) => Err(err_src!(
            Eval,
            "Spread argument not allowed inside quote",
            &context.source,
            expr.span
        )),
        _ => Ok(Value::Nil),
    }
}

/// Evaluates a quoted list by converting each element to a value.
fn evaluate_quoted_list(exprs: &[AstNode], context: &EvaluationContext) -> Result<Value, SutraError> {
    let vals: Result<Vec<_>, SutraError> =
        exprs.iter().map(|e| evaluate_quoted_expr(e, context)).collect();
    Ok(Value::List(vals?))
}

/// Evaluates a quoted if expression.
fn evaluate_quoted_if(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &AstNode,
    context: &mut EvaluationContext,
) -> Result<(Value, World), SutraError> {
    let (is_true, next_world) = evaluate_condition_as_bool(condition, context)?;
    let mut sub_context = EvaluationContext {
        world: &next_world,
        output: context.output,
        atom_registry: context.atom_registry,
        source: context.source.clone(),
        max_depth: context.max_depth,
        depth: context.depth + 1,
    };

    let branch = if is_true { then_branch } else { else_branch };
    evaluate_ast_node(branch, &mut sub_context)
}

// --- Argument Processing Helpers ---

/// Flattens spread arguments in function call arguments.
fn flatten_spread_args(
    tail: &[AstNode],
    context: &mut EvaluationContext,
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
        let (val, _) = evaluate_ast_node(expr, context)?;

        // Guard clause: ensure we have a list for spreading
        let Value::List(items) = val else {
            return Err(err_src!(
                TypeError,
                format!("spread argument must be a list: {}", &val),
                &context.source,
                expr.span
            ));
        };

        // Process list items without nesting
        for v in items {
            flat_tail.push(Spanned {
                value: Expr::from(v).into(), // FIX: wrap Expr in Arc via .into()
                span: arg.span,
            });
        }
    }

    Ok(flat_tail)
}
