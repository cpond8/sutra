//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use crate::ast::AstNode;
use crate::cli::args::{Command, SutraArgs};
use crate::cli::output::StdoutSink;
use crate::macros::{expand_macros, MacroDef, MacroRegistry};
use crate::macros::definition::{is_macro_definition, parse_macro_definition};
use crate::engine::run_sutra_source_with_output;
use crate::runtime::registry::build_canonical_macro_env;
use crate::sutra_err;
use crate::SutraError;
use clap::Parser;
use std::io::Write;
use termcolor::WriteColor;

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

use crate::ast::{Expr, Span, WithSpan};

// ============================================================================
// CORE UTILITIES - Fundamental operations used throughout the CLI
// ============================================================================

/// Converts a Path to a &str, returning an error if invalid.
fn path_to_str(path: &std::path::Path) -> Result<&str, SutraError> {
    path.to_str()
        .ok_or_else(|| sutra_err!(Internal, "Invalid filename".to_string()))
}

/// Gets a safe display name for a path, with fallback for invalid paths.
fn safe_path_display(path: &std::path::Path) -> &str {
    path.to_str().unwrap_or("<unknown>")
}

/// Reads a file to a String, given a path.
fn read_file_to_string(path: &std::path::Path) -> Result<String, SutraError> {
    let filename = path_to_str(path)?;
    std::fs::read_to_string(filename).map_err(|e| sutra_err!(Internal, "Failed to read file: {}", e))
}

/// Reads a file and normalizes line endings, trimming whitespace.
fn read_file_trimmed(path: &std::path::Path) -> std::io::Result<String> {
    Ok(std::fs::read_to_string(path)?
        .replace("\r\n", "\n")
        .trim()
        .to_string())
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
    let do_symbol = WithSpan {
        value: Expr::Symbol("do".to_string(), span.clone()).into(), // FIX: wrap Expr in Arc via .into()
        span: span.clone(),
    };
    let mut items = Vec::with_capacity(ast_nodes.len() + 1);
    items.push(do_symbol);
    items.extend(ast_nodes);
    WithSpan {
        value: Expr::List(items, span.clone()).into(),
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
            return Err(sutra_err!(Validation, "Duplicate macro name '{}'", name));
        }
        user_macros
            .macros
            .insert(name, MacroDef::Template(template));
    }
    Ok((user_macros, user_code))
}

/// Builds a complete macro environment with user macros and expands a program.
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
    let expanded = expand_macros(program, &mut env)?;

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

/// Sets color for terminal output.
fn set_output_color(stdout: &mut termcolor::StandardStream, color: termcolor::Color, bold: bool) {
    use termcolor::ColorSpec;
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(color)).set_bold(bold));
}

/// Resets terminal color.
fn reset_output_color(stdout: &mut termcolor::StandardStream) {
    let _ = stdout.reset();
}

// ============================================================================
// TEST INFRASTRUCTURE - Testing system components
// ============================================================================

#[derive(Clone)]
enum TestOutcome {
    Pass,
    Fail { expected: String, actual: String },
    Skip,
}

/// Discovers test scripts and their expected output files in a directory.
fn find_test_scripts(dir: &std::path::Path) -> Vec<(std::path::PathBuf, std::path::PathBuf)> {
    let mut tests = Vec::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        // Guard clause: skip if not a file with a .sutra extension
        if !(path.is_file() && path.extension().map(|e| e == "sutra").unwrap_or(false)) {
            continue;
        }
        let expected = path.with_extension("expected");
        if expected.exists() {
            tests.push((path.to_path_buf(), expected));
        }
    }
    tests
}

/// Determines if a test should be skipped based on content.
fn should_skip_test(content: &str) -> bool {
    content.contains("test/")
}

/// Executes a Sutra script and captures its output.
fn execute_script(source: &str) -> (Result<(), crate::SutraError>, String) {
    let mut output_buf = crate::cli::output::OutputBuffer::new();
    let result = run_sutra_source_with_output(source, &mut output_buf);
    let output = output_buf.as_str().replace("\r\n", "\n").trim().to_string();
    (result, output)
}

/// Compares test output with expected result.
fn compare_test_results(
    result: Result<(), crate::SutraError>,
    actual: &str,
    expected: &str,
) -> bool {
    match result {
        Ok(_) => actual == expected,
        Err(e) => {
            let err_str = format!("{e}").replace("\r\n", "\n").trim().to_string();
            err_str == expected
        }
    }
}

/// Processes a single test file and determines the outcome.
fn process_test_file(
    script: &std::path::Path,
    expected: &std::path::Path,
) -> Result<TestOutcome, SutraError> {
    let script_src = read_file_trimmed(script).map_err(|e| sutra_err!(Internal, "Failed to read test script: {}", e))?;
    let expected_output = read_file_trimmed(expected).map_err(|e| sutra_err!(Internal, "Failed to read expected output: {}", e))?;

    // Guard clause: check if test should be skipped
    if should_skip_test(&script_src) {
        return Ok(TestOutcome::Skip);
    }

    let (result, actual_output) = execute_script(&script_src);

    if compare_test_results(result, &actual_output, &expected_output) {
        return Ok(TestOutcome::Pass);
    }

    Ok(TestOutcome::Fail {
        expected: expected_output,
        actual: actual_output,
    })
}

/// Prints a test result with appropriate formatting.
fn print_test_result(
    script_name: &str,
    outcome: TestOutcome,
    stdout: &mut termcolor::StandardStream,
) {
    match outcome {
        TestOutcome::Pass => {
            set_output_color(stdout, termcolor::Color::Green, true);
            let _ = writeln!(stdout, "PASS: {script_name}");
        }
        TestOutcome::Fail {
            ref expected,
            ref actual,
        } => {
            set_output_color(stdout, termcolor::Color::Red, true);
            let _ = writeln!(stdout, "FAIL: {script_name}");
            set_output_color(stdout, termcolor::Color::Yellow, false);
            let _ = writeln!(stdout, "  Expected: {expected:?}");
            let _ = writeln!(stdout, "  Actual:   {actual:?}");
        }
        TestOutcome::Skip => {
            set_output_color(stdout, termcolor::Color::Yellow, true);
            let _ = writeln!(stdout, "SKIP: {script_name} (requires test atoms)");
        }
    }
    reset_output_color(stdout);
}

/// Runs a single test and prints the result.
fn run_single_test(
    script: &std::path::Path,
    expected: &std::path::Path,
    stdout: &mut termcolor::StandardStream,
) -> Result<TestOutcome, SutraError> {
    let script_name = script.file_name().unwrap().to_string_lossy();
    let outcome = process_test_file(script, expected)?;
    print_test_result(&script_name, outcome.clone(), stdout);

    // TODO: Consider using a more functional approach that avoids cloning
    match outcome {
        TestOutcome::Pass => Ok(TestOutcome::Pass),
        TestOutcome::Fail { expected, actual } => Ok(TestOutcome::Fail { expected, actual }),
        TestOutcome::Skip => Ok(TestOutcome::Skip),
    }
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
    println!("Validation is currently disabled: no validator system present.");
    Ok(())
}

/// Handles the `macroexpand` subcommand.
fn handle_macroexpand(path: &std::path::Path) -> Result<(), SutraError> {
    let (source, ast_nodes) = load_file_to_ast(path)?;
    let expanded = build_macro_environment_and_expand(ast_nodes, &source)?;

    // Print the expanded result
    println!("{}", expanded.value.pretty());

    Ok(())
}

/// Handles the `format` subcommand.
fn handle_format(path: &std::path::Path) -> Result<(), SutraError> {
    let (_source, ast_nodes) = load_file_to_ast(path)?;

    // Pretty-print each top-level AST node
    for node in ast_nodes {
        println!("{}", node.value.pretty());
    }

    Ok(())
}

// --- Execution Commands: run and macro tracing ---

/// Handles the `run` subcommand.
fn handle_run(path: &std::path::Path) -> Result<(), SutraError> {
    let source = read_file_to_string(path)?;
    let mut stdout_sink = StdoutSink;
    run_sutra_source_with_output(&source, &mut stdout_sink)?;
    Ok(())
}

/// Handles the `macrotrace` subcommand.
fn handle_macrotrace(path: &std::path::Path) -> Result<(), SutraError> {
    let (source, ast_nodes) = load_file_to_ast(path)?;
    let program = wrap_in_do_if_needed(ast_nodes, &source);
    let mut env = build_canonical_macro_env()?;
    let expanded = expand_macros(program.clone(), &mut env)?;
    println!("{}", expanded.value.pretty());
    Ok(())
}

// --- Information Commands: listing available components ---

/// Handles the `list-macros` subcommand.
fn handle_list_macros() -> Result<(), SutraError> {
    let env = build_canonical_macro_env()?;

    let core_names: Vec<_> = env.core_macros.keys().map(|k| k.as_str()).collect();
    let user_names: Vec<_> = env.user_macros.keys().map(|k| k.as_str()).collect();

    print_registry_listing(
        "Available Macros:",
        "================",
        &[
            ("Core Macros", &core_names),
            ("User-Defined Macros", &user_names),
        ],
    );

    Ok(())
}

/// Handles the `list-atoms` subcommand.
fn handle_list_atoms() -> Result<(), SutraError> {
    use crate::runtime::registry::build_default_atom_registry;

    let atom_registry = build_default_atom_registry();
    let atom_names: Vec<_> = atom_registry.atoms.keys().map(|k| k.as_str()).collect();

    print_registry_listing(
        "Available Atoms:",
        "===============",
        &[("Atoms", &atom_names)],
    );

    Ok(())
}

// --- Testing Commands: test execution and management ---

/// Handles the `test` subcommand.
fn handle_test(path: &std::path::Path) -> Result<(), SutraError> {
    use termcolor::{ColorChoice, StandardStream};

    let scripts = find_test_scripts(path);
    if scripts.is_empty() {
        println!("No .sutra test scripts found in {}", path.display());
        return Ok(());
    }

    let mut failed = false;
    let mut skipped = 0;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    for (script, expected) in scripts {
        match run_single_test(&script, &expected, &mut stdout)? {
            TestOutcome::Pass => {}
            TestOutcome::Fail { .. } => failed = true,
            TestOutcome::Skip => skipped += 1,
        }
    }

    if skipped > 0 {
        set_output_color(&mut stdout, termcolor::Color::Cyan, false);
        let _ = writeln!(
            stdout,
            "\nNote: {skipped} test(s) skipped (use 'cargo test' to run all tests)"
        );
        reset_output_color(&mut stdout);
    }

    if failed {
        return Err(sutra_err!(Internal, "One or more test scripts failed".to_string()));
    }

    Ok(())
}
