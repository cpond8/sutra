use crate::atoms::OutputSink;
use crate::cli::output::StdoutSink;
use crate::err_ctx;
use crate::macros::definition::{is_macro_definition, parse_macro_definition};
use crate::macros::{expand_macros, MacroDef};
use crate::runtime::eval::eval;
use crate::runtime::registry::build_canonical_macro_env;
use crate::runtime::world::World;
use crate::syntax::parser::wrap_in_do;
use crate::SutraError;

/// Run Sutra source with injectable output sink (engine orchestration entry point).
pub fn run_sutra_source_with_output(
    source: &str,
    output: &mut dyn OutputSink,
) -> Result<(), SutraError> {
    // 1. Parse the source into AST nodes
    let ast_nodes = crate::syntax::parser::parse(source)?;

    // 2. Partition AST nodes: macro definitions vs user code
    let (macro_defs, user_code): (Vec<_>, Vec<_>) =
        ast_nodes.into_iter().partition(is_macro_definition);

    // 3. Build canonical macro environment
    let mut env = build_canonical_macro_env()?;

    // 4. Extend env.user_macros with user-defined macros parsed from the source.
    for macro_expr in macro_defs {
        let (name, template) = parse_macro_definition(&macro_expr)?;
        if env.user_macros.contains_key(&name) {
            return Err(err_ctx!(Validation, "Duplicate macro name '{}'", name));
        }
        env.user_macros
            .insert(name.clone(), MacroDef::Template(template));
    }

    // 6. Wrap user_code in a (do ...) if needed
    let program = wrap_in_do(user_code);

    // 7. Expand macros
    let expanded = expand_macros(program, &mut env)?;

    // 8. Validation step (currently disabled)

    // 9. Evaluate the expanded AST
    let world = World::default();
    let atom_registry = crate::runtime::registry::build_default_atom_registry();
    let (result, _updated_world) = eval(&expanded, &world, output, &atom_registry, 100)?;

    // 10. If result is nil, suppress output to avoid "null" printing
    if !result.is_nil() {
        output.emit(&result.to_string(), None);
    }

    Ok(())
}

/// Run Sutra source and print output to stdout (legacy entry point).
pub fn run_sutra_source(source: &str, _filename: Option<&str>) -> Result<(), SutraError> {
    let mut stdout_sink = StdoutSink;
    run_sutra_source_with_output(source, &mut stdout_sink)
}
