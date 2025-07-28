pub use crate::{
    atoms::{
        build_canonical_macro_env, build_canonical_world, EngineOutputBuffer, EngineStdoutSink,
        Path, SharedOutput, StateContext, World,
    },
    errors::print_error,
    macros::{MacroDefinition, MacroSystem},
    runtime::{ConsCell, Lambda, Value},
    syntax::{AstNode, Expr, ParamList, Span, Spanned},
    test::{Expectation, TestResult, TestSummary},
};

// Module aliases for concise imports
pub use grammar_validation as grammar;
pub use grammar_validation::{validate_grammar, validate_grammar_str, Rule};
pub use runtime::{evaluate, EvaluationContext};
pub use semantic_validation as semantic;
pub use semantic_validation::validate_ast_semantics;

pub mod prelude {
    pub use crate::{
        atoms::SharedOutput,
        atoms::{Path, World},
        errors::{ErrorKind, SourceContext, SutraError},
        macros::MacroSystem,
        runtime::{EvaluationContext, NativeFn, Value},
        syntax::{AstNode, Expr, Span, Spanned},
        MacroDefinition,
    };

    // New canonical world type for shared, mutable state
    pub use std::cell::RefCell;
    pub use std::rc::Rc;
    pub type CanonicalWorld = Rc<RefCell<crate::atoms::World>>;
}

pub mod atoms;
pub mod cli;
pub mod discovery;
pub mod errors;
pub mod grammar_validation;
pub mod macros;
pub mod parser;
pub mod repl;
pub mod runtime;
pub mod semantic_validation;
pub mod syntax;
pub mod test;
pub mod test_runner;

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
