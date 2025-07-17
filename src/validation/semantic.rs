use crate::ast::{AstNode, Expr};
use crate::macros::MacroRegistry;
use crate::atoms::AtomRegistry;
use crate::validation::grammar::{ValidationResult, ValidationReporter};

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
    validate_node(ast, macros, atoms, &mut result);
    result
}

/// Recursively validates AST nodes for macro/atom existence and argument counts.
/// Traverses the tree, reporting errors for undefined macros/atoms and incorrect macro usage.
fn validate_node(
    node: &AstNode,
    macros: &MacroRegistry,
    atoms: &AtomRegistry,
    result: &mut ValidationResult,
) {
    match &*node.value {
        Expr::List(nodes, _) => {
            if nodes.is_empty() {
                return;
            }
            let first = &nodes[0];
            let Expr::Symbol(name, _) = &*first.value else {
                for sub_node in nodes {
                    validate_node(sub_node, macros, atoms, result);
                }
                return;
            };
            if let Some(macro_def) = macros.lookup(name) {
                if let crate::macros::MacroDefinition::Template(template) = macro_def {
                    validate_macro_args(name, template, nodes.len() - 1, result);
                }
            } else if !atoms.has(name) {
                result.report_error(format!(
                    "'{}' is not defined as an atom or macro",
                    name
                ));
            }
            for sub_node in nodes {
                validate_node(sub_node, macros, atoms, result);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            validate_node(condition, macros, atoms, result);
            validate_node(then_branch, macros, atoms, result);
            validate_node(else_branch, macros, atoms, result);
        }
        Expr::Define { body, .. } => {
            validate_node(body, macros, atoms, result);
        }
        _ => {}
    }
}

fn validate_macro_args(
    name: &str,
    template: &crate::macros::MacroTemplate,
    actual_args: usize,
    result: &mut ValidationResult,
) {
    let required_args = template.params.required.len();
    let has_rest = template.params.rest.is_some();
    if !has_rest && actual_args != required_args {
        result.report_error(format!(
            "Macro '{}' expects {} arguments, but got {}",
            name, required_args, actual_args
        ));
    } else if has_rest && actual_args < required_args {
        result.report_error(format!(
            "Macro '{}' expects at least {} arguments, but got {}",
            name, required_args, actual_args
        ));
    }
}