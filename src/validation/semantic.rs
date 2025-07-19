pub mod ast_validator;

use crate::{validation::ValidationResult, AstNode, AtomRegistry, MacroRegistry};

/// Validates an expanded AST for macro and atom correctness.
/// Returns a ValidationResult with any errors found.
///
/// # Example
/// ```rust
/// use std::sync::Arc;
///
/// use sutra::{
///     ast::{AstNode, Expr, Spanned},
///     atoms::AtomRegistry,
///     macros::MacroRegistry,
///     validation::semantic::validate_expanded_ast,
/// };
/// // Minimal dummy AST node
/// let ast = Spanned {
///     value: Arc::new(Expr::Number(0.0, Default::default())),
///     span: Default::default(),
/// };
/// let macros = MacroRegistry::default();
/// let atoms = AtomRegistry::default();
/// let result = validate_expanded_ast(&ast, &macros, &atoms);
/// assert!(result.is_valid());
/// ```
pub fn validate_expanded_ast(
    ast: &AstNode,
    macros: &MacroRegistry,
    atoms: &AtomRegistry,
) -> ValidationResult {
    let mut result = ValidationResult::new();
    ast_validator::AstValidator::validate_node(ast, macros, atoms, &mut result);
    result
}
