//! # Sutra Test Harness
//!
//! Minimal, diagnostics-compliant test loader for Sutra. All errors are surfaced as diagnostics.

use std::path::Path;
use walkdir::WalkDir;
use sutra::ast::{AstNode, Expr, value::Value};
use sutra::SutraError;
use sutra::sutra_err;

/// A single test case defined in a `.sutra` file.
#[derive(Debug, Clone)]
pub struct TestCase {
    pub name: String,
    pub body: AstNode,
    pub expectation: TestExpectation,
}

/// The expected outcome of a test case.
#[derive(Debug, Clone)]
pub enum TestExpectation {
    Success(Value),
    Error(String),
}

/// Discovers and parses all test cases from `.sutra` files in a given directory.
pub fn load_test_cases(dir: &Path) -> Result<Vec<TestCase>, SutraError> {
    let mut test_cases = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().map_or(false, |e| e == "sutra") {
            let path = entry.path();
            let source = std::fs::read_to_string(path)
                .map_err(|e| sutra_err!(Validation, format!("Failed to read test file '{}': {e}", path.display())))?;
            let ast_nodes = sutra::syntax::parser::parse(&source)
                .map_err(|e| sutra_err!(Validation, format!("Failed to parse test file '{}': {e}", path.display())))?;
            for node in ast_nodes {
                let test_case = parse_test_case(&node)?;
                test_cases.push(test_case);
            }
        }
    }
    Ok(test_cases)
}

/// Parses a single `(test ...)` form into a `TestCase`. Fails if not valid.
fn parse_test_case(node: &AstNode) -> Result<TestCase, SutraError> {
    let Expr::List(items, _) = &*node.value else {
        return Err(sutra_err!(Validation, "Test form must be a list.".to_string()));
    };
    if items.is_empty() {
        return Err(sutra_err!(Validation, "Test form is empty.".to_string()));
    }
    let Expr::Symbol(name, _) = &*items[0].value else {
        return Err(sutra_err!(Validation, "Test form must start with 'test' symbol.".to_string()));
    };
    if name != "test" {
        return Err(sutra_err!(Validation, "Form is not a test (missing 'test' symbol).".to_string()));
    }
    if items.len() != 3 {
        return Err(sutra_err!(Validation, "Test form must have exactly 3 elements: (test \"name\" body)".to_string()));
    }
    let Expr::String(test_name, _) = &*items[1].value else {
        return Err(sutra_err!(Validation, "Test name must be a string literal.".to_string()));
    };
    let body = items[2].clone();
    let expectation = parse_expectation(&body)?;
    Ok(TestCase {
        name: test_name.clone(),
        body,
        expectation,
    })
}

/// Parses the expectation from a test case body.
fn parse_expectation(body: &AstNode) -> Result<TestExpectation, SutraError> {
    if let Expr::List(items, _) = &*body.value {
        if !items.is_empty() {
            if let Expr::Symbol(name, _) = &*items[0].value {
                if name == "expect" && items.len() == 2 {
                    if let Expr::Symbol(error_name, _) = &*items[1].value {
                        return Ok(TestExpectation::Error(error_name.clone()));
                    } else {
                        return Err(sutra_err!(Validation, "Expected error name to be a symbol in (expect ...)".to_string()));
                    }
                }
            }
        }
    }
    match expr_to_value(&body.value) {
        Some(val) => Ok(TestExpectation::Success(val)),
        None => Err(sutra_err!(Validation, "Test body is not a value-like expression; cannot convert to Value for success expectation.".to_string())),
    }
}

fn expr_to_value(expr: &Expr) -> Option<Value> {
    match expr {
        Expr::Number(n, _) => Some(Value::Number(*n)),
        Expr::String(s, _) => Some(Value::String(s.clone())),
        Expr::Bool(b, _) => Some(Value::Bool(*b)),
        Expr::List(items, _) => {
            let vals = items.iter().filter_map(|n| expr_to_value(&n.value)).collect::<Vec<_>>();
            Some(Value::List(vals))
        }
        Expr::Path(p, _) => Some(Value::Path(p.clone())),
        _ => None,
    }
}
