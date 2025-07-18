//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use std::io::Write;

use clap::Parser;
use termcolor::WriteColor;

use crate::{
    cli::{
        args::{Command, SutraArgs},
        output::StdoutSink,
    },
    engine::ExecutionPipeline,
    err_ctx, err_msg, err_src, expand_macros_recursively,
    macros::{is_macro_definition, parse_macro_definition},
    runtime::world::build_canonical_macro_env,
    testing::discovery::TestDiscoverer,
    to_error_source, AstNode, Expr, MacroDefinition, MacroRegistry, SharedOutput, Span, Spanned,
    SutraError,
};

pub mod args;
pub mod output;

// ============================================================================
// TYPE DEFINITIONS - Simplify complex return types
// ============================================================================

/// Registration error with context about where it occurred
type RegistrationError = (String, SutraError);

/// Map of file names to number of tests registered from each file
type TestsPerFile = std::collections::HashMap<String, usize>;

/// Result of registering tests from multiple files
type TestRegistrationResult = Result<(Vec<RegistrationError>, TestsPerFile), SutraError>;

/// Type alias for CLI operation results
type CliResult = Result<(), SutraError>;

/// Type alias for file reading operations
type FileResult = Result<String, SutraError>;

/// Type alias for AST parsing operations
type AstResult = Result<Vec<AstNode>, SutraError>;

/// Type alias for macro parsing operations
type MacroParseResult = Result<(MacroRegistry, Vec<AstNode>), SutraError>;

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
        Command::ValidateGrammar => handle_validate_grammar(),
        Command::Macroexpand { file } => handle_macroexpand(file),
        Command::Format { file } => handle_format(file),
        Command::Test { path } => handle_test(path),
    };

    if let Err(e) = result {
        eprintln!("{e:?}");
        std::process::exit(1);
    }
}

// ============================================================================
// DIAGNOSTICS
// ============================================================================

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
fn read_file_to_string(path: &std::path::Path) -> FileResult {
    let filename = path_to_str(path)?;
    let src_arc = to_error_source(filename);
    std::fs::read_to_string(filename).map_err(|e| {
        err_ctx!(
            Internal,
            format!("Failed to read file: {}", e.to_string()),
            &src_arc,
            Span::default(),
            "Check that the file exists and is readable."
        )
    })
}

// ============================================================================
// AST PROCESSING UTILITIES - Parsing and transformation operations
// ============================================================================

/// Parses Sutra source code into AST nodes.
fn parse_source_to_ast(source: &str) -> AstResult {
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
    // Early return for single node
    if ast_nodes.len() == 1 {
        return ast_nodes.into_iter().next().unwrap();
    }

    // Create do wrapper for multiple nodes
    create_do_wrapper(ast_nodes, source)
}

/// Creates a (do ...) wrapper around multiple AST nodes.
fn create_do_wrapper(ast_nodes: Vec<AstNode>, source: &str) -> AstNode {
    let span = Span {
        start: 0,
        end: source.len(),
    };

    let do_symbol = Spanned {
        value: Expr::Symbol("do".to_string(), span).into(),
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
        println!("  {name}");
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
    let _ = stdout.set_color(
        termcolor::ColorSpec::new()
            .set_fg(Some(color))
            .set_bold(bold),
    );
}

/// Resets output color to default.
fn reset_output_color(stdout: &mut termcolor::StandardStream) {
    let _ = stdout.reset();
}

// ============================================================================
// TEST INFRASTRUCTURE - Test discovery and execution
// ============================================================================

/// Test execution summary statistics.
#[derive(Debug, Default)]
struct TestSummary {
    passed: usize,
    failed: usize,
    skipped: usize,
    errored: usize,
}

/// Discovers test files in the given directory.
fn discover_test_files(path: &std::path::Path) -> Result<Vec<std::path::PathBuf>, SutraError> {
    TestDiscoverer::discover_test_files(path)
}

/// Registers tests from all discovered files.
fn register_tests_from_files(
    test_files: &[std::path::PathBuf],
    error_log: &mut Option<std::fs::File>,
) -> TestRegistrationResult {
    use std::collections::HashMap;

    let mut registration_errors = Vec::new();
    let mut tests_per_file = HashMap::new();

    for file_path in test_files {
        let file_display = file_path.display().to_string();
        let (registered, failed, errors) =
            register_tests_from_single_file(file_path, &file_display);
        for (idx, e) in errors.into_iter().enumerate() {
            let label = if idx == 0 {
                format!("{file_display} (file-level)")
            } else {
                format!("{file_display} (test #{idx} )")
            };
            registration_errors.push((label, e));
        }
        tests_per_file.insert(file_display.clone(), registered);
        print_success_message(
            &file_display,
            registered,
            failed,
            &registration_errors,
            error_log,
        );
        // Warn if no tests were registered from this file
        if registered == 0 {
            eprintln!("[Warning] No tests registered from file: {file_display}. This may indicate a parse error or invalid test forms.");
        }
    }

    Ok((registration_errors, tests_per_file))
}

/// Registers tests from a single file.
fn register_tests_from_single_file(
    file_path: &std::path::Path,
    file_display: &str,
) -> (usize, usize, Vec<SutraError>) {
    let test_forms = match TestDiscoverer::extract_tests_from_file(file_path) {
        Ok(forms) => forms,
        Err(e) => {
            return (0, 0, vec![e]);
        }
    };

    let mut registered = 0;
    let mut failed_tests = Vec::new();
    for (test_idx, test_form) in test_forms.into_iter().enumerate() {
        match register_single_test(&test_form, test_idx, file_display) {
            Ok(_) => registered += 1,
            Err(e) => {
                failed_tests.push(e);
            }
        }
    }

    (registered, failed_tests.len(), failed_tests)
}

/// Registers a single test form.
fn register_single_test(
    test_form: &crate::testing::discovery::ASTDefinition,
    _test_idx: usize,
    _file_display: &str,
) -> CliResult {
    use crate::atoms::test::{TestDefinition, TEST_REGISTRY};

    // Convert ASTDefinition to TestDefinition
    let expect = match &test_form.expect_form {
        Some(ast) => ast.clone(),
        None => {
            return Err(err_src!(
                Validation,
                format!("Test '{}' missing (expect ...) form", test_form.name),
                &test_form.source_file,
                test_form.span
            ));
        }
    };

    let test_def = TestDefinition {
        name: test_form.name.clone(),
        expect,
        body: test_form.body.clone(),
        span: test_form.span,
        source_file: test_form.source_file.clone(),
    };

    let mut registry = TEST_REGISTRY.lock().unwrap();
    if registry.contains_key(&test_def.name) {
        return Err(err_src!(
            Validation,
            format!("Duplicate test name '{}'.", test_def.name),
            &test_def.source_file,
            test_def.span
        ));
    }
    registry.insert(test_def.name.clone(), test_def);
    Ok(())
}

/// Gets all registered tests from the test registry.
fn get_registered_tests() -> Result<Vec<crate::atoms::test::TestDefinition>, SutraError> {
    use crate::atoms::test::TEST_REGISTRY;

    let registry = TEST_REGISTRY.lock().unwrap();
    let test_definitions: Vec<_> = registry.values().cloned().collect();
    drop(registry); // Release lock before execution

    Ok(test_definitions)
}

/// Executes all registered tests and returns summary statistics.
fn execute_all_tests(test_definitions: &[crate::atoms::test::TestDefinition]) -> TestSummary {
    // Registration is metadata-only; all test execution happens here.
    let mut summary = TestSummary::default();
    let mut stdout = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);

    for test_def in test_definitions {
        let test_name = &test_def.name;
        let file_info = test_def.source_file.name();

        match execute_single_test(test_def) {
            Ok(_) => {
                summary.passed += 1;
                print_test_pass(&mut stdout, test_name, file_info);
            }
            Err(e) => {
                summary.errored += 1;
                print_test_error(&mut stdout, test_name, file_info, &e);
            }
        }
    }

    summary
}

/// Executes a single test and returns the result.
fn execute_single_test(test_def: &crate::atoms::test::TestDefinition) -> CliResult {
    let pipeline = ExecutionPipeline::default();

    // Execute test body directly using AST with enhanced error context
    pipeline.execute_ast(&test_def.body).map_err(|_| {
        err_src!(
            TestFailure,
            format!("Test '{}' failed", test_def.name),
            &test_def.source_file,
            test_def.span
        )
    })?;

    Ok(())
}

/// Prints test summary statistics.
fn print_test_summary(summary: &TestSummary) {
    let mut stdout = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);

    println!("\nTest Summary:");
    println!("=============");

    if summary.passed > 0 {
        set_output_color(&mut stdout, termcolor::Color::Green, true);
        println!("  PASSED: {}", summary.passed);
    }
    if summary.failed > 0 {
        set_output_color(&mut stdout, termcolor::Color::Red, true);
        println!("  FAILED: {}", summary.failed);
    }
    if summary.skipped > 0 {
        set_output_color(&mut stdout, termcolor::Color::Yellow, true);
        println!("  SKIPPED: {}", summary.skipped);
    }
    if summary.errored > 0 {
        set_output_color(&mut stdout, termcolor::Color::Red, true);
        println!("  ERRORED: {}", summary.errored);
    }
    reset_output_color(&mut stdout);
}

/// Prints a success message for test registration.
fn print_success_message(
    file_display: &str,
    registered: usize,
    failed: usize,
    errors: &[RegistrationError],
    error_log: &mut Option<std::fs::File>,
) {
    use termcolor::{Color, ColorSpec, WriteColor};

    let mut stdout = termcolor::StandardStream::stdout(termcolor::ColorChoice::Auto);
    if failed > 0 {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true));
        let _ = writeln!(
            stdout,
            "[WARNING] Registered {registered} test(s), {failed} failed in file: {file_display}"
        );
        for (label, error) in errors {
            let _ = writeln!(
                stdout,
                "[WARNING] Test registration error in {label}: {error}"
            );
        }
        let _ = stdout.reset();
    } else if registered == 0 {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(true));
        let _ = writeln!(
            stdout,
            "[WARNING] Registered 0 test(s) from file: {file_display}"
        );
        let _ = writeln!(stdout, "[WARNING] No tests registered from file: {file_display}. This may indicate a parse error or invalid test forms.");
        let _ = stdout.reset();
    } else {
        let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
        let _ = writeln!(
            stdout,
            "[OK] Registered {registered} test(s) from file: {file_display}"
        );
        let _ = stdout.reset();
    }

    if let Some(log) = error_log.as_mut() {
        if failed > 0 {
            let _ = writeln!(log, "[WARNING] Registered {registered} test(s), {failed} failed in file: {file_display}");
            for (label, error) in errors {
                let _ = writeln!(log, "[WARNING] Test registration error in {label}: {error}");
            }
        } else if registered == 0 {
            let _ = writeln!(
                log,
                "[WARNING] Registered 0 test(s) from file: {file_display}"
            );
            let _ = writeln!(log, "[WARNING] No tests registered from file: {file_display}. This may indicate a parse error or invalid test forms.");
        } else {
            let _ = writeln!(
                log,
                "[OK] Registered {registered} test(s) from file: {file_display}"
            );
        }
    }
}

/// Prints a test pass message.
fn print_test_pass(stdout: &mut termcolor::StandardStream, test_name: &str, file_info: &str) {
    use termcolor::{Color, ColorSpec, WriteColor};

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
    let _ = writeln!(stdout, "[PASS] {test_name} ({file_info})");
    let _ = stdout.reset();
}

/// Prints a test error message.
fn print_test_error(
    stdout: &mut termcolor::StandardStream,
    test_name: &str,
    file_info: &str,
    error: &SutraError,
) {
    use termcolor::{Color, ColorSpec, WriteColor};

    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
    let _ = writeln!(stdout, "[ERROR] {test_name} ({file_info})");
    let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::White)).set_bold(false));
    let _ = writeln!(stdout, "  Error: {error}");
    let _ = stdout.reset();
}

// ============================================================================
// COMMAND HANDLERS - CLI command implementations organized by functional area
// ============================================================================

// --- Analysis Commands: AST, validation, macro expansion, formatting ---

/// Handles the `ast` subcommand.
fn handle_ast(path: &std::path::Path) -> CliResult {
    let (_source, ast_nodes) = load_file_to_ast(path)?;

    let filename = safe_path_display(path);
    println!("AST for {filename}:");
    println!("={}=", "=".repeat(filename.len() + 9));

    if ast_nodes.is_empty() {
        println!("(empty)");
        return Ok(());
    }
    for (i, node) in ast_nodes.iter().enumerate() {
        if ast_nodes.len() > 1 {
            println!("\nNode {}:", i + 1);
        }
        println!("{node:#?}");
    }

    Ok(())
}

/// Handles the `validate` subcommand.
fn handle_validate() -> CliResult {
    use crate::validation::grammar::validate_grammar;

    let grammar_path = "src/syntax/grammar.pest";
    let src_arc = to_error_source(grammar_path);
    let validation_result = validate_grammar(grammar_path).map_err(|e| {
        err_ctx!(
            Internal,
            format!("Failed to validate grammar: {}", e.to_string()),
            &src_arc,
            Span::default(),
            "Check the grammar file for syntax errors or missing rules."
        )
    })?;

    // Early return on validation failure
    if !validation_result.is_valid() {
        return handle_validation_failure(grammar_path, &validation_result);
    }

    // Print warnings and suggestions
    print_validation_warnings(&validation_result);
    print_validation_suggestions(&validation_result);

    println!("Grammar validation passed");
    Ok(())
}

/// Handles the `validate-grammar` subcommand.
fn handle_validate_grammar() -> CliResult {
    use crate::validation::grammar::validate_grammar;

    let grammar_path = "src/syntax/grammar.pest";
    let src_arc = to_error_source(grammar_path);
    let validation_result = validate_grammar(grammar_path).map_err(|e| {
        err_ctx!(
            Internal,
            format!("Failed to validate grammar: {}", e.to_string()),
            &src_arc,
            Span::default(),
            "Check the grammar file for syntax errors or missing rules."
        )
    })?;

    // Early return on validation failure
    if !validation_result.is_valid() {
        return handle_validation_failure(grammar_path, &validation_result);
    }

    // Print warnings and suggestions
    print_validation_warnings(&validation_result);
    print_validation_suggestions(&validation_result);

    println!("Grammar validation passed");
    Ok(())
}

/// Handles validation failure by printing errors and exiting.
fn handle_validation_failure(
    grammar_path: &str,
    validation_result: &crate::validation::grammar::ValidationResult,
) -> CliResult {
    let src_arc = to_error_source(grammar_path);
    let mut error = err_ctx!(
        Validation,
        "Grammar validation failed",
        &src_arc,
        Span::default(),
        "Check the grammar file for syntax errors or missing rules."
    );
    for err in &validation_result.errors {
        // Attach each error as help (miette will show all help messages)
        error = match error {
            SutraError::Validation {
                message,
                mut ctx,
                source,
            } => {
                let help = ctx.help.get_or_insert(String::new());
                if !help.is_empty() {
                    help.push('\n');
                }
                help.push_str(&format!("• {err}"));
                SutraError::Validation {
                    message,
                    ctx,
                    source,
                }
            }
            _ => error,
        };
    }
    Err(error)
}

/// Prints validation warnings.
fn print_validation_warnings(validation_result: &crate::validation::grammar::ValidationResult) {
    for warning in &validation_result.warnings {
        eprintln!("[Warning] {warning}");
    }
}

/// Prints validation suggestions.
fn print_validation_suggestions(validation_result: &crate::validation::grammar::ValidationResult) {
    for suggestion in &validation_result.suggestions {
        eprintln!("[Suggestion] {suggestion}");
    }
}

/// Handles the `macroexpand` subcommand.
fn handle_macroexpand(path: &std::path::Path) -> CliResult {
    let (source, ast_nodes) = load_file_to_ast(path)?;
    let expanded = build_macro_environment_and_expand(ast_nodes, &source)?;
    println!("{}", expanded.value.pretty());
    Ok(())
}

/// Handles the `format` subcommand.
fn handle_format(path: &std::path::Path) -> CliResult {
    let (source, ast_nodes) = load_file_to_ast(path)?;
    let expanded = build_macro_environment_and_expand(ast_nodes, &source)?;
    println!("{}", expanded.value.pretty());
    Ok(())
}

/// Handles the `run` subcommand using the unified execution pipeline.
fn handle_run(path: &std::path::Path) -> CliResult {
    let source = read_file_to_string(path)?;
    let output = SharedOutput::new(StdoutSink);

    // Use the unified execution pipeline
    let pipeline = ExecutionPipeline::default();
    pipeline.execute(&source, output)
}

/// Handles the `macrotrace` subcommand.
fn handle_macrotrace(path: &std::path::Path) -> CliResult {
    let source = read_file_to_string(path)?;
    let output = SharedOutput::new(StdoutSink);
    let pipeline = ExecutionPipeline::default();
    pipeline.execute(&source, output)
}

/// Handles the `list-macros` subcommand.
fn handle_list_macros() -> CliResult {
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
fn handle_list_atoms() -> CliResult {
    use crate::{atoms::Atom, runtime::world::build_default_atom_registry};

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
pub fn handle_test(path: &std::path::Path) -> CliResult {
    use std::fs::OpenOptions;

    // Prepare error log file
    let mut error_log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("sutra-test-errors.log")
        .ok();

    // Discover and validate test files
    let test_files = discover_test_files(path)?;
    if test_files.is_empty() {
        println!("No .sutra test files found in {}", path.display());
        return Ok(());
    }

    println!("Discovered {} test file(s)", test_files.len());

    // Register tests from all files (metadata-only, no execution)
    let (_registration_errors, _tests_per_file) =
        register_tests_from_files(&test_files, &mut error_log)?;

    // All test execution happens after registration
    let test_definitions = get_registered_tests()?;
    if test_definitions.is_empty() {
        println!("No tests were registered successfully.");
        return Ok(());
    }

    println!(
        "\nExecuting {} registered test(s)...",
        test_definitions.len()
    );
    let test_results = execute_all_tests(&test_definitions);

    // Print summary and return result
    print_test_summary(&test_results);

    if test_results.failed > 0 || test_results.errored > 0 {
        return Err(err_msg!(Internal, "Some tests failed"));
    }

    Ok(())
}
