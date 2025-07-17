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
use crate::ast::{AstNode, Expr, Span, Spanned};
use crate::atoms::{AtomRegistry, SpecialFormAtomFn};
use crate::runtime::eval::{evaluate_ast_node, EvaluationContext};
use crate::runtime::world::World;
use crate::{err_src, SutraError};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use std::collections::HashMap;

// Use the public context helper macro
use crate::sub_eval_context;

/// Represents a single test case definition.
#[derive(Debug, Clone)]
pub struct TestDefinition {
    pub name: String,
    pub expect: AstNode,
    pub body: Vec<AstNode>,
    pub span: Span,
    pub file: Option<String>,
}

/// A global registry for storing test definitions.
pub static TEST_REGISTRY: Lazy<Mutex<HashMap<String, TestDefinition>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));


/// `register-test!` special form.
///
/// Usage: (register-test! <name> <expect-form> <body> <metadata>)
/// - <name>: A string representing the name of the test.
/// - <expect-form>: An s-expression detailing the test's expectations.
/// - <body>: A list of expressions to execute as the test body.
/// - <metadata>: A map containing metadata like :span and :file.
///
/// This atom is intended to be used by the `(test ...)` macro, not directly.
/// It registers a test definition into a global registry for later execution.
///
/// Returns `nil`.
fn register_test_atom(
    args: &[AstNode],
    ctx: &mut EvaluationContext,
    span: &Span,
) -> Result<(Value, World), SutraError> {
    if args.len() != 4 {
        return Err(err_src!(
            Validation,
            "Expected 4 arguments",
            &ctx.source,
            *span
        ));
    }

    let name = match &*args[0].value {
        Expr::String(s, _) => s.clone(),
        _ => {
            return Err(err_src!(
                Validation,
                "Test name must be a string",
                &ctx.source,
                args[0].span
            ));
        }
    };

    let expect = args[1].clone();
    let body = match &*args[2].value {
        Expr::List(l, _) => l.clone(),
        _ => {
            return Err(err_src!(
                Validation,
                "Test body must be a list of expressions",
                &ctx.source,
                args[2].span
            ));
        }
    };

    let (metadata_val, _) = evaluate_ast_node(&args[3], ctx)?;
    let metadata = match metadata_val.as_map() {
        Some(m) => m,
        _ => {
            return Err(err_src!(
                Validation,
                "Test metadata must be a map",
                &ctx.source,
                args[3].span
            ));
        }
    };

    let file = metadata.get(":file").and_then(|v| v.as_string());

    let test_def = TestDefinition {
        name: name.clone(),
        expect,
        body,
        span: *span,
        file,
    };

    let mut registry = TEST_REGISTRY.lock().unwrap();
    registry.insert(name, test_def);

    Ok((Value::Nil, ctx.world.clone()))
}


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
    args: &[AstNode],
    ctx: &mut EvaluationContext,
    span: &Span,
) -> Result<(Value, World), SutraError> {
    let Some(first) = args.first() else {
        let val = Value::String("".to_string());
        let world = ctx.world.clone();
        ctx.output.borrow_mut().emit(&val.to_string(), Some(span));
        return Ok((val, world));
    };
    let val = match &*first.value {
        Expr::String(s, _) => Value::String(s.clone()),
        _ => Value::String(format!("{}", first.value)),
    };
    let world = ctx.world.clone();
    ctx.output.borrow_mut().emit(&val.to_string(), Some(span));
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
type TestAtomFn = SpecialFormAtomFn;

/// Parse arguments for borrow stress test.
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

/// Emit formatted output for borrow stress test phases.
fn emit_borrow_stress_output(
    ctx: &mut EvaluationContext,
    depth: i64,
    msg: &str,
    span: &Span,
    phase: &str,
) {
    ctx.output
        .borrow_mut()
        .emit(&format!("[{}:{}:{}]", phase, depth, msg), Some(span));
}

/// Handle recursive case of borrow stress test.
fn handle_borrow_stress_recursion(
    ctx: &mut EvaluationContext,
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

/// Handle base case of borrow stress test (calls echo).
fn handle_borrow_stress_base_case(
    ctx: &mut EvaluationContext,
    msg: &str,
    span: &Span,
    test_echo_atom: TestAtomFn,
) -> Result<(Value, World), SutraError> {
    let mut sub_context = sub_eval_context!(ctx, ctx.world);
    sub_context.depth = ctx.depth + 1; // Manually set incremented depth
    let echo_arg = Spanned {
        value: Arc::new(Expr::String(msg.to_string(), *span)),
        span: *span,
    };
    test_echo_atom(&[echo_arg], &mut sub_context, span)
}

/// Main borrow stress test atom implementation.
fn test_borrow_stress_atom(
    args: &[AstNode],
    ctx: &mut EvaluationContext,
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
    registry.register(
        "test/echo",
        crate::atoms::Atom::SpecialForm(test_echo_atom),
    );
    registry.register(
        "test/borrow_stress",
        crate::atoms::Atom::SpecialForm(test_borrow_stress_atom),
    );
    registry.register(
        "register-test!",
        crate::atoms::Atom::SpecialForm(register_test_atom),
    );

    // Register assertion atoms for testing
    registry.register(
        "assert",
        crate::atoms::Atom::SpecialForm(assert_atom),
    );
    registry.register(
        "assert-eq",
        crate::atoms::Atom::SpecialForm(assert_eq_atom),
    );
}

/// `assert` atom - basic assertion that fails if argument is false.
///
/// Usage: (assert <expression>)
/// - <expression>: Any expression that evaluates to a boolean-like value
///
/// Returns `nil` if the assertion passes.
/// Throws an assertion error if the expression is falsy.
///
/// Example:
///   (assert true)        ; => nil (success)
///   (assert (eq? 1 1))   ; => nil (success)
///   (assert false)       ; => AssertionError
fn assert_atom(
    args: &[AstNode],
    ctx: &mut EvaluationContext,
    span: &Span,
) -> Result<(Value, World), SutraError> {
    if args.len() != 1 {
        return Err(err_src!(
            Validation,
            "Expected exactly 1 argument",
            &ctx.source,
            *span
        ));
    }

    let (value, world) = evaluate_ast_node(&args[0], ctx)?;
    let is_truthy = match value {
        Value::Bool(b) => b,
        Value::Nil => false,
        _ => true, // All other values are truthy
    };

    if !is_truthy {
        return Err(err_src!(
            Eval,
            format!("Assertion failed: expected truthy value, got {:?}", value),
            &ctx.source,
            args[0].span
        ));
    }

    Ok((Value::Nil, world))
}

/// `assert-eq` atom - equality assertion that compares two values.
///
/// Usage: (assert-eq <expected> <actual>)
/// - <expected>: The expected value
/// - <actual>: The actual value to compare
///
/// Returns `nil` if the values are equal.
/// Throws an assertion error if the values are not equal.
///
/// Example:
///   (assert-eq 1 1)           ; => nil (success)
///   (assert-eq "a" "a")       ; => nil (success)
///   (assert-eq 1 2)           ; => AssertionError
fn assert_eq_atom(
    args: &[AstNode],
    ctx: &mut EvaluationContext,
    span: &Span,
) -> Result<(Value, World), SutraError> {
    if args.len() != 2 {
        return Err(err_src!(
            Validation,
            "Expected exactly 2 arguments",
            &ctx.source,
            *span
        ));
    }

    let (expected, world1) = evaluate_ast_node(&args[0], ctx)?;
    let mut sub_context = sub_eval_context!(ctx, &world1);
    let (actual, world2) = evaluate_ast_node(&args[1], &mut sub_context)?;

    if expected != actual {
        return Err(err_src!(
            Eval,
            format!("Assertion failed: expected {:?}, got {:?}", expected, actual),
            &ctx.source,
            *span
        ));
    }

    Ok((Value::Nil, world2))
}
