//!
//! This module provides test-related atoms for the Sutra engine.
//! These atoms support test registration, execution, and assertion.
//!
//! ## Atoms Provided
//!
//! - **Test Registration**: `register-test!`
//! - **Test Assertions**: `value`, `tags`
//! - **Test Utilities**: `test/echo`
//!
//! ## Design Principles
//!
//! - **AST Storage**: Tests are stored as AST nodes to preserve source context
//! - **Diagnostic Support**: Rich error reporting with source locations
//! - **Global Registry**: Centralized test storage for discovery and execution

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use lazy_static::lazy_static;
use miette::NamedSource;

use crate::{
    ast::{AstNode, Expr, Span, Spanned},
    atoms::{helpers, helpers::AtomResult},
    engine::{evaluate_ast_node, EvaluationContext},
    errors::{to_source_span, ErrorReporting, SourceContext, SutraError},
    prelude::*,
};

use crate::atoms::helpers::sub_eval_context;
use crate::register_lazy;

/// Represents a single test case definition with source context for diagnostics.
#[derive(Debug, Clone)]
pub struct TestDefinition {
    pub name: String,
    pub expect: AstNode,
    pub body: Vec<AstNode>,
    pub span: Span,
    pub source_file: Arc<NamedSource<String>>,
    pub source_text: String,
}

/// Represents the outcome of a single test execution.
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub details: String,
}

lazy_static! {
    pub static ref TEST_REGISTRY: Mutex<HashMap<String, TestDefinition>> =
        Mutex::new(HashMap::new());
}

/// `register-test!` special form updated for AST storage.
fn register_test_atom(args: &[AstNode], ctx: &mut EvaluationContext, span: &Span) -> AtomResult {
    if args.len() < 4 {
        return Err(ctx.arity_mismatch("at least 4", args.len(), to_source_span(*span)));
    }

    let name = match &*args[0].value {
        Expr::String(s, _) => s.clone(),
        _ => {
            return Err(ctx.type_mismatch(
                "String",
                args[0].value.type_name(),
                to_source_span(args[0].span),
            ));
        }
    };

    let expect = args[1].clone();
    let body = args[2..args.len() - 1].to_vec();

    let metadata_val = evaluate_ast_node(&args[args.len() - 1], ctx)?;
    let metadata = match metadata_val.as_map() {
        Some(m) => m,
        _ => {
            return Err(ctx.type_mismatch(
                "Map",
                metadata_val.type_name(),
                to_source_span(args[args.len() - 1].span),
            ));
        }
    };

    let (source_text, source_file) = match metadata.get(":source-file") {
        Some(Value::String(file_path)) => {
            let source = match std::fs::read_to_string(file_path) {
                Ok(s) => s,
                Err(e) => {
                    return Err(ctx.invalid_operation(
                        "reading source file",
                        &e.to_string(),
                        to_source_span(Span::default()),
                    ))
                }
            };
            (
                source.clone(),
                Arc::new(NamedSource::new(file_path.clone(), source)),
            )
        }
        _ => {
            return Err(ctx.invalid_operation(
                "test metadata validation",
                "metadata must contain :source-file string",
                to_source_span(args[args.len() - 1].span),
            ));
        }
    };

    let test_def = TestDefinition {
        name: name.clone(),
        expect,
        body,
        span: *span,
        source_file,
        source_text,
    };

    let mut registry = TEST_REGISTRY.lock().map_err(|_| {
        ctx.invalid_operation(
            "test registry access",
            "mutex poisoned",
            to_source_span(Span::default()),
        )
    })?;
    registry.insert(name, test_def);

    Ok(Value::Nil)
}

fn value_atom(args: &[AstNode], _ctx: &mut EvaluationContext, _span: &Span) -> AtomResult {
    let Some(first) = args.first() else {
        return Ok(Value::Nil);
    };
    evaluate_ast_node(first, _ctx)
}

fn tags_atom(args: &[AstNode], ctx: &mut EvaluationContext, span: &Span) -> AtomResult {
    let mut list_expr_items = Vec::with_capacity(args.len() + 1);
    list_expr_items.push(AstNode {
        value: Arc::new(Expr::Symbol("list".to_string(), *span)),
        span: *span,
    });
    list_expr_items.extend(args.iter().cloned());
    let list_expr = AstNode {
        value: Arc::new(Expr::List(list_expr_items, *span)),
        span: *span,
    };
    evaluate_ast_node(&list_expr, ctx)
}

fn test_echo_atom(args: &[AstNode], ctx: &mut EvaluationContext, span: &Span) -> AtomResult {
    let Some(first) = args.first() else {
        let val = Value::String("".to_string());
        ctx.output.borrow_mut().emit(&val.to_string(), Some(span));
        return Ok(val);
    };
    let val = match &*first.value {
        Expr::String(s, _) => Value::String(s.clone()),
        _ => Value::String(format!("{}", first.value)),
    };
    ctx.output.borrow_mut().emit(&val.to_string(), Some(span));
    Ok(val)
}

fn parse_borrow_stress_args(args: &[AstNode]) -> (i64, String) {
    let first = args.first();
    let second = args.get(1);
    match (first, second) {
        (Some(d), Some(m)) => {
            let d = match &*d.value {
                Expr::Number(n, _) => *n as i64,
                _ => 0,
            };
            let m = match &*m.value {
                Expr::String(s, _) => s.clone(),
                _ => format!("{}", m.value),
            };
            (d, m)
        }
        _ => (0, "default".to_string()),
    }
}

fn emit_borrow_stress_output(
    ctx: &mut EvaluationContext,
    depth: i64,
    msg: &str,
    span: &Span,
    phase: &str,
) {
    ctx.output
        .borrow_mut()
        .emit(&format!("[{phase}:{depth}:{msg}]"), Some(span));
}

fn handle_borrow_stress_recursion(
    ctx: &mut EvaluationContext,
    depth: i64,
    msg: &str,
    span: &Span,
    test_borrow_stress_atom: NativeLazyFn,
    test_echo_atom: NativeLazyFn,
) -> AtomResult {
    if depth == 0 {
        return handle_borrow_stress_base_case(ctx, msg, span, test_echo_atom);
    }
    let mut sub_context = sub_eval_context!(ctx);
    sub_context.depth = ctx.depth + 1;
    let nested_args = vec![
        Spanned {
            value: Arc::new(Expr::Number((depth - 1) as f64, *span)),
            span: *span,
        },
        Spanned {
            value: Arc::new(Expr::String(msg.to_string(), *span)),
            span: *span,
        },
    ];
    test_borrow_stress_atom(&nested_args, &mut sub_context, span)
}

fn handle_borrow_stress_base_case(
    ctx: &mut EvaluationContext,
    msg: &str,
    span: &Span,
    test_echo_atom: NativeLazyFn,
) -> AtomResult {
    let mut sub_context = sub_eval_context!(ctx);
    sub_context.depth = ctx.depth + 1;
    let echo_arg = Spanned {
        value: Arc::new(Expr::String(msg.to_string(), *span)),
        span: *span,
    };
    test_echo_atom(&[echo_arg], &mut sub_context, span)
}

fn test_borrow_stress_atom(
    args: &[AstNode],
    ctx: &mut EvaluationContext,
    span: &Span,
) -> AtomResult {
    let (depth, msg) = parse_borrow_stress_args(args);
    emit_borrow_stress_output(ctx, depth, &msg, span, "before");
    handle_borrow_stress_recursion(
        ctx,
        depth,
        &msg,
        span,
        test_borrow_stress_atom,
        test_echo_atom,
    )?;
    emit_borrow_stress_output(ctx, depth, &msg, span, "after");
    Ok(Value::String(format!("depth:{depth};msg:{msg}")))
}

pub fn register_test_atoms(world: &mut World) {
    register_lazy!(world, "test/echo", test_echo_atom);
    register_lazy!(world, "test/borrow_stress", test_borrow_stress_atom);
    register_lazy!(world, "register-test!", register_test_atom);
    register_lazy!(world, "value", value_atom);
    register_lazy!(world, "tags", tags_atom);
    register_lazy!(world, "assert", assert_atom);
    register_lazy!(world, "assert-eq", assert_eq_atom);
}

fn assert_atom(args: &[AstNode], ctx: &mut EvaluationContext, _span: &Span) -> AtomResult {
    helpers::validate_special_form_arity(args, 1, "assert", ctx)?;
    let value = evaluate_ast_node(&args[0], ctx)?;
    let is_truthy = value.is_truthy();
    if !is_truthy {
        return Err(ctx.invalid_operation(
            "assertion",
            &format!("expected truthy value, got {value}"),
            to_source_span(args[0].span),
        ));
    }
    Ok(Value::Nil)
}

fn assert_eq_atom(args: &[AstNode], ctx: &mut EvaluationContext, span: &Span) -> AtomResult {
    helpers::validate_special_form_arity(args, 2, "assert-eq", ctx)?;
    let expected = evaluate_ast_node(&args[0], ctx)?;
    let actual = evaluate_ast_node(&args[1], ctx)?;
    if expected != actual {
        return Err(ctx.invalid_operation(
            "assertion equality",
            &format!("expected {expected}, got {actual}"),
            to_source_span(*span),
        ));
    }
    Ok(Value::Nil)
}

// ============================================================================
// TEST EXECUTION
// ============================================================================

pub fn run_all_registered_tests(world: &CanonicalWorld, output: &SharedOutput) -> Vec<TestResult> {
    let registry = TEST_REGISTRY.lock().unwrap();
    let mut results = Vec::new();

    for (_name, test_def) in registry.iter() {
        let result = execute_single_test(test_def, world, output);
        results.push(result);
    }

    results
}

fn execute_single_test(
    test_def: &TestDefinition,
    world: &CanonicalWorld,
    output: &SharedOutput,
) -> TestResult {
    let source_ctx = SourceContext::from_file(test_def.source_file.name(), &test_def.source_text);

    let mut ctx = EvaluationContext::new(world.clone(), output.clone(), source_ctx.clone());

    let actual_result = match eval_test_body(&test_def.body, &mut ctx) {
        Ok(val) => val,
        Err(e) => {
            return TestResult {
                name: test_def.name.clone(),
                passed: false,
                details: format!("Test failed during execution: {e}"),
            };
        }
    };

    let expected_result = match evaluate_ast_node(&test_def.expect, &mut ctx) {
        Ok(val) => val,
        Err(e) => {
            return TestResult {
                name: test_def.name.clone(),
                passed: false,
                details: format!("Failed to evaluate expected value: {e}"),
            };
        }
    };

    let passed = actual_result == expected_result;
    let details = if passed {
        format!("Passed. Got: {actual_result}")
    } else {
        format!("Failed. Expected: {expected_result}, Got: {actual_result}",)
    };

    TestResult {
        name: test_def.name.clone(),
        passed,
        details,
    }
}

fn eval_test_body(body: &[AstNode], ctx: &mut EvaluationContext) -> AtomResult {
    let mut result = Ok(Value::Nil);
    for expr in body {
        result = evaluate_ast_node(expr, ctx);
        if result.is_err() {
            return result;
        }
    }
    result
}

pub fn run_tests_from_file(
    path: &str,
    world: &CanonicalWorld,
    output: &SharedOutput,
) -> Result<Vec<TestResult>, SutraError> {
    let source_text = std::fs::read_to_string(path).map_err(|e| {
        let dummy_ctx =
            EvaluationContext::new(world.clone(), output.clone(), SourceContext::default());
        dummy_ctx.invalid_operation(
            "reading test file",
            &e.to_string(),
            to_source_span(Span::default()),
        )
    })?;
    let source_ctx = SourceContext::from_file(path, &source_text);
    let ast = crate::syntax::parser::parse(&source_text, source_ctx.clone())?;

    let mut reg_ctx = EvaluationContext::new(world.clone(), output.clone(), source_ctx);

    for expr in ast {
        let _ = evaluate_ast_node(&expr, &mut reg_ctx)?;
    }

    Ok(run_all_registered_tests(world, output))
}
