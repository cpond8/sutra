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
use crate::atoms::AtomRegistry;
use crate::diagnostics::SutraError;
use crate::runtime::world::{AtomExecutionContext, World};
use crate::err_ctx;
use crate::err_src;
use miette::NamedSource;
use std::sync::Arc;
use std::collections::HashMap;
use std::rc::Rc;
use crate::atoms::SharedOutput;

// ===================================================================================================
// CORE DATA STRUCTURES: Evaluation Context
// ===================================================================================================

/// The context for a single evaluation, passed to atoms and all evaluation functions.
pub struct EvaluationContext<'a> {
    pub world: &'a World,
    pub output: SharedOutput,
    pub atom_registry: &'a AtomRegistry,
    pub source: Arc<NamedSource<String>>,
    pub max_depth: usize,
    pub depth: usize,
    /// Stack of lexical environments (for let/lambda scoping)
    pub lexical_env: Vec<HashMap<String, crate::ast::value::Value>>,
}

impl<'a> EvaluationContext<'a> {
    /// Helper to increment depth for recursive calls.
    pub fn next_depth(&self) -> usize {
        self.depth + 1
    }

    /// Clone the context with a new lexical frame (for let/lambda)
    pub fn clone_with_new_lexical_frame(&self) -> Self {
        let mut new = EvaluationContext {
            world: self.world,
            output: self.output.clone(),
            atom_registry: self.atom_registry,
            source: self.source.clone(),
            max_depth: self.max_depth,
            depth: self.depth,
            lexical_env: self.lexical_env.clone(),
        };
        new.lexical_env.push(HashMap::new());
        new
    }

    /// Set a variable in the current lexical frame
    pub fn set_lexical_var(&mut self, name: &str, value: crate::ast::value::Value) {
        if let Some(frame) = self.lexical_env.last_mut() {
            frame.insert(name.to_string(), value);
        }
    }

    /// Get a variable from the lexical environment stack (innermost to outermost)
    pub fn get_lexical_var(&self, name: &str) -> Option<&crate::ast::value::Value> {
        for frame in self.lexical_env.iter().rev() {
            if let Some(val) = frame.get(name) {
                return Some(val);
            }
        }
        None
    }
}

impl<'a> EvaluationContext<'a> {
    /// Looks up and invokes an atom by name, handling errors for missing atoms.
    /// **CRITICAL**: This function does NOT handle macro expansion. Macros must be
    /// expanded before evaluation according to the strict layering principle.
    pub(crate) fn call_atom(
        &mut self,
        symbol_name: &str,
        head: &AstNode,
        args: &[AstNode],
        span: &crate::ast::Span,
    ) -> Result<(Value, World), SutraError> {
        // Look up atom in registry
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
                "define",
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
                    "CRITICAL: Atom '{symbol_name}' is a special form and MUST be registered as Atom::SpecialForm."
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
                        output: self.output.clone(),
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
                output: context.output.clone(),
                atom_registry: context.atom_registry,
                source: context.source.clone(),
                max_depth: context.max_depth,
                depth: next_depth,
                lexical_env: context.lexical_env.clone(),
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
pub fn evaluate_condition_as_bool(
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
///
/// ## Symbol Resolution Precedence
///
/// When evaluating a bare symbol (e.g., `x`), the following precedence is used:
///
/// 1. **Lexical Environment**: Check for variables bound by `let` or `lambda`
/// 2. **Atom Registry**: If symbol matches an atom, return error (atoms must be called)
/// 3. **World State**: Check if symbol is a path in world state (e.g., `x` → `world.state.x`)
/// 4. **Undefined**: Return "undefined symbol" error
///
/// ## Error Messages
///
/// - **Atom Reference**: `"atoms cannot be evaluated directly - they must be called"`
/// - **Undefined Symbol**: `"undefined symbol"`
///
/// ## Examples
///
/// ```sutra
/// (let x 42)     ; x resolves to 42 (lexical)
/// (+ 1 2)        ; + resolves to atom (callable)
/// x              ; Error: atoms cannot be evaluated directly
/// undefined-var  ; Error: undefined symbol
/// ```
fn evaluate_invalid_expr(
    expr: &AstNode,
    context: &mut EvaluationContext,
) -> Result<(Value, World), SutraError> {
    let (msg, span) = match &*expr.value {
        Expr::ParamList(_) => (
            "Cannot evaluate parameter list (ParamList AST node) at runtime".to_string(),
            expr.span,
        ),
        Expr::Symbol(s, span) => {
            // Look up in lexical environment first
            if let Some(val) = context.get_lexical_var(s) {
                return wrap_value_with_world_state(val.clone(), context.world);
            }
            // Check if it's an atom in the registry
            if context.atom_registry.has(s) {
                (
                    format!("'{s}' is an atom and must be called with arguments (e.g., ({s} ...))"),
                    *span,
                )
            } else {
                // Attempt to resolve the symbol as a path in the world state.
                let path = crate::runtime::world::Path(vec![s.clone()]);
                if let Some(value) = context.world.state.get(&path) {
                    return wrap_value_with_world_state(value.clone(), context.world);
                }
                // If the symbol is not found anywhere, it's an undefined symbol.
                (
                    format!("undefined symbol: '{s}'"),
                    *span,
                )
            }
        }
        Expr::Spread(_) => (
            "Spread argument not allowed outside of call position (list context)".to_string(),
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
/// **CRITICAL**: This function assumes all macros have been expanded before evaluation.
/// Macro expansion must happen in a separate phase according to the strict layering principle.
pub(crate) fn evaluate_ast_node(expr: &AstNode, context: &mut EvaluationContext) -> Result<(Value, World), SutraError> {
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
        Expr::If { .. } => {
            // If expressions should be handled as special forms, not as AST nodes
            // This should not be reached in normal evaluation
            Err(err_src!(
                Eval,
                "If expressions should be evaluated as special forms, not as AST nodes",
                &context.source,
                expr.span
            ))
        },
        Expr::Define {
            name, params, body, ..
        } => {
                        // If params are not empty, it's a function definition.
            if !params.required.is_empty() || params.rest.is_some() {
                // Create a Lambda value for function definitions
                // Capture the current lexical environment
                let mut captured_env = std::collections::HashMap::new();
                for frame in &context.lexical_env {
                    for (key, value) in frame {
                        captured_env.insert(key.clone(), value.clone());
                    }
                }
                let lambda = Value::Lambda(Rc::new(crate::ast::value::Lambda {
                    params: params.clone(),
                    body: body.clone(),
                    captured_env,
                }));

                // Store in world state for global access
                let path = crate::runtime::world::Path(vec![name.clone()]);
                let new_world = context.world.set(&path, lambda);

                Ok((Value::Nil, new_world))
            } else {
                // It's a variable definition.
                let (value, world) = evaluate_ast_node(body, context)?;
                let path = crate::runtime::world::Path(vec![name.clone()]);
                let new_world = world.set(&path, value.clone());
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
/// **CRITICAL**: This function assumes all macros have been expanded before evaluation.
/// Macro expansion must happen in a separate phase according to the strict layering principle.
///
/// # Usage
///
/// This function is intended for use by the `ExecutionPipeline` only. Direct usage
/// bypasses the pipeline's validation and macro expansion, which may lead to
/// inconsistent behavior. Use `ExecutionPipeline::execute` instead for most use cases.
///
/// # Safety
///
/// - All macros must be expanded before calling this function
/// - The atom registry must be consistent with the one used for validation
/// - World state must be properly initialized
pub fn evaluate(
    expr: &AstNode,
    world: &World,
    output: SharedOutput,
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
        lexical_env: vec![HashMap::new()],
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

        // Use direct atom resolution for all symbols
    let flat_tail = flatten_spread_args(tail, context)?;

    // Handle define as a special form
    if symbol_name == "define" {
        // For define, we need to construct the proper Expr::Define structure
        if flat_tail.len() != 2 {
            return Err(err_src!(
                Eval,
                "define expects exactly 2 arguments: (define name value) or (define (name params...) body)",
                &context.source,
                head.span
            ));
        }

        let name_expr = &flat_tail[0];
        let value_expr = &flat_tail[1];

        // Check if it's a function definition: (define (name params...) body)
        if let Expr::ParamList(param_list) = &*name_expr.value {
            // Function definition
            let name = param_list.required.first().cloned().ok_or_else(|| {
                err_src!(
                    Eval,
                    "define: function name missing in parameter list",
                    &context.source,
                    name_expr.span
                )
            })?;

            let actual_params = crate::ast::ParamList {
                required: param_list.required[1..].to_vec(),
                rest: param_list.rest.clone(),
                span: param_list.span,
            };

            let define_expr = AstNode {
                value: std::sync::Arc::new(Expr::Define {
                    name,
                    params: actual_params,
                    body: Box::new(value_expr.clone()),
                    span: *span,
                }),
                span: *span,
            };

            return evaluate_ast_node(&define_expr, context);
        } else {
            // Variable definition: (define name value)
            let name = match &*name_expr.value {
                Expr::Symbol(s, _) => s.clone(),
                _ => {
                    return Err(err_src!(
                        Eval,
                        "define: first argument must be a symbol or parameter list",
                        &context.source,
                        name_expr.span
                    ));
                }
            };

            let define_expr = AstNode {
                value: std::sync::Arc::new(Expr::Define {
                    name,
                    params: crate::ast::ParamList {
                        required: vec![],
                        rest: None,
                        span: *span,
                    },
                    body: Box::new(value_expr.clone()),
                    span: *span,
                }),
                span: *span,
            };

            return evaluate_ast_node(&define_expr, context);
        }
    }

    // Check if it's a lambda in the lexical environment
    if let Some(Value::Lambda(lambda_rc)) = context.get_lexical_var(symbol_name) {
        let lambda = lambda_rc.clone();
        // Evaluate arguments eagerly
        let (arg_values, world_after_args) = evaluate_eager_args(&flat_tail, context)?;
        let mut lambda_context = EvaluationContext {
            world: &world_after_args,
            output: context.output.clone(),
            atom_registry: context.atom_registry,
            source: context.source.clone(),
            max_depth: context.max_depth,
            depth: context.depth + 1,
            lexical_env: context.lexical_env.clone(),
        };
        return crate::atoms::special_forms::call_lambda(&lambda, &arg_values, &mut lambda_context);
    }

    // Check if it's a function in world state
    let world_path = crate::runtime::world::Path(vec![symbol_name.clone()]);
    if let Some(lambda_value) = context.world.state.get(&world_path) {
        if let Value::Lambda(lambda) = lambda_value {
            // Evaluate arguments eagerly
            let (arg_values, world_after_args) = evaluate_eager_args(&flat_tail, context)?;
            let mut lambda_context = EvaluationContext {
                world: &world_after_args,
                output: context.output.clone(),
                atom_registry: context.atom_registry,
                source: context.source.clone(),
                max_depth: context.max_depth,
                depth: context.depth + 1,
                lexical_env: context.lexical_env.clone(),
            };
            return crate::atoms::special_forms::call_lambda(&lambda, &arg_values, &mut lambda_context);
        }
    }

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
        output: context.output.clone(),
        atom_registry: context.atom_registry,
        source: context.source.clone(),
        max_depth: context.max_depth,
        depth: context.depth + 1,
        lexical_env: context.lexical_env.clone(),
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
                value: Expr::from(v).into(),
                span: arg.span,
            });
        }
    }

    Ok(flat_tail)
}
