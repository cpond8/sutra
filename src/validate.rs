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