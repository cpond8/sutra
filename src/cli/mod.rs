//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use crate::ast::AstNode;
use crate::cli::args::{Command, SutraArgs};
use crate::cli::output::StdoutSink;
use crate::engine::run_sutra_source_with_output;
use crate::err_ctx;
use crate::err_msg;
use crate::macros::definition::{is_macro_definition, parse_macro_definition};
use crate::macros::{expand_macros_recursively, MacroDefinition, MacroRegistry};
use crate::runtime::registry::build_canonical_macro_env;
use crate::SutraError;
use clap::Parser;
use std::io::Write;
use termcolor::WriteColor;
use std::panic::{catch_unwind, AssertUnwindSafe};
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
// MODERN TEST HARNESS - Macro-based test system
// ============================================================================

use crate::testing::discovery::TestDiscoverer;
use crate::atoms::test::TEST_REGISTRY;

#[derive(Debug, Clone)]
pub enum TestExpectation {
    Value(crate::ast::value::Value),
    Error(String),
    Output(String),
    Skip(String),
}

#[derive(Debug, Clone)]
pub enum TestResult {
    Pass,
    Fail { expected: String, actual: String },
    Skip { reason: String },
    Error { message: String },
}

/// Parses an expectation form into a TestExpectation.
fn parse_expectation(expect_node: &crate::ast::AstNode) -> Result<TestExpectation, SutraError> {
    use crate::ast::{value::Value, Expr};

    // The `expect_node` is the full `(expect ...)` form.
    let Expr::List(items, _) = &*expect_node.value else {
        return Err(err_msg!(Validation, "Expectation must be a list"));
    };

    // Basic validation: must start with 'expect' and have at least one argument.
    if items.len() < 2 {
        return Err(err_msg!(
            Validation,
            "Expectation requires at least one argument"
        ));
    }
    if let Expr::Symbol(s, _) = &*items[0].value {
        if s != "expect" {
            return Err(err_msg!(
                Validation,
                "Expectation must start with 'expect' symbol"
            ));
        }
    } else {
        return Err(err_msg!(
            Validation,
            "Expectation must start with 'expect' symbol"
        ));
    }

    // Check for a `(skip "reason")` tag anywhere in the list.
    // This allows for forms like `(expect (value 1) (skip "reason"))`.
    for item in &items[1..] {
        if let Expr::List(tag_items, _) = &*item.value {
            if let Some(Expr::Symbol(tag_name, _)) = tag_items.first().map(|h| &*h.value) {
                if tag_name == "skip" {
                    let reason = if tag_items.len() > 1 {
                        if let Expr::String(rs, _) = &*tag_items[1].value {
                            rs.clone()
                        } else {
                            "No reason provided".to_string()
                        }
                    } else {
                        "No reason provided".to_string()
                    };
                    return Ok(TestExpectation::Skip(reason));
                }
            }
        }
    }

    // If not skipped, parse the primary expectation, which must be the first argument.
    let primary_expectation_node = &items[1];
    match &*primary_expectation_node.value {
        Expr::List(primary_items, _) if !primary_items.is_empty() => {
            let head = &primary_items[0];
            let head_symbol = if let Expr::Symbol(s, _) = &*head.value {
                s.as_str()
            } else {
                ""
            };

            match head_symbol {
                "value" => {
                    if primary_items.len() != 2 {
                        return Err(err_msg!(
                            Validation,
                            "(value) expectation requires exactly one argument"
                        ));
                    }
                    let value_node = &primary_items[1];
                    match &*value_node.value {
                        Expr::Number(n, _) => Ok(TestExpectation::Value(Value::Number(*n))),
                        Expr::String(s, _) => Ok(TestExpectation::Value(Value::String(s.clone()))),
                        Expr::Bool(b, _) => Ok(TestExpectation::Value(Value::Bool(*b))),
                        Expr::Symbol(s, _) if s == "nil" => Ok(TestExpectation::Value(Value::Nil)),
                        Expr::Symbol(s, _) if s == "true" => {
                            Ok(TestExpectation::Value(Value::Bool(true)))
                        }
                        Expr::Symbol(s, _) if s == "false" => {
                            Ok(TestExpectation::Value(Value::Bool(false)))
                        }
                        _ => Err(err_msg!(
                            Validation,
                            "Unsupported value type in (value ...) expectation"
                        )),
                    }
                }
                "error" => {
                    if primary_items.len() != 2 {
                        return Err(err_msg!(
                            Validation,
                            "(error) expectation requires exactly one argument"
                        ));
                    }
                    let error_node = &primary_items[1];
                    if let Expr::Symbol(err_type, _) = &*error_node.value {
                        Ok(TestExpectation::Error(err_type.clone()))
                    } else {
                        Err(err_msg!(
                            Validation,
                            "Error type in (error ...) expectation must be a symbol"
                        ))
                    }
                }
                "output" => {
                    if primary_items.len() != 2 {
                        return Err(err_msg!(
                            Validation,
                            "(output) expectation requires exactly one argument"
                        ));
                    }
                    let output_node = &primary_items[1];
                    if let Expr::String(s, _) = &*output_node.value {
                        Ok(TestExpectation::Output(s.clone()))
                    } else {
                        Err(err_msg!(
                            Validation,
                            "Output in (output ...) expectation must be a string"
                        ))
                    }
                }
                _ => Err(err_msg!(Validation, "Unsupported primary expectation type")),
            }
        }
        _ => Err(err_msg!(
            Validation,
            "Primary expectation must be a list like (value ...) or (error ...)"
        )),
    }
}

/// Executes a test body and returns the result.
fn execute_test_body(body: &[crate::ast::AstNode]) -> (Result<crate::ast::value::Value, SutraError>, String) {
    use crate::cli::output::OutputBuffer;
    use crate::runtime::eval::evaluate;
    use crate::runtime::world::World;
    use crate::runtime::registry::build_default_atom_registry;
    use crate::ast::{Expr, Spanned, Span};
    use std::sync::Arc;
    use miette::NamedSource;

    let output_buf = Rc::new(RefCell::new(OutputBuffer::new()));
    let result = if body.is_empty() {
        Ok(crate::ast::value::Value::Nil)
    } else {
        let atom_registry = build_default_atom_registry();
        let world = World::new();
        let source = Arc::new(NamedSource::new("test".to_string(), "<test body>".to_string()));

        // Wrap the body in a (do ...) expression to execute all statements
        let do_expr = if body.len() == 1 {
            body[0].clone()
        } else {
            let span = Span { start: 0, end: 0 };
            let do_symbol = Spanned {
                value: Arc::new(Expr::Symbol("do".to_string(), span)),
                span,
            };
            let mut items = vec![do_symbol];
            items.extend_from_slice(body);
            Spanned {
                value: Arc::new(Expr::List(items, span)),
                span,
            }
        };

        // Execute the expression
        match evaluate(&do_expr, &world, SharedOutput(output_buf.clone()), &atom_registry, source, 100) {
            Ok((value, _)) => Ok(value),
            Err(e) => Err(e),
        }
    };

    let output_string = output_buf.borrow().as_str().replace("\r\n", "\n").trim().to_string();
    (result, output_string)
}

/// Compares test result against expectation.
fn check_test_expectation(
    result: Result<crate::ast::value::Value, SutraError>,
    output: &str,
    expectation: &TestExpectation
) -> TestResult {

    match expectation {
        TestExpectation::Value(expected_val) => {
            match result {
                Ok(actual_val) => {
                    if actual_val == *expected_val {
                        TestResult::Pass
                    } else {
                        TestResult::Fail {
                            expected: format!("{:?}", expected_val),
                            actual: format!("{:?}", actual_val),
                        }
                    }
                }
                Err(e) => TestResult::Fail {
                    expected: format!("{:?}", expected_val),
                    actual: format!("Error: {}", e),
                },
            }
        }
        TestExpectation::Error(expected_err) => {
            match result {
                Err(e) => {
                    let error_str = format!("{}", e);
                    if error_str.contains(expected_err) {
                        TestResult::Pass
                    } else {
                        TestResult::Fail {
                            expected: expected_err.clone(),
                            actual: error_str,
                        }
                    }
                }
                Ok(val) => TestResult::Fail {
                    expected: expected_err.clone(),
                    actual: format!("Success: {:?}", val),
                },
            }
        }
        TestExpectation::Output(expected_output) => {
            if output == expected_output {
                TestResult::Pass
            } else {
                TestResult::Fail {
                    expected: expected_output.clone(),
                    actual: output.to_string(),
                }
            }
        }
        TestExpectation::Skip(reason) => TestResult::Skip {
            reason: reason.clone(),
        },
    }
}

/// Prints a test result with appropriate formatting.
fn print_test_result(
    test_name: &str,
    result: &TestResult,
    stdout: &mut termcolor::StandardStream,
    test_file: Option<&str>,
    test_line: Option<usize>,
) {
    let location = match (test_file, test_line) {
        (Some(file), Some(line)) => format!(" ({}:{})", file, line),
        (Some(file), None) => format!(" ({})", file),
        _ => String::new(),
    };
    match result {
        TestResult::Pass => {
            set_output_color(stdout, termcolor::Color::Green, true);
            let _ = writeln!(stdout, "PASS: {}{}", test_name, location);
        }
        TestResult::Fail { expected, actual } => {
            set_output_color(stdout, termcolor::Color::Red, true);
            let _ = writeln!(stdout, "FAIL: {}{}", test_name, location);
            set_output_color(stdout, termcolor::Color::Yellow, false);
            let _ = writeln!(stdout, "  Expected: {}", expected);
            let _ = writeln!(stdout, "  Actual:   {}", actual);
        }
        TestResult::Skip { reason } => {
            set_output_color(stdout, termcolor::Color::Yellow, true);
            let _ = writeln!(stdout, "SKIP: {}{} ({})", test_name, location, reason);
        }
        TestResult::Error { message } => {
            set_output_color(stdout, termcolor::Color::Red, true);
            let _ = writeln!(stdout, "ERROR: {}{} - {}", test_name, location, message);
        }
    }
    reset_output_color(stdout);
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
    run_sutra_source_with_output(&source, SharedOutput::new(StdoutSink))?;
    Ok(())
}

/// Handles the `macrotrace` subcommand.
fn handle_macrotrace(path: &std::path::Path) -> Result<(), SutraError> {
    let (source, ast_nodes) = load_file_to_ast(path)?;
    let program = wrap_in_do_if_needed(ast_nodes, &source);
    let mut env = build_canonical_macro_env()?;
    let expanded = expand_macros_recursively(program.clone(), &mut env)?;
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

/// Handles the `test` subcommand using the modern macro-based test harness.
pub fn handle_test(path: &std::path::Path) -> Result<(), SutraError> {
    use termcolor::{ColorChoice, StandardStream};

    // Discover all .sutra files in the directory
    let test_files = TestDiscoverer::discover_test_files(path)?;
    if test_files.is_empty() {
        println!("No .sutra test files found in {}", path.display());
        return Ok(());
    }

    println!("Discovered {} test file(s)", test_files.len());

    // Clear the test registry to start fresh
    {
        let mut registry = TEST_REGISTRY.lock().unwrap();
        registry.clear();
    }

    // Process each file to register tests via macro expansion
    for file_path in &test_files {
        let (_source, ast_nodes) = load_file_to_ast(file_path)?;
        let expanded = build_macro_environment_and_expand(ast_nodes, "")?;

        // Execute the expanded AST to register tests
        let output_buf = Rc::new(RefCell::new(crate::cli::output::OutputBuffer::new()));
        let _ = run_sutra_source_with_output(&expanded.value.pretty(), SharedOutput(output_buf));
    }

    // Get all registered tests
    let tests = {
        let registry = TEST_REGISTRY.lock().unwrap();
        registry.clone()
    };

    if tests.is_empty() {
        println!("No tests found in the discovered files");
        return Ok(());
    }

    println!("Running {} test(s)...\n", tests.len());

    // Execute all registered tests
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);
    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;
    let mut errors = 0;

    for (test_name, test_def) in tests {
        let file = test_def.file.as_deref();
        let line = Some(test_def.span.start); // Use span.start as line number proxy

        // Wrap the entire test execution in catch_unwind to prevent panics from aborting the harness
        let test_result = catch_unwind(AssertUnwindSafe(|| {
            match parse_expectation(&test_def.expect) {
                Ok(expectation) => {
                    let result = match &expectation {
                        TestExpectation::Skip(reason) => TestResult::Skip { reason: reason.clone() },
                        _ => {
                            let (exec_result, output) = execute_test_body(&test_def.body);
                            check_test_expectation(exec_result, &output, &expectation)
                        }
                    };
                    result
                }
                Err(e) => TestResult::Error {
                    message: format!("Failed to parse expectation: {}", e),
                },
            }
        }));

        let result = match test_result {
            Ok(r) => r,
            Err(panic) => {
                let panic_msg = if let Some(s) = panic.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                TestResult::Error {
                    message: format!("Test panicked: {}", panic_msg),
                }
            }
        };

        print_test_result(&test_name, &result, &mut stdout, file, line);
        match result {
            TestResult::Pass => passed += 1,
            TestResult::Fail { .. } => failed += 1,
            TestResult::Skip { .. } => skipped += 1,
            TestResult::Error { .. } => errors += 1,
        }
    }

    reset_output_color(&mut stdout);

    // Print summary
    println!("\nTest Results:");
    println!("  Passed: {}", passed);
    if failed > 0 {
        set_output_color(&mut stdout, termcolor::Color::Red, false);
        println!("  Failed: {}", failed);
        reset_output_color(&mut stdout);
    }
    if skipped > 0 {
        set_output_color(&mut stdout, termcolor::Color::Yellow, false);
        println!("  Skipped: {}", skipped);
        reset_output_color(&mut stdout);
    }
    if errors > 0 {
        set_output_color(&mut stdout, termcolor::Color::Red, false);
        println!("  Errors: {}", errors);
        reset_output_color(&mut stdout);
    }

    if failed > 0 || errors > 0 {
        return Err(err_msg!(Internal, "One or more tests failed"));
    }

    Ok(())
}
