use serde::{Serialize, Deserialize};
use crate::ast::{WithSpan, Expr, Span};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SutraSeverity { Error, Warning, Info }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SutraDiagnostic {
    pub severity: SutraSeverity,
    pub message: String,
    pub span: Span,
    // Optionally: code, suggestion, etc.
}

pub trait SutraValidator {
    fn validate(&self, ast: &WithSpan<Expr>) -> Vec<SutraDiagnostic>;
}

/// Trivial validator for pipeline scaffolding (Sprint 2).
pub struct TrivialValidator;

impl SutraValidator for TrivialValidator {
    fn validate(&self, _ast: &WithSpan<Expr>) -> Vec<SutraDiagnostic> {
        vec![]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{WithSpan, Expr, Span};
    #[test]
    fn trivial_validator_returns_empty_diagnostics() {
        let validator = TrivialValidator;
        let ast = WithSpan {
            value: Expr::List(vec![], Span { start: 0, end: 2 }),
            span: Span { start: 0, end: 2 },
        };
        let diagnostics = validator.validate(&ast);
        assert!(diagnostics.is_empty());
    }
}