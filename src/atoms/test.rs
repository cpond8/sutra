//! Test utility atoms for the Sutra testing system.
//!
//! The main testing infrastructure is in TestRunner and discovery.rs.
//! This module provides utility atoms for use within test bodies.
//!
//! ## Atoms Provided
//!
//! - **Test Assertions**: `assert`, `assert-eq`
//! - **Test Utilities**: `test/echo`

use crate::prelude::*;
use crate::{
    errors::{to_source_span, ErrorReporting},
    register_atom,
    runtime::{evaluate_ast_node, EvaluationContext, SpannedResult, SpannedValue},
    syntax::{AstNode, Expr, Span},
};

/// Simple assertion - fails if argument is not truthy
fn assert_atom(args: &[AstNode], ctx: &mut EvaluationContext, call_span: &Span) -> SpannedResult {
    if args.len() != 1 {
        return Err(ctx.arity_mismatch("1", args.len(), to_source_span(*call_span)));
    }
    let value_sv = evaluate_ast_node(&args[0], ctx)?;
    if !value_sv.value.is_truthy() {
        return Err(ctx.invalid_operation(
            "assertion",
            &format!("expected truthy value, got {}", value_sv.value),
            to_source_span(value_sv.span),
        ));
    }
    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
}

/// Equality assertion - fails if arguments are not equal
fn assert_eq_atom(
    args: &[AstNode],
    ctx: &mut EvaluationContext,
    call_span: &Span,
) -> SpannedResult {
    if args.len() != 2 {
        return Err(ctx.arity_mismatch("2", args.len(), to_source_span(*call_span)));
    }
    let expected_sv = evaluate_ast_node(&args[0], ctx)?;
    let actual_sv = evaluate_ast_node(&args[1], ctx)?;
    if expected_sv.value != actual_sv.value {
        return Err(ctx.invalid_operation(
            "assertion equality",
            &format!("expected {}, got {}", expected_sv.value, actual_sv.value),
            to_source_span(*call_span),
        ));
    }
    Ok(SpannedValue {
        value: Value::Nil,
        span: *call_span,
    })
}

/// Debug output utility for tests
fn test_echo_atom(
    args: &[AstNode],
    ctx: &mut EvaluationContext,
    call_span: &Span,
) -> SpannedResult {
    let output_str = if let Some(first) = args.first() {
        match &*first.value {
            Expr::String(s, _) => s.clone(),
            _ => format!("{}", first.value),
        }
    } else {
        String::new()
    };

    ctx.output.borrow_mut().emit(&output_str, Some(call_span));
    Ok(SpannedValue {
        value: Value::String(output_str),
        span: *call_span,
    })
}

pub fn register_test_atoms(world: &mut World) {
    register_atom!(world, "assert", assert_atom);
    register_atom!(world, "assert-eq", assert_eq_atom);
    register_atom!(world, "test/echo", test_echo_atom);
}
