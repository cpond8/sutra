//! # Sutra Test Atom Library
//!
//! This module provides test-only atoms for debugging, testing, and development.
//! These atoms are only available when compiled with debug assertions or the `test-atom` feature.
//!
//! ## Test Atom Contracts
//!
//! Test atoms follow the same contracts as standard atoms but may have additional
//! behaviors for testing purposes:
//! - May emit debug output for verification
//! - May stress-test internal systems (borrowing, recursion, etc.)
//! - May have non-standard return values for testing edge cases

use crate::ast::value::Value;
use crate::ast::{Expr, Span, WithSpan};
use crate::atoms::AtomRegistry;
use crate::runtime::eval::EvalContext;
use crate::runtime::world::World;
use crate::syntax::error::SutraError;

// Use the public context helper macro
use crate::sub_eval_context;

/// Simple echo atom that outputs its first argument.
///
/// Usage: (test/echo <value>)
/// - <value>: Any value to echo
///     Returns: The echoed value
///
/// Example:
///   (test/echo "hello") ; => "hello" (also emits "hello")
///
/// # Safety
/// Emits output, does not mutate world state.
fn test_echo_atom(
    args: &[WithSpan<Expr>],
    ctx: &mut EvalContext,
    span: &Span,
) -> Result<(Value, World), SutraError> {
    let Some(first) = args.first() else {
        let val = Value::String("".to_string());
        let world = ctx.world.clone();
        ctx.output.emit(&val.to_string(), Some(span));
        return Ok((val, world));
    };
    let val = match &first.value {
        Expr::String(s, _) => Value::String(s.clone()),
        _ => Value::String(format!("{:?}", first.value)),
    };
    let world = ctx.world.clone();
    ctx.output.emit(&val.to_string(), Some(span));
    Ok((val, world))
}

/// Borrow checker/context management stress test atom.
///
/// Usage: (test/borrow_stress <depth:int> <msg:string>)
/// - <depth>: Recursion depth (integer)
/// - <msg>: Message to echo (string)
///
/// Behavior:
/// - Emits output before and after a nested call to itself (if depth > 0)
/// - Calls `test/echo` at the base case (depth = 0)
/// - Returns a string showing the recursion depth and message
///
/// Example:
///   (test/borrow_stress 2 "test") ; => "depth:2;msg:test"
///
/// This atom is designed to stress borrow splitting, nested calls, and output ordering.
///
/// # Safety
/// Emits output, does not mutate world state. May recurse up to max_depth.
// Type alias for test atom function signatures
type TestAtomFn =
    fn(&[WithSpan<Expr>], &mut EvalContext, &Span) -> Result<(Value, World), SutraError>;

/// Parse arguments for borrow stress test.
fn parse_borrow_stress_args(args: &[WithSpan<Expr>]) -> (i64, String) {
    let first = args.first();
    let second = args.get(1);
    match (first, second) {
        (Some(d), Some(m)) => {
            let d = match &d.value {
                Expr::Number(n, _) => *n as i64,
                _ => 0,
            };
            let m = match &m.value {
                Expr::String(s, _) => s.clone(),
                _ => format!("{:?}", m.value),
            };
            (d, m)
        }
        _ => (0, "default".to_string()),
    }
}

/// Emit formatted output for borrow stress test phases.
fn emit_borrow_stress_output(
    ctx: &mut EvalContext,
    depth: i64,
    msg: &str,
    span: &Span,
    phase: &str,
) {
    ctx.output
        .emit(&format!("[{}:{}:{}]", phase, depth, msg), Some(span));
}

/// Handle recursive case of borrow stress test.
fn handle_borrow_stress_recursion(
    ctx: &mut EvalContext,
    depth: i64,
    msg: &str,
    span: &Span,
    test_borrow_stress_atom: TestAtomFn,
    test_echo_atom: TestAtomFn,
) -> Result<(Value, World), SutraError> {
    if depth == 0 {
        return handle_borrow_stress_base_case(ctx, msg, span, test_echo_atom);
    }
    let mut sub_context = sub_eval_context!(ctx, ctx.world);
    sub_context.depth = ctx.depth + 1; // Manually set incremented depth
    let nested_args = vec![
        WithSpan {
            value: Expr::Number((depth - 1) as f64, span.clone()),
            span: span.clone(),
        },
        WithSpan {
            value: Expr::String(msg.to_string(), span.clone()),
            span: span.clone(),
        },
    ];
    test_borrow_stress_atom(&nested_args, &mut sub_context, span)
}

/// Handle base case of borrow stress test (calls echo).
fn handle_borrow_stress_base_case(
    ctx: &mut EvalContext,
    msg: &str,
    span: &Span,
    test_echo_atom: TestAtomFn,
) -> Result<(Value, World), SutraError> {
    let mut sub_context = sub_eval_context!(ctx, ctx.world);
    sub_context.depth = ctx.depth + 1; // Manually set incremented depth
    let echo_arg = WithSpan {
        value: Expr::String(msg.to_string(), span.clone()),
        span: span.clone(),
    };
    test_echo_atom(&[echo_arg], &mut sub_context, span)
}

/// Main borrow stress test atom implementation.
fn test_borrow_stress_atom(
    args: &[WithSpan<Expr>],
    ctx: &mut EvalContext,
    span: &Span,
) -> Result<(Value, World), SutraError> {
    let (depth, msg) = parse_borrow_stress_args(args);
    emit_borrow_stress_output(ctx, depth, &msg, span, "before");
    let (_result, world) = handle_borrow_stress_recursion(
        ctx,
        depth,
        &msg,
        span,
        test_borrow_stress_atom,
        test_echo_atom,
    )?;
    emit_borrow_stress_output(ctx, depth, &msg, span, "after");
    Ok((Value::String(format!("depth:{};msg:{}", depth, msg)), world))
}

/// Registers all test atoms in the given registry.
///
/// This function should only be called in debug or test builds.
/// It registers atoms used for testing, debugging, and development purposes.
///
/// # Safety
/// Test atoms may have side effects (output, recursion) intended for testing.
pub fn register_test_atoms(registry: &mut AtomRegistry) {
    registry.register("test/echo", test_echo_atom);
    registry.register("test/borrow_stress", test_borrow_stress_atom);
}
