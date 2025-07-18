pub use crate::diagnostics::{SutraError, ErrorContext, to_error_source};
pub use crate::runtime::world::Path;
pub use crate::runtime::world::World;
pub use crate::ast::value::Value;
pub use crate::ast::{AstNode, Expr, Span, Spanned};
pub use crate::macros::{MacroDefinition, MacroRegistry, expand_macros_recursively};
pub use crate::atoms::{StateContext, SharedOutput, AtomRegistry};
pub use crate::ast::ParamList;
pub use crate::runtime::world::AtomExecutionContext;
pub use crate::macros::MacroTemplate;
pub use crate::macros::MacroExpansionContext;
pub use crate::cli::output::OutputBuffer;

pub mod ast;
pub mod atoms;
pub mod cli;
pub mod diagnostics;
pub mod macros;
pub mod runtime;
pub mod syntax;
pub mod engine;
pub mod validation;
pub mod testing;

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
            },
        }
        // Do not panic; always return so all output is visible
    }
}
