//! Ariadne-Centered Test Harness Architecture v2
//!
//! A complete redesign of the Sutra test harness that eliminates YAML-based test definitions
//! and custom error handling in favor of a unified, Ariadne-centered approach using
//! embedded source annotations and snapshot testing.
//!
//! # Architecture Principles
//!
//! 1. **Ariadne as Single Source of Truth**: All test assertions are based on rendered
//!    Ariadne diagnostic output, eliminating custom error reporting and comparison logic.
//! 2. **Declarative Source-Based Tests**: Test expectations are embedded directly in
//!    source files using special comments, making tests self-documenting.
//! 3. **Linear Pipeline Execution**: Clear dataflow from source → compilation →
//!    diagnostics → snapshot assertion.
//! 4. **Snapshot-Based Assertions**: Expected diagnostic output is stored as snapshots
//!    that can be easily updated when compiler output changes.

use ariadne::{Color, Label, Report, ReportKind, Source};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use sutra::ast::AstNode;
use sutra::cli::output::OutputBuffer;
use sutra::macros::expand_macros;
use sutra::runtime::eval::eval;
use sutra::runtime::registry::{build_canonical_macro_env, build_default_atom_registry};
use sutra::runtime::world::World;
use sutra::syntax::error::SutraError;
use sutra::syntax::validator::{SutraDiagnostic as ValidatorDiagnostic, ValidatorRegistry};
use sutra::syntax::error::ErrorCode;
use walkdir::WalkDir;

// =============================================================================
// CORE DATA STRUCTURES
// =============================================================================

/// A single test case extracted from source code annotations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestCase {
    /// Unique identifier for the test (filename + annotation index)
    pub id: String,
    /// Source file path
    pub file_path: PathBuf,
    /// Test name from annotation
    pub name: String,
    /// Full source code content
    pub source: String,
    /// Expected test outcome
    pub expectation: TestExpectation,
    /// Whether this test should be skipped
    pub skip: bool,
    /// Whether this is an "only" test (run exclusively)
    pub only: bool,
}
/// Expected outcome of a test execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TestExpectation {
    /// Test should succeed with no diagnostics
    Success,
    /// Test should produce parse errors
    ParseError {
        /// Expected error codes (optional)
        codes: Vec<String>,
        /// Expected error message substrings (optional)
        messages: Vec<String>,
    },
    /// Test should produce validation errors
    ValidationError {
        /// Expected error codes (optional)
        codes: Vec<String>,
        /// Expected error message substrings (optional)
        messages: Vec<String>,
    },
    /// Test should produce evaluation errors
    EvalError {
        /// Expected error codes (optional)
        codes: Vec<String>,
        /// Expected error message substrings (optional)
        messages: Vec<String>,
    },
    /// Custom diagnostic snapshot expectation
    DiagnosticSnapshot {
        /// Path to snapshot file relative to test file
        snapshot_path: String,
    },
    /// Symbolic error expectation using structured error codes
    /// Examples: "arity-error", "(or arity-error type-error)", "(and parse-error recursion-limit-exceeded)"
    SymbolicError {
        /// Symbolic expression defining expected error patterns
        expression: SymbolicExpression,
    },
    /// Direct value assertion for successful evaluations
    /// Examples: "42", "true", "(1 2 3)", "\"hello\""
    Value {
        /// Expected evaluation result
        expected_value: sutra::ast::value::Value,
    },
}

/// Result of executing a single test case.
#[derive(Debug, Clone)]
pub enum TestResult {
    /// Test passed - diagnostic output matched expectations
    Pass { id: String, name: String },
    /// Test failed - diagnostic output did not match
    DiagnosticMismatch {
        id: String,
        name: String,
        expected: String,
        actual: String,
    },
    /// Test failed due to unexpected success
    UnexpectedSuccess {
        id: String,
        name: String,
        expected_error_type: String,
    },
    /// Test failed due to unexpected error type
    WrongErrorType {
        id: String,
        name: String,
        expected: String,
        actual: String,
    },
    /// Test was skipped
    Skipped {
        id: String,
        name: String,
        reason: String,
    },
}

/// Configuration for test execution.
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Root directory to search for test files
    pub test_root: PathBuf,
    /// File extension for test files
    pub test_extension: String,
    /// Maximum evaluation steps
    pub eval_limit: usize,
    /// Whether to use colored output
    pub use_colors: bool,
    /// Whether to update snapshots on mismatch
    pub update_snapshots: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_root: PathBuf::from("tests/"), // TODO: Placeholder; proper folder structure needed
            test_extension: "sutra".to_string(),
            eval_limit: 1000,
            use_colors: atty::is(atty::Stream::Stdout),
            update_snapshots: false,
        }
    }
}

/// Internal execution state for a test case.
#[derive(Debug)]
pub struct ExecutionState {
    /// Parsed AST nodes
    pub ast: Option<Vec<AstNode>>,
    /// Expanded AST after macro processing
    pub expanded: Option<AstNode>,
    /// Evaluation result
    pub eval_result: Option<Result<sutra::ast::value::Value, SutraError>>,
    /// All collected diagnostics from compilation pipeline
    pub diagnostics: Vec<CompilerDiagnostic>,
}

/// Unified diagnostic from any stage of compilation.
#[derive(Debug, Clone)]
pub enum CompilerDiagnostic {
    /// Parse error
    Parse(SutraError),
    /// Validation error
    Validation(ValidatorDiagnostic),
    /// Evaluation error
    Eval(SutraError),
}

/// Symbolic expression for error matching.
/// Supports logical operators (and, or, not) and error code atoms.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SymbolicExpression {
    /// An atomic error code like "arity-error" or "parse-error"
    ErrorCode(ErrorCode),
    /// Logical AND: all sub-expressions must match
    And(Vec<SymbolicExpression>),
    /// Logical OR: at least one sub-expression must match
    Or(Vec<SymbolicExpression>),
    /// Logical NOT: sub-expression must not match
    Not(Box<SymbolicExpression>),
}

// =============================================================================
// ANNOTATION PARSING
// =============================================================================

/// Parses test annotations from source code comments.
///
/// Supported annotation formats:
/// ```
/// //! @test "test name"
/// //! @expect success
/// //! @expect parse_error
/// //! @expect validation_error codes=["E001", "E002"]
/// //! @expect eval_error messages=["division by zero"]
/// //! @expect snapshot "path/to/snapshot.txt"
/// //! @skip "reason for skipping"
/// //! @only
/// ```
pub fn extract_test_cases(file_path: &Path) -> Result<Vec<TestCase>, Box<dyn std::error::Error>> {
    let source = fs::read_to_string(file_path)?;
    let mut test_cases = Vec::new();
    let mut current_annotations = HashMap::new();
    let mut test_index = 0;

    for (_line_num, line) in source.lines().enumerate() {
        let line = line.trim();

        if let Some(annotation) = parse_test_annotation(line)? {
            current_annotations.insert(annotation.0, annotation.1);
        } else if !line.starts_with("//") && !line.is_empty() {
            // Non-comment, non-empty line - this marks the end of an annotation block
            if !current_annotations.is_empty() {
                let test_case =
                    build_test_case(file_path, &source, test_index, &current_annotations)?;
                test_cases.push(test_case);
                current_annotations.clear();
                test_index += 1;
            }
        }
    }

    // Handle final annotation block at end of file
    if !current_annotations.is_empty() {
        let test_case = build_test_case(file_path, &source, test_index, &current_annotations)?;
        test_cases.push(test_case);
    }

    Ok(test_cases)
}

/// Parses a single test annotation line.
fn parse_test_annotation(
    line: &str,
) -> Result<Option<(String, String)>, Box<dyn std::error::Error>> {
    if !line.starts_with("//! @") {
        return Ok(None);
    }

    let content = &line[5..].trim(); // Remove "//! @"

    if let Some(space_pos) = content.find(' ') {
        let key = content[..space_pos].to_string();
        let value = content[space_pos + 1..].trim().to_string();
        Ok(Some((key, value)))
    } else {
        Ok(Some((content.to_string(), "true".to_string())))
    }
}

/// Builds a test case from collected annotations.
fn build_test_case(
    file_path: &Path,
    source: &str,
    index: usize,
    annotations: &HashMap<String, String>,
) -> Result<TestCase, Box<dyn std::error::Error>> {
    let name = annotations
        .get("test")
        .cloned()
        .unwrap_or_else(|| format!("test_{}", index));

    let id = format!("{}#{}", file_path.display(), index);

    let expectation = parse_expectation(annotations)?;

    let skip = annotations.contains_key("skip");
    let only = annotations.contains_key("only");

    Ok(TestCase {
        id,
        file_path: file_path.to_path_buf(),
        name,
        source: source.to_string(),
        expectation,
        skip,
        only,
    })
}

/// Parses test expectation from annotations.
fn parse_expectation(
    annotations: &HashMap<String, String>,
) -> Result<TestExpectation, Box<dyn std::error::Error>> {
    if let Some(expect_value) = annotations.get("expect") {
        match expect_value.as_str() {
            "success" => Ok(TestExpectation::Success),
            "parse_error" => Ok(TestExpectation::ParseError {
                codes: parse_string_list(annotations.get("codes"))?,
                messages: parse_string_list(annotations.get("messages"))?,
            }),
            "validation_error" => Ok(TestExpectation::ValidationError {
                codes: parse_string_list(annotations.get("codes"))?,
                messages: parse_string_list(annotations.get("messages"))?,
            }),
            "eval_error" => Ok(TestExpectation::EvalError {
                codes: parse_string_list(annotations.get("codes"))?,
                messages: parse_string_list(annotations.get("messages"))?,
            }),
            _ => {
                // Try to parse as symbolic error expression
                if is_symbolic_error_expression(expect_value) {
                    let expression = parse_symbolic_expression(expect_value)?;
                    Ok(TestExpectation::SymbolicError { expression })
                }
                // Try to parse as value assertion
                else if is_value_expression(expect_value) {
                    let expected_value = parse_expected_value(expect_value)?;
                    Ok(TestExpectation::Value { expected_value })
                }
                // Unknown expectation type
                else {
                    Err(format!("Unknown expectation type: {}", expect_value).into())
                }
            }
        }
    } else if let Some(snapshot_path) = annotations.get("snapshot") {
        Ok(TestExpectation::DiagnosticSnapshot {
            snapshot_path: snapshot_path.clone(),
        })
    } else {
        // Default to success if no expectation specified
        Ok(TestExpectation::Success)
    }
}

/// Checks if a string looks like a symbolic error expression.
fn is_symbolic_error_expression(input: &str) -> bool {
    let input = input.trim();

    // Check if it's a parenthesized expression (S-expression)
    if input.starts_with('(') && input.ends_with(')') {
        return true;
    }

    // Check if it's a known error code
    matches!(input,
        "parse-error" | "recursion-limit-exceeded" | "validation-error" |
        "io-error" | "malformed-ast-error" | "internal-parse-error" |
        "arity-error" | "type-error" | "division-by-zero" | "eval-error"
    )
}

/// Checks if a string looks like a value expression.
fn is_value_expression(input: &str) -> bool {
    let input = input.trim();

    // Check for numbers
    if input.parse::<i64>().is_ok() || input.parse::<f64>().is_ok() {
        return true;
    }

    // Check for booleans
    if matches!(input, "true" | "false") {
        return true;
    }

    // Check for quoted strings
    if input.starts_with('"') && input.ends_with('"') && input.len() >= 2 {
        return true;
    }

    // Check for lists (parenthesized but not symbolic expressions)
    if input.starts_with('(') && input.ends_with(')') {
        // This is a heuristic: if it contains only simple tokens (no operators like 'and', 'or', 'not')
        // then it's likely a list rather than a symbolic expression
        let inner = &input[1..input.len()-1].trim();
        if inner.is_empty() {
            return true; // Empty list
        }

        // Simple check: if the first token is not a logical operator, assume it's a list
        if let Some(first_token) = inner.split_whitespace().next() {
            return !matches!(first_token, "and" | "or" | "not");
        }
    }

    false
}

/// Parses a string list from annotation value (e.g., ["item1", "item2"]).
fn parse_string_list(value: Option<&String>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    match value {
        Some(v) if v.starts_with('[') && v.ends_with(']') => {
            // Simple JSON array parsing
            Ok(serde_json::from_str(v)?)
        }
        Some(v) => Ok(vec![v.clone()]),
        None => Ok(vec![]),
    }
}

/// Parses a symbolic error expression from a string.
///
/// Supports syntax like:
/// - "arity-error" (simple error code)
/// - "(or arity-error type-error)" (logical OR)
/// - "(and parse-error recursion-limit-exceeded)" (logical AND)
/// - "(not division-by-zero)" (logical NOT)
pub fn parse_symbolic_expression(input: &str) -> Result<SymbolicExpression, Box<dyn std::error::Error>> {
    let input = input.trim();

    // Handle simple error code (no parentheses)
    if !input.starts_with('(') {
        return Ok(SymbolicExpression::ErrorCode(parse_error_code(input)?));
    }

    // Parse S-expression
    if !input.ends_with(')') {
        return Err("Symbolic expression must be balanced parentheses".into());
    }

    let inner = &input[1..input.len()-1].trim();
    let tokens = tokenize_symbolic_expression(inner)?;

    if tokens.is_empty() {
        return Err("Empty symbolic expression".into());
    }

    let operator = &tokens[0];
    let args = &tokens[1..];

    match operator.as_str() {
        "and" => {
            let sub_expressions: Result<Vec<_>, _> = args.iter()
                .map(|arg| parse_symbolic_expression(arg))
                .collect();
            Ok(SymbolicExpression::And(sub_expressions?))
        },
        "or" => {
            let sub_expressions: Result<Vec<_>, _> = args.iter()
                .map(|arg| parse_symbolic_expression(arg))
                .collect();
            Ok(SymbolicExpression::Or(sub_expressions?))
        },
        "not" => {
            if args.len() != 1 {
                return Err("'not' operator requires exactly one argument".into());
            }
            let sub_expression = parse_symbolic_expression(&args[0])?;
            Ok(SymbolicExpression::Not(Box::new(sub_expression)))
        },
        _ => Err(format!("Unknown operator: {}", operator).into()),
    }
}

/// Converts error code strings to ErrorCode enum values.
fn parse_error_code(code_str: &str) -> Result<ErrorCode, Box<dyn std::error::Error>> {
    match code_str {
        "parse-error" => Ok(ErrorCode::ParseError),
        "recursion-limit-exceeded" => Ok(ErrorCode::RecursionLimitExceeded),
        "validation-error" => Ok(ErrorCode::ValidationError),
        "io-error" => Ok(ErrorCode::IoError),
        "malformed-ast-error" => Ok(ErrorCode::MalformedAstError),
        "internal-parse-error" => Ok(ErrorCode::InternalParseError),
        "arity-error" => Ok(ErrorCode::ArityError),
        "type-error" => Ok(ErrorCode::TypeError),
        "division-by-zero" => Ok(ErrorCode::DivisionByZero),
        "eval-error" => Ok(ErrorCode::EvalError),
        _ => Err(format!("Unknown error code: {}", code_str).into()),
    }
}

/// Simple tokenizer for symbolic expressions.
/// Handles nested parentheses correctly.
fn tokenize_symbolic_expression(input: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut tokens = Vec::new();
    let mut current_token = String::new();
    let mut paren_depth = 0;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ' ' | '\t' | '\n' => {
                if paren_depth == 0 && !current_token.is_empty() {
                    tokens.push(current_token.trim().to_string());
                    current_token.clear();
                } else if paren_depth > 0 {
                    current_token.push(ch);
                }
            },
            '(' => {
                if paren_depth > 0 {
                    current_token.push(ch);
                }
                paren_depth += 1;
            },
            ')' => {
                paren_depth -= 1;
                if paren_depth > 0 {
                    current_token.push(ch);
                } else if paren_depth == 0 && !current_token.is_empty() {
                    current_token.push(ch);
                    tokens.push(current_token.trim().to_string());
                    current_token.clear();
                }
            },
            _ => {
                current_token.push(ch);
            }
        }
    }

    if !current_token.trim().is_empty() {
        tokens.push(current_token.trim().to_string());
    }

    Ok(tokens)
}

/// Parses a value assertion from a string representation.
/// Supports basic Sutra value syntax: numbers, booleans, strings, lists.
pub fn parse_expected_value(input: &str) -> Result<sutra::ast::value::Value, Box<dyn std::error::Error>> {
    let input = input.trim();

    // Try to parse as number
    if let Ok(int_val) = input.parse::<i64>() {
        return Ok(sutra::ast::value::Value::Number(int_val as f64));
    }
    if let Ok(float_val) = input.parse::<f64>() {
        return Ok(sutra::ast::value::Value::Number(float_val));
    }

    // Try to parse as boolean
    match input {
        "true" => return Ok(sutra::ast::value::Value::Bool(true)),
        "false" => return Ok(sutra::ast::value::Value::Bool(false)),
        _ => {}
    }

    // Try to parse as string (quoted)
    if input.starts_with('"') && input.ends_with('"') && input.len() >= 2 {
        let string_content = &input[1..input.len()-1];
        return Ok(sutra::ast::value::Value::String(string_content.to_string()));
    }

    // Try to parse as list
    if input.starts_with('(') && input.ends_with(')') {
        let inner = &input[1..input.len()-1].trim();
        if inner.is_empty() {
            return Ok(sutra::ast::value::Value::List(Vec::new()));
        }

        // Simple space-separated parsing for now
        let elements: Result<Vec<_>, _> = inner.split_whitespace()
            .map(parse_expected_value)
            .collect();
        return Ok(sutra::ast::value::Value::List(elements?));
    }

    Err(format!("Cannot parse value: {}", input).into())
}

/// Evaluates a symbolic expression against a list of actual error codes.
/// Returns true if the expression matches the error codes.
pub fn evaluate_symbolic_expression(expression: &SymbolicExpression, actual_codes: &[ErrorCode]) -> bool {
    match expression {
        SymbolicExpression::ErrorCode(expected_code) => {
            actual_codes.contains(expected_code)
        },
        SymbolicExpression::And(sub_expressions) => {
            sub_expressions.iter().all(|expr| evaluate_symbolic_expression(expr, actual_codes))
        },
        SymbolicExpression::Or(sub_expressions) => {
            sub_expressions.iter().any(|expr| evaluate_symbolic_expression(expr, actual_codes))
        },
        SymbolicExpression::Not(sub_expression) => {
            !evaluate_symbolic_expression(sub_expression, actual_codes)
        },
    }
}

// =============================================================================
// TEST DISCOVERY
// =============================================================================

/// Discovers all test files in the given directory tree.
pub fn discover_test_files(
    config: &TestConfig,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut test_files = Vec::new();

    for entry in WalkDir::new(&config.test_root) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext == config.test_extension.as_str() {
                    test_files.push(entry.path().to_path_buf());
                }
            }
        }
    }

    Ok(test_files)
}

/// Loads all test cases from discovered test files.
pub fn load_all_test_cases(
    config: &TestConfig,
) -> Result<Vec<TestCase>, Box<dyn std::error::Error>> {
    let test_files = discover_test_files(config)?;
    let mut all_test_cases = Vec::new();

    for file_path in test_files {
        let test_cases = extract_test_cases(&file_path)?;
        all_test_cases.extend(test_cases);
    }

    Ok(all_test_cases)
}

// =============================================================================
// COMPILATION PIPELINE EXECUTION
// =============================================================================

/// Executes the full Sutra compilation pipeline for a test case.
pub fn execute_pipeline(test_case: &TestCase, _config: &TestConfig) -> ExecutionState {
    let mut state = ExecutionState {
        ast: None,
        expanded: None,
        eval_result: None,
        diagnostics: Vec::new(),
    };

    // Phase 1: Parsing
    match sutra::syntax::parser::parse(&test_case.source) {
        Ok(ast_nodes) => {
            state.ast = Some(ast_nodes.clone());

            // Phase 2: Validation
            let validator_registry = ValidatorRegistry::default();
            for ast_node in &ast_nodes {
                let validation_diagnostics = validator_registry.validate_all(ast_node);
                for diag in validation_diagnostics {
                    state.diagnostics.push(CompilerDiagnostic::Validation(diag));
                }
            }

            // Phase 3: Macro Expansion
            if state.diagnostics.is_empty() {
                match build_canonical_macro_env() {
                    Ok(mut macro_env) => {
                        // For simplicity, expand the first AST node (typically the main program)
                        // In a real scenario, we might need to handle multiple nodes differently
                        if let Some(first_node) = ast_nodes.first() {
                            match expand_macros(first_node.clone(), &mut macro_env) {
                                Ok(expanded_node) => {
                                    state.expanded = Some(expanded_node);
                                }
                                Err(macro_error) => {
                                    state
                                        .diagnostics
                                        .push(CompilerDiagnostic::Eval(macro_error));
                                }
                            }
                        }
                    }
                    Err(env_error) => {
                        state.diagnostics.push(CompilerDiagnostic::Eval(env_error));
                    }
                }
            }

            // Phase 4: Evaluation (if no errors so far)
            if state.diagnostics.is_empty() {
                let atom_registry = build_default_atom_registry();
                let world = World::new();
                let mut output = OutputBuffer::new();
                let max_depth = 1000; // Use config eval_limit or default

                // Use expanded node if available, otherwise use original AST
                let eval_node = if let Some(ref expanded) = state.expanded {
                    expanded
                } else if let Some(first_node) = ast_nodes.first() {
                    first_node
                } else {
                    // No nodes to evaluate
                    return state;
                };

                match eval(eval_node, &world, &mut output, &atom_registry, max_depth) {
                    Ok((value, _final_world)) => {
                        state.eval_result = Some(Ok(value));
                    }
                    Err(eval_error) => {
                        state.diagnostics.push(CompilerDiagnostic::Eval(eval_error));
                    }
                }
            }
        }
        Err(parse_error) => {
            state
                .diagnostics
                .push(CompilerDiagnostic::Parse(parse_error));
        }
    }

    state
}

// =============================================================================
// ARIADNE DIAGNOSTIC RENDERING
// =============================================================================

/// Renders all diagnostics using Ariadne into a unified string output.
pub fn render_diagnostics(
    test_case: &TestCase,
    state: &ExecutionState,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut output = Vec::new();

    for diagnostic in &state.diagnostics {
        render_single_diagnostic(&mut output, test_case, diagnostic)?;
    }

    Ok(String::from_utf8(output)?)
}

/// Renders a single diagnostic using Ariadne.
fn render_single_diagnostic(
    output: &mut Vec<u8>,
    test_case: &TestCase,
    diagnostic: &CompilerDiagnostic,
) -> Result<(), Box<dyn std::error::Error>> {
    let source = Source::from(&test_case.source);
    let filename = test_case.file_path.to_string_lossy();

    match diagnostic {
        CompilerDiagnostic::Parse(error) => {
            let mut report =
                Report::build(ReportKind::Error, &*filename, 0).with_message(error.to_string());

            if let Some(span) = &error.span {
                report = report.with_label(
                    Label::new((&*filename, span.start..span.end))
                        .with_message(format!("{}", error.kind))
                        .with_color(Color::Red),
                );
            }

            report
                .finish()
                .write((&*filename, source.clone()), output)?;
        }
        CompilerDiagnostic::Validation(diag) => {
            let color = match diag.severity {
                sutra::syntax::validator::SutraSeverity::Error => Color::Red,
                sutra::syntax::validator::SutraSeverity::Warning => Color::Yellow,
                sutra::syntax::validator::SutraSeverity::Info => Color::Blue,
            };

            let kind = match diag.severity {
                sutra::syntax::validator::SutraSeverity::Error => ReportKind::Error,
                sutra::syntax::validator::SutraSeverity::Warning => ReportKind::Warning,
                sutra::syntax::validator::SutraSeverity::Info => ReportKind::Advice,
            };

            let report = Report::build(kind, &*filename, 0)
                .with_message(&diag.message)
                .with_label(
                    Label::new((&*filename, diag.span.start..diag.span.end))
                        .with_message(&diag.message)
                        .with_color(color),
                )
                .finish();

            report.write((&*filename, source.clone()), output)?;
        }
        CompilerDiagnostic::Eval(error) => {
            let mut report =
                Report::build(ReportKind::Error, &*filename, 0).with_message(error.to_string());

            if let Some(span) = &error.span {
                report = report.with_label(
                    Label::new((&*filename, span.start..span.end))
                        .with_message(format!("{}", error.kind))
                        .with_color(Color::Red),
                );
            }

            report
                .finish()
                .write((&*filename, source.clone()), output)?;
        }
    }

    Ok(())
}

// =============================================================================
// SNAPSHOT TESTING
// =============================================================================

/// Compares actual diagnostic output against expected snapshot.
pub fn assert_diagnostic_snapshot(
    test_case: &TestCase,
    actual_output: &str,
    execution_state: &ExecutionState,
    config: &TestConfig,
) -> TestResult {
    match &test_case.expectation {
        TestExpectation::Success => {
            if actual_output.trim().is_empty() {
                TestResult::Pass {
                    id: test_case.id.clone(),
                    name: test_case.name.clone(),
                }
            } else {
                TestResult::UnexpectedSuccess {
                    id: test_case.id.clone(),
                    name: test_case.name.clone(),
                    expected_error_type: "no errors".to_string(),
                }
            }
        }
        TestExpectation::DiagnosticSnapshot { snapshot_path } => {
            let snapshot_file = test_case
                .file_path
                .parent()
                .unwrap_or(Path::new("."))
                .join(snapshot_path);

            match fs::read_to_string(&snapshot_file) {
                Ok(expected) => {
                    if actual_output.trim() == expected.trim() {
                        TestResult::Pass {
                            id: test_case.id.clone(),
                            name: test_case.name.clone(),
                        }
                    } else {
                        if config.update_snapshots {
                            if let Err(e) = fs::write(&snapshot_file, actual_output) {
                                eprintln!(
                                    "Failed to update snapshot {}: {}",
                                    snapshot_file.display(),
                                    e
                                );
                            }
                            TestResult::Pass {
                                id: test_case.id.clone(),
                                name: test_case.name.clone(),
                            }
                        } else {
                            TestResult::DiagnosticMismatch {
                                id: test_case.id.clone(),
                                name: test_case.name.clone(),
                                expected,
                                actual: actual_output.to_string(),
                            }
                        }
                    }
                }
                Err(_) => {
                    // Snapshot file doesn't exist - create it if update mode is enabled
                    if config.update_snapshots {
                        if let Err(e) = fs::write(&snapshot_file, actual_output) {
                            eprintln!(
                                "Failed to create snapshot {}: {}",
                                snapshot_file.display(),
                                e
                            );
                        }
                        TestResult::Pass {
                            id: test_case.id.clone(),
                            name: test_case.name.clone(),
                        }
                    } else {
                        TestResult::DiagnosticMismatch {
                            id: test_case.id.clone(),
                            name: test_case.name.clone(),
                            expected: "".to_string(),
                            actual: actual_output.to_string(),
                        }
                    }
                }
            }
        }
        TestExpectation::ParseError { codes, messages } => {
            validate_error_expectation(test_case, actual_output, "parse error", codes, messages)
        }
        TestExpectation::ValidationError { codes, messages } => validate_error_expectation(
            test_case,
            actual_output,
            "validation error",
            codes,
            messages,
        ),
        TestExpectation::EvalError { codes, messages } => {
            validate_error_expectation(test_case, actual_output, "eval error", codes, messages)
        }
        TestExpectation::SymbolicError { expression } => {
            if actual_output.trim().is_empty() {
                TestResult::UnexpectedSuccess {
                    id: test_case.id.clone(),
                    name: test_case.name.clone(),
                    expected_error_type: "symbolic error".to_string(),
                }
            } else {
                // Extract actual error codes from diagnostics
                let actual_error_codes: Vec<ErrorCode> = execution_state.diagnostics.iter()
                    .filter_map(|diag| {
                        match diag {
                            CompilerDiagnostic::Parse(error) => error.error_code(),
                            CompilerDiagnostic::Eval(error) => error.error_code(),
                            CompilerDiagnostic::Validation(_) => Some(ErrorCode::ValidationError),
                        }
                    })
                    .collect();

                // Evaluate symbolic expression against actual error codes
                let expression_matches = evaluate_symbolic_expression(expression, &actual_error_codes);

                if expression_matches {
                    TestResult::Pass {
                        id: test_case.id.clone(),
                        name: test_case.name.clone(),
                    }
                } else {
                    TestResult::WrongErrorType {
                        id: test_case.id.clone(),
                        name: test_case.name.clone(),
                        expected: format!("error matching symbolic expression: {:?}", expression),
                        actual: format!("error codes: {:?}", actual_error_codes),
                    }
                }
            }
        }
        TestExpectation::Value { expected_value } => {
            if !actual_output.trim().is_empty() {
                TestResult::UnexpectedSuccess {
                    id: test_case.id.clone(),
                    name: test_case.name.clone(),
                    expected_error_type: "successful evaluation".to_string(),
                }
            } else {
                // Check the actual evaluation result
                match &execution_state.eval_result {
                    Some(Ok(actual_value)) => {
                        if actual_value == expected_value {
                            TestResult::Pass {
                                id: test_case.id.clone(),
                                name: test_case.name.clone(),
                            }
                        } else {
                            TestResult::DiagnosticMismatch {
                                id: test_case.id.clone(),
                                name: test_case.name.clone(),
                                expected: format!("Value: {:?}", expected_value),
                                actual: format!("Value: {:?}", actual_value),
                            }
                        }
                    },
                    Some(Err(_)) => {
                        TestResult::UnexpectedSuccess {
                            id: test_case.id.clone(),
                            name: test_case.name.clone(),
                            expected_error_type: "successful evaluation".to_string(),
                        }
                    },
                    None => {
                        TestResult::DiagnosticMismatch {
                            id: test_case.id.clone(),
                            name: test_case.name.clone(),
                            expected: format!("Value: {:?}", expected_value),
                            actual: "No evaluation result".to_string(),
                        }
                    }
                }
            }
        }
    }
}

/// Validates that diagnostic output contains expected error codes and messages.
fn validate_error_expectation(
    test_case: &TestCase,
    actual_output: &str,
    error_type: &str,
    expected_codes: &[String],
    expected_messages: &[String],
) -> TestResult {
    if actual_output.trim().is_empty() {
        return TestResult::UnexpectedSuccess {
            id: test_case.id.clone(),
            name: test_case.name.clone(),
            expected_error_type: error_type.to_string(),
        };
    }

    // Check for expected codes
    for code in expected_codes {
        if !actual_output.contains(code) {
            return TestResult::WrongErrorType {
                id: test_case.id.clone(),
                name: test_case.name.clone(),
                expected: format!("error containing code '{}'", code),
                actual: actual_output.to_string(),
            };
        }
    }

    // Check for expected messages
    for message in expected_messages {
        if !actual_output.contains(message) {
            return TestResult::WrongErrorType {
                id: test_case.id.clone(),
                name: test_case.name.clone(),
                expected: format!("error containing message '{}'", message),
                actual: actual_output.to_string(),
            };
        }
    }

    TestResult::Pass {
        id: test_case.id.clone(),
        name: test_case.name.clone(),
    }
}

// =============================================================================
// MAIN TEST EXECUTION
// =============================================================================

/// Executes a single test case through the complete pipeline.
pub fn run_test_case(test_case: &TestCase, config: &TestConfig) -> TestResult {
    // Check for skip conditions
    if test_case.skip {
        return TestResult::Skipped {
            id: test_case.id.clone(),
            name: test_case.name.clone(),
            reason: "Marked as skip".to_string(),
        };
    }

    // Execute compilation pipeline
    let execution_state = execute_pipeline(test_case, config);

    // Render diagnostics using Ariadne
    let diagnostic_output = match render_diagnostics(test_case, &execution_state) {
        Ok(output) => output,
        Err(e) => {
            return TestResult::DiagnosticMismatch {
                id: test_case.id.clone(),
                name: test_case.name.clone(),
                expected: "valid diagnostic output".to_string(),
                actual: format!("Failed to render diagnostics: {}", e),
            };
        }
    };

    // Assert against expectations
    assert_diagnostic_snapshot(test_case, &diagnostic_output, &execution_state, config)
}

/// Runs all test cases with filtering and reporting.
pub fn run_all_tests(
    config: &TestConfig,
    filter: Option<&str>,
) -> Result<(usize, usize, usize), Box<dyn std::error::Error>> {
    let test_cases = load_all_test_cases(config)?;

    // Apply filtering
    let filtered_cases: Vec<_> = test_cases
        .into_iter()
        .filter(|case| {
            if let Some(filter_str) = filter {
                case.name.contains(filter_str) || case.id.contains(filter_str)
            } else {
                true
            }
        })
        .collect();

    // Check for "only" tests
    let has_only = filtered_cases.iter().any(|case| case.only);
    let final_cases: Vec<_> = if has_only {
        filtered_cases
            .into_iter()
            .filter(|case| case.only)
            .collect()
    } else {
        filtered_cases
    };

    // Execute tests
    let mut results = Vec::new();
    for test_case in final_cases {
        let result = run_test_case(&test_case, config);
        results.push(result);
    }

    // Report results
    report_test_results(&results, config);

    // Calculate summary
    let passed = results
        .iter()
        .filter(|r| matches!(r, TestResult::Pass { .. }))
        .count();
    let failed = results
        .iter()
        .filter(|r| {
            matches!(
                r,
                TestResult::DiagnosticMismatch { .. }
                    | TestResult::UnexpectedSuccess { .. }
                    | TestResult::WrongErrorType { .. }
            )
        })
        .count();
    let skipped = results
        .iter()
        .filter(|r| matches!(r, TestResult::Skipped { .. }))
        .count();

    Ok((passed, failed, skipped))
}

// =============================================================================
// REPORTING
// =============================================================================

/// Reports test results with colored output.
pub fn report_test_results(results: &[TestResult], config: &TestConfig) {
    for result in results {
        match result {
            TestResult::Pass { name, .. } => {
                println!("✓ {}: {}", colorize("PASS", "32", config), name);
            }
            TestResult::DiagnosticMismatch {
                name,
                expected,
                actual,
                ..
            } => {
                println!("✗ {}: {}", colorize("FAIL", "31", config), name);
                println!("  Expected diagnostic output:");
                for line in expected.lines() {
                    println!("    {}", colorize(line, "32", config));
                }
                println!("  Actual diagnostic output:");
                for line in actual.lines() {
                    println!("    {}", colorize(line, "31", config));
                }
            }
            TestResult::UnexpectedSuccess {
                name,
                expected_error_type,
                ..
            } => {
                println!(
                    "✗ {}: {} - Expected {} but test succeeded",
                    colorize("FAIL", "31", config),
                    name,
                    expected_error_type
                );
            }
            TestResult::WrongErrorType {
                name,
                expected,
                actual,
                ..
            } => {
                println!(
                    "✗ {}: {} - Expected {} but got:",
                    colorize("FAIL", "31", config),
                    name,
                    expected
                );
                for line in actual.lines() {
                    println!("    {}", colorize(line, "31", config));
                }
            }
            TestResult::Skipped { name, reason, .. } => {
                println!(
                    "- {}: {} ({})",
                    colorize("SKIP", "33", config),
                    name,
                    reason
                );
            }
        }
    }
}

/// Helper for colored output.
fn colorize(text: &str, color_code: &str, config: &TestConfig) -> String {
    if config.use_colors {
        format!("\x1b[{}m{}\x1b[0m", color_code, text)
    } else {
        text.to_string()
    }
}

// =============================================================================
// PUBLIC API
// =============================================================================

/// Main entry point for running tests with default configuration.
pub fn run_default_tests(filter: Option<&str>) -> (usize, usize, usize) {
    let config = TestConfig::default();
    match run_all_tests(&config, filter) {
        Ok(summary) => summary,
        Err(e) => {
            eprintln!("Error running tests: {}", e);
            (0, 1, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_parse_test_annotation() {
        let cases = vec![
            (
                "//! @test \"simple test\"",
                Some(("test".to_string(), "\"simple test\"".to_string())),
            ),
            (
                "//! @expect success",
                Some(("expect".to_string(), "success".to_string())),
            ),
            ("//! @skip", Some(("skip".to_string(), "true".to_string()))),
            ("// regular comment", None),
            ("//! regular doc comment", None),
        ];

        for (input, expected) in cases {
            let result = parse_test_annotation(input).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }

    #[test]
    fn test_parse_expectation() {
        let mut annotations = HashMap::new();
        annotations.insert("expect".to_string(), "success".to_string());

        let expectation = parse_expectation(&annotations).unwrap();
        assert!(matches!(expectation, TestExpectation::Success));
    }
}
