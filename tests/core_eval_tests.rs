use sutra::ast::Expr;
use sutra::atom::{AtomRegistry, NullSink};
use sutra::atoms_std::*;
use sutra::eval::{eval, EvalOptions};
use sutra::parser::parse_sexpr;
use sutra::value::Value;
use sutra::world::World;

// ---
// Test Setup
// ---

fn create_test_eval_options() -> EvalOptions {
    let mut registry = AtomRegistry::new();
    registry.register("set!", ATOM_SET);
    registry.register("del!", ATOM_DEL);
    registry.register("get", ATOM_GET);
    registry.register("+", ATOM_ADD);
    registry.register("-", ATOM_SUB);
    registry.register("*", ATOM_MUL);
    registry.register("/", ATOM_DIV);
    registry.register("eq?", ATOM_EQ);
    registry.register("gt?", ATOM_GT);
    registry.register("lt?", ATOM_LT);
    registry.register("not", ATOM_NOT);
    registry.register("cond", ATOM_COND);
    registry.register("list", ATOM_LIST);
    registry.register("len", ATOM_LEN);

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
    let expr = parse_sexpr(expr_str)?;
    let mut sink = NullSink;
    eval(&expr, world, &mut sink, opts)
}

// ---
// Parser Tests
// ---

#[test]
fn test_parse_simple_list() {
    let expr = parse_sexpr("(+ 1 2)").unwrap();
    if let Expr::List(items, _) = expr {
        assert_eq!(items.len(), 3);

        // Explicitly match the first item to handle the non-Copy String
        match &items[0] {
            Expr::Symbol(s, _) => assert_eq!(s, "+"),
            _ => panic!("Expected symbol for item 0"),
        }

        // The `matches!` macro is fine for Copy types like f64
        assert!(matches!(items[1], Expr::Number(n, _) if n == 1.0));
        assert!(matches!(items[2], Expr::Number(n, _) if n == 2.0));
    } else {
        panic!("Expected a list");
    }
}

#[test]
fn test_parse_nested_list() {
    let expr = parse_sexpr("(+ 1 (* 2 3))").unwrap();
    assert_eq!(expr.pretty(), "(+ 1 (* 2 3))");
}

#[test]
fn test_parse_string_literal() {
    let expr = parse_sexpr(r#"(set! (list "name") "sutra")"#).unwrap();
    assert_eq!(expr.pretty(), r#"(set! (list "name") "sutra")"#);
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

    let (_, world_after_set) =
        run_expr(r#"(set! (list "player" "hp") 100)"#, &world, &opts).unwrap();

    let (val, _) = run_expr(r#"(get (list "player" "hp"))"#, &world_after_set, &opts).unwrap();
    assert_eq!(val, Value::Number(100.0));
}

#[test]
fn test_auto_get_feature() {
    let world = World::new();
    let opts = create_test_eval_options();

    // 1. Set a value in the world.
    let (_, world_with_hp) = run_expr(r#"(set! (list "player" "hp") 100)"#, &world, &opts).unwrap();

    // 2. Use a symbol directly as a path to retrieve it. This is the auto-get test.
    let (val, _) = run_expr("player.hp", &world_with_hp, &opts).unwrap();
    assert_eq!(val, Value::Number(100.0));

    // 3. Use it in a calculation.
    let (val, _) = run_expr("(+ player.hp 50)", &world_with_hp, &opts).unwrap();
    assert_eq!(val, Value::Number(150.0));
}

#[test]
fn test_cond_special_form() {
    let world = World::new();
    let opts = create_test_eval_options();

    // The `cond` atom takes a flat list of arguments, not a list of pairs.
    // This test is now structured correctly.
    let expr = r#"
    (cond
        (lt? 10 5) "first"
        (gt? 10 5) "second"
        "else")
    "#;
    let (val, _) = run_expr(expr, &world, &opts).unwrap();
    assert_eq!(val, Value::String("second".to_string()));
}

#[test]
fn test_list_and_len_atoms() {
    let world = World::new();
    let opts = create_test_eval_options();

    let (val, _) = run_expr("(len (list 1 2 3 4))", &world, &opts).unwrap();
    assert_eq!(val, Value::Number(4.0));
}
