//! Golden master tests for diagnostic output.
//!
//! These tests capture the exact formatted output of `SutraDiagnostic`
//! to ensure consistent error presentation across changes.

use sutra::ast::Span;
use sutra::cli::diagnostics::SutraDiagnostic;
use sutra::syntax::error::{eval_arity_error, io_error, parse_error, validation_error};

/// Test helper to capture diagnostic output as a string.
fn capture_diagnostic_output(
    error: &sutra::syntax::error::SutraError,
    source: Option<&str>,
) -> String {
    let diagnostic = SutraDiagnostic::new(error, source);
    format!("{}", diagnostic)
}

#[test]
fn test_parse_error_diagnostic() {
    let span = Span { start: 5, end: 10 };
    let error = parse_error("Unexpected token ')'", Some(span));
    let source = "hello world) test";

    let output = capture_diagnostic_output(&error, Some(source));

    // Golden master snapshot
    let expected = "Error [at line 1, col 6 to line 1, col 11]:
Parse Error: Unexpected token ')'

1 | hello world) test
  |      ^----- Here
";

    assert_eq!(output, expected);
}

#[test]
fn test_parse_error_without_source() {
    let span = Span { start: 5, end: 10 };
    let error = parse_error("Unexpected token ')'", Some(span));

    let output = capture_diagnostic_output(&error, None);

    // Golden master snapshot
    let expected = "Error [at 5-10]:
Parse Error: Unexpected token ')'
";

    assert_eq!(output, expected);
}

#[test]
fn test_validation_error_diagnostic() {
    let span = Span { start: 15, end: 25 };
    let error = validation_error("Invalid macro usage", Some(span));
    let source = "some code here (invalid-macro) more code";

    let output = capture_diagnostic_output(&error, Some(source));

    // Golden master snapshot
    let expected = "Error [at line 1, col 16 to line 1, col 26]:
Validation Error: Invalid macro usage

1 | some code here (invalid-macro) more code
  |                ^---------- Here
";

    assert_eq!(output, expected);
}

#[test]
fn test_io_error_diagnostic() {
    let error = io_error("Failed to read file", None);

    let output = capture_diagnostic_output(&error, None);

    // Golden master snapshot
    let expected = "Error:
IO Error: Failed to read file
";

    assert_eq!(output, expected);
}

#[test]
fn test_multiline_error_diagnostic() {
    let span = Span { start: 12, end: 20 };
    let error = parse_error("Syntax error in expression", Some(span));
    let source = "line 1\nline 2\nbad syntax here\nline 4\nline 5";

    let output = capture_diagnostic_output(&error, Some(source));

    // Golden master snapshot - should show context lines around the error
    let expected = "Error [at line 3, col 1 to line 3, col 9]:
Parse Error: Syntax error in expression

1 | line 1
2 | line 2
3 | bad syntax here
  | ^-------- Here
4 | line 4
5 | line 5
";

    assert_eq!(output, expected);
}

#[test]
fn test_eval_arity_error_diagnostic() {
    use std::sync::Arc;
    use sutra::ast::{Expr, WithSpan};

    let span = Span { start: 0, end: 10 };
    let args: Vec<WithSpan<Arc<Expr>>> = vec![];
    let error = eval_arity_error(Some(span), &args, "core/set!", "exactly 2");
    let source = "(core/set!)";

    let output = capture_diagnostic_output(&error, Some(source));

    // Golden master snapshot
    let expected = "Error [at line 1, col 1 to line 1, col 11]:
Evaluation error: Arity error in core/set!: expected exactly 2, got 0

1 | (core/set!)
  | ^---------- Here
";

    assert_eq!(output, expected);
}
