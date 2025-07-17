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
use crate::ast::{Expr, Spanned};
use crate::runtime::eval::{evaluate_ast_node, EvaluationContext};
use crate::SutraError;
use crate::err_msg;

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
    /// Extracts a value of type T from this Value.
    fn extract(&self) -> Result<T, SutraError>;
}

impl ExtractValue<f64> for Value {
    fn extract(&self) -> Result<f64, SutraError> {
        match self {
            Value::Number(n) => Ok(*n),
            _ => Err(err_msg!(TypeError, "Type error")),
        }
    }
}

impl ExtractValue<bool> for Value {
    fn extract(&self) -> Result<bool, SutraError> {
        match self {
            Value::Bool(b) => Ok(*b),
            _ => Err(err_msg!(TypeError, "Type error")),
        }
    }
}

impl ExtractValue<crate::runtime::path::Path> for Value {
    fn extract(&self) -> Result<crate::runtime::path::Path, SutraError> {
        match self {
            Value::Path(path) => Ok(path.clone()),
            _ => Err(err_msg!(TypeError, "Type error")),
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
        $crate::runtime::eval::EvaluationContext {
            world: $world,
            output: $parent.output,
            atom_registry: $parent.atom_registry,
            source: $parent.source.clone(),
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
    context: &mut EvaluationContext<'_, '_>,
) -> Result<(Vec<Value>, crate::runtime::world::World), SutraError> {
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
    context: &mut EvaluationContext<'_, '_>,
) -> Result<([Value; N], crate::runtime::world::World), SutraError> {
    if args.len() != N {
        return Err(err_msg!(Eval, "Arity error"));
    }

    let mut values = Vec::with_capacity(N);
    let mut world = context.world.clone();

    for arg in args.iter().take(N) {
        let mut sub_context = sub_eval_context!(context, &world);
        let (val, next_world) = evaluate_ast_node(arg, &mut sub_context)?;
        values.push(val);
        world = next_world;
    }

    // Convert Vec to array - this is safe because we checked length
    let values_array: [Value; N] = values
        .try_into()
        .map_err(|_| err_msg!(Eval, "Arity error"))?;

    Ok((values_array, world))
}

/// Evaluates a single argument and returns the value and world
pub fn eval_single_arg(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
) -> Result<(Value, crate::runtime::world::World), SutraError> {
    let ([val], world) = eval_n_args::<1>(args, context)?;
    Ok((val, world))
}

/// Evaluates two arguments and returns both values and the final world
pub fn eval_binary_args(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
) -> Result<(Value, Value, crate::runtime::world::World), SutraError> {
    let ([val1, val2], world) = eval_n_args::<2>(args, context)?;
    Ok((val1, val2, world))
}

// ============================================================================
// TYPE EXTRACTION FUNCTIONS
// ============================================================================

/// Extracts two numbers from values with type checking using the trait
pub fn extract_numbers(
    val1: &Value,
    val2: &Value,
) -> Result<(f64, f64), SutraError> {
    let n1 = val1.extract()?;
    let n2 = val2.extract()?;
    Ok((n1, n2))
}

/// Extracts a single number from a value with type checking using the trait
pub fn extract_number(val: &Value) -> Result<f64, SutraError> {
    val.extract()
}

/// Extracts a boolean from a value with type checking using the trait
pub fn extract_bool(val: &Value) -> Result<bool, SutraError> {
    val.extract()
}

/// Extracts a path from a value with type checking using the trait
pub fn extract_path(val: &Value) -> Result<crate::runtime::path::Path, SutraError> {
    val.extract()
}

// ============================================================================
// OPERATION EVALUATION TEMPLATES
// ============================================================================

/// Evaluates a binary numeric operation atomically, with optional validation.
/// Handles arity, type checking, and error construction.
pub fn eval_binary_numeric_op<F, V>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
    op: F,
    validator: Option<V>,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(f64, f64) -> Value,
    V: Fn(f64, f64) -> Result<(), &'static str>,
{
    let (val1, val2, world) = eval_binary_args(args, context)?;
    let (n1, n2) = extract_numbers(&val1, &val2)?;

    if let Some(validate) = validator {
        validate(n1, n2)
            .map_err(|msg| err_msg!(Validation, msg))?;
    }

    Ok((op(n1, n2), world))
}

/// Evaluates an n-ary numeric operation (e.g., sum, product).
/// Handles arity, type checking, and error construction.
pub fn eval_nary_numeric_op<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
    init: f64,
    fold: F,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(f64, f64) -> f64,
{
    if args.len() < 2 {
        return Err(err_msg!(Eval, "Arity error"));
    }

    let (values, world) = eval_args(args, context)?;
    let mut acc = init;

    for v in values.iter() {
        let n = extract_number(v)
            .map_err(|_| err_msg!(TypeError, "Type error"))?;
        acc = fold(acc, n);
    }

    Ok((Value::Number(acc), world))
}

/// Evaluates a unary boolean operation.
/// Handles arity, type checking, and error construction.
pub fn eval_unary_bool_op<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
    op: F,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(bool) -> Value,
{
    let (val, world) = eval_single_arg(args, context)?;
    let b = extract_bool(&val)?;
    Ok((op(b), world))
}

/// Evaluates a unary path operation (get, del).
/// Handles arity, type checking, and error construction.
pub fn eval_unary_path_op<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
    op: F,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        crate::runtime::path::Path,
        crate::runtime::world::World,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (val, world) = eval_single_arg(args, context)?;
    let path = extract_path(&val)?;
    op(path, world)
}

/// Evaluates a binary path operation (set).
/// Handles arity, type checking, and error construction.
pub fn eval_binary_path_op<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
    op: F,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        crate::runtime::path::Path,
        Value,
        crate::runtime::world::World,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (path_val, value, world) = eval_binary_args(args, context)?;
    let path = extract_path(&path_val)?;
    op(path, value, world)
}

/// Evaluates a unary operation that takes any value.
/// Handles arity and error construction.
pub fn eval_unary_value_op<F>(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
    op: F,
) -> Result<(Value, crate::runtime::world::World), SutraError>
where
    F: Fn(
        Value,
        crate::runtime::world::World,
        &mut EvaluationContext<'_, '_>,
    ) -> Result<(Value, crate::runtime::world::World), SutraError>,
{
    let (val, world) = eval_single_arg(args, context)?;
    op(val, world, context)
}

// ============================================================================
// APPLY ATOM HELPERS
// ============================================================================

/// Evaluates normal arguments for apply (all except the last argument).
/// Returns the evaluated arguments as expressions and the final world state.
pub fn eval_apply_normal_args(
    args: &[AstNode],
    context: &mut EvaluationContext<'_, '_>,
) -> Result<(Vec<AstNode>, crate::runtime::world::World), SutraError> {
    let mut evald_args = Vec::with_capacity(args.len());
    let mut world = context.world.clone();
    for arg in args {
        let mut sub_context = sub_eval_context!(context, &world);
        let (val, next_world) = evaluate_ast_node(arg, &mut sub_context)?;
        evald_args.push(Spanned {
            value: Expr::from(val).into(), // FIX: wrap Expr in Arc via .into()
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
    context: &mut EvaluationContext<'_, '_>,
    parent_span: &crate::ast::Span,
) -> Result<(Vec<AstNode>, crate::runtime::world::World), SutraError> {
    let mut sub_context = sub_eval_context!(context, context.world);
    let (list_val, world) = evaluate_ast_node(arg, &mut sub_context)?;
    let Value::List(items) = list_val else {
        return Err(err_msg!(TypeError, "Type error"));
    };
    let list_items = items
        .into_iter()
        .map(|v| Spanned {
            value: Expr::from(v).into(), // FIX: wrap Expr in Arc via .into()
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
    parent_span: &crate::ast::Span,
) -> AstNode {
    let mut call_items = Vec::with_capacity(1 + normal_args.len() + list_args.len());
    call_items.push(func_expr.clone());
    call_items.extend(normal_args);
    call_items.extend(list_args);
    Spanned {
        value: Expr::List(call_items, *parent_span).into(), // FIX: wrap Expr in Arc via .into()
        span: *parent_span,
    }
}
