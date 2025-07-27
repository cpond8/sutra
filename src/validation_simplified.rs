// Simplified validation module - demonstrates the essential logic
// without unnecessary abstractions and complexity

use crate::{errors::SutraError, prelude::*};
use regex::Regex;
use std::collections::HashMap;

// ============================================================================
// CORE DATA STRUCTURES
// ============================================================================

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub definition: String,
    pub references: Vec<String>,
}

// Built-in rules that don't need to be defined
const BUILT_IN_RULES: &[&str] = &[
    "SOI",
    "EOI",
    "WHITESPACE",
    "COMMENT",
    "ANY",
    "ASCII_DIGIT",
    "ASCII_ALPHA",
    "ASCII_ALPHANUMERIC",
    "define",
    "quote",
    "lambda",
    "_",
    "n",
    "t",
    "r",
];

// Rules that must exist in any grammar
const REQUIRED_RULES: &[&str] = &["program", "expr", "list", "atom", "symbol"];

// ============================================================================
// GRAMMAR VALIDATION
// ============================================================================

pub fn validate_grammar_file(path: &str) -> Result<Vec<SutraError>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    validate_grammar_content(&content)
}

pub fn validate_grammar_content(
    content: &str,
) -> Result<Vec<SutraError>, Box<dyn std::error::Error>> {
    let rules = parse_grammar_rules(content)?;
    Ok(check_grammar_rules(&rules))
}

fn parse_grammar_rules(content: &str) -> Result<HashMap<String, Rule>, Box<dyn std::error::Error>> {
    let mut rules = HashMap::new();
    let mut current_rule: Option<(String, String)> = None;
    let reference_regex = Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)")?;

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.starts_with("//") || line.is_empty() {
            continue;
        }

        // Start of new rule
        if let Some(equals_pos) = line.find(" = ") {
            // Save previous rule if exists
            if let Some((name, definition)) = current_rule.take() {
                let references = extract_references(&definition, &reference_regex);
                rules.insert(
                    name.clone(),
                    Rule {
                        name,
                        definition,
                        references,
                    },
                );
            }

            // Start new rule
            let rule_name = line[..equals_pos].trim().to_string();
            current_rule = Some((rule_name, line.to_string()));
        }
        // Continue current rule
        else if let Some((_, ref mut definition)) = current_rule.as_mut() {
            definition.push('\n');
            definition.push_str(line);
        }
    }

    // Save final rule
    if let Some((name, definition)) = current_rule {
        let references = extract_references(&definition, &reference_regex);
        rules.insert(
            name.clone(),
            Rule {
                name,
                definition,
                references,
            },
        );
    }

    Ok(rules)
}

fn extract_references(definition: &str, regex: &Regex) -> Vec<String> {
    regex
        .captures_iter(definition)
        .map(|cap| cap[1].to_string())
        .filter(|name| {
            !["true", "false", "and", "or", "not", "if", "else", "do"].contains(&name.as_str())
        })
        .collect()
}

fn check_grammar_rules(rules: &HashMap<String, Rule>) -> Vec<SutraError> {
    let mut errors = Vec::new();

    // Check for required rules
    for &required in REQUIRED_RULES {
        if !rules.contains_key(required) {
            errors.push(create_grammar_error(format!(
                "Missing required rule: {}",
                required
            )));
        }
    }

    // Check rule references
    for rule in rules.values() {
        for reference in &rule.references {
            if !rules.contains_key(reference) && !BUILT_IN_RULES.contains(&reference.as_str()) {
                errors.push(create_grammar_error(format!(
                    "Rule '{}' references undefined rule '{}'",
                    rule.name, reference
                )));
            }
        }
    }

    errors
}

// ============================================================================
// SEMANTIC VALIDATION
// ============================================================================

pub fn validate_ast_semantics(
    ast: &AstNode,
    macros: &MacroRegistry,
    world: &World,
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
                errors.push(create_semantic_error(format!(
                    "Macro '{}' expects {} arguments, got {}",
                    name, required, actual
                )));
            }
        }
        return;
    }

    // Check if atom exists
    if world.get(&Path(vec![name.to_string()])).is_some() {
        return;
    }

    // Undefined symbol
    errors.push(create_semantic_error(format!("Undefined symbol: {}", name)));
}

// ============================================================================
// ERROR CREATION HELPERS
// ============================================================================

fn create_grammar_error(message: String) -> SutraError {
    // This would use the proper error creation from the existing error system
    // Simplified for demonstration
    crate::errors::grammar_validation_error(message, "", false)
}

fn create_semantic_error(message: String) -> SutraError {
    // This would use the proper error creation from the existing error system
    // Simplified for demonstration
    use crate::errors::{DiagnosticInfo, ErrorKind, FileContext, SourceInfo};
    use miette::NamedSource;
    use std::sync::Arc;

    SutraError {
        kind: ErrorKind::GeneralValidation { message },
        source_info: SourceInfo {
            source: Arc::new(NamedSource::new("semantic_validation", "")),
            primary_span: (0..0).into(),
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
