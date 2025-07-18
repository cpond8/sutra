use crate::ast::{AstNode, Expr};
use crate::macros::MacroRegistry;
use crate::atoms::AtomRegistry;
use crate::validation::grammar::{ValidationResult, ValidationReporter};

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
                Self::validate_if_expression(condition, then_branch, else_branch, macros, atoms, result);
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
        if let Some(macro_def) = macros.lookup(name) {
            if let crate::macros::MacroDefinition::Template(template) = macro_def {
                Self::validate_macro_args(name, template, nodes.len() - 1, result);
            }
        } else if !atoms.has(name) {
            result.report_error(format!(
                "'{}' is not defined as an atom or macro",
                name
            ));
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
}