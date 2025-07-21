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
    runtime::eval::{evaluate_ast_node, EvaluationContext},
    syntax::parser::to_source_span,
};
use miette::NamedSource;

// ============================================================================
// TYPE ALIASES AND CORE TYPES
// ============================================================================

/// Convenient type alias for atom return values - modern Rust idiom
pub type AtomResult = Result<(Value, World), SutraError>;

/// Type alias for evaluation context to reduce verbosity
pub type EvalContext<'a> = &'a mut EvaluationContext<'a>;

/// Type alias for pure atom functions that only return values
pub type PureResult = Result<Value, SutraError>;

/// Type alias for functions that return multiple values with world state
pub type MultiValueResult = Result<(Vec<Value>, World), SutraError>;

/// Type alias for functions that return typed arrays with world state
pub type ArrayResult<const N: usize> = Result<([Value; N], World), SutraError>;

/// Type alias for binary operations returning two values and world
pub type BinaryResult = Result<(Value, Value, World), SutraError>;

/// Type alias for validation functions that return unit
pub type ValidationResult = Result<(), SutraError>;

// ============================================================================
// TRAIT-BASED TYPE EXTRACTION
// ============================================================================

/// Trait for extracting typed values from `Value` enum with consistent error handling.
/// This provides a unified interface for type checking and extraction across all atoms.
pub trait ExtractValue<T> {
    /// Extracts a value of type T from this Value.
    fn extract(&self) -> Result<T, SutraError>;
}

impl ExtractValue<f64> for Value {
    fn extract(&self) -> Result<f64, SutraError> {
        match self {
            Value::Number(n) => Ok(*n),
            _ => Err(SutraError::TypeMismatch {
                expected: "number".to_string(),
                actual: self.type_name().to_string(),
                src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
                span: to_source_span(Span::default()),
            }),
        }
    }
}

impl ExtractValue<bool> for Value {
    fn extract(&self) -> Result<bool, SutraError> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(SutraError::TypeMismatch {
                expected: "boolean".to_string(),
                actual: self.type_name().to_string(),
                src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
                span: to_source_span(Span::default()),
            }),
        }
    }
}

impl ExtractValue<Path> for Value {
    fn extract(&self) -> Result<Path, SutraError> {
        match self {
            Value::Path(path) => Ok(path.clone()),
            _ => Err(SutraError::TypeMismatch {
                expected: "path".to_string(),
                actual: self.type_name().to_string(),
                src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
                span: to_source_span(Span::default()),
            }),
        }
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
    ($parent:expr, $world:expr) => {
        $crate::eval::EvaluationContext {
            world: $world,
            output: $parent.output.clone(),
            atom_registry: $parent.atom_registry,
            source: $parent.source.clone(),
            max_depth: $parent.max_depth,
            depth: $parent.depth,
            lexical_env: $parent.lexical_env.clone(),
        }
    };
}

// Re-export for use within this module
pub use sub_eval_context;

// ============================================================================
// ARGUMENT EVALUATION FUNCTIONS
// ============================================================================

/// Evaluates all arguments in sequence, threading world state through each evaluation.
/// This is the fundamental building block for all multi-argument atom operations.
pub fn eval_args(args: &[AstNode], context: &mut EvaluationContext<'_>) -> MultiValueResult {
    args.iter().try_fold(
        (Vec::with_capacity(args.len()), context.world.clone()),
        |(mut values, world), arg| {
            let mut sub_context = sub_eval_context!(context, &world);
            let (val, next_world) = evaluate_ast_node(arg, &mut sub_context)?;
            values.push(val);
            Ok((values, next_world))
        },
    )
}

/// Generic argument evaluation with compile-time arity checking
pub fn eval_n_args<const N: usize>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_>,
) -> ArrayResult<N> {
    if args.len() != N {
        return Err(SutraError::RuntimeGeneral {
            message: format!("expected {} arguments, got {}", N, args.len()),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
    }

    let mut values = Vec::with_capacity(N);
    let mut world = context.world.clone();

    for arg in args.iter().take(N) {
        let mut sub_context = sub_eval_context!(context, &world);
        let (val, next_world) = evaluate_ast_node(arg, &mut sub_context)?;
        values.push(val);
        world = next_world;
    }

    // Convert Vec to array - this is safe because we checked length above
    // The try_into() should never fail given the length check, but we handle it defensively
    let values_array: [Value; N] = values.try_into().map_err(|_| SutraError::Internal {
        issue: "Failed to convert evaluated arguments to array - this should never happen"
            .to_string(),
        details: "eval_n_args failed Vec->array conversion".to_string(),
        context: None,
        src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()).into(),
        span: Some(to_source_span(Span::default())),
        source: None,
    })?;

    Ok((values_array, world))
}

/// Evaluates a single argument and returns the value and world
pub fn eval_single_arg(args: &[AstNode], context: &mut EvaluationContext<'_>) -> AtomResult {
    let ([val], world) = eval_n_args::<1>(args, context)?;
    Ok((val, world))
}

/// Evaluates two arguments and returns both values and the final world
pub fn eval_binary_args(args: &[AstNode], context: &mut EvaluationContext<'_>) -> BinaryResult {
    let ([val1, val2], world) = eval_n_args::<2>(args, context)?;
    Ok((val1, val2, world))
}

// ============================================================================
// TYPE EXTRACTION FUNCTIONS
// ============================================================================

/// Extracts two numbers from values with type checking using the trait.
/// For single value extraction, use val.extract() directly.
pub fn extract_numbers(val1: &Value, val2: &Value) -> Result<(f64, f64), SutraError> {
    let n1 = val1.extract()?;
    let n2 = val2.extract()?;
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
pub fn validate_arity(args: &[Value], expected: usize, atom_name: &str) -> ValidationResult {
    if args.len() != expected {
        let msg = format!(
            "{} expects {} arguments, got {}",
            atom_name,
            expected,
            args.len()
        );
        return Err(SutraError::RuntimeGeneral {
            message: msg,
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
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
) -> ValidationResult {
    if args.len() < min_expected {
        let msg = format!(
            "{} expects at least {} arguments, got {}",
            atom_name,
            min_expected,
            args.len()
        );
        return Err(SutraError::RuntimeGeneral {
            message: msg,
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
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
pub fn validate_unary_arity(args: &[Value], atom_name: &str) -> ValidationResult {
    validate_arity(args, 1, atom_name)
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
pub fn validate_binary_arity(args: &[Value], atom_name: &str) -> ValidationResult {
    validate_arity(args, 2, atom_name)
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
pub fn validate_sequence_arity(args: &[Value], atom_name: &str) -> ValidationResult {
    validate_min_arity(args, 2, atom_name)
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
pub fn validate_even_arity(args: &[Value], atom_name: &str) -> ValidationResult {
    if args.len() % 2 != 0 {
        let msg = format!(
            "{} expects an even number of arguments, got {}",
            atom_name,
            args.len()
        );
        return Err(SutraError::RuntimeGeneral {
            message: msg,
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
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
pub fn validate_zero_arity(args: &[Value], atom_name: &str) -> ValidationResult {
    if !args.is_empty() {
        let msg = format!("{} expects 0 arguments, got {}", atom_name, args.len());
        return Err(SutraError::RuntimeGeneral {
            message: msg,
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
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
) -> ValidationResult {
    if args.len() != expected {
        let msg = format!(
            "{} expects exactly {} arguments, got {}",
            atom_name,
            expected,
            args.len()
        );
        return Err(SutraError::RuntimeGeneral {
            message: msg,
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
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
) -> ValidationResult {
    if args.len() < min_expected {
        let msg = format!(
            "{} expects at least {} arguments, got {}",
            atom_name,
            min_expected,
            args.len()
        );
        return Err(SutraError::RuntimeGeneral {
            message: msg,
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
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
    context: &mut EvaluationContext<'_>,
    op: F,
    validator: Option<V>,
) -> AtomResult
where
    F: Fn(f64, f64) -> Value,
    V: Fn(f64, f64) -> Result<(), &'static str>,
{
    let (val1, val2, world) = eval_binary_args(args, context)?;
    let (n1, n2) = extract_numbers(&val1, &val2)?;

    if let Some(validate) = validator {
        validate(n1, n2).map_err(|msg| SutraError::ValidationGeneral {
            message: msg.to_string(),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        })?;
    }

    Ok((op(n1, n2), world))
}

/// Evaluates an n-ary numeric operation (e.g., sum, product).
/// Handles arity, type checking, and error construction.
pub fn eval_nary_numeric_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_>,
    init: f64,
    fold: F,
) -> AtomResult
where
    F: Fn(f64, f64) -> f64,
{
    if args.len() < 2 {
        return Err(SutraError::RuntimeGeneral {
            message: format!("expected at least 2 arguments, got {}", args.len()),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        });
    }

    let (values, world) = eval_args(args, context)?;
    let mut acc = init;

    for v in values.iter() {
        let n = v.extract().map_err(|_| SutraError::TypeMismatch {
            expected: "number".to_string(),
            actual: v.type_name().to_string(),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
        })?;
        acc = fold(acc, n);
    }

    Ok((Value::Number(acc), world))
}

/// Evaluates a unary boolean operation.
/// Handles arity, type checking, and error construction.
pub fn eval_unary_bool_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_>,
    op: F,
) -> AtomResult
where
    F: Fn(bool) -> Value,
{
    let (val, world) = eval_single_arg(args, context)?;
    let b = val.extract()?;
    Ok((op(b), world))
}

/// Evaluates a unary path operation (get, del).
/// Handles arity, type checking, and error construction.
pub fn eval_unary_path_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_>,
    op: F,
) -> AtomResult
where
    F: Fn(Path, World) -> AtomResult,
{
    let (val, world) = eval_single_arg(args, context)?;
    let path = val.extract()?;
    op(path, world)
}

/// Evaluates a binary path operation (set).
/// Handles arity, type checking, and error construction.
pub fn eval_binary_path_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_>,
    op: F,
) -> AtomResult
where
    F: Fn(Path, Value, World) -> AtomResult,
{
    let (path_val, value, world) = eval_binary_args(args, context)?;
    let path = path_val.extract()?;
    op(path, value, world)
}

/// Evaluates a unary operation that takes any value.
/// Handles arity and error construction.
pub fn eval_unary_value_template<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_>,
    op: F,
) -> AtomResult
where
    F: Fn(Value, World, &mut EvaluationContext<'_>) -> AtomResult,
{
    let (val, world) = eval_single_arg(args, context)?;
    op(val, world, context)
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
    context: &mut EvaluationContext<'_>,
    comparison: F,
    atom_name: &str,
) -> AtomResult
where
    F: Fn(f64, f64) -> bool,
{
    let (values, world) = eval_args(args, context)?;
    validate_sequence_arity(&values, atom_name)?;

    for i in 0..values.len() - 1 {
        let a = values[i].extract()?;
        let b = values[i + 1].extract()?;
        if comparison(a, b) {
            return Ok((Value::Bool(false), world));
        }
    }
    Ok((Value::Bool(true), world))
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
    context: &mut EvaluationContext<'_>,
    init: f64,
    fold: F,
    atom_name: &str,
) -> AtomResult
where
    F: Fn(f64, f64) -> f64,
{
    let (values, world) = eval_args(args, context)?;
    validate_min_arity(&values, 1, atom_name)?;

    let mut result = init;
    for v in values.iter() {
        let n = v.extract()?;
        result = fold(result, n);
    }
    Ok((Value::Number(result), world))
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
    context: &mut EvaluationContext<'_>,
    op: F,
    _atom_name: &str,
) -> AtomResult
where
    Value: ExtractValue<T>,
    F: Fn(T) -> Value,
{
    let (val, world) = eval_single_arg(args, context)?;
    let extracted = val.extract()?;
    Ok((op(extracted), world))
}

// ============================================================================
// PURE FUNCTION HELPERS (for PureAtomFn)
// ============================================================================

/// Evaluates a sequence comparison operation on numbers for pure functions.
/// Handles arity validation, type checking, and the common comparison pattern.
///
/// # Arguments
/// * `args` - The arguments to validate and process
/// * `comparison` - The comparison function (e.g., |a, b| a <= b)
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(Value::Bool)` if comparison succeeds
/// * `Err(SutraError)` if validation fails
///
/// # Example
/// ```ignore
/// let result = pure_eval_numeric_sequence_comparison(
///     args, |a, b| a <= b, "gt?"
/// )?;
/// ```
pub fn pure_eval_numeric_sequence_comparison<F>(
    args: &[Value],
    comparison: F,
    atom_name: &str,
) -> PureResult
where
    F: Fn(f64, f64) -> bool,
{
    validate_sequence_arity(args, atom_name)?;

    for i in 0..args.len() - 1 {
        let a = args[i].extract()?;
        let b = args[i + 1].extract()?;
        if comparison(a, b) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

/// Evaluates an n-ary numeric operation with a custom initial value and fold function for pure functions.
/// Handles arity validation, type checking, and the common fold pattern.
///
/// # Arguments
/// * `args` - The arguments to validate and process
/// * `init` - The initial value for the fold
/// * `fold` - The fold function (e.g., |acc, n| acc + n)
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(Value::Number)` if operation succeeds
/// * `Err(SutraError)` if validation fails
///
/// # Example
/// ```ignore
/// let result = pure_eval_nary_numeric_op_custom(
///     args, 0.0, |acc, n| acc + n, "sum"
/// )?;
/// ```
pub fn pure_eval_nary_numeric_op_custom<F>(
    args: &[Value],
    init: f64,
    fold: F,
    atom_name: &str,
) -> PureResult
where
    F: Fn(f64, f64) -> f64,
{
    validate_min_arity(args, 1, atom_name)?;

    let mut result = init;
    for v in args.iter() {
        let n = v.extract()?;
        result = fold(result, n);
    }
    Ok(Value::Number(result))
}

/// Evaluates a unary operation with type checking using the ExtractValue trait for pure functions.
/// Handles arity validation and provides consistent error messages.
///
/// # Arguments
/// * `args` - The arguments to validate and process
/// * `op` - The operation function
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(Value)` if operation succeeds
/// * `Err(SutraError)` if validation fails
///
/// # Example
/// ```ignore
/// let result = pure_eval_unary_typed_op(
///     args, |b| Value::Bool(!b), "not"
/// )?;
/// ```
pub fn pure_eval_unary_typed_op<T, F>(args: &[Value], op: F, atom_name: &str) -> PureResult
where
    Value: ExtractValue<T>,
    F: Fn(T) -> Value,
{
    validate_unary_arity(args, atom_name)?;
    let extracted = args[0].extract()?;
    Ok(op(extracted))
}

/// Evaluates a string concatenation operation for pure functions.
/// Handles the common pattern of concatenating multiple values into a string.
///
/// # Arguments
/// * `args` - The arguments to concatenate
/// * `atom_name` - The name of the atom for error messages
///
/// # Returns
/// * `Ok(Value::String)` if operation succeeds
/// * `Err(SutraError)` if validation fails
///
/// # Example
/// ```ignore
/// let result = pure_eval_string_concat(args, "str+")?;
/// ```
pub fn pure_eval_string_concat(args: &[Value], _atom_name: &str) -> PureResult {
    let mut result = String::new();
    for arg in args {
        result.push_str(&arg.to_string());
    }
    Ok(Value::String(result))
}

// ============================================================================
// APPLY ATOM HELPERS
// ============================================================================

/// Evaluates normal arguments for apply (all except the last argument).
/// Returns the evaluated arguments as expressions and the final world state.
pub fn eval_apply_normal_args(
    args: &[AstNode],
    context: &mut EvaluationContext<'_>,
) -> Result<(Vec<AstNode>, World), SutraError> {
    let mut evald_args = Vec::with_capacity(args.len());
    let mut world = context.world.clone();
    for arg in args {
        let mut sub_context = sub_eval_context!(context, &world);
        let (val, next_world) = evaluate_ast_node(arg, &mut sub_context)?;
        evald_args.push(Spanned {
            value: Expr::from(val).into(),
            span: arg.span,
        });
        world = next_world;
    }
    Ok((evald_args, world))
}

/// Evaluates the list argument for apply (the last argument).
/// Returns the list items as expressions and the final world state.
pub fn eval_apply_list_arg(
    arg: &AstNode,
    context: &mut EvaluationContext<'_>,
    parent_span: &Span,
) -> Result<(Vec<AstNode>, World), SutraError> {
    let mut sub_context = sub_eval_context!(context, context.world);
    let (list_val, world) = evaluate_ast_node(arg, &mut sub_context)?;
    let Value::List(items) = list_val else {
        return Err(SutraError::TypeMismatch {
            expected: "list".to_string(),
            actual: list_val.type_name().to_string(),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
        });
    };
    let list_items = items
        .into_iter()
        .map(|v| Spanned {
            value: Expr::from(v).into(),
            span: *parent_span,
        })
        .collect();
    Ok((list_items, world))
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
pub fn validate_path_arg<'a>(args: &'a [Value], atom_name: &str) -> Result<&'a Path, SutraError> {
    match &args[0] {
        Value::Path(p) => Ok(p),
        _ => Err(SutraError::RuntimeGeneral {
            message: format!(
                "{} expects a Path as first argument, found {}",
                atom_name,
                args[0].to_string()
            ),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        }),
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
pub fn validate_string_value<'a>(value: &'a Value, context: &str) -> Result<&'a str, SutraError> {
    match value {
        Value::String(s) => Ok(s),
        _ => Err(SutraError::RuntimeGeneral {
            message: format!(
                "Expected String for {}, found {}",
                context,
                value.to_string()
            ),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        }),
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
pub fn validate_list_value_mut<'a>(
    value: &'a mut Value,
    context: &str,
) -> Result<&'a mut Vec<Value>, SutraError> {
    match value {
        Value::List(items) => Ok(items),
        _ => Err(SutraError::RuntimeGeneral {
            message: format!("Expected List for {}, found {}", context, value.to_string()),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        }),
    }
}

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
pub fn validate_list_value<'a>(
    value: &'a Value,
    context: &str,
) -> Result<&'a Vec<Value>, SutraError> {
    match value {
        Value::List(items) => Ok(items),
        _ => Err(SutraError::RuntimeGeneral {
            message: format!("Expected List for {}, found {}", context, value.to_string()),
            src: NamedSource::new("atoms/helpers.rs".to_string(), "".to_string()),
            span: to_source_span(Span::default()),
            suggestion: None,
        }),
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];

        // Should succeed with correct arity
        assert!(validate_arity(&args, 2, "test").is_ok());

        // Should fail with wrong arity
        assert!(validate_arity(&args, 1, "test").is_err());
        assert!(validate_arity(&args, 3, "test").is_err());
    }

    #[test]
    fn test_validate_unary_arity() {
        let args = vec![Value::Number(1.0)];

        // Should succeed with exactly one argument
        assert!(validate_unary_arity(&args, "test").is_ok());

        // Should fail with wrong arity
        assert!(validate_unary_arity(&[], "test").is_err());
        assert!(validate_unary_arity(&[Value::Number(1.0), Value::Number(2.0)], "test").is_err());
    }

    #[test]
    fn test_validate_binary_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];

        // Should succeed with exactly two arguments
        assert!(validate_binary_arity(&args, "test").is_ok());

        // Should fail with wrong arity
        assert!(validate_binary_arity(&[Value::Number(1.0)], "test").is_err());
        assert!(validate_binary_arity(
            &[Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)],
            "test"
        )
        .is_err());
    }

    #[test]
    fn test_validate_min_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];

        // Should succeed with sufficient arguments
        assert!(validate_min_arity(&args, 1, "test").is_ok());
        assert!(validate_min_arity(&args, 2, "test").is_ok());

        // Should fail with insufficient arguments
        assert!(validate_min_arity(&args, 3, "test").is_err());
    }

    #[test]
    fn test_validate_sequence_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];

        // Should succeed with at least two arguments
        assert!(validate_sequence_arity(&args, "test").is_ok());
        assert!(validate_sequence_arity(
            &[Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)],
            "test"
        )
        .is_ok());

        // Should fail with fewer than two arguments
        assert!(validate_sequence_arity(&[], "test").is_err());
        assert!(validate_sequence_arity(&[Value::Number(1.0)], "test").is_err());
    }

    #[test]
    fn test_validate_even_arity() {
        let args = vec![Value::Number(1.0), Value::Number(2.0)];

        // Should succeed with even number of arguments (including 0)
        assert!(validate_even_arity(&[], "test").is_ok());
        assert!(validate_even_arity(&args, "test").is_ok());
        assert!(validate_even_arity(
            &[
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0),
                Value::Number(4.0)
            ],
            "test"
        )
        .is_ok());

        // Should fail with odd number of arguments
        assert!(validate_even_arity(&[Value::Number(1.0)], "test").is_err());
        assert!(validate_even_arity(
            &[Value::Number(1.0), Value::Number(2.0), Value::Number(3.0)],
            "test"
        )
        .is_err());
    }

    #[test]
    fn test_error_messages() {
        let args = vec![Value::Number(1.0)];

        // Test that error messages are descriptive
        let err = validate_arity(&args, 2, "my_atom").unwrap_err();
        assert!(err
            .to_string()
            .contains("my_atom expects 2 arguments, got 1"));

        let err = validate_min_arity(&args, 3, "my_atom").unwrap_err();
        assert!(err
            .to_string()
            .contains("my_atom expects at least 3 arguments, got 1"));

        let err = validate_even_arity(&args, "my_atom").unwrap_err();
        assert!(err
            .to_string()
            .contains("my_atom expects an even number of arguments, got 1"));
    }

    #[test]
    fn test_validate_zero_arity() {
        let args = vec![Value::Number(1.0)];

        // Should fail with any arguments
        assert!(validate_zero_arity(&args, "test").is_err());
        assert!(validate_zero_arity(&[Value::Number(1.0), Value::Number(2.0)], "test").is_err());

        // Should succeed with no arguments
        assert!(validate_zero_arity(&[], "test").is_ok());
    }

    #[test]
    fn test_validate_special_form_arity() {
        let span = Span { start: 0, end: 1 };
        let args = vec![AstNode {
            value: std::sync::Arc::new(Expr::Number(1.0, span)),
            span,
        }];

        // Should succeed with correct arity
        assert!(validate_special_form_arity(&args, 1, "test").is_ok());

        // Should fail with wrong arity
        assert!(validate_special_form_arity(&args, 2, "test").is_err());
        assert!(validate_special_form_arity(&[], 1, "test").is_err());
    }

    #[test]
    fn test_validate_special_form_min_arity() {
        let span = Span { start: 0, end: 1 };
        let args = vec![AstNode {
            value: std::sync::Arc::new(Expr::Number(1.0, span)),
            span,
        }];

        // Should succeed with sufficient arguments
        assert!(validate_special_form_min_arity(&args, 1, "test").is_ok());
        assert!(validate_special_form_min_arity(&args, 0, "test").is_ok());

        // Should fail with insufficient arguments
        assert!(validate_special_form_min_arity(&args, 2, "test").is_err());
    }
}
