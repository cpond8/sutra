//! Sutra Runtime Evaluation Engine
//!
//! Transforms AST nodes into runtime values with world state consistency.
//!
//! ## Calling Conventions
//!
//! Native functions are implemented as `Value::NativeEagerFn` or `Value::NativeLazyFn`,
//! preserving the dual calling convention model. Eager functions receive evaluated
//! `Value` arguments, while lazy functions (special forms) receive raw `AstNode`
//! arguments to control their own evaluation.
//!
//! ## Error Handling
//!
//! All errors use miette-native `SutraError` variants directly.
//! See `src/errors.rs` for error types and usage rules.
//!
//! Example: let err = SutraError::RuntimeGeneral { message: "Arity error".to_string(), ... };
//! assert!(matches!(err, sutra::SutraError::Eval { .. }));
//! ```

// All functions are now first-class values in the world state. There is no separate atom registry.
use std::sync::Arc;
use std::{collections::HashMap, rc::Rc};

use miette::SourceSpan;

// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::atoms::{helpers::AtomResult, special_forms};
use crate::errors::{to_source_span, SourceContext};
use crate::errors::{
    DiagnosticInfo, ErrorKind, ErrorReporting, FileContext, SourceInfo, SutraError,
};

// ===================================================================================================
// CORE DATA STRUCTURES: Evaluation Context
// ===================================================================================================

/// The context for a single evaluation, passed to atoms and all evaluation functions.
pub struct EvaluationContext {
    pub world: CanonicalWorld,
    pub output: SharedOutput,
    pub source: SourceContext,
    pub max_depth: usize,
    pub depth: usize,
    /// The span of the AST node currently being evaluated.
    pub current_span: Span,
    /// Stack of lexical environments (for let/lambda scoping)
    pub lexical_env: Vec<HashMap<String, Value>>,
    pub test_file: Option<String>,
    pub test_name: Option<String>,
}

impl EvaluationContext {
    /// Helper to increment depth for recursive calls.
    pub fn next_depth(&self) -> usize {
        self.depth + 1
    }

    /// Clone the context with a new lexical frame (for let/lambda)
    pub fn clone_with_new_lexical_frame(&self) -> Self {
        let mut new = EvaluationContext {
            world: Rc::clone(&self.world),
            output: self.output.clone(),
            source: self.source.clone(),
            max_depth: self.max_depth,
            depth: self.depth,
            current_span: self.current_span,
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
    pub fn print_lexical_env(&self) {}
}

impl EvaluationContext {
    /// Extract span information for a given AstNode
    pub fn span_for_node(&self, node: &AstNode) -> SourceSpan {
        to_source_span(node.span)
    }

    /// Extract span information for a given Span
    pub fn span_for_span(&self, span: Span) -> SourceSpan {
        to_source_span(span)
    }
}

// ===================================================================================================
// PUBLIC API: Evaluation Helpers
// ===================================================================================================

impl EvaluationContext {
    /// Evaluates a call to a callable value with the given arguments.
    /// This is a high-level helper for atoms like `map` that need to invoke functions.
    pub fn eval_call(&mut self, callable: &Value, args: &[Value]) -> AtomResult {
        match callable {
            Value::Lambda(lambda) => {
                // Call the lambda. Note that we are not flattening spread args here
                // because we assume the arguments are already processed values.
                special_forms::call_lambda(lambda, args, self)
            }
            Value::NativeEagerFn(eager_fn) => {
                // Call the native eager function directly.
                eager_fn(args, self)
            }
            _ => {
                // Not a callable value.
                Err(self.report(
                    ErrorKind::TypeMismatch {
                        expected: "callable".to_string(),
                        actual: callable.type_name().to_string(),
                    },
                    self.span_for_span(self.current_span),
                ))
            }
        }
    }
}

// ===================================================================================================
// PUBLIC API: Main Evaluation Interface
// ===================================================================================================

pub struct EvaluationContextBuilder {
    source: SourceContext,
    world: CanonicalWorld,
    output: SharedOutput,
    max_depth: usize,
    test_file: Option<String>,
    test_name: Option<String>,
}

impl EvaluationContextBuilder {
    pub fn new(source: SourceContext, world: CanonicalWorld, output: SharedOutput) -> Self {
        Self {
            source,
            world,
            output,
            max_depth: 1000,
            test_file: None,
            test_name: None,
        }
    }
    pub fn with_test_file(mut self, test_file: Option<String>) -> Self {
        self.test_file = test_file;
        self
    }
    pub fn with_test_name(mut self, test_name: Option<String>) -> Self {
        self.test_name = test_name;
        self
    }
    pub fn with_max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }
    pub fn build(self) -> EvaluationContext {
        let mut global_env = HashMap::new();
        global_env.insert("nil".to_string(), Value::Nil);
        EvaluationContext {
            world: self.world,
            output: self.output,
            source: self.source,
            max_depth: self.max_depth,
            depth: 0,
            current_span: Span::default(),
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
    source: SourceContext,
    max_depth: usize,
    test_file: Option<String>,
    test_name: Option<String>,
) -> AtomResult {
    let mut context = EvaluationContextBuilder::new(source, world, output)
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
    // Update the context to reflect the current expression being evaluated.
    // This is critical for ensuring that any errors generated within this
    // evaluation scope can access the correct source span.
    context.current_span = expr.span;

    // Step 1: Check recursion limit
    if context.depth > context.max_depth {
        return Err(context.report(ErrorKind::RecursionLimit, context.span_for_node(expr)));
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
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            let is_true = evaluate_condition_as_bool(condition, context)?;
            if is_true {
                evaluate_ast_node(then_branch, context)
            } else {
                evaluate_ast_node(else_branch, context)
            }
        }
    }
}

// ===================================================================================================
// EXPRESSION EVALUATION: Core Expression Types
// ===================================================================================================

/// Evaluates a list expression, which is the primary mechanism for function invocation.
fn evaluate_list(items: &[AstNode], span: &Span, context: &mut EvaluationContext) -> AtomResult {
    // An empty list evaluates to nil.
    if items.is_empty() {
        return Ok(Value::Nil);
    }

    let head = &items[0];
    let tail = &items[1..];

    // First, resolve the head of the list to a value.
    // If it's a symbol, we look it up. Otherwise, we evaluate it as an expression.
    let callable_val = if let Expr::Symbol(symbol_name, _) = &*head.value {
        resolve_symbol(symbol_name, &head.span, context)?
    } else {
        evaluate_ast_node(head, context)?
    };

    // Now, match on the resolved value to see if it's a callable entity.
    match callable_val {
        Value::Lambda(lambda) => {
            let flat_args = flatten_spread_args(tail, context)?;
            let arg_values = evaluate_eager_args(&flat_args, context)?;
            special_forms::call_lambda(&lambda, &arg_values, context)
        }
        Value::NativeEagerFn(eager_fn) => {
            let flat_args = flatten_spread_args(tail, context)?;
            let arg_values = evaluate_eager_args(&flat_args, context)?;
            eager_fn(&arg_values, context)
        }
        Value::NativeLazyFn(lazy_fn) => lazy_fn(tail, context, span),
        _ => Err(context.report(
            ErrorKind::TypeMismatch {
                expected: "callable".to_string(),
                actual: callable_val.type_name().to_string(),
            },
            context.span_for_node(head),
        )),
    }
}

/// Evaluates quote expressions (preserve content)
fn evaluate_quote(inner: &AstNode, _context: &mut EvaluationContext) -> AtomResult {
    fn ast_to_quoted_value(node: &AstNode) -> Value {
        match &*node.value {
            Expr::Symbol(s, _) => Value::Symbol(s.clone()),
            Expr::Number(n, _) => Value::Number(*n),
            Expr::Bool(b, _) => Value::Bool(*b),
            Expr::String(s, _) => Value::String(s.clone()),
            Expr::List(items, _) => {
                let mut result = Value::Nil;
                for item in items.iter().rev() {
                    let cell = crate::ast::value::ConsCell {
                        car: ast_to_quoted_value(item),
                        cdr: result,
                    };
                    result = Value::Cons(Rc::new(cell));
                }
                result
            }
            Expr::Quote(inner, _) => Value::Quote(Box::new(ast_to_quoted_value(inner))),
            Expr::Path(p, _) => Value::Path(p.clone()),
            _ => Value::Nil,
        }
    }
    Ok(Value::Quote(Box::new(ast_to_quoted_value(inner))))
}

/// Evaluates literal value expressions (Path, String, Number, Bool)
fn evaluate_literal_value(expr: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    let value = match &*expr.value {
        Expr::Path(p, _) => Value::Path(p.clone()),
        Expr::String(s, _) => Value::String(s.clone()),
        Expr::Number(n, _) => Value::Number(*n),
        Expr::Bool(b, _) => Value::Bool(*b),
        _ => {
            return Err(context.report(
                ErrorKind::InvalidOperation {
                    operation: "evaluate_literal_value".to_string(),
                    operand_type: expr.value.type_name().to_string(),
                },
                context.span_for_node(expr),
            ));
        }
    };
    Ok(value)
}

/// Evaluates invalid expressions that cannot be evaluated at runtime
fn evaluate_invalid_expr(expr: &AstNode, context: &mut EvaluationContext) -> AtomResult {
    match &*expr.value {
        // Parameter lists cannot be evaluated at runtime
        Expr::ParamList(_) => Err(context.report(
            ErrorKind::InvalidOperation {
                operation: "evaluate".to_string(),
                operand_type: "ParamList".to_string(),
            },
            context.span_for_node(expr),
        )),

        // Symbols need resolution - may succeed or fail
        Expr::Symbol(symbol_name, span) => Ok(resolve_symbol(symbol_name, span, context)?),

        // Spread arguments are only valid in function calls
        Expr::Spread(_) => Err(context.report(
            ErrorKind::MalformedConstruct {
                construct: "spread argument".to_string(),
            },
            context.span_for_node(expr),
        )),

        // This should never happen with valid expressions
        _ => unreachable!("evaluate_invalid_expr called with valid expression type"),
    }
}

/// Resolves a symbol to a value by searching the lexical environment, then the global world.
/// This function is the single source of truth for symbol resolution.
fn resolve_symbol(
    symbol_name: &str,
    span: &Span,
    context: &mut EvaluationContext,
) -> Result<Value, SutraError> {
    // 1. Lexical environment (innermost to outermost)
    if let Some(value) = context.get_lexical_var(symbol_name) {
        return Ok(value.clone());
    }

    // 2. Global world state
    let world_path = Path(vec![symbol_name.to_string()]);
    if let Some(value) = context.world.borrow().state.get(&world_path) {
        return Ok(value.clone());
    }

    // 3. Undefined
    Err(context.undefined_symbol(symbol_name, context.span_for_span(*span)))
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
    Ok(convert_values_to_ast_nodes(list_items, arg.span, context)?)
}

fn extract_list_items(
    value: Value,
    expr: &AstNode,
    context: &mut EvaluationContext,
) -> Result<Vec<Value>, SutraError> {
    match value {
        Value::Cons(_) | Value::Nil => Ok(value.try_into_iter().collect::<Vec<Value>>()),
        _ => Err(context.type_mismatch("list", value.type_name(), context.span_for_node(expr))),
    }
}

fn convert_values_to_ast_nodes(
    values: Vec<Value>,
    span: Span,
    context: &EvaluationContext,
) -> Result<Vec<AstNode>, SutraError> {
    let mut nodes = Vec::with_capacity(values.len());
    for value in values {
        let expr = crate::ast::expr_from_value_with_span(value, span).map_err(|msg| {
            context.report(
                ErrorKind::InvalidOperation {
                    operation: "value-to-ast".to_string(),
                    operand_type: "conversion".to_string(),
                },
                context.span_for_span(span),
            )
        })?;
        nodes.push(Spanned {
            value: Arc::new(expr),
            span,
        });
    }
    Ok(nodes)
}

// ===================================================================================================
// UTILITY FUNCTIONS: Common Patterns
// ===================================================================================================

// `wrap_value_with_world_state` is now removed.

fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Bool(false) => false,
        Value::Nil => false,
        Value::Number(n) => *n != 0.0,
        Value::String(s) => !s.is_empty(),
        Value::Map(m) => !m.is_empty(),
        Value::Quote(inner) => is_truthy(inner),
        _ => true,
    }
}

/// Helper to evaluate a conditional expression and return a boolean result
pub fn evaluate_condition_as_bool(
    condition: &AstNode,
    context: &mut EvaluationContext,
) -> Result<bool, SutraError> {
    let cond_val = evaluate_ast_node(condition, context)?;
    Ok(is_truthy(&cond_val))
}

// The `capture_lexical_env` function was here, but is no longer used after the refactor.

impl EvaluationContext {
    /// Generate context-appropriate help for runtime errors
    fn generate_runtime_help(&self, kind: &ErrorKind) -> Option<String> {
        match kind {
            ErrorKind::TypeMismatch { expected, actual } => {
                Some(format!("Expected {}, but got {}. Check the value type.", expected, actual))
            }
            ErrorKind::UndefinedSymbol { symbol } => {
                let suggestions = self.find_similar_symbols(symbol);
                if !suggestions.is_empty() {
                    Some(format!("Did you mean one of: {}?", suggestions.join(", ")))
                } else {
                    Some("Check that the symbol is defined before use.".into())
                }
            }
            ErrorKind::ArityMismatch { expected, actual } => {
                Some(format!("This function expects {} arguments, but received {}", expected, actual))
            }
            ErrorKind::InvalidOperation { operation, operand_type } => {
                Some(format!("The operation '{}' cannot be performed on {}. Check the operation requirements.", operation, operand_type))
            }
            ErrorKind::RecursionLimit => {
                Some("The function has called itself too many times. Check for infinite recursion or increase the recursion limit.".into())
            }
            ErrorKind::StackOverflow => {
                Some("The call stack has grown too large. This usually indicates infinite recursion or very deep function calls.".into())
            }
            _ => None,
        }
    }

    /// Find similar symbol names for suggestion purposes
    // TODO: Implement this using a real edit distance algorithm
    fn find_similar_symbols(&self, _target: &str) -> Vec<String> {
        // Implementation would use edit distance algorithm
        // to find similar symbols in the current scope
        vec![]
    }
}

impl ErrorReporting for EvaluationContext {
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError {
        SutraError {
            kind: kind.clone(),
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                file_context: FileContext::Runtime {
                    test_info: self
                        .test_file
                        .as_ref()
                        .zip(self.test_name.as_ref())
                        .map(|(f, n)| (f.clone(), n.clone())),
                },
            },
            diagnostic_info: DiagnosticInfo {
                help: self.generate_runtime_help(&kind),
                related_spans: Vec::new(), // Runtime errors typically don't have related spans
                error_code: format!("sutra::runtime::{}", kind.code_suffix()),
                is_warning: false,
            },
        }
    }
}
