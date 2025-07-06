//! Evaluation pipeline and atom contract tests for Sutra engine.

#[cfg(test)]
mod tests {
    #[test]
    fn placeholder() {
        // TODO: Implement evaluation pipeline tests
        assert!(true);
    }

    #[test]
    fn eval_arithmetic_add() {
        use sutra::parser::parse;
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        let input = "(+ 1 2)";
        let ast = parse(input).unwrap().remove(0);
        let mut output = NullSink;
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let result = eval(&ast, &world, &mut output, &opts);
        assert!(result.is_ok(), "Evaluation should succeed");
        let (val, _) = result.unwrap();
        assert_eq!(val.as_number(), Some(3.0));
    }

    #[test]
    fn eval_predicate_eq() {
        use sutra::parser::parse;
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        let input = "(eq? 1 1)";
        let ast = parse(input).unwrap().remove(0);
        let mut output = NullSink;
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let result = eval(&ast, &world, &mut output, &opts);
        assert!(result.is_ok(), "Evaluation should succeed");
        let (val, _) = result.unwrap();
        assert_eq!(val.as_bool(), Some(true));
    }

    #[test]
    fn eval_state_mutation_set_get() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, SutraMacroContext, SutraMacroExpander, MacroRegistry};
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        // Macro expansion required for set! and get
        let set_input = "(set! foo 42)";
        let get_input = "(get foo)";
        let set_ast = parse(set_input).unwrap().remove(0);
        let get_ast = parse(get_input).unwrap().remove(0);
        let mut macro_registry = MacroRegistry::default();
        sutra::macros_std::register_std_macros(&mut macro_registry);
        let macro_context = SutraMacroContext { registry: macro_registry, hygiene_scope: None };
        let expander = MacroExpander::default();
        let set_ast = expander.expand_macros(set_ast, &macro_context).unwrap();
        let get_ast = expander.expand_macros(get_ast, &macro_context).unwrap();
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let set_result = eval(&set_ast, &world, &mut NullSink, &opts);
        assert!(set_result.is_ok(), "Set should succeed");
        let (_, new_world) = set_result.unwrap();
        let get_result = eval(&get_ast, &new_world, &mut NullSink, &opts);
        assert!(get_result.is_ok(), "Get should succeed");
        let (val, _) = get_result.unwrap();
        assert_eq!(val.as_number(), Some(42.0));
    }

    #[test]
    fn eval_special_form_if() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, SutraMacroContext, SutraMacroExpander, MacroRegistry};
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        // Macro expansion required for eq? in if condition
        let input = "(if (eq? 1 1) 100 200)";
        let ast = parse(input).unwrap().remove(0);
        let mut macro_registry = MacroRegistry::default();
        sutra::macros_std::register_std_macros(&mut macro_registry);
        let macro_context = SutraMacroContext { registry: macro_registry, hygiene_scope: None };
        let expander = MacroExpander::default();
        let ast = expander.expand_macros(ast, &macro_context).unwrap();
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let result = eval(&ast, &world, &mut NullSink, &opts);
        assert!(result.is_ok(), "Evaluation should succeed");
        let (val, _) = result.unwrap();
        assert_eq!(val.as_number(), Some(100.0));
    }

    #[test]
    fn eval_do_block() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, SutraMacroContext, SutraMacroExpander, MacroRegistry};
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        // Macro expansion required for do, set!, get
        let input = "(do (set! x 1) (set! y 2) (+ (get x) (get y)))";
        let ast = parse(input).unwrap().remove(0);
        let mut macro_registry = MacroRegistry::default();
        sutra::macros_std::register_std_macros(&mut macro_registry);
        let macro_context = SutraMacroContext { registry: macro_registry, hygiene_scope: None };
        let expander = MacroExpander::default();
        let ast = expander.expand_macros(ast, &macro_context).unwrap();
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let result = eval(&ast, &world, &mut NullSink, &opts);
        assert!(result.is_ok(), "Evaluation should succeed");
        let (val, _) = result.unwrap();
        assert_eq!(val.as_number(), Some(3.0));
    }

    #[test]
    fn eval_type_error_should_error() {
        use sutra::parser::parse;
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        let input = "(+ 1 true)";
        let ast = parse(input).unwrap().remove(0);
        let mut output = NullSink;
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let result = eval(&ast, &world, &mut output, &opts);
        assert!(result.is_err(), "Evaluation should error on type error");
    }

    #[test]
    fn eval_arity_error_should_error() {
        use sutra::parser::parse;
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        let input = "(+ 1)";
        let ast = parse(input).unwrap().remove(0);
        let mut output = NullSink;
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let result = eval(&ast, &world, &mut output, &opts);
        assert!(result.is_err(), "Evaluation should error on arity error");
    }

    #[test]
    fn eval_division_by_zero_should_error() {
        use sutra::parser::parse;
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        let input = "(/ 1 0)";
        let ast = parse(input).unwrap().remove(0);
        let mut output = NullSink;
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let result = eval(&ast, &world, &mut output, &opts);
        assert!(result.is_err(), "Evaluation should error on division by zero");
    }

    #[test]
    fn eval_nil_fallback_handling() {
        use sutra::parser::parse;
        use sutra::macros::{MacroExpander, SutraMacroContext, SutraMacroExpander, MacroRegistry};
        use sutra::eval::{eval, EvalOptions};
        use sutra::atom::{AtomRegistry, NullSink};
        use sutra::world::World;
        // Macro expansion required for get
        let input = "(get missing)";
        let ast = parse(input).unwrap().remove(0);
        let mut macro_registry = MacroRegistry::default();
        sutra::macros_std::register_std_macros(&mut macro_registry);
        let macro_context = SutraMacroContext { registry: macro_registry, hygiene_scope: None };
        let expander = MacroExpander::default();
        let ast = expander.expand_macros(ast, &macro_context).unwrap();
        let mut registry = AtomRegistry::new();
        sutra::atoms_std::register_std_atoms(&mut registry);
        let opts = EvalOptions { max_depth: 32, atom_registry: registry };
        let world = World::default();
        let result = eval(&ast, &world, &mut NullSink, &opts);
        assert!(result.is_ok(), "Evaluation should succeed (should return nil/default)");
        let (val, _) = result.unwrap();
        assert!(val.is_nil());
    }
}