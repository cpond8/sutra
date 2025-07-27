//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    process,
};

use clap::{Parser, Subcommand};

use crate::prelude::*;
use crate::{
    errors::print_error,
    atoms::EngineStdoutSink as StdoutSink,
    errors::{self, ErrorKind, ErrorReporting, SourceContext, SutraError},
    test::runner::TestRunner,
    validation::{grammar, ValidationContext},
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
    /// Evaluate Sutra code directly from command line or stdin.
    Eval {
        /// Sutra code to evaluate. If not provided, reads from stdin.
        code: Option<String>,
    },
    /// Start an interactive REPL (Read-Eval-Print Loop).
    Repl,
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
// MAIN ENTRY POINT - Direct engine calls
// ============================================================================

/// The main entry point for the CLI.
pub fn run() {
    let args = SutraArgs::parse();

    match args.command {
        ArgsCommand::Run { file } => {
            let source = read_file_or_exit(&file);
            let output = SharedOutput::new(StdoutSink);
            let pipeline = ExecutionPipeline::default();
            if let Err(e) = pipeline.execute(&source, output, &file.display().to_string()) {
                print_error(e.into());
                process::exit(1);
            }
        }

        ArgsCommand::Eval { code } => {
            let source = match code {
                Some(code_str) => code_str,
                None => {
                    // Read from stdin
                    let mut buffer = String::new();
                    if let Err(e) = io::stdin().read_to_string(&mut buffer) {
                        eprintln!("Error reading from stdin: {}", e);
                        process::exit(1);
                    }
                    buffer
                }
            };

            let output = SharedOutput::new(StdoutSink);
            let pipeline = ExecutionPipeline::default();
            if let Err(e) = pipeline.execute(&source, output, "<eval>") {
                print_error(e.into());
                process::exit(1);
            }
        }

        ArgsCommand::Repl => {
            crate::repl::run_repl();
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
                print_error(e.into());
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
            let validation_errors = grammar::validate_grammar("src/syntax/grammar.pest")
                .unwrap_or_else(|e| {
                    let context = ValidationContext {
                        source: SourceContext::from_file("sutra-cli", ""),
                        phase: "grammar-validation".to_string(),
                    };
                    let err = context.report(
                        ErrorKind::InvalidPath {
                            path: format!("Failed to validate grammar: {}", e),
                        },
                        errors::unspanned(),
                    );
                    print_error(err);
                    process::exit(1);
                });
            let valid = validation_errors.is_empty();
            let errors = validation_errors.iter().map(|e| e.to_string()).collect();
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
    use crate::discovery::TestDiscoverer;

    let test_files = match TestDiscoverer::discover_test_files(path) {
        Ok(files) => files,
        Err(e) => {
            eprintln!("Error discovering test files: {e}");
            return;
        }
    };

    if !test_files.is_empty() {
        println!("\nFound {} test files", test_files.len());
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
            println!(
                "\n\x1b[34mRunning tests... [{}/{}] ({:.1}%)\x1b[0m",
                current + 1,
                total_tests,
                progress
            );
        }

        match TestRunner::run_single_test(test_form) {
            Ok(()) => {
                passed += 1;
                println!("\x1b[32m✓\x1b[0m {}", test_form.name);
            }
            Err(e) => {
                failed += 1;
                // Let miette handle the rich error display (includes test name)
                let report = miette::Report::new(e);
                eprintln!("{report:?}");
            }
        }
    }

    // Simple summary (miette already handled the rich error display)
    println!("\n\x1b[1m統 Test Summary\x1b[0m");
    println!("═══════════════");
    if passed > 0 {
        println!("\x1b[32m✓ Passed:   {passed} tests\x1b[0m");
    }
    if failed > 0 {
        println!("\x1b[31m✗ Failed:    {failed} tests\x1b[0m");
    }

    let total = passed + failed;
    let rate = if total > 0 {
        (passed as f64 / total as f64) * 100.0
    } else {
        0.0
    };
    println!("\n\x1b[1m成 Success Rate: {rate:.1}% ({passed}/{total})\x1b[0m\n");
}

// ============================================================================
// HELPER FUNCTIONS - Common patterns extracted
// ============================================================================

fn read_file_or_exit(path: &Path) -> String {
    ExecutionPipeline::read_file(path).unwrap_or_else(|e| {
        print_error(e);
        process::exit(1);
    })
}

fn process_file_with_pipeline<F>(file: &Path, processor: F)
where
    F: FnOnce(&str) -> Result<String, SutraError>,
{
    let source = read_file_or_exit(file);
    let result = processor(&source).unwrap_or_else(|e| {
        print_error(e);
        process::exit(1);
    });
    println!("{result}");
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
        println!("  {item}");
    }
}

fn print_validation(valid: bool, errors: Vec<String>) {
    if valid {
        println!("Grammar validation passed");
    } else {
        eprintln!("Grammar validation failed:");
        for err in errors {
            eprintln!("• {err}");
        }
    }
}

// ============================================================================
// EXECUTION PIPELINE - Unified execution orchestration for CLI
// ============================================================================

/// Unified execution pipeline that enforces strict layering: Parse → Expand → Validate → Evaluate
/// This is the single source of truth for all Sutra execution paths, including tests and production.
/// All code execution, including test harnesses, must use this pipeline. Bypassing is forbidden.
pub struct ExecutionPipeline {
    /// Macro environment with canonical macros pre-loaded.
    pub world: crate::prelude::CanonicalWorld,
    /// Macro environment with canonical macros pre-loaded.
    pub macro_env: crate::macros::MacroExpansionContext,
    /// Maximum recursion depth for evaluation
    pub max_depth: usize,
    /// Whether to validate expanded AST before evaluation
    pub validate: bool,
}

impl Default for ExecutionPipeline {
    fn default() -> Self {
        use crate::atoms::{build_canonical_macro_env, build_canonical_world};
        use miette::NamedSource;
        use std::sync::Arc;

        Self {
            world: build_canonical_world(),
            macro_env: build_canonical_macro_env().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to load standard macros: {}", e);
                // Create a minimal macro environment with only core macros
                crate::macros::MacroExpansionContext {
                    user_macros: std::collections::HashMap::new(),
                    core_macros: std::collections::HashMap::new(),
                    trace: Vec::new(),
                    source: Arc::new(NamedSource::new("fallback", String::new())),
                }
            }),
            max_depth: 100,
            validate: false, // Keep validation disabled for now
        }
    }
}

impl ExecutionPipeline {
    // ============================================================================
    // CLI SERVICE METHODS - Pure execution services for CLI orchestration
    // ============================================================================

    /// Executes source code with pure execution logic (no I/O, no formatting)
    pub fn execute_source(
        source: &str,
        output: crate::atoms::SharedOutput,
    ) -> Result<(), SutraError> {
        Self::default().execute(source, output, "source")
    }

    /// Parses source code with pure parsing logic (no I/O)
    pub fn parse_source(source: &str) -> Result<Vec<AstNode>, SutraError> {
        use crate::syntax::parser;
        let source_context = SourceContext::from_file("source", source);
        parser::parse(source, source_context)
    }

    /// Expands macros in source code with pure expansion logic (no I/O)
    pub fn expand_macros_source(source: &str) -> Result<String, SutraError> {
        use crate::{macros::MacroProcessor, syntax::parser};
        let processor = MacroProcessor::default();
        let source_context = SourceContext::from_file("source", source);
        let ast_nodes = parser::parse(source, source_context)?;
        let (expanded, _env) = processor.partition_and_process_macros(ast_nodes)?;
        Ok(expanded.value.pretty())
    }

    /// Reads a file with standardized error handling
    pub fn read_file(path: &std::path::Path) -> Result<String, SutraError> {
        use crate::validation::ValidationContext;

        let filename = path.to_str().ok_or_else(|| {
            let context = ValidationContext {
                source: SourceContext::fallback("ExecutionPipeline::read_file"),
                phase: "file-system".to_string(),
            };
            context.report(
                crate::errors::ErrorKind::InvalidPath {
                    path: path.to_string_lossy().to_string(),
                },
                crate::errors::unspanned(),
            )
        })?;

        std::fs::read_to_string(filename).map_err(|error| {
            let context = ValidationContext {
                source: SourceContext::fallback("ExecutionPipeline::read_file"),
                phase: "file-system".to_string(),
            };
            context.report(
                crate::errors::ErrorKind::InvalidPath {
                    path: format!("{} ({})", filename, error),
                },
                crate::errors::unspanned(),
            )
        })
    }

    // ============================================================================
    // REGISTRY ACCESS SERVICES - Pure registry access for CLI
    // ============================================================================

    /// Gets the macro registry (pure access, no I/O)
    pub fn get_macro_registry() -> crate::macros::MacroExpansionContext {
        use crate::atoms::build_canonical_macro_env;
        build_canonical_macro_env().expect("Standard macro env should build")
    }

    /// Lists all available atoms (pure access, no I/O)
    pub fn list_atoms() -> Vec<String> {
        use crate::atoms::build_canonical_world;
        let world = build_canonical_world();
        let world = world.borrow();
        if let Some(Value::Map(map)) = world.state.get(&crate::atoms::Path(vec![])) {
            map.keys().cloned().collect()
        } else {
            vec![]
        }
    }

    /// Lists all available macros (pure access, no I/O)
    pub fn list_macros() -> Vec<String> {
        let macro_registry = Self::get_macro_registry();
        let mut items = Vec::new();
        items.extend(macro_registry.core_macros.keys().cloned());
        items.extend(macro_registry.user_macros.keys().cloned());
        items
    }

    // ============================================================================
    // PUBLIC EXECUTION METHODS
    // ============================================================================

    /// Core execution method that processes AST nodes through the full pipeline.
    pub fn execute_nodes(
        &self,
        nodes: &[AstNode],
        output: crate::atoms::SharedOutput,
        source_context: SourceContext,
    ) -> Result<Value, SutraError> {
        use crate::macros::MacroProcessor;

        // Step 1: Create a macro processor with the pipeline's configuration.
        let processor = MacroProcessor::new(self.validate, self.max_depth);
        let mut env = self.macro_env.clone();

        // Step 2: Expand macros using the pipeline's environment.
        let expanded = processor.process_with_existing_macros(nodes.to_vec(), &mut env)?;

        // Step 3: Validate the expanded AST.
        processor.validate_expanded_ast(&expanded, &env, &self.world.borrow(), &source_context)?;

        // Step 4: Evaluate the final AST, using the pipeline's world and output sink.
        crate::runtime::evaluate(&expanded, self.world.clone(), output, source_context)
    }

    /// Executes Sutra source code through the complete pipeline.
    /// This parses source then calls execute_nodes() for unified processing.
    pub fn execute(
        &self,
        source_text: &str,
        output: crate::atoms::SharedOutput,
        filename: &str,
    ) -> Result<(), SutraError> {
        use crate::syntax::parser;

        // Step 1: Create a source context from the raw text.
        let source_context = SourceContext::from_file(filename, source_text);

        // Step 2: Parse the source into AST nodes.
        let ast_nodes = parser::parse(source_text, source_context.clone())?;

        // Step 3: Execute the nodes through the unified pipeline.
        let result = self.execute_nodes(&ast_nodes, output.clone(), source_context)?;

        // Step 4: Emit the final result to the output sink if it's not nil.
        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }

        Ok(())
    }

    /// Executes already-expanded AST nodes, bypassing macro processing.
    /// This is optimized for test execution where AST is already available.
    pub fn execute_expanded_ast(
        &self,
        expanded_ast: &AstNode,
        world: crate::prelude::CanonicalWorld,
        output: crate::atoms::SharedOutput,
        source: SourceContext,
    ) -> Result<Value, SutraError> {
        crate::runtime::evaluate(expanded_ast, world, output, source)
    }
}
