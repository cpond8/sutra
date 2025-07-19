//! Sutra Runtime Evaluation Engine
//!
//! Transforms AST nodes into runtime values with world state consistency.
//!
//! ## CRITICAL: Atom Calling Conventions
//!
//! Atoms are `Pure`/`Stateful` (eager evaluation) or `SpecialForm` (unevaluated args).
//! **Safety**: Misclassifying atoms causes runtime failures. See `src/atoms.rs`.
//!
//! ## Error Handling
//!
//! All errors use `SutraError` via `err_msg!` or `err_ctx!` macros.
//! See `src/diagnostics.rs` for usage rules.
//!
//! Example:
//! ```rust
//! use sutra::err_msg;
//! let err = err_msg!(Eval, "Arity error");
//! assert!(matches!(err, sutra::SutraError::Eval { .. }));
//! ```

// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
use std::{collections::HashMap, rc::Rc, sync::Arc};

use miette::NamedSource;

use crate::{
    ast::{value::Lambda, AstNode, Expr, Spanned},
    atoms::{helpers::AtomResult, special_forms, Atom},
    err_ctx, err_src,
    runtime::world::{AtomExecutionContext, World},
    AtomRegistry, ParamList, Path, SharedOutput, Span, SutraError, Value,
};

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
    pub lexical_env: Vec<HashMap<String, Value>>,
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
    pub fn set_lexical_var(&mut self, name: &str, value: Value) {
        if let Some(frame) = self.lexical_env.last_mut() {
            frame.insert(name.to_string(), value);
        }
    }

    /// Get a variable from the lexical environment stack (innermost to outermost)
    pub fn get_lexical_var(&self, name: &str) -> Option<&Value> {
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
        span: &Span,
    ) -> AtomResult {
        // Step 1: Look up atom in registry
        let atom = self.lookup_atom(symbol_name, head)?;

        // Step 2: Validate atom classification (debug only)
        self.validate_atom_classification(symbol_name, &atom);

        // Step 3: Dispatch to appropriate atom type
        self.dispatch_atom(atom, args, span)
    }

    fn lookup_atom(&self, symbol_name: &str, head: &AstNode) -> Result<Atom, SutraError> {
        self.atom_registry.get(symbol_name).cloned().ok_or_else(|| {
            err_src!(
                Eval,
                format!("Undefined symbol: '{}'", symbol_name),
                &self.source,
                head.span
            )
        })
    }

    fn validate_atom_classification(&self, symbol_name: &str, atom: &Atom) {
        #[cfg(debug_assertions)]
        {
            const SPECIAL_FORM_NAMES: &[&str] = &[
                "define",
                "do",
                "error",
                "apply",
                "assert",
                "assert-eq",
                "test/echo",
                "test/borrow_stress",
                "register-test!",
            ];

            if SPECIAL_FORM_NAMES.contains(&symbol_name) {
                assert!(
                    matches!(atom, Atom::SpecialForm(_)),
                    "CRITICAL: Atom '{symbol_name}' is a special form and MUST be registered as Atom::SpecialForm."
                );
            }
        }
    }

    fn dispatch_atom(&mut self, atom: Atom, args: &[AstNode], span: &Span) -> AtomResult {
        match atom {
            // Special forms control their own evaluation
            Atom::SpecialForm(special_form_fn) => special_form_fn(args, self, span),

            // Eagerly evaluated atoms
            Atom::Stateful(stateful_fn) => self.call_eager_atom_with_state(stateful_fn, args),
            Atom::Pure(pure_fn) => self.call_eager_atom_without_state(pure_fn, args),
        }
    }

    fn call_eager_atom_with_state(
        &mut self,
        stateful_fn: fn(&[Value], &mut AtomExecutionContext) -> Result<Value, SutraError>,
        args: &[AstNode],
    ) -> AtomResult {
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

    fn call_eager_atom_without_state(
        &mut self,
        pure_fn: fn(&[Value]) -> Result<Value, SutraError>,
        args: &[AstNode],
    ) -> AtomResult {
        let (values, world_after_args) = evaluate_eager_args(args, self)?;
        let result = pure_fn(&values)?;
        Ok((result, world_after_args))
    }
}

// ===================================================================================================
// PUBLIC API: Main Evaluation Interface
// ===================================================================================================

/// Evaluates a Sutra expression with world state and context.
///
/// **CRITICAL**: Macros must be expanded before evaluation. Use `ExecutionPipeline::execute`
/// for most use cases to ensure proper validation and macro expansion.
///
/// # Safety Requirements
/// - All macros expanded before calling
/// - Atom registry consistent with validation
/// - World state properly initialized
pub fn evaluate(
    expr: &AstNode,
    world: &World,
    output: SharedOutput,
    atom_registry: &AtomRegistry,
    source: Arc<NamedSource<String>>,
    max_depth: usize,
) -> AtomResult {
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

/// Evaluates a single AST node with recursion depth tracking.
///
/// **CRITICAL**: Macros must be expanded before evaluation.
/// This is a low-level function; prefer `ExecutionPipeline::execute` for most use cases.
pub(crate) fn evaluate_ast_node(expr: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    // Step 1: Check recursion limit
    if context.depth > context.max_depth {
        return Err(err_src!(
            Internal,
            "Recursion limit exceeded",
            &context.source,
            expr.span
        ));
    }

    // Step 2: Handle each expression type based on its structure
    match &*expr.value {
        // Lists are function calls - delegate to list evaluator
        Expr::List(items, span) => evaluate_list(items, span, context),

        // Quotes preserve their content - delegate to quote evaluator
        Expr::Quote(inner, _) => evaluate_quote(inner, context),

        // Define expressions create bindings in the world
        Expr::Define {
            name, params, body, ..
        } => {
            // Function definition: create a lambda and store it
            if !params.required.is_empty() || params.rest.is_some() {
                let captured_env = capture_lexical_env(&context.lexical_env);
                let lambda = Value::Lambda(Rc::new(Lambda {
                    params: params.clone(),
                    body: body.clone(),
                    captured_env,
                }));
                let path = Path(vec![name.clone()]);
                let new_world = context.world.set(&path, lambda);
                Ok((Value::Nil, new_world))
            } else {
                // Variable definition: evaluate the body and store the result
                let (value, world) = evaluate_ast_node(body, context)?;
                let path = Path(vec![name.clone()]);
                let new_world = world.set(&path, value.clone());
                Ok((value, new_world))
            }
        }

        // Literal values can be evaluated directly
        Expr::Path(..) | Expr::String(..) | Expr::Number(..) | Expr::Bool(..) => {
            evaluate_literal_value(expr, context)
        }

        // Invalid expressions that cannot be evaluated at runtime
        Expr::ParamList(..) | Expr::Symbol(..) | Expr::Spread(..) => {
            evaluate_invalid_expr(expr, context)
        }

        // If expressions must be handled as special forms, not direct evaluation
        Expr::If { .. } => Err(err_src!(
            Eval,
            "If expressions should be evaluated as special forms, not as AST nodes",
            &context.source,
            expr.span
        )),
    }
}

// ===================================================================================================
// EXPRESSION EVALUATION: Core Expression Types
// ===================================================================================================

/// Evaluates list expressions (function calls)
fn evaluate_list(items: &[AstNode], span: &Span, context: &mut EvaluationContext) -> AtomResult {
    // Empty list returns empty list
    if items.is_empty() {
        return wrap_value_with_world_state(Value::List(vec![]), context.world);
    }

    // Extract head symbol
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

    // Flatten arguments and handle special forms
    let flat_tail = flatten_spread_args(tail, context)?;
    if symbol_name == "define" {
        return handle_define_special_form(&flat_tail, span, context, head);
    }

    // Evaluate arguments once
    let (arg_values, world_after_args) = evaluate_eager_args(&flat_tail, context)?;

    // Create lambda context once
    let mut lambda_context = EvaluationContext {
        world: &world_after_args,
        output: context.output.clone(),
        atom_registry: context.atom_registry,
        source: context.source.clone(),
        max_depth: context.max_depth,
        depth: context.depth + 1,
        lexical_env: context.lexical_env.clone(),
    };

    // Try lexical environment first
    if let Some(Value::Lambda(lambda)) = context.get_lexical_var(symbol_name) {
        return special_forms::call_lambda(lambda, &arg_values, &mut lambda_context);
    }

    // Try world state - separate lookup and pattern match
    let world_path = Path(vec![symbol_name.to_string()]);
    let world_value = context.world.state.get(&world_path);
    if let Some(Value::Lambda(lambda)) = world_value {
        return special_forms::call_lambda(lambda, &arg_values, &mut lambda_context);
    }

    // Fall back to atom registry
    context.call_atom(symbol_name, head, &flat_tail, span)
}

/// Evaluates quote expressions (preserve content)
fn evaluate_quote(inner: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    match &*inner.value {
        Expr::Symbol(s, _) => wrap_value_with_world_state(Value::String(s.clone()), context.world),
        Expr::List(exprs, _) => {
            wrap_value_with_world_state(evaluate_quoted_list(exprs, context)?, context.world)
        }
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
            &context.source,
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

/// Evaluates define expressions (create bindings)
fn handle_define_special_form(
    flat_tail: &[AstNode],
    span: &Span,
    context: &mut EvaluationContext,
    head: &AstNode,
) -> AtomResult {
    // Step 1: Validate argument count
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

    // Step 2: Parse definition and create AST node
    let define_ast = parse_define_definition(name_expr, value_expr, span, context)?;

    // Step 3: Evaluate the definition
    evaluate_ast_node(&define_ast, context)
}

fn parse_define_definition(
    name_expr: &AstNode,
    value_expr: &AstNode,
    span: &Span,
    context: &mut EvaluationContext,
) -> Result<AstNode, SutraError> {
    // Extract name and parameters based on definition type
    let (name, params) = match &*name_expr.value {
        // Function definition: (define (name param1 param2...) body)
        Expr::ParamList(param_list) => {
            let function_name = param_list.required.first().cloned().ok_or_else(|| {
                err_src!(
                    Eval,
                    "define: function name missing in parameter list",
                    &context.source,
                    name_expr.span
                )
            })?;

            let function_parameters = ParamList {
                required: param_list.required[1..].to_vec(),
                rest: param_list.rest.clone(),
                span: param_list.span,
            };

            (function_name, function_parameters)
        }

        // Variable definition: (define name value)
        Expr::Symbol(variable_name, _) => {
            let empty_parameters = ParamList {
                required: vec![],
                rest: None,
                span: *span,
            };
            (variable_name.clone(), empty_parameters)
        }

        // Invalid: first argument must be symbol or parameter list
        _ => {
            return Err(err_src!(
                Eval,
                "define: first argument must be a symbol or parameter list",
                &context.source,
                name_expr.span
            ));
        }
    };

    // Create the define AST node
    Ok(AstNode {
        value: Arc::new(Expr::Define {
            name,
            params,
            body: Box::new(value_expr.clone()),
            span: *span,
        }),
        span: *span,
    })
}

/// Evaluates literal value expressions (Path, String, Number, Bool)
fn evaluate_literal_value(expr: &AstNode, context: &mut EvaluationContext) -> AtomResult {
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

/// Evaluates invalid expressions that cannot be evaluated at runtime
fn evaluate_invalid_expr(expr: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    match &*expr.value {
        // Parameter lists cannot be evaluated at runtime
        Expr::ParamList(_) => Err(err_src!(
            Eval,
            "Cannot evaluate parameter list (ParamList AST node) at runtime",
            &context.source,
            expr.span
        )),

        // Symbols need resolution - may succeed or fail
        Expr::Symbol(symbol_name, span) => resolve_symbol(symbol_name, span, context),

        // Spread arguments are only valid in function calls
        Expr::Spread(_) => Err(err_src!(
            Eval,
            "Spread argument not allowed outside of call position (list context)",
            &context.source,
            expr.span
        )),

        // This should never happen with valid expressions
        _ => unreachable!("evaluate_invalid_expr called with valid expression type"),
    }
}

fn resolve_symbol(symbol_name: &str, span: &Span, context: &mut EvaluationContext) -> AtomResult {
    // Try to resolve symbol in precedence order: lexical → atom → world → undefined

    // Step 1: Check lexical environment (let/lambda bindings)
    if let Some(value) = context.get_lexical_var(symbol_name) {
        return wrap_value_with_world_state(value.clone(), context.world);
    }

    // Step 2: Check if symbol is an atom (must be called, not evaluated)
    if context.atom_registry.has(symbol_name) {
        return Err(err_src!(
            Eval,
            format!("'{symbol_name}' is an atom and must be called with arguments (e.g., ({symbol_name} ...))"),
            &context.source,
            *span
        ));
    }

    // Step 3: Check world state (global variables)
    let world_path = Path(vec![symbol_name.to_string()]);
    if let Some(value) = context.world.state.get(&world_path) {
        return wrap_value_with_world_state(value.clone(), context.world);
    }

    // Step 4: Symbol is undefined
    Err(err_src!(
        Eval,
        format!("undefined symbol: '{symbol_name}'"),
        &context.source,
        *span
    ))
}

// ===================================================================================================
// QUOTE EVALUATION: Special Quote Handling
// ===================================================================================================

/// Evaluates a single expression within a quote context
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

/// Evaluates a quoted list by converting each element to a value
fn evaluate_quoted_list(
    exprs: &[AstNode],
    context: &EvaluationContext,
) -> Result<Value, SutraError> {
    let vals: Result<Vec<_>, SutraError> = exprs
        .iter()
        .map(|e| evaluate_quoted_expr(e, context))
        .collect();
    Ok(Value::List(vals?))
}

/// Evaluates a quoted if expression
fn evaluate_quoted_if(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &AstNode,
    context: &mut EvaluationContext,
) -> AtomResult {
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

// ===================================================================================================
// ARGUMENT PROCESSING: Function Call Support
// ===================================================================================================

/// Evaluates arguments for eager atoms (Pure, Stateful), threading world state
fn evaluate_eager_args(
    args: &[AstNode],
    context: &mut EvaluationContext,
) -> Result<(Vec<Value>, World), SutraError> {
    // Step 1: Evaluate each argument with proper context
    let mut values = Vec::with_capacity(args.len());
    let mut current_world = context.world.clone();

    for arg in args {
        let (value, next_world) = evaluate_single_argument(arg, &current_world, context)?;
        values.push(value);
        current_world = next_world;
    }

    Ok((values, current_world))
}

fn evaluate_single_argument(
    arg: &AstNode,
    current_world: &World,
    context: &mut EvaluationContext,
) -> AtomResult {
    let mut arg_context = create_argument_context(current_world, context);
    evaluate_ast_node(arg, &mut arg_context)
}

fn create_argument_context<'a>(
    world: &'a World,
    context: &'a mut EvaluationContext,
) -> EvaluationContext<'a> {
    EvaluationContext {
        world,
        output: context.output.clone(),
        atom_registry: context.atom_registry,
        source: context.source.clone(),
        max_depth: context.max_depth,
        depth: context.next_depth(),
        lexical_env: context.lexical_env.clone(),
    }
}

/// Flattens spread arguments in function call arguments
fn flatten_spread_args(
    tail: &[AstNode],
    context: &mut EvaluationContext,
) -> Result<Vec<AstNode>, SutraError> {
    // Step 1: Process each argument
    let mut flat_tail = Vec::new();

    for arg in tail {
        let processed_args = process_single_argument(arg, context)?;
        flat_tail.extend(processed_args);
    }

    Ok(flat_tail)
}

fn process_single_argument(
    arg: &AstNode,
    context: &mut EvaluationContext,
) -> Result<Vec<AstNode>, SutraError> {
    // Handle non-spread expressions
    let Expr::Spread(expr) = &*arg.value else {
        return Ok(vec![arg.clone()]);
    };

    // Evaluate spread expression
    let (spread_value, _) = evaluate_ast_node(expr, context)?;

    // Validate and extract list items
    let list_items = extract_list_items(spread_value, expr, context)?;

    // Convert list items to AST nodes
    Ok(convert_values_to_ast_nodes(list_items, arg.span))
}

fn extract_list_items(
    value: Value,
    expr: &AstNode,
    context: &mut EvaluationContext,
) -> Result<Vec<Value>, SutraError> {
    let Value::List(items) = value else {
        return Err(err_src!(
            TypeError,
            format!("spread argument must be a list: {}", &value),
            &context.source,
            expr.span
        ));
    };
    Ok(items)
}

fn convert_values_to_ast_nodes(values: Vec<Value>, span: Span) -> Vec<AstNode> {
    values
        .into_iter()
        .map(|value| Spanned {
            value: Expr::from(value).into(),
            span,
        })
        .collect()
}

// ===================================================================================================
// UTILITY FUNCTIONS: Common Patterns
// ===================================================================================================

/// Wraps a value with the current world state in the standard (Value, World) result format
fn wrap_value_with_world_state(value: Value, world: &World) -> AtomResult {
    Ok((value, world.clone()))
}

/// Helper to evaluate a conditional expression and return a boolean result
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

/// Captures the current lexical environment for lambda closures
fn capture_lexical_env(lexical_env: &[HashMap<String, Value>]) -> HashMap<String, Value> {
    let mut captured_env = HashMap::new();
    for frame in lexical_env {
        for (key, value) in frame {
            captured_env.insert(key.clone(), value.clone());
        }
    }
    captured_env
}
