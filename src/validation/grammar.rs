use crate::errors::SutraError;
use regex::Regex;
use std::collections::HashMap;

// =====================
// Core Data Structures
// =====================

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
    "POP",
    "PUSH",
    "PEEK",
    "PEEK_ALL",
    "DROP",
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

// =====================
// Public API
// =====================

/// Validates grammar from file path
pub fn validate_grammar(path: &str) -> Result<Vec<SutraError>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    validate_grammar_str(&content)
}

/// Validates grammar from string content
pub fn validate_grammar_str(content: &str) -> Result<Vec<SutraError>, Box<dyn std::error::Error>> {
    let rules = parse_grammar_rules(content)?;
    Ok(check_grammar_rules(&rules))
}

// =====================
// Grammar Parsing
// =====================

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
            ![
                "true", "false", "and", "or", "not", "if", "else", "do", "...",
            ]
            .contains(&name.as_str())
        })
        .collect()
}

// =====================
// Grammar Validation
// =====================

fn check_grammar_rules(rules: &HashMap<String, Rule>) -> Vec<SutraError> {
    let mut errors = Vec::new();

    // Check for required rules
    for &required in REQUIRED_RULES {
        if !rules.contains_key(required) {
            let message = format!("Missing required rule: {}", required);
            errors.push(crate::errors::grammar_validation_error(message, "", false));
        }
    }

    // Check rule references
    for rule in rules.values() {
        for reference in &rule.references {
            if !rules.contains_key(reference) && !BUILT_IN_RULES.contains(&reference.as_str()) {
                let message = format!(
                    "Rule '{}' references undefined rule '{}'",
                    rule.name, reference
                );
                errors.push(crate::errors::grammar_validation_error(
                    message,
                    &rule.definition,
                    false,
                ));
            }
        }
    }

    errors
}
