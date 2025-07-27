//! This module provides the shared infrastructure used by all atom implementations.
//! It contains argument evaluation, type extraction, operation templates, and error
//! construction utilities.
//!
//! ## Design Principles
//!
//! - **Single Responsibility**: Each helper function has one clear purpose
//! - **Reusability**: Functions are designed to be used across all atom domains
//! - **Consistency**: Standardized error messages and evaluation patterns
//! - **Safety**: All functions handle ownership and borrowing correctly

use crate::prelude::*;
use crate::{
    errors::{self, ErrorReporting, SutraError},
    runtime::eval::{evaluate_ast_node, EvaluationContext},
};
use std::sync::Arc;

// ============================================================================
// TYPE ALIASES AND CORE TYPES
// ============================================================================

/// Convenient type alias for atom return values - modern Rust idiom
pub type AtomResult = Result<Value, SutraError>;

/// Type alias for evaluation context to reduce verbosity
pub type EvalContext<'a> = &'a mut EvaluationContext;

/// Type alias for functions that return multiple values with world state
pub type MultiValueResult = Result<Vec<Value>, SutraError>;

/// Type alias for functions that return typed arrays with world state
pub type ArrayResult<const N: usize> = Result<[Value; N], SutraError>;

/// Type alias for binary operations returning two values and world
pub type BinaryResult = Result<(Value, Value), SutraError>;

/// Type alias for validation functions that return unit
pub type ValidationResult = Result<(), SutraError>;

// ============================================================================
// TRAIT-BASED TYPE EXTRACTION
// ============================================================================

/// Trait for extracting typed values from `Value` enum with consistent error handling.
/// This provides a unified interface for type checking and extraction across all atoms.
pub trait ExtractValue<T> {
    /// Extracts a value of type T from this Value.
    /// The `context` is optional to support both eager and lazy evaluation patterns.
    fn extract(&self, context: &EvaluationContext) -> Result<T, SutraError>;
}

impl ExtractValue<f64> for Value {
    fn extract(&self, context: &EvaluationContext) -> Result<f64, SutraError> {
        if let Value::Number(n) = self {
            return Ok(*n);
        }

        Err(context.type_mismatch(
            "Number",
            self.type_name(),
            context.span_for_span(context.current_span),
        ))
    }
}

impl ExtractValue<bool> for Value {
    fn extract(&self, context: &EvaluationContext) -> Result<bool, SutraError> {
        if let Value::Bool(b) = self {
            return Ok(*b);
        }

        Err(context.type_mismatch(
            "Boolean",
            self.type_name(),
            context.span_for_span(context.current_span),
        ))
    }
}

impl ExtractValue<Path> for Value {
    fn extract(&self, context: &EvaluationContext) -> Result<Path, SutraError> {
        if let Value::Path(path) = self {
            return Ok(path.clone());
        }

        Err(context.type_mismatch(
            "Path",
            self.type_name(),
            context.span_for_span(context.current_span),
        ))
    }
}

// ============================================================================
// EVALUATION CONTEXT UTILITIES
// ============================================================================

/// Macro to create a sub-evaluation context with a new world state.
/// This centralizes the repetitive context construction pattern used throughout atoms.
///
/// # Usage
/// ```ignore
/// let mut sub_context = sub_eval_context!(parent_context, &new_world);
/// let (result, world) = evaluate_ast_node(&args[0], &mut sub_context)?;
/// ```
#[macro_export]
macro_rules! sub_eval_context {
    ($parent:expr) => {{
        let mut new_context = $parent.clone_with_new_lexical_frame();
        new_context.depth += 1;
        new_context
    }};
}

// Re-export for use within this module
pub use sub_eval_context;

// ============================================================================
// ARGUMENT EVALUATION FUNCTIONS
// ============================================================================

/// Evaluates all arguments in sequence, threading world state through each evaluation.
/// This is the fundamental building block for all multi-argument atom operations.
pub fn eval_args(args: &[AstNode], context: &mut EvaluationContext) -> MultiValueResult {
    let mut values = Vec::with_capacity(args.len());
    for arg in args {
        let val = evaluate_ast_node(arg, context)?;
        values.push(val);
    }
    Ok(values)
}

/// Generic argument evaluation with compile-time arity checking
pub fn eval_n_args<const N: usize>(
    args: &[AstNode],
    context: &mut EvaluationContext,
) -> ArrayResult<N> {
    if args.len() != N {
        return Err(
            context.arity_mismatch(&N.to_string(), args.len(), context.span_for_span(context.current_span))
        );
    }

    let mut values = Vec::with_capacity(N);
    for arg in args.iter().take(N) {
        let val = evaluate_ast_node(arg, context)?;
        values.push(val);
    }

    // Convert Vec to array - this is safe because we checked length above
    // The try_into() should never fail given the length check, but we handle it defensively
    let values_array: [Value; N] = values.try_into().map_err(|_| {
        context
            .internal_error(
                "Failed to convert evaluated arguments to array",
                context.span_for_span(context.current_span),
            )
            .with_suggestion("This is an internal engine error. Please report this as a bug.")
    })?;

    Ok(values_array)
}

/// Evaluates a single argument and returns the value and world
pub fn eval_single_arg(args: &[AstNode], context: &mut EvaluationContext) -> AtomResult {
    let [val] = eval_n_args::<1>(args, context)?;
    Ok(val)
}

/// Evaluates two arguments and returns both values and the final world
pub fn eval_binary_args(args: &[AstNode], context: &mut EvaluationContext) -> BinaryResult {
    let [val1, val2] = eval_n_args::<2>(args, context)?;
    Ok((val1, val2))
}

// ============================================================================
// TYPE EXTRACTION FUNCTIONS
// ============================================================================

/// Extracts two numbers from values with type checking using the trait.
/// For single value extraction, use val.extract() directly.
pub fn extract_numbers(
    val1: &Value,
    val2: &Value,
    context: &EvaluationContext,
) -> Result<(f64, f64), SutraError> {
    let n1 = val1.extract(context)?;
    let n2 = val2.extract(context)?;
    Ok((n1, n2))
}

/// Validates that the number of arguments matches the expected count.
/// Provides consistent error messages for arity validation across all atoms.
///
/// # Arguments
/// * `args` - The arguments to validate
/// * `expected` - The expected number of arguments
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if arity matches
/// * `Err(SutraError)` with descriptive error message if mismatch
///
/// # Example
/// ```ignore
/// validate_arity(args, 2, "eq?")?;
/// ```
pub fn validate_arity(
    args: &[Value],
    expected: usize,
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    if args.len() != expected {
        return Err(ctx
            .arity_mismatch(
                &expected.to_string(),
                args.len(),
                ctx.span_for_span(ctx.current_span),
            )
            .with_suggestion(format!("{} expects {} arguments", atom_name, expected)));
    }
    Ok(())
}

/// Validates that the number of arguments is at least the minimum required.
/// Useful for atoms that accept variable numbers of arguments with a minimum.
///
/// # Arguments
/// * `args` - The arguments to validate
/// * `min_expected` - The minimum number of arguments required
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if arity meets minimum requirement
/// * `Err(SutraError)` with descriptive error message if too few arguments
///
/// # Example
/// ```ignore
/// validate_min_arity(args, 1, "min")?;
/// ```
pub fn validate_min_arity(
    args: &[Value],
    min_expected: usize,
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    if args.len() < min_expected {
        return Err(ctx
            .arity_mismatch(
                &format!("at least {}", min_expected),
                args.len(),
                ctx.span_for_span(ctx.current_span),
            )
            .with_suggestion(format!(
                "{} expects at least {} arguments",
                atom_name, min_expected
            )));
    }
    Ok(())
}

/// Validates that the number of arguments is exactly one.
/// Common case optimization for unary operations.
///
/// # Arguments
/// * `args` - The arguments to validate
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if exactly one argument
/// * `Err(SutraError)` with descriptive error message if not
///
/// # Example
/// ```ignore
/// validate_unary_arity(args, "abs")?;
/// ```
pub fn validate_unary_arity(
    args: &[Value],
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    validate_arity(args, 1, atom_name, ctx)
}

/// Validates that the number of arguments is exactly two.
/// Common case optimization for binary operations.
///
/// # Arguments
/// * `args` - The arguments to validate
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if exactly two arguments
/// * `Err(SutraError)` with descriptive error message if not
///
/// # Example
/// ```ignore
/// validate_binary_arity(args, "eq?")?;
/// ```
pub fn validate_binary_arity(
    args: &[Value],
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    validate_arity(args, 2, atom_name, ctx)
}

/// Validates that the number of arguments is at least two.
/// Common case for comparison operations that work on sequences.
///
/// # Arguments
/// * `args` - The arguments to validate
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if at least two arguments
/// * `Err(SutraError)` with descriptive error message if fewer than two
///
/// # Example
/// ```ignore
/// validate_sequence_arity(args, "eq?")?;
/// ```
pub fn validate_sequence_arity(
    args: &[Value],
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    if args.len() < 2 {
        return Err(ctx
            .arity_mismatch("at least 2", args.len(), ctx.span_for_span(ctx.current_span))
            .with_suggestion(format!("{} expects at least 2 arguments", atom_name)));
    }
    Ok(())
}

/// Validates that the number of arguments is even.
/// Useful for map construction and similar operations.
///
/// # Arguments
/// * `args` - The arguments to validate
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if even number of arguments
/// * `Err(SutraError)` with descriptive error message if odd
///
/// # Example
/// ```ignore
/// validate_even_arity(args, "core/map")?;
/// ```
pub fn validate_even_arity(
    args: &[Value],
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    if args.len() % 2 != 0 {
        return Err(ctx
            .arity_mismatch("even number", args.len(), ctx.span_for_span(ctx.current_span))
            .with_suggestion(format!(
                "{} expects an even number of arguments, got {}",
                atom_name,
                args.len()
            )));
    }
    Ok(())
}

/// Validates that the number of arguments is exactly zero.
/// Useful for atoms that take no arguments.
///
/// # Arguments
/// * `args` - The arguments to validate
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if no arguments
/// * `Err(SutraError)` with descriptive error message if any arguments provided
///
/// # Example
/// ```ignore
/// validate_zero_arity(args, "rand")?;
/// ```
pub fn validate_zero_arity(
    args: &[Value],
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    if args.len() != 0 {
        return Err(ctx
            .arity_mismatch("0", args.len(), ctx.span_for_span(ctx.current_span))
            .with_suggestion(format!("{} expects no arguments", atom_name)));
    }
    Ok(())
}

/// Validates that the number of AstNode arguments matches the expected count.
/// Provides consistent error messages for special form arity validation.
///
/// # Arguments
/// * `args` - The AstNode arguments to validate
/// * `expected` - The expected number of arguments
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if arity matches
/// * `Err(SutraError)` with descriptive error message if mismatch
///
/// # Example
/// ```ignore
/// validate_special_form_arity(args, 3, "if")?;
/// ```
pub fn validate_special_form_arity(
    args: &[AstNode],
    expected: usize,
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    if args.len() != expected {
        return Err(ctx
            .arity_mismatch(
                &expected.to_string(),
                args.len(),
                ctx.span_for_span(ctx.current_span),
            )
            .with_suggestion(format!("{} expects {} arguments", atom_name, expected)));
    }
    Ok(())
}

/// Validates that the number of AstNode arguments is at least the minimum required.
/// Useful for special forms that accept variable numbers of arguments with a minimum.
///
/// # Arguments
/// * `args` - The AstNode arguments to validate
/// * `min_expected` - The minimum number of arguments required
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(())` if arity meets minimum requirement
/// * `Err(SutraError)` with descriptive error message if too few arguments
///
/// # Example
/// ```ignore
/// validate_special_form_min_arity(args, 2, "lambda")?;
/// ```
pub fn validate_special_form_min_arity(
    args: &[AstNode],
    min_expected: usize,
    atom_name: &str,
    ctx: &EvaluationContext,
) -> ValidationResult {
    if args.len() < min_expected {
        return Err(ctx
            .arity_mismatch(
                &format!("at least {}", min_expected),
                args.len(),
                ctx.span_for_span(ctx.current_span),
            )
            .with_suggestion(format!(
                "{} expects at least {} arguments",
                atom_name, min_expected
            )));
    }
    Ok(())
}

// ============================================================================
// OPERATION EVALUATION TEMPLATES
// ============================================================================

/// Evaluates a binary numeric operation atomically, with optional validation.
/// Handles arity, type checking, and error construction.
pub fn eval_binary_numeric_template<F, V>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    op: F,
    validator: Option<V>,
) -> AtomResult
where
    F: Fn(f64, f64) -> Value,
    V: Fn(f64, f64) -> Result<(), &'static str>,
{
    let (val1, val2) = eval_binary_args(args, context)?;
    let (n1, n2) = extract_numbers(&val1, &val2, context)?;

    if let Some(validate) = validator {
        validate(n1, n2).map_err(|msg| context.invalid_operation(msg, "Number", context.span_for_span(context.current_span)))?;
    }

    Ok(op(n1, n2))
}

/// Evaluates an n-ary numeric operation (e.g., sum, product).
/// Handles arity, type checking, and error construction.
pub fn eval_nary_numeric_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    init: f64,
    fold: F,
) -> AtomResult
where
    F: Fn(f64, f64) -> f64,
{
    if args.len() < 2 {
        return Err(context.arity_mismatch(
            "at least 2",
            args.len(),
            context.span_for_span(context.current_span),
        ));
    }

    let values = eval_args(args, context)?;
    let mut acc = init;

    for v in values.iter() {
        let n = v.extract(context)?;
        acc = fold(acc, n);
    }

    Ok(Value::Number(acc))
}

/// Evaluates a unary boolean operation.
/// Handles arity, type checking, and error construction.
pub fn eval_unary_bool_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    op: F,
) -> AtomResult
where
    F: Fn(bool) -> Value,
{
    let val = eval_single_arg(args, context)?;
    let b = val.extract(context)?;
    Ok(op(b))
}

/// Evaluates a unary path operation (get, del).
/// Handles arity, type checking, and error construction.
pub fn eval_unary_path_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    op: F,
) -> AtomResult
where
    F: Fn(&mut World, Path) -> AtomResult,
{
    let val = eval_single_arg(args, context)?;
    let path = val.extract(context)?;
    op(&mut context.world.borrow_mut(), path)
}

/// Evaluates a binary path operation (set).
/// Handles arity, type checking, and error construction.
pub fn eval_binary_path_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    op: F,
) -> AtomResult
where
    F: Fn(&mut World, Path, Value) -> AtomResult,
{
    let (path_val, value) = eval_binary_args(args, context)?;
    let path = path_val.extract(context)?;
    op(&mut context.world.borrow_mut(), path, value)
}

/// Evaluates a unary operation that takes any value.
/// Handles arity and error construction.
pub fn eval_unary_value_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    op: F,
) -> AtomResult
where
    F: Fn(Value, &mut EvaluationContext) -> AtomResult,
{
    let val = eval_single_arg(args, context)?;
    op(val, context)
}

/// Evaluates a sequence comparison operation on numbers.
/// Handles arity validation, type checking, and the common comparison pattern.
///
/// # Arguments
/// * `args` - The arguments to evaluate
/// * `context` - The evaluation context
/// * `comparison` - The comparison function (e.g., |a, b| a <= b)
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok((Value::Bool, World))` if comparison succeeds
/// * `Err(SutraError)` if validation fails
///
/// # Example
/// ```ignore
/// let result = eval_numeric_sequence_comparison_template(
///     args, context, |a, b| a <= b, "gt?"
/// )?;
/// ```
pub fn eval_numeric_sequence_comparison_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    comparison: F,
    atom_name: &str,
) -> AtomResult
where
    F: Fn(f64, f64) -> bool,
{
    let values = eval_args(args, context)?;
    validate_sequence_arity(&values, atom_name, context)?;

    for i in 0..values.len() - 1 {
        let a = values[i].extract(context)?;
        let b = values[i + 1].extract(context)?;
        if comparison(a, b) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

/// Evaluates an n-ary numeric operation with a custom initial value and fold function.
/// Handles arity validation, type checking, and the common fold pattern.
///
/// # Arguments
/// * `args` - The arguments to evaluate
/// * `context` - The evaluation context
/// * `init` - The initial value for the fold
/// * `fold` - The fold function (e.g., |acc, n| acc + n)
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok((Value::Number, World))` if operation succeeds
/// * `Err(SutraError)` if validation fails
///
/// # Example
/// ```ignore
/// let result = eval_nary_numeric_op_custom_template(
///     args, context, 0.0, |acc, n| acc + n, "sum"
/// )?;
/// ```
pub fn eval_nary_numeric_op_custom_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    init: f64,
    fold: F,
    atom_name: &str,
) -> AtomResult
where
    F: Fn(f64, f64) -> f64,
{
    let values = eval_args(args, context)?;
    validate_min_arity(&values, 1, atom_name, context)?;

    let mut result = init;
    for v in values.iter() {
        let n = v.extract(context)?;
        result = fold(result, n);
    }
    Ok(Value::Number(result))
}

/// Evaluates a unary operation with type checking using the ExtractValue trait.
/// Handles arity validation and provides consistent error messages.
///
/// # Arguments
/// * `args` - The arguments to evaluate
/// * `context` - The evaluation context
/// * `op` - The operation function
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok((Value, World))` if operation succeeds
/// * `Err(SutraError)` if validation fails
///
/// # Example
/// ```ignore
/// let result = eval_unary_typed_template(
///     args, context, |b| Value::Bool(!b), "not"
/// )?;
/// ```
pub fn eval_unary_typed_template<T, F>(
    args: &[AstNode],
    context: &mut EvaluationContext,
    op: F,
    _atom_name: &str,
) -> AtomResult
where
    Value: ExtractValue<T>,
    F: Fn(T) -> Value,
{
    let val = eval_single_arg(args, context)?;
    let extracted = val.extract(context)?;
    Ok(op(extracted))
}

// ============================================================================
// APPLY ATOM HELPERS
// ============================================================================

/// Evaluates normal arguments for apply (all except the last argument).
/// Returns the evaluated arguments as expressions and the final world state.
pub fn eval_apply_normal_args(
    args: &[AstNode],
    context: &mut EvaluationContext,
) -> Result<Vec<AstNode>, SutraError> {
    let mut evaluated_arg_nodes = Vec::with_capacity(args.len());
    for arg in args {
        let val = crate::runtime::eval::evaluate_ast_node(arg, context)?;
        let expr = crate::ast::expr_from_value_with_span(val, arg.span)
            .map_err(|msg| context.internal_error(&msg, context.span_for_span(arg.span)))?;
        evaluated_arg_nodes.push(Spanned {
            value: Arc::new(expr),
            span: arg.span,
        });
    }
    Ok(evaluated_arg_nodes)
}

/// Evaluates the list argument for apply (the last argument).
/// Returns the list items as expressions and the final world state.
pub fn eval_apply_list_arg(
    arg: &AstNode,
    context: &mut EvaluationContext,
    parent_span: &Span,
) -> Result<Vec<AstNode>, SutraError> {
    let list_val = evaluate_ast_node(arg, context)?;
    match list_val {
        Value::Cons(_) | Value::Nil => {
            // Note: We use `parent_span` for each element because after evaluation,
            // the original span information for each list element is lost.
            // To preserve per-element spans, the evaluation pipeline would need to
            // carry spans alongside values (e.g., Spanned<Value>), which is not currently implemented.
            let mut items = Vec::new();
            let mut current = list_val;
            loop {
                match current {
                    Value::Cons(boxed) => {
                        let head = boxed.car.clone();
                        let tail = boxed.cdr.clone();
                        items.push(Spanned {
                            value: Expr::from(head).into(),
                            span: *parent_span,
                        });
                        current = tail;
                    }
                    Value::Nil => break,
                    _ => {
                        return Err(context
                            .type_mismatch(
                                "proper List",
                                current.type_name(),
                                context.span_for_span(*parent_span),
                            )
                            .with_suggestion("The last argument to 'apply' must be a proper list (ending in nil)."));
                    }
                }
            }
            Ok(items)
        }
        _ => {
            return Err(context
                .type_mismatch("List", list_val.type_name(), context.span_for_span(*parent_span))
                .with_suggestion("The last argument to 'apply' must be a list."))
        }
    }
}

/// Builds the call expression for apply by combining function, normal args, and list args.
pub fn build_apply_call_expr(
    func_expr: &AstNode,
    normal_args: Vec<AstNode>,
    list_args: Vec<AstNode>,
    parent_span: &Span,
) -> AstNode {
    let mut call_items = Vec::with_capacity(1 + normal_args.len() + list_args.len());
    call_items.push(func_expr.clone());
    call_items.extend(normal_args);
    call_items.extend(list_args);
    Spanned {
        value: Expr::List(call_items, *parent_span).into(),
        span: *parent_span,
    }
}

// ============================================================================
// COLLECTION VALIDATION HELPERS
// ============================================================================

/// Validates that a value is a Path and extracts it.
/// Provides consistent error messages for path validation across collection atoms.
///
/// # Arguments
/// * `args` - The arguments array
/// * `atom_name` - Name of the atom for error messages
///
/// # Returns
/// * `Ok(&Path)` - The path if valid
/// * `Err(SutraError)` - Error with descriptive message
pub fn validate_path_arg<'a>(
    value: &'a Value,
    span: Span,
    atom_name: &str,
    ctx: &EvaluationContext,
) -> Result<&'a Path, SutraError> {
    match value {
        Value::Path(p) => Ok(p),
        _ => Err(ctx
            .type_mismatch("Path", value.type_name(), ctx.span_for_span(span))
            .with_suggestion(format!(
                "{} expects a Path as its first argument",
                atom_name
            ))),
    }
}

/// Validates that a value is a String and extracts it.
/// Provides consistent error messages for string validation across collection atoms.
///
/// # Arguments
/// * `value` - The value to validate
/// * `context` - Context for error messages (e.g., "map key", "string argument")
///
/// # Returns
/// * `Ok(&str)` - The string slice if valid
/// * `Err(SutraError)` - Error with descriptive message
pub fn validate_string_value<'a>(
    value: &'a Value,
    context: &str,
    ctx: &EvaluationContext,
) -> Result<&'a str, SutraError> {
    match value {
        Value::String(s) => Ok(s),
        _ => Err(ctx
            .type_mismatch("String", value.type_name(), ctx.span_for_span(ctx.current_span))
            .with_suggestion(format!("Expected a String for {}", context))),
    }
}

/// Validates that a value is a List and extracts a mutable reference to it.
/// Provides consistent error messages for list validation across collection atoms.
///
/// # Arguments
/// * `value` - The value to validate
/// * `context` - Context for error messages (e.g., "first argument", "at path")
///
/// # Returns
/// * `Ok(&mut Vec<Value>)` - Mutable reference to the list if valid
/// * `Err(SutraError)` - Error with descriptive message
// This function is removed as mutable operations on immutable lists are not supported.

/// Validates that a value is a List and extracts a reference to it.
/// Provides consistent error messages for list validation across collection atoms.
///
/// # Arguments
/// * `value` - The value to validate
/// * `context` - Context for error messages (e.g., "first argument", "at path")
///
/// # Returns
/// * `Ok(&Vec<Value>)` - Reference to the list if valid
/// * `Err(SutraError)` - Error with descriptive message
// This function is temporarily disabled and will be replaced by an iterator-based
// helper in the next step.
pub fn validate_list_value<'a>(
    value: &'a Value,
    context: &str,
    ctx: &EvaluationContext,
) -> Result<(), SutraError> {
    match value {
        Value::Cons(_) | Value::Nil => Ok(()),
        _ => Err(ctx
            .type_mismatch("List", value.type_name(), ctx.span_for_span(ctx.current_span))
            .with_suggestion(format!("Expected a List for {}", context))),
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        atoms::{NullSink, SharedOutput},
        errors::SourceContext,
        runtime::{eval::EvaluationContextBuilder, world::World},
    };
    use std::{cell::RefCell, rc::Rc};

    fn dummy_context() -> EvaluationContext {
        let source = SourceContext::from_file("test.sutra", "()");
        let world = Rc::new(RefCell::new(World::new()));
        let output = SharedOutput::new(NullSink);
        EvaluationContextBuilder::new(source, world, output).build()
    }

    #[test]
    fn test_validate_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];
        let ctx = dummy_context();

        // Should succeed with correct arity
        assert!(validate_arity(&args, 2, "test", &ctx).is_ok());

        // Should fail with wrong arity
        assert!(validate_arity(&args, 1, "test", &ctx).is_err());
        assert!(validate_arity(&args, 3, "test", &ctx).is_err());
    }

    #[test]
    fn test_validate_unary_arity() {
        let args = vec![Value::Number(1.0)];
        let ctx = dummy_context();

        // Should succeed with exactly one argument
        assert!(validate_unary_arity(&args, "test", &ctx).is_ok());

        // Should fail with wrong arity
        assert!(validate_unary_arity(&[], "test", &ctx).is_err());
        assert!(
            validate_unary_arity(&[Value::Number(1.0), Value::Number(2.0)], "test", &ctx).is_err()
        );
    }

    #[test]
    fn test_validate_binary_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];
        let ctx = dummy_context();

        // Should succeed with exactly two arguments
        assert!(validate_binary_arity(&args, "test", &ctx).is_ok());

        // Should fail with wrong arity
        assert!(validate_binary_arity(&[Value::Number(1.0)], "test", &ctx).is_err());
        assert!(validate_binary_arity(
            &[Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)],
            "test",
            &ctx
        )
        .is_err());
    }

    #[test]
    fn test_validate_min_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];
        let ctx = dummy_context();

        // Should succeed with sufficient arguments
        assert!(validate_min_arity(&args, 1, "test", &ctx).is_ok());
        assert!(validate_min_arity(&args, 2, "test", &ctx).is_ok());

        // Should fail with insufficient arguments
        assert!(validate_min_arity(&args, 3, "test", &ctx).is_err());
    }

    #[test]
    fn test_validate_sequence_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];
        let ctx = dummy_context();

        // Should succeed with at least two arguments
        assert!(validate_sequence_arity(&args, "test", &ctx).is_ok());
        assert!(validate_sequence_arity(
            &[Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)],
            "test",
            &ctx
        )
        .is_ok());

        // Should fail with fewer than two arguments
        assert!(validate_sequence_arity(&[], "test", &ctx).is_err());
        assert!(validate_sequence_arity(&[Value::Number(1.0)], "test", &ctx).is_err());
    }

    #[test]
    fn test_validate_even_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];
        let ctx = dummy_context();

        // Should succeed with even number of arguments (including 0)
        assert!(validate_even_arity(&[], "test", &ctx).is_ok());
        assert!(validate_even_arity(&args, "test", &ctx).is_ok());
        assert!(validate_even_arity(
            &[
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
                Value::Number(4.0)
            ],
            "test",
            &ctx
        )
        .is_ok());

        // Should fail with odd number of arguments
        assert!(validate_even_arity(&[Value::Number(1.0)], "test", &ctx).is_err());
        assert!(validate_even_arity(
            &[Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)],
            "test",
            &ctx
        )
        .is_err());
    }

    #[test]
    fn test_error_messages() {
        let args = vec![Value::Number(1.0)];
        let ctx = dummy_context();

        // Test that error messages are descriptive
        let err = validate_arity(&args, 2, "my_atom", &ctx).unwrap_err();
        assert!(err.to_string().contains("expected 2, got 1"));

        let err = validate_min_arity(&args, 3, "my_atom", &ctx).unwrap_err();
        assert!(err.to_string().contains("expected at least 3, got 1"));

        let err = validate_even_arity(&args, "my_atom", &ctx).unwrap_err();
        assert!(err.to_string().contains("expected an even number of arguments, got 1"));
    }

    #[test]
    fn test_validate_zero_arity() {
        let args = vec![Value::Number(1.0)];
        let ctx = dummy_context();

        // Should fail with any arguments
        assert!(validate_zero_arity(&args, "test", &ctx).is_err());
        assert!(
            validate_zero_arity(&[Value::Number(1.0), Value::Number(2.0)], "test", &ctx).is_err()
        );

        // Should succeed with no arguments
        assert!(validate_zero_arity(&[], "test", &ctx).is_ok());
    }

    #[test]
    fn test_validate_special_form_arity() {
        let span = Span { start: 0, end: 1 };
        let args = vec![AstNode {
            value: std::sync::Arc::new(Expr::Number(1.0, span)),
            span,
        }];
        let ctx = dummy_context();

        // Should succeed with correct arity
        assert!(validate_special_form_arity(&args, 1, "test", &ctx).is_ok());

        // Should fail with wrong arity
        assert!(validate_special_form_arity(&args, 2, "test", &ctx).is_err());
        assert!(validate_special_form_arity(&[], 1, "test", &ctx).is_err());
    }

    #[test]
    fn test_validate_special_form_min_arity() {
        let span = Span { start: 0, end: 1 };
        let args = vec![AstNode {
            value: std::sync::Arc::new(Expr::Number(1.0, span)),
            span,
        }];
        let ctx = dummy_context();

        // Should succeed with sufficient arguments
        assert!(validate_special_form_min_arity(&args, 1, "test", &ctx).is_ok());
        assert!(validate_special_form_min_arity(&args, 0, "test", &ctx).is_ok());

        // Should fail with insufficient arguments
        assert!(validate_special_form_min_arity(&args, 2, "test", &ctx).is_err());
    }
}
