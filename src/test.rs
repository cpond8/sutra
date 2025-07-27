use crate::{errors::ErrorType, errors::OldSutraError, Value};

/// Type-safe expectation enum for test assertions.
/// This replaces fragile string-based expectation handling.
#[derive(Debug, Clone, PartialEq)]
pub enum Expectation {
    /// Expect a specific value result
    Value(Value),
    /// Expect a specific error type
    Error(ErrorType),
    /// Expect specific output to be produced
    Output(String),
}

impl Expectation {
    /// Creates a Value expectation
    pub fn value(val: Value) -> Self {
        Self::Value(val)
    }

    /// Creates an Error expectation
    pub fn error(error_type: ErrorType) -> Self {
        Self::Error(error_type)
    }

    /// Checks if this expectation matches the given result
    pub fn matches(&self, result: &Result<Value, OldSutraError>) -> bool {
        match (self, result) {
            (Expectation::Value(expected), Ok(actual)) => expected == actual,
            (Expectation::Error(expected_type), Err(error)) => &error.error_type() == expected_type,
            _ => false,
        }
    }
}

/// Test result summary for CLI reporting
#[derive(Debug, Default)]
pub struct TestSummary {
    pub passed: usize,
    pub failed: usize,
}

impl TestSummary {
    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }

    pub fn total_tests(&self) -> usize {
        self.passed + self.failed
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tests() == 0 {
            return 0.0;
        }
        (self.passed as f64 / self.total_tests() as f64) * 100.0
    }
}

/// Test result for individual test execution
#[derive(Debug, Clone)]
pub enum TestResult {
    Passed,
    Failed,
}

pub mod runner;
