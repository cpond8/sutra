// Regression test: Ensure CLI errors are rendered with miette diagnostics
// Requires: assert_cmd, predicates crates in [dev-dependencies]

use std::fs;

use assert_cmd::Command;
use predicates::{prelude::PredicateBooleanExt, str::contains};

#[test]
fn cli_reports_miette_diagnostics_on_error() {
    // Create a temporary invalid Sutra file
    let bad_file = "tests/bad_script.sutra";
    fs::write(bad_file, "(define x 42" /* missing closing paren */).unwrap();

    let mut cmd = Command::cargo_bin("sutra").unwrap();
    cmd.arg("run").arg(bad_file);
    cmd.assert().failure().stderr(
        contains("sutra::parse")
            .or(contains("sutra::validation"))
            .or(contains("help:")),
    );

    // Clean up
    let _ = fs::remove_file(bad_file);
}
