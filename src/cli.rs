//!
//! This module is the main entry point for all CLI commands and orchestrates
//! the core library functions.

use std::{
    collections::HashMap,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

use clap::Parser;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{
    cli::{
        args::{Command, SutraArgs},
        output::StdoutSink,
    },
    engine::ExecutionPipeline,
    err_ctx, err_msg, err_src,
    error_messages::*,
    expand_macros_recursively,
    macros::{is_macro_definition, parse_macro_definition, MacroValidationContext},
    runtime::world::build_canonical_macro_env,
    testing::discovery::{ASTDefinition, TestDiscoverer},
    to_error_source, AstNode, Expr, MacroRegistry, SharedOutput, Span, Spanned, SutraError,
    Value,
};

pub mod args;
pub mod output;

// ============================================================================
// SEMANTIC CONSTANTS - Centralize status messages and formatting
// ============================================================================

const TEST_REGISTRATION_WARNING: &str =
    "[WARNING] Registered {registered} test(s), {failed} failed in file: {file_display}";
const TEST_REGISTRATION_SUCCESS: &str =
    "[OK] Registered {registered} test(s) from file: {file_display}";
const TEST_REGISTRATION_EMPTY: &str = "[WARNING] Registered 0 test(s) from file: {file_display}";
const TEST_REGISTRATION_EMPTY_DETAIL: &str = "[WARNING] No tests registered from file: {file_display}. This may indicate a parse error or invalid test forms.";
const TEST_PASS_MESSAGE: &str = "[PASS] {test_name} ({file_info})";
const TEST_ERROR_MESSAGE: &str = "[ERROR] {test_name} ({file_info})";
const TEST_ERROR_DETAIL: &str = "  Error: {error}";
const GRAMMAR_VALIDATION_PATH: &str = "src/syntax/grammar.pest";
const TEST_ERROR_LOG_FILE: &str = "sutra-test-errors.log";
const GRAMMAR_VALIDATION_ERROR: &str = "Failed to validate grammar: {}";
const GRAMMAR_VALIDATION_HELP: &str = "Check the grammar file for syntax errors or missing rules.";
const GRAMMAR_VALIDATION_SUCCESS: &str = "Grammar validation passed";
const FILE_READ_HELP: &str = "Check that the file exists and is readable.";

// ============================================================================
// TYPE DEFINITIONS - Simplify complex return types
// ============================================================================

/// Registration error with context about where it occurred
type RegistrationError = (String, SutraError);

/// Map of file names to number of tests registered from each file
type TestsPerFile = HashMap<String, usize>;

/// Type alias for test definition to reduce verbosity
type TestDefinition = crate::atoms::test::TestDefinition;

/// Type alias for validation result to reduce verbosity
type ValidationResult = crate::validation::grammar::ValidationResult;

/// Type alias for test definition list to reduce verbosity
type TestDefinitionList = Vec<TestDefinition>;

/// Type alias for test definition slice to reduce verbosity
type TestDefinitionSlice = [TestDefinition];

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

// ============================================================================
// MAIN ENTRY POINT
// ============================================================================

/// The main entry point for the CLI.
pub fn run() {
    let args = SutraArgs::parse();

    // Dispatch to the appropriate subcommand handler.
    let result = match &args.command {
        Command::Macrotrace { file } => handle_execution(file),
        Command::Run { file } => handle_execution(file),
        Command::ListMacros => handle_list_macros(),
        Command::ListAtoms => handle_list_atoms(),
        Command::Ast { file } => handle_ast(file),
        Command::Validate { .. } => ValidationOrchestrator.validate_grammar(),
        Command::ValidateGrammar => ValidationOrchestrator.validate_grammar(),
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
// CORE INFRASTRUCTURE - Low-level utilities and operations
// ============================================================================

// --- File Operations ---

/// Converts a Path to a &str, returning an error if invalid.
fn convert_path_to_string(path: &Path) -> Result<&str, SutraError> {
    path.to_str()
        .ok_or_else(|| err_msg!(Internal, ERROR_INVALID_FILENAME))
}

/// Gets a safe display name for a path, with fallback for invalid paths.
fn get_safe_path_display_name(path: &Path) -> &str {
    path.to_str().unwrap_or("<unknown>")
}

/// Reads a file to a String, given a path.
fn read_file_to_string(path: &Path) -> FileResult {
    let filename = convert_path_to_string(path)?;
    let src_arc = to_error_source(filename);
    std::fs::read_to_string(filename).map_err(|error| {
        err_ctx!(
            Internal,
            format!("{}", ERROR_FILE_READ.replace("{}", &error.to_string())),
            &src_arc,
            Span::default(),
            FILE_READ_HELP
        )
    })
}

// --- AST Processing ---

/// Consolidated AST processing operations.
/// Handles the common pipeline of loading, parsing, and preparing AST nodes.
struct AstProcessor;

impl AstProcessor {
    /// Parses Sutra source code into AST nodes.
    fn parse_source(&self, source: &str) -> AstResult {
        crate::syntax::parser::parse(source)
    }

    /// Common pipeline: read file and parse to AST nodes.
    fn load_and_parse(&self, path: &Path) -> Result<(String, Vec<AstNode>), SutraError> {
        let source = read_file_to_string(path)?;
        let ast_nodes = self.parse_source(&source)?;
        Ok((source, ast_nodes))
    }

    /// Wraps AST nodes in a (do ...) if needed.
    fn wrap_in_do_block(&self, ast_nodes: Vec<AstNode>, source: &str) -> AstNode {
        // Early return for single node
        if ast_nodes.len() == 1 {
            return ast_nodes
                .into_iter()
                .next()
                .expect("AST nodes should not be empty");
        }

        // Create do wrapper for multiple nodes
        self.build_do_block_wrapper(ast_nodes, source)
    }

    /// Creates a (do ...) wrapper around multiple AST nodes.
    fn build_do_block_wrapper(&self, ast_nodes: Vec<AstNode>, source: &str) -> AstNode {
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
}

// --- Output Formatting ---

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

/// Consolidated output formatting utilities
struct OutputFormatter {
    stdout: StandardStream,
}

impl OutputFormatter {
    /// Creates a new output formatter with auto color choice
    fn new() -> Self {
        Self {
            stdout: StandardStream::stdout(ColorChoice::Auto),
        }
    }

    /// Sets output color with specified color and bold setting
    fn set_color(&mut self, color: Color, bold: bool) {
        let _ = self
            .stdout
            .set_color(ColorSpec::new().set_fg(Some(color)).set_bold(bold));
    }

    /// Resets output color to default
    fn reset_color(&mut self) {
        let _ = self.stdout.reset();
    }

    /// Writes a colored message with automatic reset
    fn write_colored_message(&mut self, message: &str, color: Color, bold: bool) {
        self.set_color(color, bold);
        let _ = writeln!(self.stdout, "{message}");
        self.reset_color();
    }

    /// Writes a success message in green
    fn write_success(&mut self, message: &str) {
        self.write_colored_message(message, Color::Green, true);
    }

    /// Writes a warning message in yellow
    fn write_warning(&mut self, message: &str) {
        self.write_colored_message(message, Color::Yellow, true);
    }

    /// Writes an error message in red
    fn write_error(&mut self, message: &str) {
        self.write_colored_message(message, Color::Red, true);
    }

    /// Writes a test pass message
    fn write_test_pass(&mut self, test_name: &str, file_info: &str) {
        let message = TEST_PASS_MESSAGE
            .replace("{test_name}", test_name)
            .replace("{file_info}", file_info);
        self.write_success(&message);
    }

    /// Writes a test error message with details
    fn write_test_error(&mut self, test_name: &str, file_info: &str, error: &SutraError) {
        let message = TEST_ERROR_MESSAGE
            .replace("{test_name}", test_name)
            .replace("{file_info}", file_info);
        self.write_error(&message);

        // Write error details in white with better formatting
        self.set_color(Color::White, false);
        let error_str = error.to_string();
        let detail_message = if error_str.contains("Test failure:") {
            // Extract the actual error message from the test failure
            if let Some(actual_error) = error_str.split("Test failure:").nth(1) {
                format!("  Error:{}", actual_error.trim())
            } else {
                TEST_ERROR_DETAIL.replace("{error}", &error_str)
            }
        } else {
            TEST_ERROR_DETAIL.replace("{error}", &error_str)
        };
        let _ = writeln!(self.stdout, "{detail_message}");
        self.reset_color();
    }

    /// Writes test registration status messages
    fn write_registration_status(
        &mut self,
        file_display: &str,
        registered: usize,
        failed: usize,
        registration_errors: &[RegistrationError],
    ) {
        if failed > 0 {
            let message = TEST_REGISTRATION_WARNING
                .replace("{registered}", &registered.to_string())
                .replace("{failed}", &failed.to_string())
                .replace("{file_display}", file_display);
            self.write_warning(&message);

            for (label, error) in registration_errors {
                let _ = writeln!(
                    self.stdout,
                    "[WARNING] Test registration error in {label}: {error}"
                );
            }
        } else if registered == 0 {
            let message = TEST_REGISTRATION_EMPTY.replace("{file_display}", file_display);
            self.write_warning(&message);

            let detail_message =
                TEST_REGISTRATION_EMPTY_DETAIL.replace("{file_display}", file_display);
            self.write_warning(&detail_message);
        } else {
            let message = TEST_REGISTRATION_SUCCESS
                .replace("{registered}", &registered.to_string())
                .replace("{file_display}", file_display);
            self.write_success(&message);
        }
    }

    /// Writes test summary with color coding
    fn write_test_summary(&mut self, summary: &TestSummary) {
        println!("\nTest Summary:");
        println!("=============");

        if summary.passed > 0 {
            self.write_success(&format!("  PASSED: {}", summary.passed));
        }
        if summary.failed > 0 {
            self.write_error(&format!("  FAILED: {}", summary.failed));
        }
        if summary.skipped > 0 {
            self.write_warning(&format!("  SKIPPED: {}", summary.skipped));
        }
        if summary.errored > 0 {
            self.write_error(&format!("  ERRORED: {}", summary.errored));
        }

        // Print formatted summary
        println!("\n{}", summary.formatted_summary());
    }
}

// ============================================================================
// BUSINESS LOGIC - High-level operations and orchestration
// ============================================================================

// --- Test Infrastructure ---

/// Test execution summary statistics.
#[derive(Debug, Default)]
struct TestSummary {
    passed: usize,
    failed: usize,
    skipped: usize,
    errored: usize,
}

impl TestSummary {
    /// Returns true if any tests failed or errored
    fn has_failures(&self) -> bool {
        self.failed > 0 || self.errored > 0
    }

    /// Returns the total number of tests
    fn total_tests(&self) -> usize {
        self.passed + self.failed + self.skipped + self.errored
    }

    /// Returns the success rate as a percentage
    fn success_rate(&self) -> f64 {
        if self.total_tests() == 0 {
            0.0
        } else {
            (self.passed as f64 / self.total_tests() as f64) * 100.0
        }
    }

    /// Returns a formatted summary string
    fn formatted_summary(&self) -> String {
        format!(
            "{} tests: {} passed, {} failed, {} errored, {} skipped ({:.1}% success)",
            self.total_tests(),
            self.passed,
            self.failed,
            self.errored,
            self.skipped,
            self.success_rate()
        )
    }
}

/// Discovers test files in the given directory.
fn discover_test_files(path: &Path) -> Result<Vec<PathBuf>, SutraError> {
    TestDiscoverer::discover_test_files(path)
}

/// Registers tests from all discovered files.
fn register_tests_from_files(
    test_files: &[PathBuf],
    error_log: &mut Option<File>,
) -> TestRegistrationResult {
    let mut registration_errors = Vec::new();
    let mut tests_per_file = HashMap::new();

    for file_path in test_files {
        let file_display = file_path.display().to_string();
        let registration_result = register_tests_from_single_file(file_path, &file_display);

        // Process registration results with consolidated error handling
        process_test_registration_results(
            &file_display,
            registration_result,
            &mut registration_errors,
            &mut tests_per_file,
            error_log,
        );
    }

    Ok((registration_errors, tests_per_file))
}

/// Processes test registration results with consolidated error handling
fn process_test_registration_results(
    file_display: &str,
    registration_result: (usize, usize, Vec<SutraError>),
    registration_errors: &mut Vec<RegistrationError>,
    tests_per_file: &mut TestsPerFile,
    error_log: &mut Option<File>,
) {
    let (registered, failed, errors) = registration_result;

    // Add errors to registration errors list
    for (error_index, error) in errors.into_iter().enumerate() {
        let label = format_test_registration_error_label(file_display, error_index);
        registration_errors.push((label, error));
    }

    tests_per_file.insert(file_display.to_string(), registered);

    // Print status message using consolidated formatter
    let mut formatter = OutputFormatter::new();
    formatter.write_registration_status(file_display, registered, failed, registration_errors);

    // Write to error log if available
    write_registration_status_to_log(
        file_display,
        registered,
        failed,
        registration_errors,
        error_log,
    );

    // Warn if no tests were registered from this file
    if registered == 0 {
        eprintln!(
            "{}",
            TEST_REGISTRATION_EMPTY_DETAIL.replace("{file_display}", file_display)
        );
    }
}

/// Formats test registration error labels consistently
fn format_test_registration_error_label(file_display: &str, error_index: usize) -> String {
    if error_index == 0 {
        format!("{file_display} (file-level)")
    } else {
        format!("{file_display} (test #{error_index})")
    }
}

/// Writes registration status to error log file
fn write_registration_status_to_log(
    file_display: &str,
    registered: usize,
    failed: usize,
    registration_errors: &[RegistrationError],
    error_log: &mut Option<File>,
) {
    if let Some(log) = error_log.as_mut() {
        if failed > 0 {
            let _ = writeln!(
                log,
                "{}",
                TEST_REGISTRATION_WARNING
                    .replace("{registered}", &registered.to_string())
                    .replace("{failed}", &failed.to_string())
                    .replace("{file_display}", file_display)
            );
            for (label, error) in registration_errors {
                let _ = writeln!(log, "[WARNING] Test registration error in {label}: {error}");
            }
        } else if registered == 0 {
            let _ = writeln!(
                log,
                "{}",
                TEST_REGISTRATION_EMPTY.replace("{file_display}", file_display)
            );
            let _ = writeln!(
                log,
                "{}",
                TEST_REGISTRATION_EMPTY_DETAIL.replace("{file_display}", file_display)
            );
        } else {
            let _ = writeln!(
                log,
                "{}",
                TEST_REGISTRATION_SUCCESS
                    .replace("{registered}", &registered.to_string())
                    .replace("{file_display}", file_display)
            );
        }
    }
}

/// Registers tests from a single file.
fn register_tests_from_single_file(
    file_path: &Path,
    file_display: &str,
) -> (usize, usize, Vec<SutraError>) {
    let test_forms = match TestDiscoverer::extract_tests_from_file(file_path) {
        Ok(forms) => forms,
        Err(error) => {
            return (0, 0, vec![error]);
        }
    };

    let mut registered = 0;
    let mut failed_tests = Vec::new();
    for (test_idx, test_form) in test_forms.into_iter().enumerate() {
        match register_single_test(&test_form, test_idx, file_display) {
            Ok(_) => registered += 1,
            Err(error) => {
                failed_tests.push(error);
            }
        }
    }

    (registered, failed_tests.len(), failed_tests)
}

/// Registers a single test form.
fn register_single_test(
    test_form: &ASTDefinition,
    _test_idx: usize,
    _file_display: &str,
) -> CliResult {
    use crate::atoms::{TestDefinition, TEST_REGISTRY};

    // Convert ASTDefinition to TestDefinition
    let expect = match &test_form.expect_form {
        Some(ast) => ast.clone(),
        None => {
            return Err(err_src!(
                Validation,
                format!(
                    "{}",
                    ERROR_MISSING_EXPECT_FORM.replace("{}", &test_form.name)
                ),
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

    let mut registry = TEST_REGISTRY
        .lock()
        .map_err(|_| err_msg!(Internal, ERROR_TEST_REGISTRY_POISONED))?;
    if registry.contains_key(&test_def.name) {
        return Err(err_src!(
            Validation,
            format!(
                "{}",
                ERROR_DUPLICATE_TEST_NAME.replace("{}", &test_def.name)
            ),
            &test_def.source_file,
            test_def.span
        ));
    }
    registry.insert(test_def.name.clone(), test_def);
    Ok(())
}

/// Gets all registered tests from the test registry.
fn get_registered_tests() -> Result<TestDefinitionList, SutraError> {
    use crate::atoms::TEST_REGISTRY;

    let registry = TEST_REGISTRY
        .lock()
        .map_err(|_| err_msg!(Internal, ERROR_TEST_REGISTRY_POISONED))?;
    let test_definitions: Vec<_> = registry.values().cloned().collect();
    drop(registry); // Release lock before execution

    Ok(test_definitions)
}

/// Executes all registered tests and returns summary statistics.
fn execute_all_tests(test_definitions: &TestDefinitionSlice) -> TestSummary {
    // Registration is metadata-only; all test execution happens here.
    let mut summary = TestSummary::default();
    let mut formatter = OutputFormatter::new();

    for test_def in test_definitions {
        let test_name = &test_def.name;
        let file_info = test_def.source_file.name();

        match execute_single_test(test_def) {
            Ok(_) => {
                summary.passed += 1;
                formatter.write_test_pass(test_name, file_info);
            }
            Err(error) => {
                summary.errored += 1;
                formatter.write_test_error(test_name, file_info, &error);
            }
        }
    }

    summary
}

/// Executes a single test and returns the result.
fn execute_single_test(test_def: &TestDefinition) -> CliResult {
    let pipeline = ExecutionPipeline::default();

    // Execute test body and capture the result
    let test_result = pipeline.execute_ast(&test_def.body);

    match test_result {
        Ok(actual_value) => {
            // Test body executed successfully - now validate expectations
            validate_test_expectations(test_def, &actual_value)
        }
        Err(original_error) => {
            // Test body failed - preserve the original error details
            Err(err_src!(
                TestFailure,
                format!("Test '{}' failed: {}", test_def.name, original_error),
                &test_def.source_file,
                test_def.span
            ))
        }
    }
}

/// Validates test expectations against the actual result.
fn validate_test_expectations(test_def: &TestDefinition, actual_value: &Value) -> CliResult {
    // For now, we'll implement basic value expectation checking
    // TODO: Implement full expectation validation including error types, tags, etc.

    // Parse the expect form to extract value expectations
    if let Some(expected_value) = extract_expected_value(&test_def.expect) {
        if actual_value != &expected_value {
            return Err(err_src!(
                TestFailure,
                format!(
                    "Test '{}' failed: expected {:?}, got {:?}",
                    test_def.name, expected_value, actual_value
                ),
                &test_def.source_file,
                test_def.span
            ));
        }
    }

    Ok(())
}

/// Extracts the expected value from an expect form.
/// TODO: This is a simplified implementation - should be expanded to handle all expectation types
fn extract_expected_value(expect_node: &AstNode) -> Option<Value> {
    // Look for (value <expected>) in the expect form
    if let Expr::List(items, _) = &*expect_node.value {
        for item in items {
            if let Expr::List(value_items, _) = &*item.value {
                if value_items.len() >= 2 {
                    if let Expr::String(s, _) = &*value_items[0].value {
                        if s == "value" {
                            // Found (value <expected>) - evaluate the expected value
                            // For now, return None to indicate no value expectation
                            // TODO: Implement proper evaluation of the expected value
                            return None;
                        }
                    }
                }
            }
        }
    }
    None
}

/// Prints test summary statistics using consolidated formatter.
fn print_test_summary(summary: &TestSummary) {
    let mut formatter = OutputFormatter::new();
    formatter.write_test_summary(summary);
}

// --- Validation Orchestration ---

/// Orchestrates the complete grammar validation workflow.
/// Encapsulates validation execution, result handling, and output formatting.
struct ValidationOrchestrator;

impl ValidationOrchestrator {
    /// Executes grammar validation and handles the complete workflow.
    fn validate_grammar(&self) -> CliResult {
        use crate::validation::validate_grammar;

        let src_arc = to_error_source(GRAMMAR_VALIDATION_PATH);
        let validation_result = validate_grammar(GRAMMAR_VALIDATION_PATH).map_err(|error| {
            err_ctx!(
                Internal,
                format!(
                    "{}",
                    GRAMMAR_VALIDATION_ERROR.replace("{}", &error.to_string())
                ),
                &src_arc,
                Span::default(),
                GRAMMAR_VALIDATION_HELP
            )
        })?;

        // Handle validation result
        self.handle_validation_result(&validation_result)
    }

    /// Handles validation result by checking validity and formatting output.
    fn handle_validation_result(&self, validation_result: &ValidationResult) -> CliResult {
        // Early return on validation failure
        if !validation_result.is_valid() {
            return self.handle_validation_failure(GRAMMAR_VALIDATION_PATH, validation_result);
        }

        // Print warnings and suggestions
        self.print_validation_warnings(validation_result);
        self.print_validation_suggestions(validation_result);

        println!("{GRAMMAR_VALIDATION_SUCCESS}");
        Ok(())
    }

    /// Handles validation failure by constructing detailed error messages.
    fn handle_validation_failure(
        &self,
        grammar_path: &str,
        validation_result: &ValidationResult,
    ) -> CliResult {
        let src_arc = to_error_source(grammar_path);
        let mut error = err_ctx!(
            Validation,
            "Grammar validation failed",
            &src_arc,
            Span::default(),
            GRAMMAR_VALIDATION_HELP
        );

        // Build comprehensive error message with all validation errors
        error = self.build_comprehensive_error_message(error, validation_result);

        Err(error)
    }

    /// Builds a comprehensive error message by appending all validation errors.
    fn build_comprehensive_error_message(
        &self,
        mut error: SutraError,
        validation_result: &ValidationResult,
    ) -> SutraError {
        for err in &validation_result.errors {
            error = self.append_error_to_help_message(error, err);
        }
        error
    }

    /// Appends a single error to the help message of a validation error.
    fn append_error_to_help_message(&self, error: SutraError, err: &str) -> SutraError {
        match error {
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
        }
    }

    /// Prints validation warnings to stderr.
    fn print_validation_warnings(&self, validation_result: &ValidationResult) {
        for warning in &validation_result.warnings {
            eprintln!("[Warning] {warning}");
        }
    }

    /// Prints validation suggestions to stderr.
    fn print_validation_suggestions(&self, validation_result: &ValidationResult) {
        for suggestion in &validation_result.suggestions {
            eprintln!("[Suggestion] {suggestion}");
        }
    }
}

// --- Macro Processing ---

/// Partitions AST nodes into macro definitions and user code, and builds a user macro registry.
fn separate_macros_from_user_code(ast_nodes: Vec<AstNode>) -> MacroParseResult {
    let (macro_defs, user_code): (Vec<_>, Vec<_>) =
        ast_nodes.into_iter().partition(is_macro_definition);
    let mut user_macros = MacroRegistry::new();

    let ctx = MacroValidationContext::for_user_macros();
    let mut macros = Vec::new();

    for macro_expr in macro_defs {
        let (name, template) = parse_macro_definition(&macro_expr)?;
        macros.push((name, template));
    }

    ctx.validate_and_insert_many(macros, &mut user_macros.macros)?;
    Ok((user_macros, user_code))
}

/// Builds a complete macro environment and expands a program.
/// Returns the expanded AST ready for execution or further processing.
fn build_macro_environment_and_expand(
    ast_nodes: Vec<AstNode>,
    source: &str,
) -> Result<AstNode, SutraError> {
    let (user_macros, user_code) = separate_macros_from_user_code(ast_nodes)?;

    // Build complete macro environment
    let mut macro_environment = build_canonical_macro_env()?;
    macro_environment.user_macros.extend(user_macros.macros);

    // Wrap user code if needed
    let processor = AstProcessor;
    let program = processor.wrap_in_do_block(user_code, source);

    // Expand macros
    let expanded = expand_macros_recursively(program, &mut macro_environment)?;

    Ok(expanded)
}

// ============================================================================
// COMMAND HANDLERS - High-level CLI command implementations
// ============================================================================

// --- Analysis Commands: AST, validation, macro expansion, formatting ---

/// Handles the `ast` subcommand.
fn handle_ast(path: &Path) -> CliResult {
    let processor = AstProcessor;
    let (_source, ast_nodes) = processor.load_and_parse(path)?;

    let filename = get_safe_path_display_name(path);
    println!("AST for {filename}:");
    println!("={}=", "=".repeat(filename.len() + 9));

    if ast_nodes.is_empty() {
        println!("(empty)");
        return Ok(());
    }
    for (node_index, node) in ast_nodes.iter().enumerate() {
        if ast_nodes.len() > 1 {
            println!("\nNode {}:", node_index + 1);
        }
        println!("{node:#?}");
    }

    Ok(())
}

/// Handles the `macroexpand` subcommand.
fn handle_macroexpand(path: &Path) -> CliResult {
    let processor = AstProcessor;
    let (source, ast_nodes) = processor.load_and_parse(path)?;
    let expanded = build_macro_environment_and_expand(ast_nodes, &source)?;
    println!("{}", expanded.value.pretty());
    Ok(())
}

/// Handles the `format` subcommand.
fn handle_format(path: &Path) -> CliResult {
    let processor = AstProcessor;
    let (source, ast_nodes) = processor.load_and_parse(path)?;
    let expanded = build_macro_environment_and_expand(ast_nodes, &source)?;
    println!("{}", expanded.value.pretty());
    Ok(())
}

// --- Execution Commands: Run, macrotrace ---

/// Handles file execution using the unified execution pipeline.
/// Used by both `run` and `macrotrace` subcommands.
fn handle_execution(path: &Path) -> CliResult {
    let source = read_file_to_string(path)?;
    let output = SharedOutput::new(StdoutSink);

    // Use the unified execution pipeline
    let pipeline = ExecutionPipeline::default();
    pipeline.execute(&source, output)
}

// --- Registry Commands: List macros, list atoms ---

/// Handles the `list-macros` subcommand.
fn handle_list_macros() -> CliResult {
    use crate::runtime::build_canonical_macro_env;

    let macro_environment = build_canonical_macro_env()?;
    let core_macro_names: Vec<_> = macro_environment.core_macros.keys().collect();
    let user_macro_names: Vec<_> = macro_environment.user_macros.keys().collect();

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
    use crate::{atoms::Atom, runtime::build_default_atom_registry};

    let atom_registry = build_default_atom_registry();
    let mut pure_atoms = Vec::new();
    let mut stateful_atoms = Vec::new();
    let mut special_forms = Vec::new();

    for (name, atom) in atom_registry.atoms.iter() {
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

// --- Testing Commands: Test execution ---

/// Handles the `test` subcommand using the unified execution pipeline.
pub fn handle_test(path: &Path) -> CliResult {
    use std::fs::OpenOptions;

    // Prepare error log file
    let mut error_log = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(TEST_ERROR_LOG_FILE)
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

    if test_results.has_failures() {
        return Err(err_msg!(Internal, ERROR_SOME_TESTS_FAILED));
    }

    Ok(())
}
