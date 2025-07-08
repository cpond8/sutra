#![allow(dead_code)]
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Shared utilities for file-based testing in the Sutra Engine test suite.
///
/// This module provides a protocol-compliant test runner that executes .sutra
/// scripts and compares their output to corresponding .expected files.
/// All tests should use this approach for consistency and maintainability.

/// Result type for test operations
pub type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Configuration for test execution
#[derive(Debug, Clone)]
pub struct TestConfig {
    /// Path to the sutra binary to execute
    pub binary_path: PathBuf,
    /// Whether to normalize whitespace in comparisons
    pub normalize_whitespace: bool,
    /// Whether to show diff output on failures
    pub show_diff: bool,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            binary_path: PathBuf::from("./target/debug/sutra"),
            normalize_whitespace: true,
            show_diff: true,
        }
    }
}

/// Represents a single .sutra/.expected test pair
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub sutra_file: PathBuf,
    pub expected_file: PathBuf,
}

impl TestCase {
    /// Create a new test case from a .sutra file path
    /// Will automatically derive the .expected file path
    pub fn from_sutra_file<P: AsRef<Path>>(
        sutra_file: P,
    ) -> Result<TestCase, Box<dyn std::error::Error>> {
        let sutra_path = sutra_file.as_ref();
        let expected_path = sutra_path.with_extension("expected");

        if !sutra_path.exists() {
            return Err(format!("Sutra file does not exist: {}", sutra_path.display()).into());
        }

        if !expected_path.exists() {
            return Err(
                format!("Expected file does not exist: {}", expected_path.display()).into(),
            );
        }

        let name = sutra_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        Ok(TestCase {
            name,
            sutra_file: sutra_path.to_path_buf(),
            expected_file: expected_path,
        })
    }
}

/// Execute a single test case and return the result
pub fn run_test_case(test_case: &TestCase, config: &TestConfig) -> TestResult {
    // Execute the sutra script
    let output = Command::new(&config.binary_path)
        .arg("run")
        .arg(&test_case.sutra_file)
        .output()?;

    // Read the expected output
    let expected_content = fs::read_to_string(&test_case.expected_file)?;

    // Get actual output (stdout + stderr)
    let actual_output = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Normalize whitespace if configured
    let (actual, expected) = if config.normalize_whitespace {
        (
            normalize_whitespace(&actual_output),
            normalize_whitespace(&expected_content),
        )
    } else {
        (actual_output, expected_content)
    };

    // Compare outputs
    if actual.trim() == expected.trim() {
        Ok(())
    } else {
        let error_msg = if config.show_diff {
            format!(
                "Test '{}' failed:\n\nExpected:\n{}\n\nActual:\n{}\n\nDiff:\n{}",
                test_case.name,
                expected.trim(),
                actual.trim(),
                create_diff(&expected, &actual)
            )
        } else {
            format!("Test '{}' failed: output mismatch", test_case.name)
        };
        Err(error_msg.into())
    }
}

/// Discover all .sutra/.expected test pairs in a directory
pub fn discover_test_cases<P: AsRef<Path>>(
    dir: P,
) -> Result<Vec<TestCase>, Box<dyn std::error::Error>> {
    let mut test_cases = Vec::new();

    fn scan_directory(dir: &Path, test_cases: &mut Vec<TestCase>) -> TestResult {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                scan_directory(&path, test_cases)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("sutra") {
                // Found a .sutra file, check if it has a corresponding .expected file
                if let Ok(test_case) = TestCase::from_sutra_file(&path) {
                    test_cases.push(test_case);
                }
            }
        }
        Ok(())
    }

    scan_directory(dir.as_ref(), &mut test_cases)?;
    Ok(test_cases)
}

/// Run all test cases in a directory
pub fn run_test_directory<P: AsRef<Path>>(dir: P, config: &TestConfig) -> TestResult {
    let test_cases = discover_test_cases(dir)?;

    if test_cases.is_empty() {
        println!("No test cases found");
        return Ok(());
    }

    let mut passed = 0;
    let mut failed = 0;

    for test_case in &test_cases {
        print!("Running test '{}'... ", test_case.name);
        match run_test_case(test_case, config) {
            Ok(()) => {
                println!("✅ PASS");
                passed += 1;
            }
            Err(e) => {
                println!("❌ FAIL");
                eprintln!("{}", e);
                failed += 1;
            }
        }
    }

    println!(
        "\nTest Results: {} passed, {} failed, {} total",
        passed,
        failed,
        test_cases.len()
    );

    if failed > 0 {
        Err(format!("{} test(s) failed", failed).into())
    } else {
        Ok(())
    }
}

/// Normalize whitespace for consistent comparisons
fn normalize_whitespace(s: &str) -> String {
    s.lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Create a simple diff between two strings
fn create_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();

    let mut diff = String::new();
    let max_lines = expected_lines.len().max(actual_lines.len());

    for i in 0..max_lines {
        let expected_line = expected_lines.get(i).unwrap_or(&"");
        let actual_line = actual_lines.get(i).unwrap_or(&"");

        if expected_line != actual_line {
            diff.push_str(&format!("Line {}: \n", i + 1));
            diff.push_str(&format!("  - {}\n", expected_line));
            diff.push_str(&format!("  + {}\n", actual_line));
        }
    }

    if diff.is_empty() {
        "No line-by-line differences found (possible whitespace/formatting issue)".to_string()
    } else {
        diff
    }
}

/// Generate expected output for a .sutra file by running it and saving the output
pub fn generate_expected_output<P: AsRef<Path>>(sutra_file: P, config: &TestConfig) -> TestResult {
    let sutra_path = sutra_file.as_ref();
    let expected_path = sutra_path.with_extension("expected");

    // Execute the sutra script
    let output = Command::new(&config.binary_path)
        .arg("run")
        .arg(sutra_path)
        .output()?;

    // Get output (prefer stdout, fall back to stderr for errors)
    let content = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).to_string()
    } else {
        String::from_utf8_lossy(&output.stdout).to_string()
    };

    // Write to .expected file
    fs::write(&expected_path, content.trim())?;

    println!("Generated expected output: {}", expected_path.display());
    Ok(())
}
