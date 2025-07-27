// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::{
    errors::{to_source_span, ErrorReporting},
    validation::grammar::{ValidationReporter, ValidationResult},
    validation::semantic::ValidationContext,
    MacroDefinition, MacroTemplate,
};

/// Validates AST nodes for semantic correctness
/// Focuses on macro/atom existence and argument validation
pub struct AstValidator;

impl AstValidator {
    /// Recursively validates AST nodes for macro/atom existence and argument counts.
    /// Traverses the tree, reporting errors for undefined macros/atoms and incorrect macro usage.
    pub fn validate_node(
        node: &AstNode,
        context: &ValidationContext,
        result: &mut ValidationResult,
    ) {
        match &*node.value {
            Expr::List(nodes, _) => {
                Self::validate_list_expression(nodes, context, result);
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                Self::validate_if_expression(condition, then_branch, else_branch, context, result);
            }
            _ => {}
        }
    }

    fn validate_list_expression(
        nodes: &[AstNode],
        context: &ValidationContext,
        result: &mut ValidationResult,
    ) {
        if nodes.is_empty() {
            return;
        }

        let first = &nodes[0];
        let Expr::Symbol(name, _) = &*first.value else {
            // Not a function call, validate all sub-nodes
            for sub_node in nodes {
                Self::validate_node(sub_node, context, result);
            }
            return;
        };

        // Validate function call
        Self::validate_function_call(name, nodes, context, result);

        // Validate all arguments
        for sub_node in nodes.iter().skip(1) {
            Self::validate_node(sub_node, context, result);
        }
    }

    fn validate_function_call(
        name: &str,
        nodes: &[AstNode],
        context: &ValidationContext,
        result: &mut ValidationResult,
    ) {
        if context.is_special_form(name) {
            return;
        }

        if let Some(macro_def) = context.macros.lookup(name) {
            if let MacroDefinition::Template(template) = macro_def {
                Self::validate_macro_args(
                    name,
                    template,
                    nodes.len() - 1,
                    result,
                    &nodes[0],
                    context,
                );
            }
        } else if context.world.get(&Path::from(name)).is_none() {
            let span = to_source_span(nodes[0].span);
            let error = context.undefined_symbol(name, span);
            result.report_error(error);
        }
    }

    fn validate_if_expression(
        condition: &AstNode,
        then_branch: &AstNode,
        else_branch: &AstNode,
        context: &ValidationContext,
        result: &mut ValidationResult,
    ) {
        Self::validate_node(condition, context, result);
        Self::validate_node(then_branch, context, result);
        Self::validate_node(else_branch, context, result);
    }

    fn validate_macro_args(
        _name: &str,
        template: &MacroTemplate,
        actual_args: usize,
        result: &mut ValidationResult,
        call_node: &AstNode,
        context: &ValidationContext,
    ) {
        let required_args = template.params.required.len();
        let has_rest = template.params.rest.is_some();

        let arity_ok = if has_rest {
            actual_args >= required_args
        } else {
            actual_args == required_args
        };

        if !arity_ok {
            let expected = if has_rest {
                format!("at least {}", required_args)
            } else {
                required_args.to_string()
            };
            let error =
                context.arity_mismatch(&expected, actual_args, to_source_span(call_node.span));
            result.report_error(error);
        }
    }
}
