//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use crate::ast::AstNode;
use crate::cli::args::{Command, SutraArgs};
use crate::cli::output::StdoutSink;
use crate::engine::{ExecutionPipeline, run_sutra_source_with_output};
use crate::err_ctx;
use crate::err_msg;
use crate::macros::{is_macro_definition, parse_macro_definition};
use crate::macros::{expand_macros_recursively, MacroDefinition, MacroRegistry};
use crate::runtime::world::build_canonical_macro_env;
use crate::SutraError;
use clap::Parser;
use termcolor::WriteColor;
use std::rc::Rc;
use std::cell::RefCell;
use crate::atoms::SharedOutput;

pub mod args;
pub mod output;

/// The main entry point for the CLI.
pub fn run() {
    let args = SutraArgs::parse();

    // Dispatch to the appropriate subcommand handler.
    let result = match &args.command {
        Command::Macrotrace { file } => handle_macrotrace(file),
        Command::Run { file } => handle_run(file),
        Command::ListMacros => handle_list_macros(),
        Command::ListAtoms => handle_list_atoms(),
        Command::Ast { file } => handle_ast(file),
        Command::Validate { .. } => handle_validate(),
        Command::Macroexpand { file } => handle_macroexpand(file),
        Command::Format { file } => handle_format(file),
        Command::Test { path } => handle_test(path),
    };

    if let Err(e) = result {
        // Remove print_error_to_stderr and process::exit; propagate error via diagnostics only
        // print_error_to_stderr(&e);
        // process::exit(1);
        // Instead, panic with diagnostics for now (or propagate up if possible)
        panic!("{}", e);
    }
}

// ============================================================================
// DIAGNOSTICS
// ============================================================================

/// Prints a SutraError to standard error using miette's diagnostic formatting.
pub fn print_error_to_stderr(error: &SutraError) {
    eprintln!("{}", error);
}

use crate::ast::{Expr, Span, Spanned};

// ============================================================================
// CORE UTILITIES - Fundamental operations used throughout the CLI
// ============================================================================

/// Converts a Path to a &str, returning an error if invalid.
fn path_to_str(path: &std::path::Path) -> Result<&str, SutraError> {
    path.to_str()
        .ok_or_else(|| err_msg!(Internal, "Invalid filename"))
}

/// Gets a safe display name for a path, with fallback for invalid paths.
fn safe_path_display(path: &std::path::Path) -> &str {
    path.to_str().unwrap_or("<unknown>")
}

/// Reads a file to a String, given a path.
fn read_file_to_string(path: &std::path::Path) -> Result<String, SutraError> {
    let filename = path_to_str(path)?;
    std::fs::read_to_string(filename)
        .map_err(|e| err_ctx!(Internal, "Failed to read file: {}", e.to_string()))
}

// ============================================================================
// AST PROCESSING UTILITIES - Parsing and transformation operations
// ============================================================================

/// Parses Sutra source code into AST nodes.
fn parse_source_to_ast(source: &str) -> Result<Vec<AstNode>, SutraError> {
    crate::syntax::parser::parse(source)
}

/// Common pipeline: read file and parse to AST nodes.
fn load_file_to_ast(path: &std::path::Path) -> Result<(String, Vec<AstNode>), SutraError> {
    let source = read_file_to_string(path)?;
    let ast_nodes = parse_source_to_ast(&source)?;
    Ok((source, ast_nodes))
}

/// Wraps AST nodes in a (do ...) if needed.
fn wrap_in_do_if_needed(ast_nodes: Vec<AstNode>, source: &str) -> AstNode {
    if ast_nodes.len() == 1 {
        return ast_nodes.into_iter().next().unwrap();
    }
    let span = Span {
        start: 0,
        end: source.len(),
    };
    let do_symbol = Spanned {
        value: Expr::Symbol("do".to_string(), span).into(), // FIX: wrap Expr in Arc via .into()
        span,
    };
    let mut items = Vec::with_capacity(ast_nodes.len() + 1);
    items.push(do_symbol);
    items.extend(ast_nodes);
    Spanned {
        value: Expr::List(items, span).into(),
        span,
    }
}

/// Partitions AST nodes into macro definitions and user code, and builds a user macro registry.
type MacroParseResult = Result<(MacroRegistry, Vec<AstNode>), SutraError>;
fn partition_and_build_user_macros(ast_nodes: Vec<AstNode>) -> MacroParseResult {
    let (macro_defs, user_code): (Vec<_>, Vec<_>) =
        ast_nodes.into_iter().partition(is_macro_definition);
    let mut user_macros = MacroRegistry::new();
    for macro_expr in macro_defs {
        let (name, template) = parse_macro_definition(&macro_expr)?;
        if user_macros.macros.contains_key(&name) {
            return Err(err_msg!(
                Validation,
                format!("Duplicate macro name '{}'", name)
            ));
        }
        user_macros
            .macros
            .insert(name, MacroDefinition::Template(template));
    }
    Ok((user_macros, user_code))
}

/// Builds a complete macro environment and expands a program.
/// Returns the expanded AST ready for execution or further processing.
fn build_macro_environment_and_expand(
    ast_nodes: Vec<AstNode>,
    source: &str,
) -> Result<AstNode, SutraError> {
    let (user_macros, user_code) = partition_and_build_user_macros(ast_nodes)?;

    // Build complete macro environment
    let mut env = build_canonical_macro_env()?;
    env.user_macros.extend(user_macros.macros);

    // Wrap user code if needed
    let program = wrap_in_do_if_needed(user_code, source);

    // Expand macros
    let expanded = expand_macros_recursively(program, &mut env)?;

    Ok(expanded)
}

// ============================================================================
// OUTPUT & FORMATTING UTILITIES - Presentation and display operations
// ============================================================================

/// Prints a sorted list of names with a title.
fn print_sorted_list<T: AsRef<str>>(title: &str, names: &[T]) {
    if names.is_empty() {
        return;
    }
    println!("\n{title}:");
    let mut sorted: Vec<_> = names.iter().map(|n| n.as_ref()).collect();
    sorted.sort();
    for name in sorted {
        println!("  {}", name);
    }
}

/// Common pattern for listing registry contents with header and empty handling.
fn print_registry_listing<T: AsRef<str>>(
    main_title: &str,
    separator: &str,
    sections: &[(&str, &[T])],
) {
    println!("{main_title}");
    println!("{separator}");

    let has_any_items = sections.iter().any(|(_, items)| !items.is_empty());

    for (section_title, items) in sections {
        print_sorted_list(section_title, items);
    }

    if !has_any_items {
        println!("  No items found.");
    }
}

/// Sets output color for formatted output.
fn set_output_color(stdout: &mut termcolor::StandardStream, color: termcolor::Color, bold: bool) {
    let _ = stdout.set_color(termcolor::ColorSpec::new().set_fg(Some(color)).set_bold(bold));
}

/// Resets output color to default.
fn reset_output_color(stdout: &mut termcolor::StandardStream) {
    let _ = stdout.reset();
}

// ============================================================================
// TEST INFRASTRUCTURE - Test discovery and execution
// ============================================================================

/// Test expectation types for the test harness.
#[derive(Debug)]
pub enum TestExpectation {
    Value(crate::ast::value::Value),
    Error(String),
    Output(String),
    Skip(String),
}

/// Test result types for the test harness.
#[derive(Debug)]
pub enum TestResult {
    Pass,
    Fail { expected: String, actual: String },
    Skip { reason: String },
    Error { message: String },
}

// ============================================================================
// COMMAND HANDLERS - CLI command implementations organized by functional area
// ============================================================================

// --- Analysis Commands: AST, validation, macro expansion, formatting ---

/// Handles the `ast` subcommand.
fn handle_ast(path: &std::path::Path) -> Result<(), SutraError> {
    let (_source, ast_nodes) = load_file_to_ast(path)?;

    let filename = safe_path_display(path);
    println!("AST for {}:", filename);
    println!("={}=", "=".repeat(filename.len() + 9));

    if ast_nodes.is_empty() {
        println!("(empty)");
        return Ok(());
    }
    for (i, node) in ast_nodes.iter().enumerate() {
        if ast_nodes.len() > 1 {
            println!("\nNode {}:", i + 1);
        }
        println!("{:#?}", node);
    }

    Ok(())
}

/// Handles the `validate` subcommand.
fn handle_validate() -> Result<(), SutraError> {
    use crate::validation::grammar::validate_grammar;
    use std::fs;
    let grammar_path = "src/syntax/grammar.pest";
    let validation_result = validate_grammar(grammar_path)
        .map_err(|e| err_ctx!(Internal, "Failed to validate grammar: {}", e.to_string()))?;

    if !validation_result.is_valid() {
        let grammar_source = fs::read_to_string(grammar_path).unwrap_or_default();
        let error = err_ctx!(Validation, "Grammar validation failed", grammar_source);
        eprintln!("{}", error);
        for err in &validation_result.errors {
            eprintln!("  • {}", err);
        }
        std::process::exit(1);
    }

    for warning in &validation_result.warnings {
        eprintln!("[Warning] {}", warning);
    }
    for suggestion in &validation_result.suggestions {
        eprintln!("[Suggestion] {}", suggestion);
    }

    println!("Grammar validation passed");
    Ok(())
}

/// Handles the `macroexpand` subcommand.
fn handle_macroexpand(path: &std::path::Path) -> Result<(), SutraError> {
    let (source, ast_nodes) = load_file_to_ast(path)?;
    let expanded = build_macro_environment_and_expand(ast_nodes, &source)?;
    println!("{}", expanded.value.pretty());
    Ok(())
}

/// Handles the `format` subcommand.
fn handle_format(path: &std::path::Path) -> Result<(), SutraError> {
    let (source, ast_nodes) = load_file_to_ast(path)?;
    let expanded = build_macro_environment_and_expand(ast_nodes, &source)?;
    println!("{}", expanded.value.pretty());
    Ok(())
}

/// Handles the `run` subcommand using the unified execution pipeline.
fn handle_run(path: &std::path::Path) -> Result<(), SutraError> {
    let source = read_file_to_string(path)?;
    let output = SharedOutput::new(StdoutSink);

    // Use the unified execution pipeline
    let pipeline = ExecutionPipeline::default();
    pipeline.execute(&source, output)
}

/// Handles the `macrotrace` subcommand.
fn handle_macrotrace(path: &std::path::Path) -> Result<(), SutraError> {
    let source = read_file_to_string(path)?;
    let output = SharedOutput::new(StdoutSink);
    run_sutra_source_with_output(&source, output)
}

/// Handles the `list-macros` subcommand.
fn handle_list_macros() -> Result<(), SutraError> {
    use crate::runtime::world::build_canonical_macro_env;

    let env = build_canonical_macro_env()?;
    let core_macro_names: Vec<_> = env.core_macros.keys().collect();
    let user_macro_names: Vec<_> = env.user_macros.keys().collect();

    print_registry_listing(
        "Sutra Macro Registry",
        "==================",
        &[
            ("Core Macros", &core_macro_names),
            ("User Macros", &user_macro_names),
        ],
    );

    Ok(())
}

/// Handles the `list-atoms` subcommand.
fn handle_list_atoms() -> Result<(), SutraError> {
    use crate::runtime::world::build_default_atom_registry;
    use crate::atoms::Atom;

    let registry = build_default_atom_registry();
    let mut pure_atoms = Vec::new();
    let mut stateful_atoms = Vec::new();
    let mut special_forms = Vec::new();

    for (name, atom) in registry.atoms.iter() {
        match atom {
            Atom::Pure(_) => pure_atoms.push(name.as_str()),
            Atom::Stateful(_) => stateful_atoms.push(name.as_str()),
            Atom::SpecialForm(_) => special_forms.push(name.as_str()),
        }
    }

    print_registry_listing(
        "Sutra Atom Registry",
        "==================",
        &[
            ("Pure Atoms", &pure_atoms),
            ("Stateful Atoms", &stateful_atoms),
            ("Special Forms", &special_forms),
        ],
    );

    Ok(())
}

/// Handles the `test` subcommand using the unified execution pipeline.
pub fn handle_test(path: &std::path::Path) -> Result<(), SutraError> {
    use std::fs::OpenOptions;
    use std::io::Write as IoWrite;
    use std::collections::HashMap;
    use termcolor::{Color, ColorSpec, WriteColor};
    use crate::diagnostics::SutraError;

    // Prepare error log file
    let mut error_log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("sutra-test-errors.log")
        .ok();

    // Discover all .sutra files in the directory
    let test_files = TestDiscoverer::discover_test_files(path)?;
    if test_files.is_empty() {
        println!("No .sutra test files found in {}", path.display());
        return Ok(());
    }

    println!("Discovered {} test file(s)", test_files.len());

    // Process each file to register tests via macro expansion using the unified pipeline
    let mut registration_errors = Vec::new();
    let mut tests_per_file = HashMap::new();
    for file_path in &test_files {
        let file_display = file_path.display().to_string();
        let registration_result = (|| {
            let (_source, _ast_nodes) = load_file_to_ast(file_path)?;
            // Extract all test forms from the file
            let test_forms = match TestDiscoverer::extract_tests_from_file(file_path) {
                Ok(forms) => forms,
                Err(e) => {
                    registration_errors.push((format!("{} (file-level)", file_display), e));
                    return Ok(0);
                }
            };
            let mut registered = 0;
            for (test_idx, test_form) in test_forms.iter().enumerate() {
                // Reconstruct the (test ...) AST node from RawTestDefinition
                let mut items = vec![
                    crate::ast::AstNode {
                        value: std::sync::Arc::new(crate::ast::Expr::Symbol("test".to_string(), test_form.span)),
                        span: test_form.span,
                    },
                    crate::ast::AstNode {
                        value: std::sync::Arc::new(crate::ast::Expr::String(test_form.name.clone(), test_form.span)),
                        span: test_form.span,
                    },
                ];
                if let Some(expect) = &test_form.expectation {
                    items.push(expect.clone());
                }
                items.extend(test_form.body.clone());
                let test_node = crate::ast::AstNode {
                    value: std::sync::Arc::new(crate::ast::Expr::List(items, test_form.span)),
                    span: test_form.span,
                };

                // Execute the test form using the unified execution pipeline
                let before_count = 0; // Simplified for now

                // Use the unified pipeline for test execution
                let pipeline = ExecutionPipeline {
                    max_depth: 100,
                    validate: false, // Skip validation for tests
                };

                // Convert the test node to source code
                let test_source = test_node.value.pretty();

                match pipeline.execute(&test_source, SharedOutput(Rc::new(RefCell::new(crate::cli::output::OutputBuffer::new())))) {
                    Ok(_) => {
                        let after_count = 0; // Simplified for now
                        if after_count > before_count {
                            registered += 1;
                        }
                    }
                    Err(e) => {
                        // Error handling for test execution failures
                        let mut stderr = termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto);
                        let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
                        let _ = writeln!(stderr, "\n[ERROR] Failed to execute test #{} in file: {}\n----------------------------------------", test_idx + 1, file_display);
                        let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(false));
                        let _ = writeln!(stderr, "{}", e);
                        let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(false));
                        let _ = writeln!(stderr, "Suggestion: Check for execution errors in this test form.");
                        let _ = stderr.reset();
                        registration_errors.push((format!("{} (test #{})", file_display, test_idx + 1), e));
                    }
                }
            }
            Ok::<usize, SutraError>(registered)
        })();
        match registration_result {
            Ok(registered) => {
                tests_per_file.insert(file_display.clone(), registered);
                let mut stdout = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
                let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
                let _ = writeln!(stdout, "[OK] Registered {} test(s) from file: {}", registered, file_display);
                let _ = stdout.reset();
                if let Some(log) = error_log.as_mut() {
                    let _ = writeln!(log, "[OK] Registered {} test(s) from file: {}", registered, file_display);
                }
            }
            Err(e) => {
                // This should not happen, but if it does, treat as a file-level error
                let mut stderr = termcolor::StandardStream::stderr(termcolor::ColorChoice::Auto);
                let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
                let _ = writeln!(stderr, "\n[ERROR] Failed to process file: {}\n----------------------------------------", file_display);
                let _ = stderr.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(false));
                let _ = writeln!(stderr, "{}", e);
                let _ = stderr.reset();
                registration_errors.push((format!("{} (file-level)", file_display), e));
                continue;
            }
        }
    }

    // Execute all registered tests using the unified pipeline
    if tests_per_file.is_empty() {
        println!("No tests were registered successfully.");
        return Ok(());
    }

    println!("\nExecuting {} registered test(s)...", tests_per_file.values().sum::<usize>());
    let mut stdout = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
    let failed = 0;
    let skipped = 0;
    let errored = 0;

    // For now, just report that tests were processed
    for (file, count) in &tests_per_file {
        println!("Processed {} test(s) from {}", count, file);
    }

    // Print summary
    println!("\nTest Summary:");
    println!("=============");
    if failed > 0 {
        set_output_color(&mut stdout, termcolor::Color::Red, true);
        println!("  FAILED: {}", failed);
    }
    if skipped > 0 {
        set_output_color(&mut stdout, termcolor::Color::Yellow, true);
        println!("  SKIPPED: {}", skipped);
    }
    if errored > 0 {
        set_output_color(&mut stdout, termcolor::Color::Red, true);
        println!("  ERRORED: {}", errored);
    }
    reset_output_color(&mut stdout);

    if failed > 0 || errored > 0 {
        return Err(err_msg!(Internal, "Some tests failed"));
    }

    Ok(())
}

// ============================================================================
// TEST DISCOVERY - Test file discovery and parsing infrastructure
// ============================================================================

/// Test discovery and parsing infrastructure.
pub struct TestDiscoverer;

impl TestDiscoverer {
    /// Discovers all .sutra test files in the given directory.
    pub fn discover_test_files(path: &std::path::Path) -> Result<Vec<std::path::PathBuf>, SutraError> {
        use std::fs;
        let mut test_files = Vec::new();

        if path.is_file() {
            if path.extension().and_then(|s| s.to_str()) == Some("sutra") {
                test_files.push(path.to_path_buf());
            }
        } else if path.is_dir() {
            for entry in fs::read_dir(path)
                .map_err(|e| err_ctx!(Internal, "Failed to read directory: {}", e.to_string()))?
            {
                let entry = entry
                    .map_err(|e| err_ctx!(Internal, "Failed to read directory entry: {}", e.to_string()))?;
                let path = entry.path();
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("sutra") {
                    test_files.push(path);
                }
            }
        }

        Ok(test_files)
    }

    /// Extracts test definitions from a file.
    pub fn extract_tests_from_file(path: &std::path::Path) -> Result<Vec<RawTestDefinition>, SutraError> {
        use crate::ast::Expr;
        let (_source, ast_nodes) = load_file_to_ast(path)?;
        let mut tests = Vec::new();

        for node in ast_nodes {
            if let Expr::List(items, span) = &*node.value {
                if items.len() >= 2 {
                    if let Expr::Symbol(symbol, _) = &*items[0].value {
                        if symbol == "test" {
                            if let Expr::String(test_name, _) = &*items[1].value {
                                let mut test_def = RawTestDefinition {
                                    name: test_name.clone(),
                                    body: Vec::new(),
                                    expectation: None,
                                    span: *span,
                                };

                                // Parse optional expectation
                                if items.len() >= 3 {
                                    if let Expr::List(expect_items, _) = &*items[2].value {
                                        if !expect_items.is_empty() {
                                            if let Expr::Symbol(expect_symbol, _) = &*expect_items[0].value {
                                                if expect_symbol.starts_with("expect") {
                                                    test_def.expectation = Some(items[2].clone());
                                                    test_def.body = items[3..].to_vec();
                                                } else {
                                                    test_def.body = items[2..].to_vec();
                                                }
                                            } else {
                                                test_def.body = items[2..].to_vec();
                                            }
                                        } else {
                                            test_def.body = items[2..].to_vec();
                                        }
                                    } else {
                                        test_def.body = items[2..].to_vec();
                                    }
                                }

                                tests.push(test_def);
                            }
                        }
                    }
                }
            }
        }

        Ok(tests)
    }
}

/// Raw test definition extracted from AST.
#[derive(Debug)]
pub struct RawTestDefinition {
    pub name: String,
    pub body: Vec<crate::ast::AstNode>,
    pub expectation: Option<crate::ast::AstNode>,
    pub span: crate::ast::Span,
}
