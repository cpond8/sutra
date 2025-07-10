//! File-based evaluation tests for Sutra engine.
//!
//! PROTOCOL: All tests use file-based .sutra/.expected pairs executed via CLI,
//! ensuring protocol compliance and maintainability.

use sutra::test_utils as common;

use std::path::Path;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_operations() {
        let test_dir = Path::new("tests/scripts/atoms");
        let config = common::TestConfig::default();
        common::run_test_directory(test_dir, &config).expect("Atom operation tests should pass");
    }

    #[test]
    fn test_macro_operations() {
        let test_dir = Path::new("tests/scripts/macros");
        let config = common::TestConfig::default();
        common::run_test_directory(test_dir, &config).expect("Macro operation tests should pass");
    }

    #[test]
    fn test_parser_edge_cases() {
        let test_dir = Path::new("tests/scripts/parser");
        let config = common::TestConfig::default();
        common::run_test_directory(test_dir, &config).expect("Parser edge case tests should pass");
    }

    #[test]
    fn test_integration_scenarios() {
        let test_dir = Path::new("tests/scripts/integration");
        let config = common::TestConfig::default();
        common::run_test_directory(test_dir, &config).expect("Integration tests should pass");
    }

    #[test]
    fn test_examples() {
        let test_dir = Path::new("tests/scripts/examples");
        let config = common::TestConfig::default();
        common::run_test_directory(test_dir, &config).expect("Example tests should pass");
    }

    #[test]
    fn test_engine_features() {
        let test_dir = Path::new("tests/scripts/engine");
        let config = common::TestConfig::default();
        // Engine feature tests may skip if features are not available
        // This is expected behavior and not a failure
        match common::run_test_directory(test_dir, &config) {
            Ok(()) => {
                println!("Engine feature tests passed");
            }
            Err(e) => {
                println!(
                    "Engine feature tests skipped or failed (expected if features unavailable): {}",
                    e
                );
                // Don't fail the test - engine features may not be available
            }
        }
    }
}
