pub use crate::ast::value::Value;
pub use crate::ast::ParamList;
pub use crate::ast::{AstNode, Expr, Span, Spanned};
pub use crate::atoms::{AtomRegistry, SharedOutput, StateContext};
pub use crate::cli::output::OutputBuffer;
pub use crate::diagnostics::{to_error_source, ErrorContext, SutraError};
pub use crate::macros::MacroExpansionContext;
pub use crate::macros::MacroTemplate;
pub use crate::macros::{expand_macros_recursively, MacroDefinition, MacroRegistry};
pub use crate::runtime::world::AtomExecutionContext;
pub use crate::runtime::world::Path;
pub use crate::runtime::world::World;

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
    use crate::cli::handle_test;
    use std::path::Path;
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
