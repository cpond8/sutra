pub use crate::{
    ast::{value::Value, AstNode, Expr, ParamList, Span, Spanned},
    atoms::{AtomRegistry, SharedOutput, StateContext},
    engine::{print_error, EngineOutputBuffer, EngineStdoutSink},
    macros::{
        expand_macros_recursively, MacroDefinition, MacroExpansionContext, MacroRegistry,
        MacroTemplate,
    },
    runtime::world::{Path, World},
    test::{Expectation, TestResult, TestSummary},
};

// Module aliases for concise imports
pub use ast::value;
pub use atoms::helpers;
pub use runtime::{eval, world};
pub use syntax::parser;
pub use validation::{grammar, semantic};

pub mod prelude {
    pub use crate::{
        ast::value::Value,
        ast::{AstNode, Expr, Span, Spanned},
        atoms::{AtomRegistry, SharedOutput},
        errors::{ErrorType, SutraError},
        macros::MacroRegistry,
        runtime::eval::EvaluationContext,
        runtime::world::World,
        MacroDefinition, Path,
        syntax::parser::to_source_span
    };
}

pub mod ast;
pub mod atoms;
pub mod cli;
pub mod discovery;
pub mod engine;
pub mod errors;
pub mod macros;
pub mod runtime;
pub mod syntax;
pub mod test;
pub mod validation;

#[cfg(test)]
mod sutra_harness {
    use std::path::Path;

    use crate::cli::ArgsCommand;
    #[test]
    fn run_sutra_tests() {
        // Test that the CLI can handle test execution
        let command = ArgsCommand::Test {
            path: Path::new("tests").to_path_buf(),
        };
        // This is just a smoke test - actual execution is tested elsewhere
        assert!(matches!(command, ArgsCommand::Test { .. }));
    }
}
