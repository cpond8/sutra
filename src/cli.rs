//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use std::{
    io::Read,
    path::{Path, PathBuf},
    process,
};

use clap::{Parser, Subcommand};

use crate::prelude::*;
use crate::{
    atoms::{EngineStdoutSink, SharedOutput},
    build_canonical_macro_env, build_canonical_world,
    discovery::TestDiscoverer,
    errors::{self, print_error, ErrorKind, ErrorReporting, SourceContext, SutraError},
    evaluate,
    macros::MacroSystem,
    syntax::parser,
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
// MAIN ENTRY POINT - Simplified engine calls
// ============================================================================

/// Simplified Sutra execution engine that replaces ExecutionPipeline
struct SutraEngine {
    world: CanonicalWorld,
    macro_env: MacroSystem,
}

impl SutraEngine {
    fn new() -> Self {
        Self {
            world: build_canonical_world(),
            macro_env: build_canonical_macro_env().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to load standard macros: {}", e);
                MacroSystem::new()
            }),
        }
    }

    fn execute(&mut self, source: &str, filename: &str) -> Result<(), SutraError> {
        let source_context = SourceContext::from_file(filename, source);
        let ast_nodes = parser::parse(source, source_context.clone())?;
        let program = parser::wrap_in_do(ast_nodes);
        let expanded = self.macro_env.expand(program)?;
        let output = SharedOutput::new(EngineStdoutSink);
        let result = evaluate(
            &expanded,
            self.world.clone(),
            output.clone(),
            source_context,
        )?;

        if !result.is_nil() {
            output.emit(&result.to_string(), None);
        }
        Ok(())
    }

    fn expand_macros(&mut self, source: &str) -> Result<String, SutraError> {
        let source_context = SourceContext::from_file("source", source);
        let ast_nodes = parser::parse(source, source_context)?;
        let program = parser::wrap_in_do(ast_nodes);
        let expanded = self.macro_env.expand(program)?;
        Ok(expanded.value.pretty())
    }

    fn trace_macros(&mut self, source: &str) -> Result<(), SutraError> {
        // TODO: Implement actual tracing with step-by-step expansion
        let expanded = self.expand_macros(source)?;
        println!("{expanded}");
        Ok(())
    }

    fn format(&self, source: &str) -> Result<String, SutraError> {
        let source_context = SourceContext::from_file("source", source);
        let ast_nodes = parser::parse(source, source_context)?;
        let program = parser::wrap_in_do(ast_nodes);
        Ok(program.value.pretty())
    }

    fn parse(&self, source: &str) -> Result<Vec<AstNode>, SutraError> {
        let source_context = SourceContext::from_file("source", source);
        parser::parse(source, source_context)
    }

    fn list_macros(&self) -> Vec<String> {
        self.macro_env.macro_names()
    }

    fn list_atoms(&self) -> Vec<String> {
        let world = self.world.borrow();
        if let Some(Value::Map(map)) = world.state.get(&crate::atoms::Path(vec![])) {
            map.keys().cloned().collect()
        } else {
            vec![]
        }
    }
}

/// Read a file with proper error handling
fn read_file(path: &Path) -> Result<String, SutraError> {
    let filename = path.to_str().ok_or_else(|| {
        let context = ValidationContext {
            source: SourceContext::fallback("read_file"),
            phase: "file-system".to_string(),
        };
        context.report(
            ErrorKind::InvalidPath {
                path: path.to_string_lossy().to_string(),
            },
            errors::unspanned(),
        )
    })?;

    std::fs::read_to_string(filename).map_err(|error| {
        let context = ValidationContext {
            source: SourceContext::fallback("read_file"),
            phase: "file-system".to_string(),
        };
        context.report(
            ErrorKind::InvalidPath {
                path: format!("{} ({})", filename, error),
            },
            errors::unspanned(),
        )
    })
}

/// Read from stdin with proper error handling
fn read_stdin() -> Result<String, SutraError> {
    let mut buffer = String::new();
    std::io::stdin().read_to_string(&mut buffer).map_err(|e| {
        let context = ValidationContext {
            source: SourceContext::fallback("read_stdin"),
            phase: "input".to_string(),
        };
        context.report(
            ErrorKind::InvalidPath {
                path: format!("stdin ({})", e),
            },
            errors::unspanned(),
        )
    })?;
    Ok(buffer)
}

/// Validate grammar file
fn validate_grammar() -> Result<(), SutraError> {
    let validation_errors = grammar::validate_grammar("src/syntax/grammar.pest").map_err(|e| {
        let context = ValidationContext {
            source: SourceContext::from_file("sutra-cli", ""),
            phase: "grammar-validation".to_string(),
        };
        context.report(
            ErrorKind::InvalidPath {
                path: format!("Failed to validate grammar: {}", e),
            },
            errors::unspanned(),
        )
    })?;

    if validation_errors.is_empty() {
        println!("Grammar validation passed");
    } else {
        eprintln!("Grammar validation failed:");
        for err in validation_errors.iter() {
            eprintln!("• {err}");
        }
        process::exit(1);
    }
    Ok(())
}

/// Simplified test runner
fn run_tests(path: PathBuf) -> Result<(), SutraError> {
    let test_files = TestDiscoverer::discover_test_files(path).map_err(|e| {
        let context = ValidationContext {
            source: SourceContext::fallback("run_tests"),
            phase: "test-discovery".to_string(),
        };
        context.report(
            ErrorKind::InvalidPath {
                path: format!("Test discovery failed: {}", e),
            },
            errors::unspanned(),
        )
    })?;

    let mut total_tests = 0;
    let mut passed = 0;
    let mut failed = 0;

    for file_path in test_files {
        let test_forms = TestDiscoverer::extract_tests_from_file(&file_path).map_err(|e| {
            let context = ValidationContext {
                source: SourceContext::fallback("run_tests"),
                phase: "test-extraction".to_string(),
            };
            context.report(
                ErrorKind::InvalidPath {
                    path: format!(
                        "Failed to extract tests from {}: {}",
                        file_path.display(),
                        e
                    ),
                },
                errors::unspanned(),
            )
        })?;

        for test_form in test_forms {
            total_tests += 1;
            match TestRunner::run_single_test(&test_form) {
                Ok(()) => {
                    passed += 1;
                    println!("✓ {}", test_form.name);
                }
                Err(e) => {
                    failed += 1;
                    let report = miette::Report::new(e);
                    eprintln!("{report:?}");
                }
            }
        }
    }

    println!("\nTest Summary: {passed}/{total_tests} passed");
    if failed > 0 {
        process::exit(1);
    }
    Ok(())
}

/// The main entry point for the CLI.
pub fn run() {
    if let Err(e) = run_inner() {
        print_error(e.into());
        process::exit(1);
    }
}

fn run_inner() -> Result<(), SutraError> {
    let args = SutraArgs::parse();
    let mut engine = SutraEngine::new();

    match args.command {
        ArgsCommand::Run { file } => {
            let source = read_file(&file)?;
            engine.execute(&source, &file.display().to_string())
        }

        ArgsCommand::Eval { code } => {
            let source = match code {
                Some(code_str) => code_str,
                None => read_stdin()?,
            };
            engine.execute(&source, "<eval>")
        }

        ArgsCommand::Repl => {
            crate::repl::run_repl();
            Ok(())
        }

        ArgsCommand::Macroexpand { file } => {
            let source = read_file(&file)?;
            let expanded = engine.expand_macros(&source)?;
            println!("{expanded}");
            Ok(())
        }

        ArgsCommand::Macrotrace { file } => {
            let source = read_file(&file)?;
            engine.trace_macros(&source)
        }

        ArgsCommand::Format { file } => {
            let source = read_file(&file)?;
            let formatted = engine.format(&source)?;
            println!("{formatted}");
            Ok(())
        }

        ArgsCommand::Ast { file } => {
            let source = read_file(&file)?;
            let ast = engine.parse(&source)?;
            println!("{ast:#?}");
            Ok(())
        }

        ArgsCommand::ListMacros => {
            for macro_name in engine.list_macros() {
                println!("  {macro_name}");
            }
            Ok(())
        }

        ArgsCommand::ListAtoms => {
            for atom_name in engine.list_atoms() {
                println!("  {atom_name}");
            }
            Ok(())
        }

        ArgsCommand::ValidateGrammar => validate_grammar(),

        ArgsCommand::Test { path } => run_tests(path),
    }
}

// ============================================================================
// EXECUTION PIPELINE - Legacy code kept for compatibility
// ============================================================================

/// Unified execution pipeline that enforces strict layering: Parse → Expand → Validate → Evaluate
/// This is the single source of truth for all Sutra execution paths, including tests and production.
/// All code execution, including test harnesses, must use this pipeline. Bypassing is forbidden.
pub struct ExecutionPipeline {
    /// Macro environment with canonical macros pre-loaded.
    pub world: crate::prelude::CanonicalWorld,
    /// Macro environment with canonical macros pre-loaded.
    pub macro_env: MacroSystem,
    /// Maximum recursion depth for evaluation
    pub max_depth: usize,
    /// Whether to validate expanded AST before evaluation
    pub validate: bool,
}

impl Default for ExecutionPipeline {
    fn default() -> Self {
        Self {
            world: build_canonical_world(),
            macro_env: build_canonical_macro_env().unwrap_or_else(|e| {
                eprintln!("Warning: Failed to load standard macros: {}", e);
                // Create a minimal macro environment with only core macros
                MacroSystem::new()
            }),
            max_depth: 100,
            validate: false, // Keep validation disabled for now
        }
    }
}

impl ExecutionPipeline {
    /// Core execution method that processes AST nodes through the full pipeline.
    /// Kept for compatibility with test runner.
    pub fn execute_nodes(
        &self,
        nodes: &[AstNode],
        output: SharedOutput,
        source_context: SourceContext,
    ) -> Result<Value, SutraError> {
        use crate::syntax::parser;
        
        let env = self.macro_env.clone();

        // Wrap user code in a (do ...) block if needed
        let program = parser::wrap_in_do(nodes.to_vec());

        // Expand macros using the pipeline's environment.
        let expanded = env.expand(program)?;

        // Evaluate the final AST, using the pipeline's world and output sink.
        evaluate(&expanded, self.world.clone(), output, source_context)
    }

    /// Executes already-expanded AST nodes, bypassing macro processing.
    /// This is optimized for test execution where AST is already available.
    pub fn execute_expanded_ast(
        &self,
        expanded_ast: &AstNode,
        world: CanonicalWorld,
        output: SharedOutput,
        source: SourceContext,
    ) -> Result<Value, SutraError> {
        evaluate(expanded_ast, world, output, source)
    }
}
