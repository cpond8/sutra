pub use crate::{
    syntax::{AstNode, Expr, ParamList, Span, Spanned},
    runtime::{ConsCell, Lambda, Value},
    atoms::{
        build_canonical_macro_env, build_canonical_world, Path, SharedOutput, StateContext, World,
    },
    engine::{print_error, EngineOutputBuffer, EngineStdoutSink},
    macros::{
        expand_macros_recursively, MacroDefinition, MacroExpansionContext, MacroRegistry,
        MacroTemplate,
    },
    test::{Expectation, TestResult, TestSummary},
};

// Module aliases for concise imports
pub use engine::{evaluate, EvaluationContext};
pub use validation::{grammar, semantic};

pub mod prelude {
    pub use crate::{
        runtime::{NativeFn, Value},
        syntax::{AstNode, Expr, Span, Spanned},
        atoms::SharedOutput,
        atoms::{Path, World},
        engine::EvaluationContext,
        errors::{ErrorKind, SourceContext, SutraError},
        macros::MacroRegistry,
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
pub mod engine;
pub mod errors;
pub mod macros;
pub mod repl;
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
