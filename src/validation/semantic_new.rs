use crate::{
    errors::{to_source_span, ErrorKind, SutraError},
    prelude::*,
    runtime::source::SourceContext,
    MacroDefinition, MacroTemplate,
};

/// Validates an AST for semantic correctness: undefined symbols and macro arity.
/// Returns all validation errors found.
pub fn validate_ast_semantics(
    ast: &AstNode,
    macros: &MacroRegistry,
    world: &World,
    _source: &SourceContext,
) -> Vec<SutraError> {
    let mut errors = Vec::new();
    validate_node(ast, macros, world, &mut errors);
    errors
}

fn validate_node(
    node: &AstNode,
    macros: &MacroRegistry,
    world: &World,
    errors: &mut Vec<SutraError>,
) {
    match &*node.value {
        Expr::List(nodes, _) if !nodes.is_empty() => {
            if let Expr::Symbol(name, _) = &*nodes[0].value {
                validate_call(name, nodes, macros, world, errors);
            }
            // Validate all children
            for child in nodes {
                validate_node(child, macros, world, errors);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_branch,
            ..
        } => {
            validate_node(condition, macros, world, errors);
            validate_node(then_branch, macros, world, errors);
            validate_node(else_branch, macros, world, errors);
        }
        _ => {} // Atoms don't need validation
    }
}

fn validate_call(
    name: &str,
    nodes: &[AstNode],
    macros: &MacroRegistry,
    world: &World,
    errors: &mut Vec<SutraError>,
) {
    // Skip special forms
    if matches!(
        name,
        "define" | "if" | "lambda" | "let" | "do" | "error" | "apply"
    ) {
        return;
    }

    // Check if macro exists and validate arity
    if let Some(macro_def) = macros.lookup(name) {
        if let MacroDefinition::Template(template) = macro_def {
            let required = template.params.required.len();
            let actual = nodes.len() - 1;
            let has_rest = template.params.rest.is_some();

            let valid = if has_rest {
                actual >= required
            } else {
                actual == required
            };

            if !valid {
                let expected_str = if has_rest {
                    format!("at least {}", required)
                } else {
                    required.to_string()
                };

                errors.push(create_semantic_error(
                    ErrorKind::ArityMismatch {
                        expected: expected_str,
                        actual,
                    },
                    &nodes[0],
                ));
            }
        }
        return;
    }

    // Check if atom exists
    if world.get(&Path(vec![name.to_string()])).is_some() {
        return;
    }

    // Undefined symbol
    errors.push(create_semantic_error(
        ErrorKind::UndefinedSymbol {
            symbol: name.to_string(),
        },
        &nodes[0],
    ));
}

fn create_semantic_error(kind: ErrorKind, node: &AstNode) -> SutraError {
    use crate::errors::{DiagnosticInfo, FileContext, SourceInfo};
    use miette::NamedSource;
    use std::sync::Arc;

    let span = to_source_span(node.span);

    SutraError {
        kind,
        source_info: SourceInfo {
            source: Arc::new(NamedSource::new("semantic_validation", "")),
            primary_span: span,
            file_context: FileContext::Validation {
                phase: "Semantic".into(),
            },
        },
        diagnostic_info: DiagnosticInfo {
            help: None,
            related_spans: vec![],
            error_code: "validation.semantic.error".to_string(),
            is_warning: false,
        },
    }
}
