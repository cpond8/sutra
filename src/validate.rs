//! Sutra Engine - Validation Module
//!
//! This module provides structural and semantic validation for Sutra ASTs.
//! All errors are span-carrying and user-friendly.

use crate::ast::{Expr, Span, WithSpan};
use crate::error::validation_error;
use crate::error::SutraError;
use crate::macros::MacroEnv;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SutraSeverity {
    Error,
    Warning,
    Info,
}

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

/// Validates the expanded AST for structural and semantic correctness.
/// Returns Ok(()) if valid, or Err(SutraError) with span and message.
// The atom registry is a single source of truth and must be passed by reference to all validation and evaluation code. Never construct a local/hidden registry.
pub fn validate(
    expr: &WithSpan<Expr>,
    env: &MacroEnv,
    atom_registry: &crate::atom::AtomRegistry,
) -> Result<(), SutraError> {
    match &expr.value {
        Expr::List(items, _) => {
            for item in items {
                validate(item, env, atom_registry)?;
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            validate(condition, env, atom_registry)?;
            validate(then_branch, env, atom_registry)?;
            validate(else_branch, env, atom_registry)?;
        }
        Expr::Symbol(name, span) => {
            // Check if symbol is a known macro or atom (basic check)
            if !env.user_macros.contains_key(name)
                && !env.core_macros.contains_key(name)
                && atom_registry.get(name).is_none()
            {
                return Err(validation_error(
                    format!("Unknown macro or atom: {}", name),
                    Some(span.clone()),
                ));
            }
        }
        // Add more cases as needed for other Expr variants
        _ => {}
    }
    Ok(())
}
