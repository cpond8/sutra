// Sutra Engine - Core Evaluation Tests (Rewritten)
// ===============================================
// These tests verify the end-to-end evaluation pipeline:
// parse -> macro expand -> eval
// Only proper lists and ...rest parameter conventions are used.

use sutra::ast::{Expr, WithSpan};
use sutra::eval::{eval, EvalOptions};
use sutra::macros::{MacroExpander, SutraMacroContext, SutraMacroExpander};
use sutra::parser::parse;
use sutra::registry::{build_default_atom_registry, build_default_macro_registry};
use sutra::value::Value;
use sutra::world::World;

/// Helper: Parse, expand, and evaluate a single expression string.
fn eval_expr(expr_str: &str, world: &World, opts: &EvalOptions) -> Result<(Value, World), String> {
    let parsed = parse(expr_str).map_err(|e| format!("parse error: {e:?}"))?;
    if parsed.len() != 1 {
        return Err(format!("expected 1 top-level expr, got {}", parsed.len()));
    }
    let program = parsed.into_iter().next().unwrap();
    let registry = build_default_macro_registry();
    let context = SutraMacroContext { registry, hygiene_scope: None };
    let expander = MacroExpander::default();
    let expanded = expander.expand_macros(program, &context).map_err(|e| format!("expand error: {e:?}"))?;
    let mut sink = sutra::atom::NullSink;
    eval(&expanded, world, &mut sink, opts).map_err(|e| format!("eval error: {e:?}"))
}

/// Helper: Standard evaluation options.
fn default_eval_opts() -> EvalOptions {
    EvalOptions {
        max_depth: 100,
        atom_registry: build_default_atom_registry(),
    }
}

#[test]
fn test_arithmetic_add() {
    let world = World::new();
    let opts = default_eval_opts();
    let (val, _) = eval_expr("(+ 1 2)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Number(3.0));
}

#[test]
fn test_arithmetic_sub_mul_div() {
    let world = World::new();
    let opts = default_eval_opts();
    let (val, _) = eval_expr("(- 5 2)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Number(3.0));
    let (val, _) = eval_expr("(* 3 4)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Number(12.0));
    let (val, _) = eval_expr("(/ 8 2)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Number(4.0));
}

#[test]
fn test_predicate_atoms() {
    let world = World::new();
    let opts = default_eval_opts();
    let (val, _) = eval_expr("(gt? 10 5)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Bool(true));
    let (val, _) = eval_expr("(lt? 3 7)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Bool(true));
    let (val, _) = eval_expr("(eq? 4 4)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Bool(true));
    let (val, _) = eval_expr("(not (eq? 1 2))", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn test_state_set_and_get() {
    let world = World::new();
    let opts = default_eval_opts();
    let (_, world2) = eval_expr("(set! foo 42)", &world, &opts).expect("should set");
    let (val, _) = eval_expr("(get foo)", &world2, &opts).expect("should get");
    assert_eq!(val, Value::Number(42.0));
}

#[test]
fn test_if_special_form() {
    let world = World::new();
    let opts = default_eval_opts();
    let (val, _) = eval_expr("(if (gt? 2 1) 100 200)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Number(100.0));
    let (val, _) = eval_expr("(if (lt? 2 1) 100 200)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Number(200.0));
}

#[test]
fn test_do_special_form() {
    let world = World::new();
    let opts = default_eval_opts();
    let (val, _) = eval_expr("(do (set! x 1) (set! y 2) (+ (get x) (get y)))", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Number(3.0));
}

#[test]
fn test_list_and_len_atoms() {
    let world = World::new();
    let opts = default_eval_opts();
    let (val, _) = eval_expr("(len (list 1 2 3))", &world, &opts).expect("should eval");
    assert_eq!(val, Value::Number(3.0));
    let (val, _) = eval_expr("(list 4 5 6)", &world, &opts).expect("should eval");
    assert_eq!(val, Value::List(vec![Value::Number(4.0), Value::Number(5.0), Value::Number(6.0)]));
}

#[test]
fn test_error_wrong_arity() {
    let world = World::new();
    let opts = default_eval_opts();
    let err = eval_expr("(+ 1)", &world, &opts).err().expect("should error");
    assert!(err.contains("+") && err.contains("expects"), "Unexpected error message: {}", err);
}

#[test]
fn test_error_type_error() {
    let world = World::new();
    let opts = default_eval_opts();
    let err = eval_expr("(+ 1 true)", &world, &opts).err().expect("should error");
    assert!(err.contains("+"));
    assert!(err.contains("expects"));
    assert!(err.contains("Number") || err.contains("Bool") || err.contains("type"));
}
