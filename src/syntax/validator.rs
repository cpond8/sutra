//! # Validator Module
//!
//! ## Purpose
//! Performs all semantic and macro-level checks on the macroexpanded AST. Returns a list of diagnostics (errors, warnings, info) for author feedback.
//!
//! ## Core Principles
//! - Pure, stateless, composable
//! - All data/errors carry spans
//! - Serde-compatible, testable, minimal
//!
//! ## Invariants
//! - Never mutates input
//! - Validators may be chained or composed
//!
//! ## Changelog
//! - 2025-07-05: Modular validator system by AI. Rationale: Canonical modular pipeline contract.

use crate::ast::{Expr as SutraAstNode, Span as SutraSpan, WithSpan};
use serde::{Deserialize, Serialize};

/// Severity of a diagnostic.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum SutraSeverity {
    Error,
    Warning,
    Info,
}

/// A single validation diagnostic (error, warning, or info).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SutraDiagnostic {
    pub severity: SutraSeverity,
    pub message: String, // Must start with rule name and describe expected vs. found
    pub span: SutraSpan,
}

/// Trait for all validators (built-in and user-defined).
pub trait SutraValidator: Send + Sync {
    /// Validates the macroexpanded AST. Returns a list of diagnostics.
    fn validate(&self, ast: &WithSpan<SutraAstNode>) -> Vec<SutraDiagnostic>;
}

/// Registry for validator rules (built-in and user-defined).
pub struct ValidatorRegistry {
    validators: Vec<Box<dyn SutraValidator>>,
}

impl ValidatorRegistry {
    /// Creates a new, empty validator registry.
    pub fn new() -> Self { Self { validators: vec![] } }
    /// Registers a validator (built-in or user-defined).
    pub fn register(&mut self, validator: Box<dyn SutraValidator>) {
        self.validators.push(validator);
    }
    /// Runs all registered validators and collects diagnostics.
    pub fn validate_all(&self, ast: &WithSpan<SutraAstNode>) -> Vec<SutraDiagnostic> {
        self.validators.iter().flat_map(|v| v.validate(ast)).collect()
    }
}

/// Example: Validator that checks for empty lists.
pub struct NoEmptyListValidator;
impl SutraValidator for NoEmptyListValidator {
    fn validate(&self, ast: &WithSpan<SutraAstNode>) -> Vec<SutraDiagnostic> {
        let mut diags = Vec::new();
        match &ast.value {
            SutraAstNode::List(items, _) if items.is_empty() => {
                diags.push(SutraDiagnostic {
                    severity: SutraSeverity::Warning,
                    message: "list: expected non-empty list, found empty".to_string(),
                    span: ast.span.clone(),
                });
            }
            SutraAstNode::List(items, _) => {
                for item in items {
                    diags.extend(self.validate(item));
                }
            }
            _ => {}
        }
        diags
    }
}
