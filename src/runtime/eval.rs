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
//! All errors use miette-native `SutraError` variants directly.
//! See `src/errors.rs` for error types and usage rules.
//!
//! Example: let err = SutraError::RuntimeGeneral { message: "Arity error".to_string(), ... };
//! assert!(matches!(err, sutra::SutraError::Eval { .. }));
//! ```

// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
use std::{collections::HashMap, rc::Rc, sync::Arc};

use miette::{NamedSource, SourceSpan};

// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::atoms::{helpers::AtomResult, special_forms, Atom, EagerAtomFn};
use crate::errors;

// ===================================================================================================
// CORE DATA STRUCTURES: Evaluation Context
// ===================================================================================================

/// The context for a single evaluation, passed to atoms and all evaluation functions.
pub struct EvaluationContext<'a> {
    pub world: CanonicalWorld,
    pub output: SharedOutput,
    pub atom_registry: &'a AtomRegistry,
    pub source: Arc<NamedSource<String>>,
    pub max_depth: usize,
    pub depth: usize,
    /// Stack of lexical environments (for let/lambda scoping)
    pub lexical_env: Vec<HashMap<String, Value>>,
    pub test_file: Option<String>,
    pub test_name: Option<String>,
}

impl<'a> EvaluationContext<'a> {
    /// Helper to increment depth for recursive calls.
    pub fn next_depth(&self) -> usize {
        self.depth + 1
    }

    /// Clone the context with a new lexical frame (for let/lambda)
    pub fn clone_with_new_lexical_frame(&self) -> Self {
        let mut new = EvaluationContext {
            world: Rc::clone(&self.world),
            output: self.output.clone(),
            atom_registry: self.atom_registry,
            source: self.source.clone(),
            max_depth: self.max_depth,
            depth: self.depth,
            lexical_env: self.lexical_env.clone(),
            test_file: self.test_file.clone(),
            test_name: self.test_name.clone(),
        };
        new.lexical_env.push(HashMap::new());
        new
    }

    /// Set a variable in the current lexical frame
    pub fn set_lexical_var(&mut self, name: &str, value: Value) {
        if let Some(frame) = self.lexical_env.last_mut() {
            frame.insert(name.to_string(), value.clone());
        }
    }

    /// Get a variable from the lexical environment stack (innermost to outermost)
    pub fn get_lexical_var(&self, name: &str) -> Option<&Value> {
        for (_i, frame) in self.lexical_env.iter().rev().enumerate() {
            if let Some(val) = frame.get(name) {
                return Some(val);
            }
        }
        None
    }

    /// Print the current lexical environment stack for debugging
    pub fn print_lexical_env(&self) {
    }
}

impl<'a> EvaluationContext<'a> {
    /// Extract the current file name for error construction
    pub fn current_file(&self) -> String {
        self.source.name().to_string()
    }

    /// Extract the current source code for error construction
    pub fn current_source(&self) -> String {
        self.source.inner().clone()
    }

    /// Extract span information for a given AstNode
    pub fn span_for_node(&self, node: &AstNode) -> SourceSpan {
        to_source_span(node.span)
    }

    /// Extract span information for a given Span
    pub fn span_for_span(&self, span: Span) -> SourceSpan {
        to_source_span(span)
    }
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
        self.atom_registry
            .get(symbol_name)
            .cloned()
            .ok_or_else(|| {
                let mut err = errors::runtime_undefined_symbol(
                    symbol_name,
                    self.current_file(),
                    self.current_source(),
                    self.span_for_node(head)
                ).with_suggestion(format!("Define '{}' before using it", symbol_name));

                if let (Some(ref tf), Some(ref tn)) = (&self.test_file, &self.test_name) {
                    err = err.with_test_context(tf.clone(), tn.clone());
                }
                err
            })
    }

    fn validate_atom_classification(&self, symbol_name: &str, atom: &Atom) {
        #[cfg(debug_assertions)]
        {
            const SPECIAL_FORM_NAMES: &[&str] = &[
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
                    matches!(atom, Atom::Lazy(_)),
                    "CRITICAL: Atom '{symbol_name}' is a special form and MUST be registered as Atom::Lazy."
                );
            }
        }
    }

    fn dispatch_atom(&mut self, atom: Atom, args: &[AstNode], span: &Span) -> AtomResult {
        match atom {
            // Lazily evaluated atoms (formerly special forms) control their own evaluation
            Atom::Lazy(lazy_fn) => lazy_fn(args, self, span),

            // Eagerly evaluated atoms now use a single calling convention
            Atom::Eager(eager_fn) => self.call_eager_atom(eager_fn, args, span),
        }
    }

    fn call_eager_atom(
        &mut self,
        eager_fn: EagerAtomFn,
        args: &[AstNode],
        _parent_span: &Span,
    ) -> AtomResult {
        // Eagerly evaluate arguments. The world state is mutated directly.
        let evaluated_args = evaluate_eager_args(args, self)?;

        // Invoke the eager atom. It receives the current context and can mutate the world.
        eager_fn(&evaluated_args, self)
    }
}

// ===================================================================================================
// PUBLIC API: Main Evaluation Interface
// ===================================================================================================

pub struct EvaluationContextBuilder<'a> {
    source: Arc<NamedSource<String>>,
    world: CanonicalWorld,
    output: SharedOutput,
    atom_registry: &'a AtomRegistry,
    max_depth: usize,
    test_file: Option<String>,
    test_name: Option<String>,
}

impl<'a> EvaluationContextBuilder<'a> {
    pub fn new(source: Arc<NamedSource<String>>, world: CanonicalWorld, output: SharedOutput, atom_registry: &'a AtomRegistry) -> Self {
        Self {
            source,
            world,
            output,
            atom_registry,
            max_depth: 1000,
            test_file: None,
            test_name: None,
        }
    }
    pub fn with_test_file(mut self, test_file: Option<String>) -> Self { self.test_file = test_file; self }
    pub fn with_test_name(mut self, test_name: Option<String>) -> Self { self.test_name = test_name; self }
    pub fn with_max_depth(mut self, max_depth: usize) -> Self { self.max_depth = max_depth; self }
    pub fn build(self) -> EvaluationContext<'a> {
        let mut global_env = HashMap::new();
        global_env.insert("nil".to_string(), Value::Nil);
        EvaluationContext {
            world: self.world,
            output: self.output,
            atom_registry: self.atom_registry,
            source: self.source,
            max_depth: self.max_depth,
            depth: 0,
            lexical_env: vec![global_env],
            test_file: self.test_file,
            test_name: self.test_name,
        }
    }
}

// Update the main evaluate() function to use the builder
pub fn evaluate(
    expr: &AstNode,
    world: CanonicalWorld,
    output: SharedOutput,
    atom_registry: &AtomRegistry,
    source: Arc<NamedSource<String>>,
    max_depth: usize,
    test_file: Option<String>,
    test_name: Option<String>,
) -> AtomResult {
    let mut context = EvaluationContextBuilder::new(source, world, output, atom_registry)
        .with_max_depth(max_depth)
        .with_test_file(test_file)
        .with_test_name(test_name)
        .build();
    evaluate_ast_node(expr, &mut context)
}

/// Evaluates a single AST node with recursion depth tracking.
///
/// **CRITICAL**: Macros must be expanded before evaluation.
/// This is a low-level function; prefer `ExecutionPipeline::execute` for most use cases.
pub(crate) fn evaluate_ast_node(expr: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    // Step 1: Check recursion limit
    if context.depth > context.max_depth {
        let mut err = errors::runtime_general(
            "Recursion limit exceeded",
            context.current_file(),
            context.current_source(),
            context.span_for_node(expr)
        ).with_suggestion("Reduce recursion depth or increase the limit");

        if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
            err = err.with_test_context(tf.clone(), tn.clone());
        }
        return Err(err);
    }

    // Step 2: Handle each expression type based on its structure
    match &*expr.value {
        // Lists are function calls - delegate to list evaluator
        Expr::List(items, span) => evaluate_list(items, span, context),

        // Quotes preserve their content - delegate to quote evaluator
        Expr::Quote(inner, _) => evaluate_quote(inner, context),

        // Literal values can be evaluated directly
        Expr::Path(..) | Expr::String(..) | Expr::Number(..) | Expr::Bool(..) => {
            evaluate_literal_value(expr, context)
        }

        // Invalid expressions that cannot be evaluated at runtime
        Expr::ParamList(..) | Expr::Symbol(..) | Expr::Spread(..) => {
            evaluate_invalid_expr(expr, context)
        }

        // If expressions must be handled as special forms, not direct evaluation
        Expr::If { .. } => {
            let mut err = errors::runtime_general(
                "If expressions should be evaluated as special forms, not as AST nodes",
                context.current_file(),
                context.current_source(),
                context.span_for_node(expr)
            ).with_suggestion("Use the 'if' special form instead");

            if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                err = err.with_test_context(tf.clone(), tn.clone());
            }
            Err(err)
        }
    }
}

// ===================================================================================================
// EXPRESSION EVALUATION: Core Expression Types
// ===================================================================================================

/// Evaluates list expressions (function calls)
fn evaluate_list(items: &[AstNode], span: &Span, context: &mut EvaluationContext) -> AtomResult {
    // Step 1: Early validation - check for empty list
    if items.is_empty() {
        return Ok(Value::List(vec![]));
    }

    // Step 2: Evaluate the head (first element) as an expression
    let head = &items[0];
    let tail = &items[1..];
    // If the head is a symbol, use resolve_callable as before
    if let Expr::Symbol(symbol_name, _) = &*head.value {
        return resolve_callable(symbol_name, head, tail, span, context);
    }
    // Otherwise, evaluate the head as an expression
    let head_val = evaluate_ast_node(head, context)?;

    match head_val {
        Value::Lambda(ref lambda) => {
            // Eagerly evaluate arguments for lambda call, mutating world state.
            let arg_values = evaluate_eager_args(tail, context)?;
            let mut lambda_context = context.clone_with_new_lexical_frame();
            lambda_context.depth += 1;
            special_forms::call_lambda(lambda, &arg_values, &mut lambda_context)
        }
        _ => {
            let mut err = errors::runtime_general(
                "first element must be a callable entity (lambda or symbol)",
                context.current_file(),
                context.current_source(),
                context.span_for_node(head)
            ).with_suggestion("Use a lambda or symbol as the function name");

            if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                err = err.with_test_context(tf.clone(), tn.clone());
            }
            Err(err)
        }
    }
}

/// Resolves a callable entity following strict precedence order.
/// This is the single source of truth for function call resolution.
///
/// Resolution order (per language specification):
/// 1. Lexical environment (let/lambda bindings)
/// 2. Atom registry (built-in functions and special forms)
/// 3. World state (global state paths)
fn resolve_callable(
    symbol_name: &str,
    head: &AstNode,
    args: &[AstNode], // Raw, unevaluated arguments
    span: &Span,
    context: &mut EvaluationContext,
) -> AtomResult {
    // Resolution order (per language specification):
    // 1. Lexical environment (let/lambda bindings)
    if let Some(value) = context.get_lexical_var(symbol_name).cloned() {
        return match value {
            Value::Lambda(lambda) => {
                // Eagerly evaluate arguments for lambda call
                let flat_args = flatten_spread_args(args, context)?;
                let arg_values = evaluate_eager_args(&flat_args, context)?;
                let mut lambda_context = context.clone_with_new_lexical_frame();
                lambda_context.depth += 1;
                special_forms::call_lambda(&lambda, &arg_values, &mut lambda_context)
            }
            _ => {
                let mut err = errors::runtime_general(
                    format!(
                        "'{}' is not a callable entity (found in lexical environment but not a lambda)",
                        symbol_name
                    ),
                    context.current_file(),
                    context.current_source(),
                    context.span_for_node(head),
                )
                .with_suggestion("Ensure the variable contains a lambda function");

                if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                    err = err.with_test_context(tf.clone(), tn.clone());
                }
                Err(err)
            }
        };
    }

    // 2. Atom registry (built-in functions and special forms)
    // We clone the atom to release the borrow on the registry.
    if let Some(atom) = context.atom_registry.get(symbol_name).cloned() {
        return match atom {
            // Lazy atoms receive raw, unevaluated arguments
            Atom::Lazy(_) => context.call_atom(symbol_name, head, args, span),
            // Eager atoms receive evaluated arguments, so we must flatten spreads first.
            Atom::Eager(_) => {
                let flat_args = flatten_spread_args(args, context)?;
                context.call_atom(symbol_name, head, &flat_args, span)
            }
        };
    }

    // 3. World state (global state paths)
    // First, borrow immutably to check for existence and clone the value.
    let world_path = Path(vec![symbol_name.to_string()]);
    let value_from_world = context.world.borrow().state.get(&world_path).cloned();

    // Now that the immutable borrow is dropped, we can proceed with operations
    // that might require a mutable borrow of the world (e.g., evaluating arguments).
    if let Some(value) = value_from_world {
        return match value {
            Value::Lambda(lambda) => {
                let flat_args = flatten_spread_args(args, context)?;
                let arg_values = evaluate_eager_args(&flat_args, context)?;
                let mut lambda_context = context.clone_with_new_lexical_frame();
                lambda_context.depth += 1;
                special_forms::call_lambda(&lambda, &arg_values, &mut lambda_context)
            }
            _ => {
                let mut err = errors::runtime_general(
                    format!(
                        "'{}' is not a callable entity (found in world state but not a lambda)",
                        symbol_name
                    ),
                    context.current_file(),
                    context.current_source(),
                    context.span_for_node(head),
                )
                .with_suggestion("Ensure the global variable contains a lambda function");

                if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                    err = err.with_test_context(tf.clone(), tn.clone());
                }
                Err(err)
            }
        };
    }

    // 4. Symbol not found anywhere
    let mut err = errors::runtime_undefined_symbol(
        symbol_name,
        context.current_file(),
        context.current_source(),
        context.span_for_node(head),
    )
    .with_suggestion(format!("Define '{}' before using it", symbol_name));

    if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
        err = err.with_test_context(tf.clone(), tn.clone());
    }
    Err(err)
}

/// Evaluates quote expressions (preserve content)
fn evaluate_quote(inner: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    match &*inner.value {
        Expr::Symbol(s, _) => Ok(Value::String(s.clone())),
        Expr::List(exprs, _) => evaluate_quoted_list(exprs, context).map(Value::List),
        Expr::Number(n, _) => Ok(Value::Number(*n)),
        Expr::Bool(b, _) => Ok(Value::Bool(*b)),
        Expr::String(s, _) => Ok(Value::String(s.clone())),
        Expr::Path(p, _) => Ok(Value::Path(p.clone())),
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => evaluate_quoted_if(condition, then_branch, else_branch, context),
        Expr::Quote(_, _) => Ok(Value::Nil),
        Expr::ParamList(_) => {
            let mut err = errors::runtime_general(
                "Cannot evaluate parameter list (ParamList AST node) at runtime",
                context.current_file(),
                context.current_source(),
                context.span_for_node(inner)
            ).with_suggestion("Parameter lists are only valid in lambda definitions");

            if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                err = err.with_test_context(tf.clone(), tn.clone());
            }
            Err(err)
        }
        Expr::Spread(_) => {
            let mut err = errors::runtime_general(
                "Spread argument not allowed inside quote",
                context.current_file(),
                context.current_source(),
                context.span_for_node(inner)
            ).with_suggestion("Remove the spread operator inside quotes");

            if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                err = err.with_test_context(tf.clone(), tn.clone());
            }
            Err(err)
        }
    }
}


/// Evaluates literal value expressions (Path, String, Number, Bool)
fn evaluate_literal_value(expr: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    let value = match &*expr.value {
        Expr::Path(p, _) => Value::Path(p.clone()),
        Expr::String(s, _) => Value::String(s.clone()),
        Expr::Number(n, _) => Value::Number(*n),
        Expr::Bool(b, _) => Value::Bool(*b),
        _ => {
            let err = errors::runtime_general(
                "eval_literal_value called with non-literal expression",
                context.current_file(),
                context.current_source(),
                context.span_for_node(expr)
            ).with_suggestion("This should not happen - please report as a bug");
            return Err(err);
        }
    };
    Ok(value)
}

/// Evaluates invalid expressions that cannot be evaluated at runtime
fn evaluate_invalid_expr(expr: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    match &*expr.value {
        // Parameter lists cannot be evaluated at runtime
        Expr::ParamList(_) => {
            let mut err = errors::runtime_general(
                "Cannot evaluate parameter list (ParamList AST node) at runtime",
                context.current_file(),
                context.current_source(),
                context.span_for_node(expr)
            ).with_suggestion("Parameter lists are only valid in lambda definitions");

            if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                err = err.with_test_context(tf.clone(), tn.clone());
            }
            Err(err)
        }

        // Symbols need resolution - may succeed or fail
        Expr::Symbol(symbol_name, span) => resolve_symbol(symbol_name, span, context),

        // Spread arguments are only valid in function calls
        Expr::Spread(_) => {
            let mut err = errors::runtime_general(
                "Spread argument not allowed outside of call position (list context)",
                context.current_file(),
                context.current_source(),
                context.span_for_node(expr)
            ).with_suggestion("Use spread only in function call arguments");

            if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                err = err.with_test_context(tf.clone(), tn.clone());
            }
            Err(err)
        }

        // This should never happen with valid expressions
        _ => unreachable!("evaluate_invalid_expr called with valid expression type"),
    }
}

fn resolve_symbol(symbol_name: &str, span: &Span, context: &mut EvaluationContext) -> AtomResult {
    // Try to resolve symbol in precedence order: lexical → atom → world → undefined

    // Step 1: Check lexical environment (let/lambda bindings)
    if let Some(value) = context.get_lexical_var(symbol_name) {
        return Ok(value.clone());
    }

    // Step 2: Check if symbol is an atom (must be called, not evaluated)
    if context.atom_registry.has(symbol_name) {
        let mut err = errors::runtime_general(
            format!("'{}' is an atom and must be called with arguments (e.g., ({} ...))", symbol_name, symbol_name),
            context.current_file(),
            context.current_source(),
            context.span_for_span(*span)
        ).with_suggestion(format!("Use '({} ...)' to call the atom", symbol_name));

        if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
            err = err.with_test_context(tf.clone(), tn.clone());
        }
        return Err(err);
    }

    // Step 3: Check world state (global variables)
    let world_path = Path(vec![symbol_name.to_string()]);
    if let Some(value) = context.world.borrow().state.get(&world_path) {
        return Ok(value.clone());
    }

    // Step 4: Symbol is undefined
    let mut err = errors::runtime_undefined_symbol(
        symbol_name,
        context.current_file(),
        context.current_source(),
        context.span_for_span(*span)
    ).with_suggestion(format!("Define '{}' before using it", symbol_name));

    if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
        err = err.with_test_context(tf.clone(), tn.clone());
    }
    Err(err)
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
        Expr::ParamList(_) => {
            let mut err = errors::runtime_general(
                "Cannot evaluate parameter list (ParamList AST node) inside quote",
                context.current_file(),
                context.current_source(),
                context.span_for_node(expr)
            ).with_suggestion("Parameter lists are not allowed in quotes");

            if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                err = err.with_test_context(tf.clone(), tn.clone());
            }
            Err(err)
        }
        Expr::Spread(_) => {
            let mut err = errors::runtime_general(
                "Spread argument not allowed inside quote",
                context.current_file(),
                context.current_source(),
                context.span_for_node(expr)
            ).with_suggestion("Remove the spread operator inside quotes");

            if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
                err = err.with_test_context(tf.clone(), tn.clone());
            }
            Err(err)
        }
        _ => Ok(Value::Nil),
    }
}

/// Evaluates a quoted list by converting each element to a value
fn evaluate_quoted_list(
    exprs: &[AstNode],
    context: &EvaluationContext,
) -> Result<Vec<Value>, SutraError> {
    exprs.iter().map(|e| evaluate_quoted_expr(e, context)).collect()
}

/// Evaluates a quoted if expression
fn evaluate_quoted_if(
    condition: &AstNode,
    then_branch: &AstNode,
    else_branch: &AstNode,
    context: &mut EvaluationContext,
) -> AtomResult {
    let is_true = evaluate_condition_as_bool(condition, context)?;
    let mut sub_context = context.clone_with_new_lexical_frame();
    sub_context.depth += 1;
    let branch = if is_true { then_branch } else { else_branch };
    evaluate_ast_node(branch, &mut sub_context)
}

// ===================================================================================================
// ARGUMENT PROCESSING: Function Call Support
// ===================================================================================================

/// Evaluates arguments for eager atoms, directly mutating the shared world state.
fn evaluate_eager_args(
    args: &[AstNode],
    context: &mut EvaluationContext,
) -> Result<Vec<Value>, SutraError> {
    let mut values = Vec::new();
    for arg in args {
        values.push(evaluate_ast_node(arg, context)?);
    }
    Ok(values)
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
    let spread_value = evaluate_ast_node(expr, context)?;

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
        let mut err = errors::type_mismatch(
            "list",
            value.type_name(),
            context.current_file(),
            context.current_source(),
            context.span_for_node(expr)
        ).with_suggestion("Use a list for spread operations");

        if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
            err = err.with_test_context(tf.clone(), tn.clone());
        }
        return Err(err);
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

// `wrap_value_with_world_state` is now removed.

/// Helper to evaluate a conditional expression and return a boolean result
pub fn evaluate_condition_as_bool(
    condition: &AstNode,
    context: &mut EvaluationContext,
) -> Result<bool, SutraError> {
    let cond_val = evaluate_ast_node(condition, context)?;

    // Guard clause: ensure condition evaluates to boolean
    let Value::Bool(b) = cond_val else {
        let mut err = errors::type_mismatch(
            "bool",
            cond_val.type_name(),
            context.current_file(),
            context.current_source(),
            context.span_for_node(condition)
        ).with_suggestion("Use a boolean value for conditions");

        if let (Some(ref tf), Some(ref tn)) = (&context.test_file, &context.test_name) {
            err = err.with_test_context(tf.clone(), tn.clone());
        }
        return Err(err);
    };

    Ok(b)
}

// The `capture_lexical_env` function was here, but is no longer used after the refactor.
