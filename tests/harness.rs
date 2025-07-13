// Unified Sutra test harness: discovers, loads, filters, runs, and reports YAML-based tests
// Usage: cargo run --bin harness [substring]
// This harness is standalone and also integrated with cargo test.

use serde::Deserialize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use sutra::ast::{AstNode, Expr, ParamList, Span, WithSpan};
use sutra::cli::output::OutputBuffer;
use sutra::macros::{expand_macros, MacroDef, MacroTemplate};
use sutra::runtime::eval::eval;
use sutra::runtime::registry::build_default_atom_registry;
use sutra::runtime::world::World;
use sutra::syntax::error::SutraError;
use sutra::syntax::parser;
use walkdir::WalkDir;

// =============================================================================
// TEST DISCOVERY MODULE
// =============================================================================

/// Represents a single YAML test case for the Sutra test harness.
#[derive(Debug, Deserialize)]
pub struct TestCase {
    pub name: String,
    #[allow(dead_code)]
    pub style: String,
    pub input: String,
    pub expected: Option<String>,
    pub expect_error: Option<String>,
    pub expect_error_code: Option<String>,
    #[serde(default)]
    pub skip: bool,
    #[serde(default)]
    pub only: bool,
}

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

/// Helper for test skipping logic.
fn skip_reason(case: &TestCase, has_only: bool, filter: Option<&str>) -> Option<String> {
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
// TEST EXECUTION MODULE
// =============================================================================

pub enum TestResult {
    Pass {
        file: String,
        name: String,
    },
    Fail {
        file: String,
        name: String,
        error: String,
        expanded: Option<String>,
        eval: Option<String>,
    },
    Skipped {
        file: String,
        name: String,
        reason: String,
    },
}

pub struct PhaseState {
    pub world: World,
    pub atom_registry: sutra::atoms::AtomRegistry,
    pub output_sink: OutputBuffer,
    pub expanded: Option<String>,
    pub eval: Option<String>,
}

pub enum SutraTestError {
    Setup(String),
    Parse(SutraError),
    MacroDef(String),
    MacroExpand(SutraError, Option<String>),
    Eval(SutraError, Option<String>, Option<String>),
}

// --- Private helpers for macro detection and wrapping ---
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

fn wrap_in_do_if_needed(nodes: Vec<AstNode>, _input: &str) -> AstNode {
    if nodes.len() == 1 {
        nodes.into_iter().next().unwrap()
    } else {
        let span = Span::default();
        let mut list = Vec::with_capacity(nodes.len() + 1);

        // Create the "do" symbol properly
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

/// NEW: Use unified error system for proper error code matching
fn matches_error_code(error: &SutraError, expected_code: &str) -> bool {
    if let Some(actual_code) = error.error_code() {
        actual_code == expected_code
    } else {
        false
    }
}

/// NEW: Unified error result creation using SutraError
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

pub fn run_test_case(file: String, case: TestCase, eval_limit: usize) -> TestResult {
    fn format_output_mismatch(expected: &str, actual: &str) -> String {
        format!(
            "Output did not match expected\n  Expected: {}\n  Actual:   {}",
            expected.trim(),
            actual.trim()
        )
    }
    fn expected_error(case: &TestCase) -> bool {
        case.expect_error.is_some() || case.expect_error_code.is_some()
    }
    fn expected_error_message(case: &TestCase, val: &impl std::fmt::Debug) -> String {
        if let Some(err) = case.expect_error.as_deref() {
            format!(
                "Expected error '{}' but evaluation succeeded with result: {:?}",
                err, val
            )
        } else if let Some(code) = case.expect_error_code.as_deref() {
            format!(
                "Expected error code '{}' but evaluation succeeded with result: {:?}",
                code, val
            )
        } else {
            String::new()
        }
    }
    fn setup_env_phase() -> Result<PhaseState, SutraTestError> {
        let mut world = World::default();
        world.macros = match sutra::runtime::registry::build_canonical_macro_env() {
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
    fn parse_phase(
        state: PhaseState,
        case: &TestCase,
    ) -> Result<(PhaseState, Vec<AstNode>), SutraTestError> {
        match parser::parse(&case.input) {
            Ok(nodes) => Ok((state, nodes)),
            Err(e) => Err(SutraTestError::Parse(e)),
        }
    }
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
                        ParamList {
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
        state.eval = eval_result.as_ref().ok().map(|val| format!("{:?}", val));
        if let Ok(val) = &eval_result {
            if expected_error(case) {
                // Convert the success case to a SutraError for consistent handling
                let error_msg = expected_error_message(case, val);
                let sutra_error = sutra::syntax::error::parse_error(error_msg, None);
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
                error: format_output_mismatch(
                    case.expected.as_deref().unwrap_or(""),
                    actual_output,
                ),
                expanded: state.expanded,
                eval: state.eval,
            }
        }
    }
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
                // Use the new unified error handling for parse errors
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
                // Use the new unified error handling for macro expansion errors
                make_error_result(sutra_error, case, file, expanded, None)
            }
            SutraTestError::Eval(sutra_error, expanded, eval) => {
                // Use the new unified error handling for eval errors
                make_error_result(sutra_error, case, file, expanded, eval)
            }
        }
    }
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

// --- Macro definition parser ---
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

// =============================================================================
// TEST REPORTING MODULE
// =============================================================================

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
        if self.use_colors {
            format!("{}{}{}", color, s, RESET)
        } else {
            s.to_string()
        }
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

fn partition_results(results: &[TestResult]) -> (usize, usize, usize) {
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

fn report_results(results: &[TestResult], config: &TestConfig) {
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
// MAIN HARNESS LOGIC
// =============================================================================

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
