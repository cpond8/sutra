use crate::{
    errors::{to_source_span, ErrorReporting, SutraError},
    prelude::*,
    runtime::source::SourceContext,
    validation::semantic::ValidationContext,
    MacroDefinition, MacroTemplate,
};

/// Validates an AST for semantic correctness: undefined symbols and macro arity.
/// Returns all validation errors found.
pub fn validate_ast_semantics(
    ast: &AstNode,
    macros: &MacroRegistry,
    world: &World,
    source: &SourceContext,
) -> Vec<SutraError> {
    let mut errors = Vec::new();
    let context = ValidationContext::new(source.clone(), "semantic".to_string());

    validate_node_recursive(ast, macros, world, &context, &mut errors);
    errors
}

fn validate_node_recursive(
    node: &AstNode,
    macros: &MacroRegistry,
    world: &World,
    context: &ValidationContext,
    errors: &mut Vec<SutraError>,
) {
    match &*node.value {
        Expr::List(nodes, _) if !nodes.is_empty() => {
            // Check if this is a function call
            if let Expr::Symbol(name, _) = &*nodes[0].value {
                validate_function_call(name, nodes, macros, world, context, errors);
            }

            // Always validate all child nodes
            for child in nodes {
                validate_node_recursive(child, macros, world, context, errors);
            }
        }

        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            validate_node_recursive(condition, macros, world, context, errors);
            validate_node_recursive(then_branch, macros, world, context, errors);
            validate_node_recursive(else_branch, macros, world, context, errors);
        }

        // Other expression types either have no children or don't need validation
        _ => {}
    }
}

fn validate_function_call(
    name: &str,
    nodes: &[AstNode],
    macros: &MacroRegistry,
    world: &World,
    context: &ValidationContext,
    errors: &mut Vec<SutraError>,
) {
    // Skip special forms - they have their own validation
    if matches!(
        name,
        "define" | "if" | "lambda" | "let" | "do" | "error" | "apply"
    ) {
        return;
    }

    // Check macros first
    if let Some(macro_def) = macros.lookup(name) {
        if let MacroDefinition::Template(template) = macro_def {
            validate_macro_arity(name, template, nodes.len() - 1, &nodes[0], context, errors);
        }
        return;
    }

    // Check atoms
    if world.get(&Path(vec![name.to_string()])).is_some() {
        return;
    }

    // Undefined symbol
    let span = to_source_span(nodes[0].span);
    let error = context.undefined_symbol(name, span);
    errors.push(error);
}

fn validate_macro_arity(
    _name: &str,
    template: &MacroTemplate,
    actual_args: usize,
    call_node: &AstNode,
    context: &ValidationContext,
    errors: &mut Vec<SutraError>,
) {
    let required = template.params.required.len();
    let has_rest = template.params.rest.is_some();

    let valid = if has_rest {
        actual_args >= required
    } else {
        actual_args == required
    };

    if !valid {
        let expected = if has_rest {
            format!("at least {}", required)
        } else {
            required.to_string()
        };

        let span = to_source_span(call_node.span);
        let error = context.arity_mismatch(&expected, actual_args, span);
        errors.push(error);
    }
}
