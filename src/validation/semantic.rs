pub mod ast_validator;

use crate::prelude::*;
use crate::validation::ValidationResult;

/// Builder for semantic validation with optional source context
pub struct SemanticValidator {
    source_context: Option<(String, String)>, // (file_name, source_code)
}

impl SemanticValidator {
    /// Create a new semantic validator
    pub fn new() -> Self {
        Self {
            source_context: None,
        }
    }

    /// Add source context for better error reporting
    pub fn with_source_context(mut self, file_name: impl Into<String>, source_code: impl Into<String>) -> Self {
        self.source_context = Some((file_name.into(), source_code.into()));
        self
    }

    /// Validate the AST with the configured options
    pub fn validate(
        self,
        ast: &AstNode,
        macros: &MacroRegistry,
        atoms: &World,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();
        let source_ctx = self.source_context.as_ref().map(|(f, s)| (f.as_str(), s.as_str()));
        ast_validator::AstValidator::validate_node(ast, macros, atoms, &mut result, source_ctx);
        result
    }
}

impl Default for SemanticValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Validates an expanded AST for macro and atom correctness.
/// Returns a ValidationResult with any errors found.
///
/// # Example
/// ```rust
/// use std::sync::Arc;
///
/// use sutra::{
///     ast::{AstNode, Expr, Spanned},
///     macros::MacroRegistry,
///     validation::semantic::validate_expanded_ast,
///     World,
///     atoms,
/// };
/// // Minimal dummy AST node
/// let ast = Spanned {
///     value: Arc::new(Expr::Number(0.0, Default::default())),
///     span: Default::default(),
/// };
/// let macros = MacroRegistry::default();
/// let mut world = World::default();
/// atoms::register_all_atoms(&mut world);
/// let result = validate_expanded_ast(&ast, &macros, &world);
/// assert!(result.is_valid());
/// ```
pub fn validate_expanded_ast(
    ast: &AstNode,
    macros: &MacroRegistry,
    atoms: &World,
) -> ValidationResult {
    // Use builder pattern with no source context for backward compatibility
    SemanticValidator::new().validate(ast, macros, atoms)
}
