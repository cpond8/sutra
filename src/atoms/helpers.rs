//! # Atom Helper Infrastructure
//!
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

use crate::ast::value::Value;
use crate::ast::AstNode;
use crate::ast::{Expr, WithSpan};
use crate::runtime::eval::{eval_expr, EvalContext};
use crate::syntax::error::SutraError;
use crate::syntax::error::{eval_arity_error, eval_general_error, eval_type_error};

// ============================================================================
// TYPE ALIASES AND CORE TYPES
// ============================================================================

/// Convenient type alias for atom return values - modern Rust idiom
pub type AtomResult = Result<(Value, crate::runtime::world::World), SutraError>;

// ============================================================================
// TRAIT-BASED TYPE EXTRACTION
// ============================================================================

/// Trait for extracting typed values from `Value` enum with consistent error handling.
/// This provides a unified interface for type checking and extraction across all atoms.
pub trait ExtractValue<T> {
    /// Extracts a value of type T from this Value, providing detailed error context.
    fn extract(
        &self,
        args: &[AstNode],
        arg_index: usize,
        parent_span: &crate::ast::Span,
        name: &str,
        expected_type: &str,
    ) -> Result<T, SutraError>;
}

impl ExtractValue<f64> for Value {
    fn extract(
        &self,
        args: &[AstNode],
        arg_index: usize,
        parent_span: &crate::ast::Span,
        name: &str,
        expected_type: &str,
    ) -> Result<f64, SutraError> {
        match self {
            Value::Number(n) => Ok(*n),
            _ => Err(type_error(
                Some(parent_span.clone()),
                &args[arg_index],
                name,
                expected_type,
                self,
            )),
        }
    }
}

impl ExtractValue<bool> for Value {
    fn extract(
        &self,
        args: &[AstNode],
        arg_index: usize,
        parent_span: &crate::ast::Span,
        name: &str,
        expected_type: &str,
    ) -> Result<bool, SutraError> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(type_error(
                Some(parent_span.clone()),
                &args[arg_index],
                name,
                expected_type,
                self,
            )),
        }
    }
}

impl ExtractValue<crate::runtime::path::Path> for Value {
    fn extract(
        &self,
        args: &[AstNode],
        arg_index: usize,
        parent_span: &crate::ast::Span,
        name: &str,
        expected_type: &str,
    ) -> Result<crate::runtime::path::Path, SutraError> {
        match self {
            Value::Path(path) => Ok(path.clone()),
            _ => Err(type_error(
                Some(parent_span.clone()),
                &args[arg_index],
                name,
                expected_type,
                self,
            )),
        }
    }
}

// ============================================================================
// ERROR CONSTRUCTION UTILITIES
// ============================================================================

/// Creates an arity error for atoms with consistent messaging
pub fn arity_error(
    span: Option<crate::ast::Span>,
    args: &[AstNode],
    name: &str,
    expected: impl ToString,
) -> SutraError {
    eval_arity_error(span, args, name, expected)
}

/// Creates a type error for atoms with consistent messaging
pub fn type_error(
    span: Option<crate::ast::Span>,
    arg: &AstNode,
    name: &str,
    expected: &str,
    found: &Value,
) -> SutraError {
    eval_type_error(span, arg, name, expected, found)
}

/// Creates a validation error for atoms with consistent messaging
pub fn validation_error(
    span: Option<crate::ast::Span>,
    arg: &AstNode,
    message: &str,
) -> SutraError {
    eval_general_error(span, arg, message)
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
/// let (result, world) = eval_expr(&args[0], &mut sub_context)?;
/// ```
#[macro_export]
macro_rules! sub_eval_context {
    ($parent:expr, $world:expr) => {
        $crate::runtime::eval::EvalContext {
            world: $world,
            output: $parent.output,
            atom_registry: $parent.atom_registry,
            max_depth: $parent.max_depth,
            depth: $parent.depth,
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
pub fn eval_args(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
) -> Result<(Vec<Value>, crate::runtime::world::World), SutraError> {
    args.iter().try_fold(
        (Vec::with_capacity(args.len()), context.world.clone()),
        |(mut values, world), arg| {
            let mut sub_context = sub_eval_context!(context, &world);
            let (val, next_world) = eval_expr(arg, &mut sub_context)?;
            values.push(val);
            Ok((values, next_world))
        },
    )
}

/// Generic argument evaluation with compile-time arity checking
pub fn eval_n_args<const N: usize>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<([Value; N], crate::runtime::world::World), SutraError> {
    if args.len() != N {
        return Err(arity_error(Some(parent_span.clone()), args, name, N));
    }

    let mut values = Vec::with_capacity(N);
    let mut world = context.world.clone();

    for arg in args.iter().take(N) {
        let mut sub_context = sub_eval_context!(context, &world);
        let (val, next_world) = eval_expr(arg, &mut sub_context)?;
        values.push(val);
        world = next_world;
    }

    // Convert Vec to array - this is safe because we checked length
    let values_array: [Value; N] = values
        .try_into()
        .map_err(|_| arity_error(Some(parent_span.clone()), args, name, N))?;

    Ok((values_array, world))
}

/// Evaluates a single argument and returns the value and world
pub fn eval_single_arg(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError> {
    let ([val], world) = eval_n_args::<1>(args, context, parent_span, name)?;
    Ok((val, world))
}

/// Evaluates two arguments and returns both values and the final world
pub fn eval_binary_args(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<(Value, Value, crate::runtime::world::World), SutraError> {
    let ([val1, val2], world) = eval_n_args::<2>(args, context, parent_span, name)?;
    Ok((val1, val2, world))
}

// ============================================================================
// TYPE EXTRACTION FUNCTIONS
// ============================================================================

/// Extracts two numbers from values with type checking using the trait
pub fn extract_numbers(
    val1: &Value,
    val2: &Value,
    args: &[AstNode],
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<(f64, f64), SutraError> {
    let n1 = val1.extract(args, 0, parent_span, name, "a Number")?;
    let n2 = val2.extract(args, 1, parent_span, name, "a Number")?;
    Ok((n1, n2))
}

/// Extracts a single number from a value with type checking using the trait
pub fn extract_number(
    val: &Value,
    args: &[AstNode],
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<f64, SutraError> {
    val.extract(args, 0, parent_span, name, "a Number")
}

/// Extracts a boolean from a value with type checking using the trait
pub fn extract_bool(
    val: &Value,
    args: &[AstNode],
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<bool, SutraError> {
    val.extract(args, 0, parent_span, name, "a Boolean")
}

/// Extracts a path from a value with type checking using the trait
pub fn extract_path(
    val: &Value,
    args: &[AstNode],
    parent_span: &crate::ast::Span,
    name: &str,
) -> Result<crate::runtime::path::Path, SutraError> {
    val.extract(args, 0, parent_span, name, "a Path")
}

// ============================================================================
// OPERATION EVALUATION TEMPLATES
// ============================================================================

/// Evaluates a binary numeric operation atomically, with optional validation.
/// Handles arity, type checking, and error construction.
pub fn eval_binary_numeric_op<F, V>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    validator: Option<V>,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(f64, f64) -> Value,
    V: Fn(f64, f64) -> Result<(), &'static str>,
{
    let (val1, val2, world) = eval_binary_args(args, context, parent_span, name)?;
    let (n1, n2) = extract_numbers(&val1, &val2, args, parent_span, name)?;

    if let Some(validate) = validator {
        validate(n1, n2)
            .map_err(|msg| validation_error(Some(parent_span.clone()), &args[1], msg))?;
    }

    Ok((op(n1, n2), world))
}

/// Evaluates an n-ary numeric operation (e.g., sum, product).
/// Handles arity, type checking, and error construction.
pub fn eval_nary_numeric_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    init: f64,
    fold: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(f64, f64) -> f64,
{
    if args.len() < 2 {
        return Err(arity_error(
            Some(parent_span.clone()),
            args,
            name,
            "at least 2",
        ));
    }

    let (values, world) = eval_args(args, context)?;
    let mut acc = init;

    for (i, v) in values.iter().enumerate() {
        let n = extract_number(v, args, parent_span, name)
            .map_err(|_| type_error(Some(parent_span.clone()), &args[i], name, "a Number", v))?;
        acc = fold(acc, n);
    }

    Ok((Value::Number(acc), world))
}

/// Evaluates a unary boolean operation.
/// Handles arity, type checking, and error construction.
pub fn eval_unary_bool_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(bool) -> Value,
{
    let (val, world) = eval_single_arg(args, context, parent_span, name)?;
    let b = extract_bool(&val, args, parent_span, name)?;
    Ok((op(b), world))
}

/// Evaluates a unary path operation (get, del).
/// Handles arity, type checking, and error construction.
pub fn eval_unary_path_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        crate::runtime::path::Path,
        crate::runtime::world::World,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (val, world) = eval_single_arg(args, context, parent_span, name)?;
    let path = extract_path(&val, args, parent_span, name)?;
    op(path, world)
}

/// Evaluates a binary path operation (set).
/// Handles arity, type checking, and error construction.
pub fn eval_binary_path_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        crate::runtime::path::Path,
        Value,
        crate::runtime::world::World,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (path_val, value, world) = eval_binary_args(args, context, parent_span, name)?;
    let path = extract_path(&path_val, args, parent_span, name)?;
    op(path, value, world)
}

/// Evaluates a unary operation that takes any value.
/// Handles arity and error construction.
pub fn eval_unary_value_op<F>(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
    op: F,
    name: &str,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        Value,
        crate::runtime::world::World,
        &crate::ast::Span,
        &mut EvalContext<'_, '_>,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (val, world) = eval_single_arg(args, context, parent_span, name)?;
    op(val, world, parent_span, context)
}

// ============================================================================
// APPLY ATOM HELPERS
// ============================================================================

/// Evaluates normal arguments for apply (all except the last argument).
/// Returns the evaluated arguments as expressions and the final world state.
pub fn eval_apply_normal_args(
    args: &[AstNode],
    context: &mut EvalContext<'_, '_>,
) -> Result<(Vec<AstNode>, crate::runtime::world::World), SutraError> {
    let mut evald_args = Vec::with_capacity(args.len());
    let mut world = context.world.clone();
    for arg in args {
        let mut sub_context = sub_eval_context!(context, &world);
        let (val, next_world) = eval_expr(arg, &mut sub_context)?;
        evald_args.push(WithSpan {
            value: Expr::from(val).into(), // FIX: wrap Expr in Arc via .into()
            span: arg.span.clone(),
        });
        world = next_world;
    }
    Ok((evald_args, world))
}

/// Evaluates the list argument for apply (the last argument).
/// Returns the list items as expressions and the final world state.
pub fn eval_apply_list_arg(
    arg: &AstNode,
    context: &mut EvalContext<'_, '_>,
    parent_span: &crate::ast::Span,
) -> Result<(Vec<AstNode>, crate::runtime::world::World), SutraError> {
    let mut sub_context = sub_eval_context!(context, context.world);
    let (list_val, world) = eval_expr(arg, &mut sub_context)?;
    let Value::List(items) = list_val else {
        return Err(type_error(
            Some(parent_span.clone()),
            arg,
            "apply",
            "a List as the last argument",
            &list_val,
        ));
    };
    let list_items = items
        .into_iter()
        .map(|v| WithSpan {
            value: Expr::from(v).into(), // FIX: wrap Expr in Arc via .into()
            span: parent_span.clone(),
        })
        .collect();
    Ok((list_items, world))
}

/// Builds the call expression for apply by combining function, normal args, and list args.
pub fn build_apply_call_expr(
    func_expr: &AstNode,
    normal_args: Vec<AstNode>,
    list_args: Vec<AstNode>,
    parent_span: &crate::ast::Span,
) -> AstNode {
    let mut call_items = Vec::with_capacity(1 + normal_args.len() + list_args.len());
    call_items.push(func_expr.clone());
    call_items.extend(normal_args);
    call_items.extend(list_args);
    WithSpan {
        value: Expr::List(call_items, parent_span.clone()).into(), // FIX: wrap Expr in Arc via .into()
        span: parent_span.clone(),
    }
}
