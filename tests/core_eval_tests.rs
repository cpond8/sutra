use sutra::ast::{Expr, Span};
use sutra::atom::{AtomRegistry, NullSink};
use sutra::atoms_std;
use sutra::eval::{eval, EvalOptions};
use sutra::macros::expand;
use sutra::parser::parse;
use sutra::value::Value;
use sutra::world::World;

// ---
// Test Setup
// ---

fn create_test_eval_options() -> EvalOptions {
    let mut registry = AtomRegistry::new();
    // Centralized registration of all standard atoms.
    // This also ensures that our test environment is identical to production.
    atoms_std::register_std_atoms(&mut registry);

    EvalOptions {
        max_depth: 100,
        atom_registry: registry,
    }
}

fn run_expr(
    expr_str: &str,
    world: &World,
    opts: &EvalOptions,
) -> Result<(Value, World), sutra::error::SutraError> {
    // The canonical pipeline: parse -> expand -> eval
    let parsed_exprs = parse(expr_str).map_err(|e| e.with_source(expr_str))?;

    // The test runner is responsible for wrapping multiple expressions in a `do` block.
    let program = if parsed_exprs.len() == 1 {
        parsed_exprs.into_iter().next().unwrap()
    } else {
        let span = Span {
            start: 0,
            end: expr_str.len(),
        };
        Expr::List(
            {
                let mut vec = vec![Expr::Symbol("do".to_string(), span.clone())];
                vec.extend(parsed_exprs);
                vec
            },
            span,
        )
    };

    let expanded_ast = expand(&program)?;

    let mut sink = NullSink;
    eval(&expanded_ast, world, &mut sink, opts)
}

// ---
// Atom and Integration Tests
// ---

#[test]
fn test_simple_math_atoms() {
    let world = World::new();
    let opts = create_test_eval_options();

    let (val, _) = run_expr("(+ 10 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(15.0));

    let (val, _) = run_expr("(- 10 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(5.0));

    let (val, _) = run_expr("(* 10 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(50.0));

    let (val, _) = run_expr("(/ 10 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(2.0));
}

#[test]
fn test_predicate_atoms() {
    let world = World::new();
    let opts = create_test_eval_options();

    let (val, _) = run_expr("(gt? 10 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(true));

    let (val, _) = run_expr("(lt? 10 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(false));

    let (val, _) = run_expr("(eq? 10 10)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(true));

    let (val, _) = run_expr("(not (eq? 10 5))", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(true));
}

#[test]
fn test_set_and_get_atoms() {
    let world = World::new();
    let opts = create_test_eval_options();

    let (_, world_after_set) = run_expr("(set! player.hp 100)", &world, &opts).unwrap();

    let (val, _) = run_expr("(get player.hp)", &world_after_set, &opts).unwrap();
    assert_eq!(val, Value::Number(100.0));
}

#[test]
fn test_auto_get_feature_via_macro() {
    let world = World::new();
    let opts = create_test_eval_options();

    // 1. Set a value in the world.
    let (_, world_with_hp) = run_expr("(set! player.hp 100)", &world, &opts).unwrap();

    // 2. Use a macro `is?` that performs the auto-get expansion.
    // The author writes `player.hp`, but the macro expands it to `(get player.hp)`.
    let (val, _) = run_expr("(is? player.hp 100)", &world_with_hp, &opts).unwrap();
    assert_eq!(val, Value::Bool(true));

    let (val, _) = run_expr("(is? player.hp 99)", &world_with_hp, &opts).unwrap();
    assert_eq!(val, Value::Bool(false));
}

#[test]
fn test_if_special_form() {
    let world = World::new();
    let opts = create_test_eval_options();

    // Test the `then` branch
    let expr_then = r#"(if (gt? 10 5) "then-branch" "else-branch")"#;
    let (val_then, _) = run_expr(expr_then, &world, &opts).unwrap();
    assert_eq!(val_then, Value::String("then-branch".to_string()));

    // Test the `else` branch
    let expr_else = r#"(if (lt? 10 5) "then-branch" "else-branch")"#;
    let (val_else, _) = run_expr(expr_else, &world, &opts).unwrap();
    assert_eq!(val_else, Value::String("else-branch".to_string()));

    // Test with nested expressions and state changes
    let expr_nested = r#"
    (do
        (set! x 10)
        (if (is? x 10)
            (set! y 20)
            (set! y 30))
        (get y))
    "#;
    let (val_nested, _) = run_expr(expr_nested, &world, &opts).unwrap();
    assert_eq!(val_nested, Value::Number(20.0));
}

#[test]
fn test_list_and_len_atoms() {
    let world = World::new();
    let opts = create_test_eval_options();

    let (val, _) = run_expr("(len (list 1 2 3 4))", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(4.0));
}

#[test]
fn test_assignment_macros() {
    let world = World::new();
    let opts = create_test_eval_options();

    // 1. Set initial score
    let (_, world1) = run_expr("(set! score 10)", &world, &opts).unwrap();

    // 2. Test add!
    let (_, world2) = run_expr("(add! score 5)", &world1, &opts).unwrap();
    let (val, _) = run_expr("(get score)", &world2, &opts).unwrap();
    assert_eq!(val, Value::Number(15.0));

    // 3. Test sub!
    let (_, world3) = run_expr("(sub! score 2)", &world2, &opts).unwrap();
    let (val, _) = run_expr("(get score)", &world3, &opts).unwrap();
    assert_eq!(val, Value::Number(13.0));

    // 4. Test inc!
    let (_, world4) = run_expr("(inc! score)", &world3, &opts).unwrap();
    let (val, _) = run_expr("(get score)", &world4, &opts).unwrap();
    assert_eq!(val, Value::Number(14.0));

    // 5. Test dec!
    let (_, world5) = run_expr("(dec! score)", &world4, &opts).unwrap();
    let (val, _) = run_expr("(get score)", &world5, &opts).unwrap();
    assert_eq!(val, Value::Number(13.0));
}

#[test]
fn test_state_propagation_in_do_block() {
    let world = World::new();
    let opts = create_test_eval_options();

    // This test is crucial for verifying the core state propagation hypothesis.
    // If this passes, the issue is likely in test setup or macro expansion,
    // not in the `do` atom's world threading.
    let expr = r#"(do (set! score 5) (add! score 10) (get score))"#;
    let (val, _) = run_expr(expr, &world, &opts).unwrap();
    assert_eq!(val, Value::Number(15.0));
}

#[test]
fn test_mod_atom() {
    let world = World::new();
    let opts = create_test_eval_options();

    // Normal case: 10 % 3 = 1
    let (val, _) = run_expr("(mod 10 3)", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(1.0));

    // Negative dividend: -10 % 3 = -1
    let (val, _) = run_expr("(mod -10 3)", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(-1.0));

    // Negative divisor: 10 % -3 = 1
    let (val, _) = run_expr("(mod 10 -3)", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(1.0));

    // Zero dividend: 0 % 5 = 0
    let (val, _) = run_expr("(mod 0 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(0.0));

    // Error: division by zero
    let err = run_expr("(mod 10 0)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("Modulo by zero"));

    // Error: non-integer arguments
    let err = run_expr("(mod 10.5 3)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("two Integers"));
    let err = run_expr("(mod 10 3.2)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("two Integers"));

    // Error: wrong arity
    let err = run_expr("(mod 10)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("expects 2 arguments"));
    let err = run_expr("(mod 10 2 1)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("expects 2 arguments"));
}

#[test]
fn test_gte_and_lte_atoms() {
    let world = World::new();
    let opts = create_test_eval_options();

    // gte? normal cases
    let (val, _) = run_expr("(gte? 10 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(true));
    let (val, _) = run_expr("(gte? 5 10)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(false));
    let (val, _) = run_expr("(gte? 10 10)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(true));

    // lte? normal cases
    let (val, _) = run_expr("(lte? 5 10)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(true));
    let (val, _) = run_expr("(lte? 10 5)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(false));
    let (val, _) = run_expr("(lte? 10 10)", &world, &opts).unwrap();
    assert_eq!(val, Value::Bool(true));

    // Error: non-numeric arguments
    let err = run_expr("(gte? 10 \"foo\")", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("two Numbers"));
    let err = run_expr("(lte? \"foo\" 10)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("two Numbers"));

    // Error: wrong arity
    let err = run_expr("(gte? 10)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("expects 2 arguments"));
    let err = run_expr("(lte? 10 2 1)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("expects 2 arguments"));
}

#[test]
fn test_list_get_len_edge_cases() {
    let world = World::new();
    let opts = create_test_eval_options();

    // Empty list
    let (val, _) = run_expr("(list)", &world, &opts).unwrap();
    assert_eq!(val, Value::List(vec![]));
    let (val, _) = run_expr("(len (list))", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(0.0));

    // Nested lists
    let (val, _) = run_expr("(list (list 1 2) 3)", &world, &opts).unwrap();
    assert_eq!(
        val,
        Value::List(vec![
            Value::List(vec![Value::Number(1.0), Value::Number(2.0)]),
            Value::Number(3.0)
        ])
    );
    let (val, _) = run_expr("(len (list (list 1 2) 3))", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(2.0));
    let (val, _) = run_expr("(len (list (list 1)))", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(1.0));

    // Invalid types for len
    let err = run_expr("(len 42)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("a List or String"));
    let err = run_expr("(len true)", &world, &opts)
        .err()
        .expect("should error");
    assert!(err.to_string().contains("a List or String"));

    // Invalid types for get
    let err = run_expr("(get 42)", &world, &opts)
        .err()
        .expect("should error");
    // This error now comes from the macro expansion phase, which is correct.
    // The test is updated to reflect the new, more accurate error message.
    assert!(err
        .to_string()
        .contains("Invalid path format: expected a symbol or a list."));

    // Out-of-bounds and missing keys
    let (val, _) = run_expr("(get missing)", &world, &opts).unwrap();
    assert_eq!(val, Value::default());
    let (val, _) = run_expr("(get player.inventory.0)", &world, &opts).unwrap();
    assert_eq!(val, Value::default());

    // Mixed types and deep nesting
    let (val, _) = run_expr("(list 1 \"two\" true (list 3 4))", &world, &opts).unwrap();
    assert_eq!(
        val,
        Value::List(vec![
            Value::Number(1.0),
            Value::String("two".to_string()),
            Value::Bool(true),
            Value::List(vec![Value::Number(3.0), Value::Number(4.0)])
        ])
    );
    let (val, _) = run_expr("(len (list 1 \"two\" true (list 3 4)))", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(4.0));

    // String length
    let (val, _) = run_expr("(len \"hello\")", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(5.0));
    let (val, _) = run_expr("(len \"\")", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(0.0));
}
