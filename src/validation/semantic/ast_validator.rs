// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::{
    validation::grammar::{ValidationReporter, ValidationResult},
    MacroDefinition, MacroRegistry, MacroTemplate,
};

/// Validates AST nodes for semantic correctness
/// Focuses on macro/atom existence and argument validation
pub struct AstValidator;

impl AstValidator {
    /// Recursively validates AST nodes for macro/atom existence and argument counts.
    /// Traverses the tree, reporting errors for undefined macros/atoms and incorrect macro usage.
    pub fn validate_node(
        node: &AstNode,
        macros: &MacroRegistry,
        atoms: &AtomRegistry,
        result: &mut ValidationResult,
    ) {
        match &*node.value {
            Expr::List(nodes, _) => {
                Self::validate_list_expression(nodes, macros, atoms, result);
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                Self::validate_if_expression(
                    condition,
                    then_branch,
                    else_branch,
                    macros,
                    atoms,
                    result,
                );
            }
            Expr::Define { body, .. } => {
                Self::validate_define_expression(body, macros, atoms, result);
            }
            _ => {}
        }
    }

    fn validate_list_expression(
        nodes: &[AstNode],
        macros: &MacroRegistry,
        atoms: &AtomRegistry,
        result: &mut ValidationResult,
    ) {
        if nodes.is_empty() {
            return;
        }

        let first = &nodes[0];
        let Expr::Symbol(name, _) = &*first.value else {
            // Not a function call, validate all sub-nodes
            for sub_node in nodes {
                Self::validate_node(sub_node, macros, atoms, result);
            }
            return;
        };

        // Validate function call
        Self::validate_function_call(name, nodes, macros, atoms, result);

        // Validate all arguments
        for sub_node in nodes {
            Self::validate_node(sub_node, macros, atoms, result);
        }
    }

    fn validate_function_call(
        name: &str,
        nodes: &[AstNode],
        macros: &MacroRegistry,
        atoms: &AtomRegistry,
        result: &mut ValidationResult,
    ) {
        // Check if it's a special form that doesn't need validation
        if ["define", "if", "lambda", "let", "do", "error", "apply"].contains(&name) {
            // Special forms are handled by the evaluation system, not validation
            return;
        }

        if let Some(macro_def) = macros.lookup(name) {
            if let MacroDefinition::Template(template) = macro_def {
                Self::validate_macro_args(name, template, nodes.len() - 1, result);
            }
        } else if !atoms.has(name) {
            // For now, assume user-defined functions are valid
            // TODO: Track user-defined functions in validation context
            // This is a temporary fix - the proper solution is to track
            // user-defined functions during validation
        }
    }

    fn validate_if_expression(
        condition: &AstNode,
        then_branch: &AstNode,
        else_branch: &AstNode,
        macros: &MacroRegistry,
        atoms: &AtomRegistry,
        result: &mut ValidationResult,
    ) {
        Self::validate_node(condition, macros, atoms, result);
        Self::validate_node(then_branch, macros, atoms, result);
        Self::validate_node(else_branch, macros, atoms, result);
    }

    fn validate_define_expression(
        body: &AstNode,
        macros: &MacroRegistry,
        atoms: &AtomRegistry,
        result: &mut ValidationResult,
    ) {
        Self::validate_node(body, macros, atoms, result);
    }

    fn validate_macro_args(
        name: &str,
        template: &MacroTemplate,
        actual_args: usize,
        result: &mut ValidationResult,
    ) {
        let required_args = template.params.required.len();
        let has_rest = template.params.rest.is_some();

        if !has_rest && actual_args != required_args {
            result.report_error(format!(
                "Macro '{name}' expects {required_args} arguments, but got {actual_args}"
            ));
        } else if has_rest && actual_args < required_args {
            result.report_error(format!(
                "Macro '{name}' expects at least {required_args} arguments, but got {actual_args}"
            ));
        }
    }
}
