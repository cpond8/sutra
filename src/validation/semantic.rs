pub mod ast_validator;

use crate::validation::grammar::ValidationResult;
use crate::{AstNode, AtomRegistry, MacroRegistry};

/// Validates an expanded AST for macro and atom correctness.
/// Returns a ValidationResult with any errors found.
///
/// # Example
/// ```rust
/// use sutra::validation::semantic::validate_expanded_ast;
/// use sutra::ast::{AstNode, Expr, Spanned};
/// use sutra::macros::MacroRegistry;
/// use sutra::atoms::AtomRegistry;
/// use std::sync::Arc;
/// // Minimal dummy AST node
/// let ast = Spanned { value: Arc::new(Expr::Number(0.0, Default::default())), span: Default::default() };
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
