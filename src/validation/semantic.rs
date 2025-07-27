pub mod ast_validator;

use crate::{
    prelude::*, runtime::source, semantic::ast_validator::AstValidator,
    errors::{
        DiagnosticInfo, ErrorKind, ErrorReporting, FileContext, SourceInfo, SutraError,
    },
    validation::ValidationResult,
};
use miette::SourceSpan;

pub struct ValidationContext {
    pub source: source::SourceContext,
    pub phase: String,
}

impl ValidationContext {
    /// Generate context-appropriate help for validation errors
    fn generate_validation_help(&self, kind: &ErrorKind) -> Option<String> {
        match kind {
            ErrorKind::InvalidMacro { macro_name, reason } => {
                Some(format!("The macro '{}' is invalid: {}", macro_name, reason))
            }
            ErrorKind::InvalidPath { path } => {
                Some(format!("The path '{}' is not valid or cannot be resolved", path))
            }
            ErrorKind::ArityMismatch { expected, actual } => {
                Some(format!("Expected {} arguments, but got {}. Check the function signature.", expected, actual))
            }
            ErrorKind::DuplicateDefinition { symbol, .. } => {
                Some(format!("The symbol '{}' is already defined. Use a different name or check for conflicting definitions.", symbol))
            }
            ErrorKind::ScopeViolation { symbol, scope } => {
                Some(format!("The symbol '{}' is not accessible in {} scope. Check variable visibility rules.", symbol, scope))
            }
            _ => None,
        }
    }

    fn current_phase(&self) -> &str {
        // Return current validation phase (e.g., "semantic", "grammar", etc.)
        &self.phase
    }
}

impl ErrorReporting for ValidationContext {
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError {
        let help = self.generate_validation_help(&kind);
        let error_code = format!("sutra::validation::{}", kind.code_suffix());

        SutraError {
            kind,
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                file_context: FileContext::Validation { phase: self.current_phase().into() },
            },
            diagnostic_info: DiagnosticInfo {
                help,
                related_spans: Vec::new(), // Validation errors typically don't have related spans
                error_code,
                is_warning: false,
            },
        }
    }
}

/// Validates an expanded AST for macro and atom correctness.
/// Returns a ValidationResult with any errors found.
///
/// This is the primary entry point for semantic validation.
pub fn validate_expanded_ast(
    ast: &AstNode,
    macros: &MacroRegistry,
    atoms: &World,
    source: &source::SourceContext,
) -> ValidationResult {
    let mut result = ValidationResult::new();
    AstValidator::validate_node(ast, macros, atoms, &mut result, source);
    result
}
