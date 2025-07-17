use crate::atoms::OutputSink;
use crate::cli::output::StdoutSink;
use crate::err_ctx;
use crate::macros::definition::{is_macro_definition, parse_macro_definition};
use crate::macros::{expand_macros_recursively, MacroDef};
use crate::runtime::eval::evaluate;
use crate::runtime::registry::build_canonical_macro_env;
use crate::runtime::world::World;
use crate::syntax::parser::wrap_in_do;
use crate::SutraError;
use miette::NamedSource;
use std::sync::Arc;

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
    let expanded = expand_macros_recursively(program, &mut env)?;

    // 8. Validation step
    let atom_registry = crate::runtime::registry::build_default_atom_registry();
    let mut combined_macros = env.core_macros.clone();
    combined_macros.extend(env.user_macros.clone());
    let macro_registry_for_validation = crate::macros::MacroRegistry {
        macros: combined_macros,
    };
    let validation_result = crate::validation::semantic::validate_expanded_ast(
        &expanded,
        &macro_registry_for_validation,
        &atom_registry,
    );

    if !validation_result.is_valid() {
        // For now, just print errors to stderr. A more robust error handling
        // mechanism will be added later.
        let error_message = validation_result.errors.join("\n");
        return Err(err_ctx!(
            Validation,
            format!("Semantic validation failed:\n{}", error_message),
            source,
            expanded.span
        ));
    }

    // 9. Evaluate the expanded AST
    let world = World::default();
    let source = Arc::new(NamedSource::new("source", source.to_string()));
    let (result, _updated_world) =
        evaluate(&expanded, &world, output, &atom_registry, source.clone(), 100)?;

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
