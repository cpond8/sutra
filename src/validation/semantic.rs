pub mod ast_validator;

use crate::{
    prelude::*, runtime::source, semantic::ast_validator::AstValidator,
    validation::ValidationResult,
};

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
