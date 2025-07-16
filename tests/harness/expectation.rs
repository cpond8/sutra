//! # Expectation Parsing and Matching
//!
//! This module defines the `Expectation` enum and all logic for parsing and matching
//! tagged, multivariadic, order-insensitive expectations in Sutra tests.
//!
//! ## Philosophy Alignment
//! - **Minimalism:** One enum, one parser, no magic.
//! - **Compositionality:** Each tag is a self-contained variant and matcher.
//! - **Transparency:** All expectation logic is explicit and surfaced in diagnostics.
//! - **Extensibility:** New tags are added by extending the enum and parser.

use crate::ast::value::Value;

/// The canonical expectation for a Sutra test.
///
/// Each variant corresponds to a tagged expectation form as described in the test harness README.
///
/// # Examples
///
/// ```lisp
/// (expect (value 42) (output "foo\n") (tags "math" "regression") (timeout 1000))
/// ```
#[derive(Debug, Clone, PartialEq)]
pub enum Expectation {
    /// Expected value: (value ...)
    Value(Value),
    /// Expected error: (error code [message])
    Error {
        code: String,
        message: Option<String>,
        span: Option<(usize, usize)>,
        help: Option<String>,
    },
    /// Expected output: (output ...)
    Output(Value),
    /// Parameterization: (params ((1 2 3) ...))
    Params(Vec<Vec<Value>>),
    /// Skip with reason: (skip "reason")
    Skip(Option<String>),
    /// Tags: (tags "a" "b")
    Tags(Vec<String>),
    /// Timeout in ms: (timeout 1000)
    Timeout(u64),
    /// Fixture setup: (fixture "name")
    Fixture(String),
    /// Grouping: (group "name")
    Group(String),
    /// Snapshot assertion: (snapshot "file.txt")
    Snapshot(String),
}

/// Parse a tagged, multivariadic, order-insensitive (expect ...) form into a vector of Expectation tags.
///
/// # Arguments
/// * `expr` - The parsed s-expression AST node representing the (expect ...) form.
///
/// # Returns
/// * `Ok(Vec<Expectation>)` on success, or an error surfaced via miette on failure.
///
pub fn parse_expectations(expr: &crate::ast::Expr) -> miette::Result<Vec<Expectation>> {
    // TODO: Implement parser
    Ok(vec![])
}

/// Match an actual result/output/diagnostic against the expectations.
///
/// # Arguments
/// * `actual` - The actual value/output/error produced by the test.
/// * `expectations` - The parsed expectations for the test.
///
/// # Returns
/// * `true` if the actual result matches the expectations, else false.
///
pub fn match_expectations(actual: &Value, expectations: &[Expectation]) -> bool {
    // TODO: Implement matcher
    true
}