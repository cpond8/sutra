// Sutra Integration Test Runner
// Discovers all .sutra files in tests/scripts/, runs them, and compares output to .expected files.
// Reports pass/fail with colorized diagnostics. Integrates with cargo test.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};
use walkdir::WalkDir;

// Import the Sutra engine public API (adjust as needed)
use sutra::cli::output::OutputBuffer;
use sutra::run_sutra_source_with_output; // Assumed public API; adjust if needed

fn find_test_scripts(dir: &str) -> Vec<(PathBuf, PathBuf)> {
    let mut tests = Vec::new();
    for entry in WalkDir::new(dir) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_file() && path.extension().map(|e| e == "sutra").unwrap_or(false) {
            let expected = path.with_extension("expected");
            if expected.exists() {
                tests.push((path.to_path_buf(), expected));
            }
        }
    }
    tests
}

fn read_file_trimmed(path: &Path) -> io::Result<String> {
    Ok(fs::read_to_string(path)?
        .replace("\r\n", "\n")
        .trim()
        .to_string())
}

#[test]
fn integration_scripts() {
    let scripts = find_test_scripts("tests/scripts");
    assert!(
        !scripts.is_empty(),
        "No .sutra test scripts found in tests/scripts/"
    );

    let mut failed = false;
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    for (script, expected) in scripts {
        let script_name = script.file_name().unwrap().to_string_lossy();
        let script_src = read_file_trimmed(&script).expect("Failed to read script");
        let expected_output = read_file_trimmed(&expected).expect("Failed to read expected output");

        // Run the script using the Sutra engine public API with OutputBuffer
        let mut output_buf = OutputBuffer::new();
        let result =
            run_sutra_source_with_output(&script_src, &mut output_buf);
        let actual_output = output_buf.as_str().replace("\r\n", "\n").trim().to_string();

        let pass = match result {
            Ok(_) => actual_output == expected_output,
            Err(e) => {
                // If expected output is an error message, compare to error string
                let err_str = format!("{e}").replace("\r\n", "\n").trim().to_string();
                err_str == expected_output
            }
        };

        if pass {
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true));
            let _ = writeln!(stdout, "PASS: {script_name}");
        } else {
            failed = true;
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true));
            let _ = writeln!(stdout, "FAIL: {script_name}");
            let _ = stdout.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)).set_bold(false));
            let _ = writeln!(stdout, "  Expected: {expected_output:?}");
            let _ = writeln!(stdout, "  Actual:   {actual_output:?}");
        }
        let _ = stdout.reset();
    }

    if failed {
        panic!("One or more integration scripts failed. See output above.");
    }
}
