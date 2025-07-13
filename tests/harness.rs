// Sutra test harness: discovers, loads, filters, runs, and reports YAML-based tests in a flat, minimal, functional style.
// Usage: cargo run --bin harness [substring]
// This harness is standalone and also integrated with cargo test.

// =========================
// 1. Imports
// =========================
use atty;
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use sutra::ast::{AstNode, Expr, Span, WithSpan};
use sutra::cli::output::OutputBuffer;
use sutra::macros::expand_macros;
use sutra::runtime::eval::eval;
use sutra::runtime::registry::build_default_atom_registry;
use sutra::runtime::world::World;
use sutra::syntax::parser;
use walkdir::WalkDir;

mod test_discovery;
mod test_execution;
mod test_reporting;

use test_discovery::{TestCase, discover_yaml_files, load_test_cases, skip_reason};
use test_execution::{TestResult, run_test_case};
use test_reporting::{TestConfig, report_results, partition_results};
use std::env;

pub fn run_all_tests(filter: Option<&str>) -> (usize, usize, usize) {
    let config = TestConfig::default();
    let yaml_files = discover_yaml_files(config.test_root);

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
    report_results(&results, &config);
    partition_results(&results)
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let filter = if args.len() > 1 {
        Some(args[1].to_lowercase())
    } else {
        None
    };
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
