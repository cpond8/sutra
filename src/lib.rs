pub use crate::diagnostics::{SutraError, ErrorContext};

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
        if let Err(e) = handle_test(Path::new("tests")) {
            panic!("Sutra test harness failed: {}", e);
        }
    }
}
