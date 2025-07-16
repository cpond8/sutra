//! Grammar Validation Tool for Sutra Engine
//!
//! Validates grammar.pest for consistency issues that could lead to parsing bugs.
//! Provides comprehensive validation including rule reference checking, pattern
//! analysis, and critical rule coverage validation.
//!
//! ## Usage
//! ```bash
//! cargo run --bin validate_grammar
//! ```
//!
//! ## Features
//! - Parses grammar.pest and validates rule consistency
//! - Detects undefined rule references and duplicate patterns
//! - Validates critical rule coverage and spread_arg usage
//! - Reports detailed warnings, errors, and suggestions

use sutra::SutraError;
use sutra::err_ctx;
use sutra::validation::grammar::validate_grammar;

fn main() -> Result<(), SutraError> {
    let grammar_path = "src/syntax/grammar.pest";
    let validation_result = validate_grammar(grammar_path)
        .map_err(|e| err_ctx!(Internal, "Failed to validate grammar: {}", e.to_string()))?;

    if !validation_result.is_valid() {
        let mut msg = String::from("Grammar validation failed:\n");
        for err in &validation_result.errors {
            msg.push_str(&format!("  • {}\n", err));
        }
        return Err(err_ctx!(Validation, "{}", msg));
    }

    for warning in &validation_result.warnings {
        eprintln!("[Warning] {}", warning);
    }
    for suggestion in &validation_result.suggestions {
        eprintln!("[Suggestion] {}", suggestion);
    }

    Ok(())
}