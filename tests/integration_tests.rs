mod common;

use miette::Report;
use std::path::Path;
use sutra::SutraError;
use sutra::sutra_err;
use sutra::cli::output::OutputBuffer;
use sutra::engine::run_sutra_source_with_output;

#[test]
fn run_sutra_tests() -> Result<(), SutraError> {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let test_cases = common::load_test_cases(&test_dir)?;

    for test_case in test_cases {
        let source = test_case.body.value.pretty();
        let mut sink = OutputBuffer::new();
        let result = run_sutra_source_with_output(&source, &mut sink);

        match (result, &test_case.expectation) {
            (Ok(_), common::TestExpectation::Success(expected)) => {
                let actual = sink.as_str();
                let expected_str = expected.to_string();
                if actual.trim() != expected_str.trim() {
                    return Err(sutra_err!(Validation, format!("Test '{}' failed: expected output '{}', got '{}'", test_case.name, expected_str, actual)));
                }
            }
            (Err(e), common::TestExpectation::Error(expected)) => {
                let report = Report::new(e);
                let diag_str = format!("{report:?}");
                if !diag_str.contains(expected.as_str()) {
                    return Err(sutra_err!(Validation, format!("Test '{}' failed: error message did not contain expected string '{}'. Diagnostic: {}", test_case.name, expected, diag_str)));
                }
            }
            (Ok(_), common::TestExpectation::Error(expected)) => {
                return Err(sutra_err!(Validation, format!("Test '{}' succeeded but was expected to fail with: {}", test_case.name, expected)));
            }
            (Err(e), common::TestExpectation::Success(_)) => {
                return Err(sutra_err!(Validation, format!("Test '{}' failed but was expected to succeed. Error: {:?}", test_case.name, Report::new(e))));
            }
        }
    }
    Ok(())
}
