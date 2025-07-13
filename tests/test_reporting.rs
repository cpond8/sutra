// Reporting, output, and config for Sutra test harness.
use super::test_execution::TestResult;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const DEFAULT_EVAL_LIMIT: usize = 1000;

pub struct TestConfig {
    pub test_root: &'static str,
    pub eval_limit: usize,
    pub use_colors: bool,
}

impl TestConfig {
    pub fn colorize<'a>(&self, s: &'a str, color: &str) -> String {
        if self.use_colors { format!("{}{}{}", color, s, RESET) } else { s.to_string() }
    }
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            test_root: "tests",
            eval_limit: DEFAULT_EVAL_LIMIT,
            use_colors: atty::is(atty::Stream::Stderr),
        }
    }
}

pub fn partition_results(results: &[TestResult]) -> (usize, usize, usize) {
    let passed = results.iter().filter(|r| matches!(r, TestResult::Pass { .. })).count();
    let failed = results.iter().filter(|r| matches!(r, TestResult::Fail { .. })).count();
    let skipped = results.iter().filter(|r| matches!(r, TestResult::Skipped { .. })).count();
    (passed, failed, skipped)
}

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
            TestResult::Pass { file, name } => println!("{}: {} [{}]", config.colorize("PASS", GREEN), name, file),
            TestResult::Fail { .. } => print_failure(r, config),
            TestResult::Skipped { file, name, reason } => {
                println!("{}: {} [{}] ({})", config.colorize("SKIP", YELLOW), name, file, reason)
            }
        }
    }
    println!(
        "\nTest summary: total {}, {} {}, {} {}, {} {}",
        total,
        config.colorize("passed", GREEN), passed_count,
        config.colorize("failed", RED), failed_count,
        config.colorize("skipped", YELLOW), skipped_count,
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

fn print_failure(r: &TestResult, config: &TestConfig) {
    match r {
        TestResult::Fail {
            file,
            name,
            error,
            expanded,
            eval,
        } => {
            let fail = config.colorize("FAIL", RED);
            eprintln!("{fail}: {} [{}]", name, file, fail=fail);
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

fn print_output_diff(error: &str, config: &TestConfig) {
    let lines: Vec<_> = error.lines().collect();
    if lines.len() >= 3 {
        let expected = lines[1].trim_start_matches("Expected: ").trim();
        let actual = lines[2].trim_start_matches("Actual: ").trim();
        eprintln!("  Diff:");
        print_diff(expected, actual, config);
    }
}

fn print_diff(expected: &str, actual: &str, config: &TestConfig) {
    let expected_lines: Vec<_> = expected.lines().collect();
    let actual_lines: Vec<_> = actual.lines().collect();
    let max = expected_lines.len().max(actual_lines.len());
    for i in 0..max {
        let exp = *expected_lines.get(i).unwrap_or("");
        let act = *actual_lines.get(i).unwrap_or("");
        if exp != act {
            eprintln!("  - expected: {}", config.colorize(exp, GREEN));
            eprintln!("  + actual:   {}", config.colorize(act, RED));
        } else {
            eprintln!("    {}", exp);
        }
    }
}