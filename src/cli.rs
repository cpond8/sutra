//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use std::{path::PathBuf, process};

use clap::{Parser, Subcommand};

use crate::{
    engine::{print_error, EngineStdoutSink as StdoutSink, ExecutionPipeline},
    validation::validate_grammar,
    SharedOutput, SutraError,
};

// ============================================================================
// CLI ARGUMENTS - Command-line argument definitions
// ============================================================================

/// The main CLI argument structure.
#[derive(Debug, Parser)]
#[command(
    name = "sutra",
    version,
    about = "A compositional, emergent, and narrative-rich game engine."
)]
pub struct SutraArgs {
    #[command(subcommand)]
    pub command: ArgsCommand,
}

/// An enumeration of all available CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum ArgsCommand {
    /// Full pipeline: parse, expand, validate, eval, and output.
    Run {
        /// The path to the Sutra script file to run.
        #[arg(required = true)]
        file: PathBuf,
    },
    /// Print the fully macro-expanded code.
    Macroexpand {
        /// The path to the Sutra script file to expand.
        #[arg(required = true)]
        file: PathBuf,
    },
    /// Show a stepwise macro expansion trace with diffs.
    Macrotrace {
        /// The path to the Sutra script file to trace.
        #[arg(required = true)]
        file: PathBuf,
    },
    /// Validate the grammar.pest file for correctness.
    ValidateGrammar,
    /// Pretty-print and normalize a script.
    Format {
        /// The path to the Sutra script file to format.
        #[arg(required = true)]
        file: PathBuf,
    },
    /// Discover and run all test scripts in a directory.
    Test {
        /// The path to the directory containing test scripts.
        #[arg(default_value = "tests")]
        path: PathBuf,
    },
    /// List all available macros with their documentation.
    ListMacros,
    /// List all available atoms with their documentation.
    ListAtoms,
    /// Show the Abstract Syntax Tree (AST) for a script.
    Ast {
        /// The path to the Sutra script file to parse.
        #[arg(required = true)]
        file: PathBuf,
    },
}

// ============================================================================
// OUTPUT TYPES - Re-export engine types for CLI
// ============================================================================

// ============================================================================
// MAIN ENTRY POINT - Direct engine calls
// ============================================================================

/// The main entry point for the CLI.
pub fn run() {
    let args = SutraArgs::parse();

    match args.command {
        ArgsCommand::Run { file } => {
            let source = read_file_or_exit(&file);
            let output = SharedOutput::new(StdoutSink);
            if let Err(e) = ExecutionPipeline::execute_source(&source, output) {
                print_error(e);
                process::exit(1);
            }
        }

        ArgsCommand::Macroexpand { file } => {
            process_file_with_pipeline(&file, ExecutionPipeline::expand_macros_source);
        }

        ArgsCommand::Macrotrace { file } => {
            // TODO: Implement actual macro tracing with diffs
            process_file_with_pipeline(&file, ExecutionPipeline::expand_macros_source);
        }

        ArgsCommand::Format { file } => {
            process_file_with_pipeline(&file, ExecutionPipeline::expand_macros_source);
        }

        ArgsCommand::Ast { file } => {
            let source = read_file_or_exit(&file);
            let ast = ExecutionPipeline::parse_source(&source).unwrap_or_else(|e| {
                print_error(e);
                process::exit(1);
            });
            print_ast(&ast);
        }

        ArgsCommand::ListMacros => {
            list_registry_items(ExecutionPipeline::list_macros);
        }

        ArgsCommand::ListAtoms => {
            list_registry_items(ExecutionPipeline::list_atoms);
        }

        ArgsCommand::ValidateGrammar => {
            let validation_result =
                validate_grammar("src/syntax/grammar.pest").unwrap_or_else(|e| {
                    print_error(crate::err_msg!(
                        Internal,
                        format!("Failed to validate grammar: {}", e)
                    ));
                    process::exit(1);
                });
            let valid = validation_result.is_valid();
            let errors = validation_result
                .errors
                .iter()
                .map(|e| e.to_string())
                .collect();
            print_validation(valid, errors);
        }

        ArgsCommand::Test { path } => {
            run_test_suite(path);
        }
    }
}

// ============================================================================
// FLAT, LINEAR TEST RUNNER (Encapsulated)
// ============================================================================

fn run_test_suite(path: PathBuf) {
    use crate::{
        discovery::TestDiscoverer,
        engine::ExecutionPipeline,
    };

    let test_files = match TestDiscoverer::discover_test_files(path) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("Error discovering test files: {}", e);
            return;
        }
    };

    if !test_files.is_empty() {
        println!("Found {} test files", test_files.len());
    }

    // Collect all tests first for progress tracking
    let mut all_tests = Vec::new();
    for file_path in test_files {
        let test_forms = match TestDiscoverer::extract_tests_from_file(&file_path) {
            Ok(forms) => forms,
            Err(e) => {
                eprintln!("Error parsing test file {}: {}", file_path.display(), e);
                continue;
            }
        };
        all_tests.extend(test_forms.into_iter().map(|tf| (file_path.clone(), tf)));
    }

    let total_tests = all_tests.len();
    let mut passed = 0;
    let mut failed = 0;

    // Run tests with progress
    for (current, (_file_path, test_form)) in all_tests.iter().enumerate() {
        // Progress indicator
        if current % 5 == 0 || current == total_tests - 1 {
            let progress = ((current + 1) as f64 / total_tests as f64) * 100.0;
            println!("\x1b[34mRunning tests... [{}/{}] ({:.1}%)\x1b[0m", current + 1, total_tests, progress);
        }

        match ExecutionPipeline::run_single_test(&test_form) {
            Ok(()) => {
                passed += 1;
                println!("\x1b[32m✓\x1b[0m {}", test_form.name);
            }
            Err(e) => {
                failed += 1;
                println!("\x1b[31m✗\x1b[0m {}", test_form.name);

                // Let miette handle the rich error display
                let report = miette::Report::new(e);
                eprintln!("{report:?}\n----------------\n");
            }
        }
    }

    // Simple summary (miette already handled the rich error display)
    println!("\n\x1b[1m統 Test Summary\x1b[0m");
    println!("═══════════════");
    if passed > 0 {
        println!("\x1b[32m✓ Passed:   {} tests\x1b[0m", passed);
    }
    if failed > 0 {
        println!("\x1b[31m✗ Failed:    {} tests\x1b[0m", failed);
    }

    let total = passed + failed;
    let rate = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    println!("\n\x1b[1m成 Success Rate: {:.1}% ({}/{})\x1b[0m", rate, passed, total);
}

// ============================================================================
// HELPER FUNCTIONS - Common patterns extracted
// ============================================================================

fn read_file_or_exit(path: &PathBuf) -> String {
    ExecutionPipeline::read_file(path).unwrap_or_else(|e| {
        print_error(e);
        process::exit(1);
    })
}

fn process_file_with_pipeline<F>(file: &PathBuf, processor: F)
where
    F: FnOnce(&str) -> Result<String, SutraError>,
{
    let source = read_file_or_exit(file);
    let result = processor(&source).unwrap_or_else(|e| {
        print_error(e);
        process::exit(1);
    });
    println!("{}", result);
}

fn list_registry_items<F>(list_fn: F)
where
    F: FnOnce() -> Vec<String>,
{
    let items = list_fn();
    print_registry(&items);
}

// ============================================================================
// OUTPUT FUNCTIONS - Simple, direct output
// ============================================================================

fn print_ast(ast: &[crate::AstNode]) {
    if ast.is_empty() {
        println!("(empty)");
        return;
    }

    for (node_index, node) in ast.iter().enumerate() {
        if ast.len() > 1 {
            println!("\nNode {}:", node_index + 1);
        }
        println!("{node:#?}");
    }
}

fn print_registry(items: &[String]) {
    if items.is_empty() {
        println!("  No items found.");
        return;
    }

    for item in items {
        println!("  {}", item);
    }
}

fn print_validation(valid: bool, errors: Vec<String>) {
    if valid {
        println!("Grammar validation passed");
    } else {
        eprintln!("Grammar validation failed:");
        for err in errors {
            eprintln!("• {}", err);
        }
    }
}
