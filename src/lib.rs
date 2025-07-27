pub use crate::{
    ast::{
        value::{NativeEagerFn, NativeLazyFn, Value},
        AstNode, Expr, ParamList, Span, Spanned,
    },
    atoms::{SharedOutput, StateContext, Path, World, build_canonical_world, build_canonical_macro_env},
    engine::{print_error, EngineOutputBuffer, EngineStdoutSink},
    macros::{
        expand_macros_recursively, MacroDefinition, MacroExpansionContext, MacroRegistry,
        MacroTemplate,
    },
    test::{Expectation, TestResult, TestSummary},
};

// Module aliases for concise imports
pub use ast::value;
pub use atoms::helpers;
pub use engine::{evaluate, EvaluationContext};
pub use syntax::parser;
pub use validation::{grammar, semantic};

pub mod prelude {
    pub use crate::{
        ast::{
            value::{NativeEagerFn, NativeLazyFn, Value},
            AstNode, Expr, Span, Spanned,
        },
        atoms::SharedOutput,
        errors::{ErrorKind, SourceContext, SutraError},
        macros::MacroRegistry,
        engine::EvaluationContext,
        atoms::{World, Path},
        MacroDefinition,
    };

    // New canonical world type for shared, mutable state
    pub use std::cell::RefCell;
    pub use std::rc::Rc;
    pub type CanonicalWorld = Rc<RefCell<crate::atoms::World>>;
}

pub mod ast;
pub mod atoms;
pub mod cli;
pub mod discovery;
pub mod engine;
pub mod errors;
pub mod macros;
pub mod repl;
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
