// Core types via prelude
use crate::prelude::*;

// Domain modules with aliases
use crate::{
    errors::{self, SourceContext},
    syntax::parser::to_source_span,
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
        world: &World,
        result: &mut ValidationResult,
        source: &SourceContext,
    ) {
        match &*node.value {
            Expr::List(nodes, _) => {
                Self::validate_list_expression(nodes, macros, world, result, source);
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
                    world,
                    result,
                    source,
                );
            }
            _ => {}
        }
    }

    fn validate_list_expression(
        nodes: &[AstNode],
        macros: &MacroRegistry,
        world: &World,
        result: &mut ValidationResult,
        source: &SourceContext,
    ) {
        if nodes.is_empty() {
            return;
        }

        let first = &nodes[0];
        let Expr::Symbol(name, _) = &*first.value else {
            // Not a function call, validate all sub-nodes
            for sub_node in nodes {
                Self::validate_node(sub_node, macros, world, result, source);
            }
            return;
        };

        // Validate function call
        Self::validate_function_call(name, nodes, macros, world, result, source);

        // Validate all arguments
        for sub_node in nodes {
            Self::validate_node(sub_node, macros, world, result, source);
        }
    }

    fn validate_function_call(
        name: &str,
        nodes: &[AstNode],
        macros: &MacroRegistry,
        world: &World,
        result: &mut ValidationResult,
        source: &SourceContext,
    ) {
        // Check if it's a special form that doesn't need validation
        if ["define", "if", "lambda", "let", "do", "error", "apply"].contains(&name) {
            // Special forms are handled by the evaluation system, not validation
            return;
        }

        if let Some(macro_def) = macros.lookup(name) {
            if let MacroDefinition::Template(template) = macro_def {
                Self::validate_macro_args(
                    name,
                    template,
                    nodes.len() - 1,
                    result,
                    &nodes[0],
                    source,
                );
            }
        } else if world.get(&Path(vec![name.to_string()])).is_none() {
            // Report undefined symbol with proper error
            let error =
                errors::runtime_undefined_symbol(name, source, to_source_span(nodes[0].span))
                    .with_suggestion(format!(
                        "Define '{}' before using it, or check if it's a built-in atom",
                        name
                    ));
            result.report_error(error);
        }
    }

    fn validate_if_expression(
        condition: &AstNode,
        then_branch: &AstNode,
        else_branch: &AstNode,
        macros: &MacroRegistry,
        world: &World,
        result: &mut ValidationResult,
        source: &SourceContext,
    ) {
        Self::validate_node(condition, macros, world, result, source);
        Self::validate_node(then_branch, macros, world, result, source);
        Self::validate_node(else_branch, macros, world, result, source);
    }

    fn validate_macro_args(
        name: &str,
        template: &MacroTemplate,
        actual_args: usize,
        result: &mut ValidationResult,
        call_node: &AstNode,
        source: &SourceContext,
    ) {
        let required_args = template.params.required.len();
        let has_rest = template.params.rest.is_some();

        if !has_rest && actual_args != required_args {
            let error = errors::validation_arity(
                required_args.to_string(),
                actual_args,
                source,
                to_source_span(call_node.span),
            )
            .with_suggestion(format!(
                "Macro '{}' requires exactly {} arguments",
                name, required_args
            ));
            result.report_error(error);
        } else if has_rest && actual_args < required_args {
            let error = errors::validation_arity(
                format!("at least {}", required_args),
                actual_args,
                source,
                to_source_span(call_node.span),
            )
            .with_suggestion(format!(
                "Macro '{}' requires at least {} arguments",
                name, required_args
            ));
            result.report_error(error);
        }
    }
}
