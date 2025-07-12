// Sutra test harness: discovers, loads, filters, runs, and reports YAML-based tests in a flat, minimal, functional style.
// Usage: cargo run --bin harness [substring]
// This harness is standalone and also integrated with cargo test.

// =========================
// 1. Imports
// =========================
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use sutra::syntax::parser;
use sutra::macros::expand_macros;
use sutra::runtime::registry::build_default_atom_registry;
use sutra::runtime::eval::eval;
use sutra::runtime::world::World;
use sutra::cli::output::OutputBuffer;
use std::env;
use atty;

// =========================
// 2. Constants
// =========================
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const YELLOW: &str = "\x1b[33m";
const RESET: &str = "\x1b[0m";

// =========================
// 3. Type Definitions
// =========================
/// Represents a single YAML test case for the Sutra test harness.
/// Fields:
/// - name: test name
/// - style: syntax style (list/block)
/// - input: test input string
/// - expected: expected output (optional)
/// - expect_error: expected error message (optional)
/// - skip: if true, skip this test (default: false)
/// - only: if true, run only this test (default: false)
#[derive(Debug, Deserialize)]
struct TestCase {
    name: String,
    #[allow(dead_code)]
    style: String,
    input: String,
    expected: Option<String>,
    expect_error: Option<String>,
    #[serde(default)]
    skip: Option<bool>,
    #[serde(default)]
    only: Option<bool>,
}

enum TestResult {
    Pass { file: String, name: String },
    Fail {
        file: String,
        name: String,
        error: String,
        expanded: Option<String>,
        eval: Option<String>,
    },
    Skipped { file: String, name: String, reason: String },
}

// =========================
// 4. Utility Functions
// =========================
/// Discovers all YAML files recursively under the given root directory.
fn discover_yaml_files<P: AsRef<Path>>(root: P) -> Vec<PathBuf> {
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

/// Loads all test cases from a YAML file at the given path.
fn load_test_cases(path: &Path) -> Vec<TestCase> {
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

fn is_stderr_tty() -> bool {
    atty::is(atty::Stream::Stderr)
}

fn print_diff(expected: &str, actual: &str) {
    let expected_lines: Vec<_> = expected.lines().collect();
    let actual_lines: Vec<_> = actual.lines().collect();
    let max = expected_lines.len().max(actual_lines.len());
    for i in 0..max {
        let exp = expected_lines.get(i).unwrap_or(&"");
        let act = actual_lines.get(i).unwrap_or(&"");
        if exp != act {
            eprintln!("  - expected: {}{}{}", GREEN, exp, RESET);
            eprintln!("  + actual:   {}{}{}", RED, act, RESET);
        } else {
            eprintln!("    {}", exp);
        }
    }
}

// =========================
// 5. Core Test Logic
// =========================
/// Compares the expected and actual output, returning true if they match.
fn compare_result(expected: Option<&str>, actual_output: &str, expect_error: Option<&str>) -> bool {
    if let Some(expected_error) = expect_error {
        // If an error is expected, check if the actual output contains the error message
        actual_output.contains(expected_error)
    } else if let Some(expected_output) = expected {
        // If a specific output is expected, compare trimmed strings
        actual_output.trim() == expected_output.trim()
    } else {
        // No expected output or error, so any non-error output is a pass
        true
    }
}

/// Runs a single test case, returning a TestResult with detailed error info on failure.
fn run_test_case(file: String, case: TestCase) -> TestResult {
    // Build macro env and atom registry once per test case
    let mut current_world = World::default();
    let atom_registry = build_default_atom_registry();
    let mut output_sink = OutputBuffer::default();
    let mut last_eval_value: Option<String> = None;
    let mut last_expanded_str: Option<String> = None;
    // Parse input
    let ast_nodes = match parser::parse(&case.input) {
        Ok(nodes) => nodes,
        Err(e) => {
            let error_msg = format!("Parse error: {}", e);
            if let Some(expected_error) = case.expect_error.as_deref() {
                if error_msg.contains(expected_error) {
                    return TestResult::Pass { file, name: case.name };
                }
            }
            return TestResult::Fail {
                file,
                name: case.name,
                error: error_msg,
                expanded: None,
                eval: None,
            };
        }
    };
    for node in ast_nodes {
        // Expand macros
        let expanded = match expand_macros(node.clone(), &mut current_world.macros) {
            Ok(expanded) => expanded,
            Err(e) => {
                let error_msg = format!("Macro expansion error: {}", e);
                if let Some(expected_error) = case.expect_error.as_deref() {
                    if error_msg.contains(expected_error) {
                        return TestResult::Pass { file, name: case.name };
                    }
                }
                return TestResult::Fail {
                    file,
                    name: case.name,
                    error: error_msg,
                    expanded: last_expanded_str,
                    eval: last_eval_value,
                };
            }
        };
        last_expanded_str = Some(expanded.value.pretty());

        // Eval
        let _eval_result = match eval(&expanded, &mut current_world, &mut output_sink, &atom_registry, 1000) {
            Ok(val) => {
                last_eval_value = Some(format!("{:?}", val));
                // If an error was expected, but eval succeeded, this is a failure
                if let Some(expected_error) = case.expect_error.as_deref() {
                    return TestResult::Fail {
                        file,
                        name: case.name,
                        error: format!("Expected error '{}' but evaluation succeeded with result: {:?}", expected_error, val),
                        expanded: last_expanded_str,
                        eval: last_eval_value,
                    };
                }
                Some(format!("{:?}", val))
            },
            Err(e) => {
                let error_msg = format!("Eval error: {}", e);
                if let Some(expected_error) = case.expect_error.as_deref() {
                    if error_msg.contains(expected_error) {
                        return TestResult::Pass { file, name: case.name };
                    }
                }
                return TestResult::Fail {
                    file,
                    name: case.name,
                    error: error_msg,
                    expanded: last_expanded_str,
                    eval: last_eval_value,
                };
            }
        };
    }


    // Compare result
    let actual_output = output_sink.as_str();
    let passed = compare_result(case.expected.as_deref(), actual_output, case.expect_error.as_deref());

    if passed {
        TestResult::Pass { file, name: case.name }
    } else {
        let expected_output = case.expected.as_deref().unwrap_or("").trim();
        let actual_output_trimmed = actual_output.trim();
        TestResult::Fail {
            file,
            name: case.name,
            error: format!("Output did not match expected\n  Expected: {}\n  Actual:   {}", expected_output, actual_output_trimmed),
            expanded: last_expanded_str,
            eval: last_eval_value,
        }
    }
}

// =========================
// 6. Reporting/Output Functions
// =========================
/// Prints a detailed failure report to stderr for a failed test result, with color if stderr is a tty.
fn print_failure(r: &TestResult) {
    match r {
        TestResult::Fail { file, name, error, expanded, eval } => {
            let color = if is_stderr_tty() { RED } else { "" };
            let reset = if is_stderr_tty() { RESET } else { "" };
            eprintln!("{color}FAIL{reset}: {} [{}]", name, file, color=color, reset=reset);
            eprintln!("  Error: {}", error);
            if let Some(expanded) = expanded {
                eprintln!("  Expanded: {}", expanded);
            }
            if let Some(eval) = eval {
                eprintln!("  Eval: {}", eval);
            }
            // If error is output mismatch, print diff
            if error.starts_with("Output did not match expected") {
                let lines: Vec<_> = error.lines().collect();
                if lines.len() >= 3 {
                    let expected = lines[1].trim_start_matches("Expected: ").trim();
                    let actual = lines[2].trim_start_matches("Actual: ").trim();
                    eprintln!("  Diff:");
                    print_diff(expected, actual);
                }
            }
        }
        _ => {}
    }
}

/// Reports all test results, printing summary statistics and details for failures and skips.
fn report_results(results: &[TestResult]) {
    let (passed, rest): (Vec<_>, Vec<_>) = results.iter().partition(|r| matches!(r, TestResult::Pass { .. }));
    let (failed, skipped): (Vec<_>, Vec<_>) = rest.into_iter().partition(|r| matches!(r, TestResult::Fail { .. }));
    let total = results.len();
    let passed_count = passed.len();
    let failed_count = failed.len();
    let skipped_count = skipped.len();
    let mut failed_names = Vec::new();

    for r in results {
        match r {
            TestResult::Pass { file, name } => println!("{GREEN}PASS{RESET}: {} [{}]", name, file),
            TestResult::Fail { name, .. } => {
                print_failure(r);
                failed_names.push(name.clone());
            },
            TestResult::Skipped { file, name, reason } => println!("{YELLOW}SKIP{RESET}: {} [{}] ({})", name, file, reason),
        }
    }
    println!(
        "\nTest summary: total {}, {GREEN}passed{RESET} {}, {RED}failed{RESET} {}, {YELLOW}skipped{RESET} {}",
        total, passed_count, failed_count, skipped_count,
        GREEN=GREEN, RED=RED, YELLOW=YELLOW, RESET=RESET
    );
    if !failed_names.is_empty() {
        eprintln!("\nFailed tests:");
        for name in failed_names {
            eprintln!("  - {}", name);
        }
    }
}

// =========================
// 7. Entrypoints
// =========================
/// Runs all tests and returns (passed, failed, skipped) counts. Used for both CLI and cargo test integration.
pub fn run_all_tests(filter: Option<&str>) -> (usize, usize, usize) {
    let test_root = "tests";
    let yaml_files = discover_yaml_files(test_root);
    // Load and flatten all test cases
    let all_cases: Vec<(String, TestCase)> = yaml_files.iter()
        .flat_map(|file| {
            load_test_cases(file)
                .into_iter()
                .map(move |case| (file.display().to_string(), case))
        })
        .collect();
    let only_mode = all_cases.iter().any(|(_, c)| c.only.unwrap_or(false));
    let results: Vec<TestResult> = all_cases.into_iter()
        .map(|(file, case)| {
            if only_mode && !case.only.unwrap_or(false) {
                return TestResult::Skipped { file, name: case.name, reason: "Not marked 'only' in 'only' mode".to_string() };
            }
            if case.skip.unwrap_or(false) {
                return TestResult::Skipped { file, name: case.name, reason: "Marked 'skip'".to_string() };
            }
            if let Some(f) = filter {
                if !case.name.to_lowercase().contains(f) {
                    return TestResult::Skipped { file, name: case.name, reason: format!("Filtered out by substring: {}", f) };
                }
            }
            run_test_case(file, case)
        })
        .collect();
    report_results(&results);
    let passed = results.iter().filter(|r| matches!(r, TestResult::Pass { .. })).count();
    let failed = results.iter().filter(|r| matches!(r, TestResult::Fail { .. })).count();
    let skipped = results.iter().filter(|r| matches!(r, TestResult::Skipped { .. })).count();
    (passed, failed, skipped)
}

/// Main entrypoint for the Sutra test harness.
/// Discovers, loads, filters, runs, and reports YAML-based tests.
fn main() {
    let args: Vec<String> = env::args().collect();
    let filter = if args.len() > 1 { Some(args[1].to_lowercase()) } else { None };
    let (_passed, failed, _skipped) = run_all_tests(filter.as_deref());
    if failed > 0 {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn harness_runs_without_failures() {
        let (_passed, failed, _skipped) = run_all_tests(None);
        assert_eq!(failed, 0, "Test harness failures detected");
    }
}