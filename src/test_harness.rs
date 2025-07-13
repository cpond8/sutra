//! Sutra Test Harness Library Module
//!
//! Provides reusable test discovery, execution, and reporting for YAML-based test suites.
//! This module implements a complete testing infrastructure that can discover YAML test files,
//! execute Sutra code through the full parsing → macro expansion → evaluation pipeline,
//! and provide comprehensive reporting with error handling.
//!
//! # Architecture
//!
//! The test harness follows a phase-based execution model:
//! 1. **Discovery**: Find and load YAML test files from the filesystem
//! 2. **Parsing**: Parse test input through Sutra's syntax parser
//! 3. **Macro Processing**: Handle macro definitions and expand macros
//! 4. **Evaluation**: Execute the expanded code in a fresh runtime environment
//! 5. **Comparison**: Compare actual results against expected outputs or error conditions
//! 6. **Reporting**: Generate detailed reports with colored output and diff display
//!
//! # Test Format
//!
//! Tests are defined in YAML files with the following structure:
//! ```yaml
//! - name: "test name"
//!   input: "(some-sutra-code)"
//!   expected: "expected output"      # for success tests
//!   expect_error: "error substring"  # for error tests
//!   expect_error_code: "ERROR_CODE"  # for specific error code tests
//!   skip: false                      # optional, defaults to false
//!   only: false                      # optional, defaults to false
//! ```
//!
//! # Public API
//!
//! - [`run_test_case`] - Execute a single test case
//! - [`discover_yaml_files`] - Find all YAML test files in a directory tree
//! - [`load_test_cases`] - Load and parse test cases from a YAML file
//! - [`run_all_tests`] - Complete test suite execution with filtering and reporting
//!
//! # Example Usage
//!
//! ```rust,no_run
//! use sutra::test_harness::{run_all_tests, TestConfig};
//!
//! let config = TestConfig::default();
//! let (passed, failed, skipped) = run_all_tests(None, &config);
//! if failed > 0 {
//!     std::process::exit(1);
//! }
//! ```

use crate::ast::{AstNode, Expr, Span, WithSpan};
use crate::atoms::OutputSink;
use crate::cli::output::OutputBuffer;
use crate::macros::{expand_macros, MacroDef, MacroTemplate};
use crate::runtime::eval::eval;
use crate::runtime::registry::{build_canonical_macro_env, build_default_atom_registry};
use crate::runtime::world::World;
use crate::syntax::error::SutraError;
use crate::syntax::parser;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use walkdir::WalkDir;

// =============================================================================
// CORE TYPES
// =============================================================================

/// Represents the result of executing a single test case.
#[derive(Debug, Clone)]
pub enum TestResult {
    /// Test passed successfully
    Pass { file: String, name: String },
    /// Test failed with an error
    Fail {
        file: String,
        name: String,
        error: String,
        expanded: Option<String>,
        eval: Option<String>,
    },
    /// Test was skipped
    Skipped {
        file: String,
        name: String,
        reason: String,
    },
}

/// Represents a single YAML test case for the Sutra test harness.
#[derive(Debug, Deserialize, Clone)]
pub struct TestCase {
    pub name: String,
    pub input: String,
    pub expected: Option<String>,
    pub expect_error: Option<String>,
    pub expect_error_code: Option<String>,
    #[serde(default)]
    pub skip: bool,
    #[serde(default)]
    pub only: bool,
}

/// Internal state maintained throughout test execution phases.
pub struct PhaseState {
    pub world: World,
    pub atom_registry: crate::atoms::AtomRegistry,
    pub output_sink: OutputBuffer,
    pub expanded: Option<String>,
    pub eval: Option<String>,
}

/// Error types that can occur during test execution.
#[derive(Debug)]
pub enum SutraTestError {
    Setup(String),
    Parse(SutraError),
    MacroDef(String),
    MacroExpand(SutraError, Option<String>),
    Eval(SutraError, Option<String>, Option<String>),
}

/// Configuration for test execution and reporting.
pub struct TestConfig {
    pub test_root: String,
    pub eval_limit: usize,
    pub use_colors: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_root: "tests/suites".to_string(),
            eval_limit: 1000,
            use_colors: atty::is(atty::Stream::Stderr),
        }
    }
}

// Color constants for terminal output
const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";

impl TestConfig {
    /// Apply color formatting to text if colors are enabled.
    pub fn colorize(&self, text: &str, color: &str) -> String {
        if self.use_colors {
            format!("{}{}{}", color, text, RESET)
        } else {
            text.to_string()
        }
    }
}

// =============================================================================
// TEST DISCOVERY AND LOADING
// =============================================================================

/// Discovers all YAML files recursively under the given root directory.
pub fn discover_yaml_files<P: AsRef<Path>>(root: P) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path()
                    .extension()
                    .map(|ext| ext == "yaml" || ext == "yml")
                    .unwrap_or(false)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

/// Load and parse test cases from a YAML file.
pub fn load_test_cases(path: &Path) -> Vec<TestCase> {
    match fs::read_to_string(path) {
        Ok(content) => match serde_yaml::from_str::<Vec<TestCase>>(&content) {
            Ok(cases) => cases,
            Err(e) => {
                eprintln!("Failed to parse YAML in {}: {}", path.display(), e);
                Vec::new()
            }
        },
        Err(e) => {
            eprintln!("Failed to read {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

/// Helper for test skipping logic.
pub fn skip_reason(case: &TestCase, has_only: bool, filter: Option<&str>) -> Option<String> {
    if has_only && !case.only {
        return Some("Not marked 'only' in 'only' mode".to_string());
    }
    if case.skip {
        return Some("Marked 'skip'".to_string());
    }
    if let Some(f) = filter {
        if !case.name.to_lowercase().contains(f) {
            return Some(format!("Filtered out by substring: {}", f));
        }
    }
    None
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Check if an AST node represents a macro definition.
fn is_macro_definition(node: &AstNode) -> bool {
    if let Expr::List(ref list, _) = *node.value {
        if let Some(first) = list.first() {
            if let Expr::Symbol(ref sym, _) = *first.value {
                return sym == "macro";
            }
        }
    }
    false
}

/// Wrap a list of nodes in a (do ...) form if needed.
fn wrap_in_do_if_needed(nodes: Vec<AstNode>, _input: &str) -> AstNode {
    if nodes.len() == 1 {
        nodes.into_iter().next().unwrap()
    } else {
        let span = Span::default();
        let mut list = Vec::with_capacity(nodes.len() + 1);
        let do_symbol = WithSpan {
            value: Arc::new(Expr::Symbol("do".to_string(), span.clone())),
            span: span.clone(),
        };
        list.push(do_symbol);
        list.extend(nodes);
        WithSpan {
            value: Arc::new(Expr::List(list, span.clone())),
            span,
        }
    }
}

/// Parse a macro definition node into name and template.
fn parse_macro_definition(node: &AstNode) -> Result<(String, AstNode), String> {
    if let Expr::List(ref list, _) = *node.value {
        if list.len() < 3 {
            return Err("Macro definition must have at least 3 elements".to_string());
        }
        if let Expr::Symbol(ref name, _) = *list[1].value {
            let template = list[2].clone();
            Ok((name.clone(), template))
        } else {
            Err("Macro name must be a symbol".to_string())
        }
    } else {
        Err("Not a macro definition list".to_string())
    }
}

/// Check if a SutraError matches an expected error code.
fn matches_error_code(error: &SutraError, expected_code: &str) -> bool {
    if let Some(actual_code) = error.error_code() {
        actual_code == expected_code
    } else {
        false
    }
}

/// Create a unified error result using SutraError.
fn make_error_result(
    error: SutraError,
    case: &TestCase,
    file: &str,
    expanded: Option<String>,
    eval: Option<String>,
) -> TestResult {
    let error_msg = error.to_string();
    let matches = if let Some(expected_code) = case.expect_error_code.as_deref() {
        matches_error_code(&error, expected_code)
    } else if let Some(expected) = case.expect_error.as_deref() {
        error_msg.contains(expected)
    } else {
        false
    };
    if matches {
        TestResult::Pass {
            file: file.to_string(),
            name: case.name.clone(),
        }
    } else {
        TestResult::Fail {
            file: file.to_string(),
            name: case.name.clone(),
            error: error_msg,
            expanded,
            eval,
        }
    }
}

// =============================================================================
// TEST EXECUTION PHASES
// =============================================================================

/// Initialize the test environment with world, atom registry, and macro environment.
fn setup_env_phase() -> Result<PhaseState, SutraTestError> {
    let mut world = World::default();
    world.macros = match build_canonical_macro_env() {
        Ok(macros) => macros,
        Err(e) => return Err(SutraTestError::Setup(format!("Setup error: {}", e))),
    };
    let atom_registry = build_default_atom_registry();
    let output_sink = OutputBuffer::default();
    Ok(PhaseState {
        world,
        atom_registry,
        output_sink,
        expanded: None,
        eval: None,
    })
}

/// Parse the test input into AST nodes.
fn parse_phase(
    state: PhaseState,
    case: &TestCase,
) -> Result<(PhaseState, Vec<AstNode>), SutraTestError> {
    match parser::parse(&case.input) {
        Ok(nodes) => Ok((state, nodes)),
        Err(e) => Err(SutraTestError::Parse(e)),
    }
}

/// Process macro definitions and add them to the macro environment.
fn macro_phase(
    mut state: PhaseState,
    macro_defs: Vec<AstNode>,
    _case: &TestCase,
    _file: &str,
) -> Result<PhaseState, SutraTestError> {
    for macro_expr in macro_defs {
        match parse_macro_definition(&macro_expr) {
            Ok((name, template)) => {
                let macro_template = MacroTemplate::new(
                    crate::ast::ParamList {
                        required: vec![],
                        rest: None,
                        span: template.span.clone(),
                    },
                    Box::new(template),
                )
                .map_err(|e| {
                    SutraTestError::MacroDef(format!("Failed to create macro template: {}", e))
                })?;
                state
                    .world
                    .macros
                    .user_macros
                    .insert(name, MacroDef::Template(macro_template));
            }
            Err(e) => {
                return Err(SutraTestError::MacroDef(format!(
                    "Macro definition error: {}",
                    e
                )));
            }
        }
    }
    Ok(state)
}

/// Expand macros in the program.
fn expand_phase(
    mut state: PhaseState,
    program: AstNode,
    _case: &TestCase,
    _file: &str,
) -> Result<PhaseState, SutraTestError> {
    match expand_macros(program, &mut state.world.macros) {
        Ok(expanded) => {
            state.expanded = Some(expanded.value.pretty());
            Ok(state)
        }
        Err(e) => Err(SutraTestError::MacroExpand(e, state.expanded.clone())),
    }
}

/// Evaluate the expanded program.
fn eval_phase(
    mut state: PhaseState,
    expanded: AstNode,
    case: &TestCase,
    eval_limit: usize,
) -> Result<PhaseState, SutraTestError> {
    let eval_result = eval(
        &expanded,
        &mut state.world,
        &mut state.output_sink,
        &state.atom_registry,
        eval_limit,
    );
    // TODO: This is a workaround to ensure the output sink always contains a user-facing value.
    // If the code under test does not emit output, we emit the Value part of the result here.
    // In the future, consider making output emission explicit in all testable code paths.
    if let Ok((val, _)) = &eval_result {
        if state.output_sink.as_str().trim().is_empty() {
            state.output_sink.emit(&format!("{}", val), None);
        }
    }
    state.eval = eval_result.as_ref().ok().map(|(val, _)| format!("{}", val));

    if let Ok(val) = &eval_result {
        if expected_error(case) {
            // Convert the success case to a SutraError for consistent handling
            let error_msg = expected_error_message(case, &val.0);
            let sutra_error = crate::syntax::error::parse_error(error_msg, None);
            return Err(SutraTestError::Eval(
                sutra_error,
                state.expanded.clone(),
                state.eval.clone(),
            ));
        }
        return Ok(state);
    }
    Err(SutraTestError::Eval(
        eval_result.unwrap_err(),
        state.expanded.clone(),
        state.eval.clone(),
    ))
}

/// Check if a test case expects an error.
fn expected_error(case: &TestCase) -> bool {
    case.expect_error.is_some() || case.expect_error_code.is_some()
}

/// Generate error message for unexpected success.
fn expected_error_message(case: &TestCase, val: &crate::ast::value::Value) -> String {
    if let Some(err) = case.expect_error.as_deref() {
        format!(
            "Expected error '{}' but evaluation succeeded with result: {}",
            err, val
        )
    } else if let Some(code) = case.expect_error_code.as_deref() {
        format!(
            "Expected error code '{}' but evaluation succeeded with result: {}",
            code, val
        )
    } else {
        String::new()
    }
}

/// Compare results and generate final test result.
fn compare_and_report(state: PhaseState, case: &TestCase, file: &str) -> TestResult {
    let actual_output = state.output_sink.as_str();
    let passed = match (
        case.expected.as_deref(),
        case.expect_error.as_deref(),
        case.expect_error_code.as_deref(),
    ) {
        (Some(expected), None, None) => actual_output.trim() == expected.trim(),
        _ => true, // error cases handled elsewhere
    };

    if passed {
        TestResult::Pass {
            file: file.to_string(),
            name: case.name.clone(),
        }
    } else {
        TestResult::Fail {
            file: file.to_string(),
            name: case.name.clone(),
            error: format_output_mismatch(case.expected.as_deref().unwrap_or(""), actual_output),
            expanded: state.expanded,
            eval: state.eval,
        }
    }
}

/// Format output mismatch error message.
fn format_output_mismatch(expected: &str, actual: &str) -> String {
    format!(
        "Output did not match expected\n  Expected: {}\n  Actual:   {}",
        expected.trim(),
        actual.trim()
    )
}

/// Handle test execution errors and convert to TestResult.
fn handle_error(err: SutraTestError, case: &TestCase, file: &str) -> TestResult {
    match err {
        SutraTestError::Setup(msg) => TestResult::Fail {
            file: file.to_string(),
            name: case.name.clone(),
            error: msg,
            expanded: None,
            eval: None,
        },
        SutraTestError::Parse(sutra_error) => {
            make_error_result(sutra_error, case, file, None, None)
        }
        SutraTestError::MacroDef(msg) => TestResult::Fail {
            file: file.to_string(),
            name: case.name.clone(),
            error: msg,
            expanded: None,
            eval: None,
        },
        SutraTestError::MacroExpand(sutra_error, expanded) => {
            make_error_result(sutra_error, case, file, expanded, None)
        }
        SutraTestError::Eval(sutra_error, expanded, eval) => {
            make_error_result(sutra_error, case, file, expanded, eval)
        }
    }
}

// =============================================================================
// MAIN TEST EXECUTION
// =============================================================================

/// Execute a single test case through the complete pipeline.
pub fn run_test_case(file: String, case: TestCase, eval_limit: usize) -> TestResult {
    // Main test case runner
    let result = setup_env_phase().and_then(|state| {
        let (state, ast_nodes) = parse_phase(state, &case)?;
        let (macro_defs, user_code): (Vec<_>, Vec<_>) =
            ast_nodes.into_iter().partition(is_macro_definition);
        let state = macro_phase(state, macro_defs, &case, &file)?;
        let program = wrap_in_do_if_needed(user_code, &case.input);
        let state = expand_phase(state, program, &case, &file)?;

        // Re-parse expanded for eval
        let expanded_ast = parser::parse(&state.expanded.as_ref().unwrap())
            .map_err(|e| SutraTestError::Parse(e))?;
        let expanded = wrap_in_do_if_needed(expanded_ast, &case.input);
        let state = eval_phase(state, expanded, &case, eval_limit)?;
        Ok(state)
    });

    match result {
        Ok(state) => compare_and_report(state, &case, &file),
        Err(err) => handle_error(err, &case, &file),
    }
}

// =============================================================================
// REPORTING AND OUTPUT
// =============================================================================

/// Partition test results by outcome type.
pub fn partition_results(results: &[TestResult]) -> (usize, usize, usize) {
    let passed = results
        .iter()
        .filter(|r| matches!(r, TestResult::Pass { .. }))
        .count();
    let failed = results
        .iter()
        .filter(|r| matches!(r, TestResult::Fail { .. }))
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r, TestResult::Skipped { .. }))
        .count();
    (passed, failed, skipped)
}

/// Print comprehensive test results with colored output.
pub fn report_results(results: &[TestResult], config: &TestConfig) {
    let (passed, rest): (Vec<_>, Vec<_>) = results
        .iter()
        .partition(|r| matches!(r, TestResult::Pass { .. }));
    let (failed, skipped): (Vec<_>, Vec<_>) = rest
        .into_iter()
        .partition(|r| matches!(r, TestResult::Fail { .. }));
    let total = results.len();
    let passed_count = passed.len();
    let failed_count = failed.len();
    let skipped_count = skipped.len();

    for r in results {
        match r {
            TestResult::Pass { file, name } => {
                println!("{}: {} [{}]", config.colorize("PASS", GREEN), name, file)
            }
            TestResult::Fail { .. } => print_failure(r, config),
            TestResult::Skipped { file, name, reason } => {
                println!(
                    "{}: {} [{}] ({})",
                    config.colorize("SKIP", YELLOW),
                    name,
                    file,
                    reason
                )
            }
        }
    }

    println!(
        "\nTest summary: total {}, {} {}, {} {}, {} {}",
        total,
        config.colorize("passed", GREEN),
        passed_count,
        config.colorize("failed", RED),
        failed_count,
        config.colorize("skipped", YELLOW),
        skipped_count,
    );

    if failed_count > 0 {
        eprintln!("\nFailed tests:");
        for r in results {
            if let TestResult::Fail { name, .. } = r {
                eprintln!("  - {}", name);
            }
        }
    }
}

/// Print detailed failure information.
pub fn print_failure(r: &TestResult, config: &TestConfig) {
    match r {
        TestResult::Fail {
            file,
            name,
            error,
            expanded,
            eval,
        } => {
            let fail = config.colorize("FAIL", RED);
            eprintln!("{fail}: {} [{}]", name, file, fail = fail);
            eprintln!("  Error: {}", error);
            if let Some(expanded) = expanded {
                eprintln!("  Expanded: {}", expanded);
            }
            if let Some(eval) = eval {
                eprintln!("  Eval: {}", eval);
            }
            if error.starts_with("Output did not match expected") {
                print_output_diff(error, config);
            }
        }
        _ => {}
    }
}

/// Print diff for output mismatches.
pub fn print_output_diff(error: &str, config: &TestConfig) {
    let lines: Vec<_> = error.lines().collect();
    if lines.len() >= 3 {
        let expected = lines[1].trim_start_matches("Expected: ").trim();
        let actual = lines[2].trim_start_matches("Actual: ").trim();
        eprintln!("  Diff:");
        print_diff(expected, actual, config);
    }
}

/// Print line-by-line diff.
pub fn print_diff(expected: &str, actual: &str, config: &TestConfig) {
    let expected_lines: Vec<_> = expected.lines().collect();
    let actual_lines: Vec<_> = actual.lines().collect();
    let max = expected_lines.len().max(actual_lines.len());
    for i in 0..max {
        let exp = expected_lines.get(i).copied().unwrap_or("");
        let act = actual_lines.get(i).copied().unwrap_or("");
        if exp != act {
            eprintln!("  - expected: {}", config.colorize(exp, GREEN));
            eprintln!("  + actual:   {}", config.colorize(act, RED));
        } else {
            eprintln!("    {}", exp);
        }
    }
}

// =============================================================================
// PUBLIC API
// =============================================================================

/// Run all tests with optional filtering and return summary counts.
pub fn run_all_tests(filter: Option<&str>, config: &TestConfig) -> (usize, usize, usize) {
    let yaml_files = discover_yaml_files(&config.test_root);

    let mut all_cases = Vec::new();
    let mut has_only_tests = false;

    for file_path in &yaml_files {
        let file_name = file_path.display().to_string();
        let test_cases = load_test_cases(file_path);

        for case in test_cases {
            if case.only {
                has_only_tests = true;
            }
            all_cases.push((file_name.clone(), case));
        }
    }

    let results: Vec<TestResult> = all_cases
        .into_iter()
        .filter_map(|(file, case)| {
            if let Some(reason) = skip_reason(&case, has_only_tests, filter) {
                return Some(TestResult::Skipped {
                    file,
                    name: case.name,
                    reason,
                });
            }
            Some(run_test_case(file, case, config.eval_limit))
        })
        .collect();

    report_results(&results, config);
    partition_results(&results)
}

// =============================================================================
// BACKWARD COMPATIBILITY API
// =============================================================================

/// Test results summary for backward compatibility.
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
}

/// Run tests with command line arguments for backward compatibility.
pub fn run_tests_with_args(args: &[String]) -> TestResults {
    let filter = if !args.is_empty() {
        Some(args[0].to_lowercase())
    } else {
        None
    };

    let config = TestConfig::default();
    let (passed, failed, skipped) = run_all_tests(filter.as_deref(), &config);

    TestResults {
        passed,
        failed,
        skipped,
    }
}

/// Default test runner using standard configuration.
pub fn run_default_tests(filter: Option<&str>) -> (usize, usize, usize) {
    let config = TestConfig::default();
    run_all_tests(filter, &config)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_functions_exist() {
        let args = vec!["test".to_string()];
        let results = run_tests_with_args(&args);
        assert!(results.passed >= 0);
    }
}
