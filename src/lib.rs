pub use crate::{
    ast::{value::Value, AstNode, Expr, ParamList, Span, Spanned},
    atoms::{AtomRegistry, SharedOutput, StateContext},
    cli::output::OutputBuffer,
    diagnostics::{to_error_source, ErrorContext, SutraError},
    macros::{
        expand_macros_recursively, MacroDefinition, MacroExpansionContext, MacroRegistry,
        MacroTemplate,
    },
    runtime::world::{AtomExecutionContext, Path, World},
};

pub mod ast;
pub mod atoms;
pub mod cli;
pub mod diagnostics;
pub mod engine;
pub mod macros;
pub mod runtime;
pub mod syntax;
pub mod testing;
pub mod validation;

#[cfg(test)]
mod sutra_harness {
    use std::path::Path;

    use crate::cli::handle_test;
    #[test]
    fn run_sutra_tests() {
        // Run the Sutra test harness on the tests directory
        let result = handle_test(Path::new("tests"));
        match result {
            Ok(_) => println!("All Sutra tests passed."),
            Err(e) => {
                eprintln!("{e:?}");
                panic!("Sutra test harness failed");
            }
        }
        // Do not panic; always return so all output is visible
    }
}
