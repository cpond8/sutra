// Grammar validation engine for Sutra Engine
// Extracted from src/bin/validate_grammar.rs for reuse and testability

use std::collections::HashMap;
use regex::Regex;

// =====================
// 1. Core Data Structures
// =====================

#[derive(Debug, Clone)]
pub struct Rule {
    pub name: String,
    pub definition: String,
    pub line_number: usize,
    pub references: Vec<String>,
    pub inline_patterns: Vec<String>,
}

#[derive(Debug)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

pub struct GrammarConstants {
    pub built_ins: &'static [&'static str],
    pub critical_rules: &'static [&'static str],
}

#[derive(Debug)]
pub struct CollectionState {
    pub definition: String,
    pub brace_count: i32,
    pub in_rule: bool,
    pub current_index: usize,
}

pub const GRAMMAR_CONSTANTS: GrammarConstants = GrammarConstants {
    built_ins: &[
        "SOI", "EOI", "WHITESPACE", "COMMENT", "ANY",
        "ASCII_DIGIT", "ASCII_ALPHA", "ASCII_ALPHANUMERIC",
        "POP", "PUSH", "PEEK", "PEEK_ALL", "DROP",
        "define", "quote",
    ],
    critical_rules: &["program", "expr", "list", "atom", "symbol"],
};

// =====================
// 2. Traits
// =====================

pub trait ValidationReporter {
    fn report_error(&mut self, message: impl Into<String>);
    fn report_warning(&mut self, message: impl Into<String>);
    fn report_suggestion(&mut self, message: impl Into<String>);
}

impl ValidationReporter for ValidationResult {
    fn report_error(&mut self, message: impl Into<String>) {
        self.errors.push(message.into());
    }
    fn report_warning(&mut self, message: impl Into<String>) {
        self.warnings.push(message.into());
    }
    fn report_suggestion(&mut self, message: impl Into<String>) {
        self.suggestions.push(message.into());
    }
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        }
    }
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

// =====================
// 3. Public API
// =====================

pub fn validate_grammar(path: &str) -> Result<ValidationResult, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    validate_grammar_str(&content)
}

pub fn validate_grammar_str(content: &str) -> Result<ValidationResult, Box<dyn std::error::Error>> {
    let mut result = ValidationResult::new();
    let rules = parse_grammar_rules(content)?;
    check_duplicate_patterns(&rules, &mut result);
    check_rule_references(&rules, &mut result);
    check_inline_vs_reference_consistency(&rules, &mut result);
    check_critical_rule_coverage(&rules, &mut result);
    check_spread_arg_usage(&rules, &mut result);
    Ok(result)
}

// =====================
// 4. Parsing and Validation Functions
// =====================

pub fn parse_grammar_rules(content: &str) -> Result<HashMap<String, Rule>, Box<dyn std::error::Error>> {
    if is_empty_or_whitespace(content) {
        return Err("Empty grammar content".into());
    }
    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err("No lines found in grammar content".into());
    }
    let mut rules = HashMap::new();
    let mut i = 0;
    while i < lines.len() {
        if let Some((rule, new_index)) = parse_single_rule(&lines, i)? {
            rules.insert(rule.name.clone(), rule);
            i = new_index;
        } else {
            i += 1;
        }
    }
    Ok(rules)
}

pub fn parse_single_rule(lines: &[&str], start_index: usize) -> Result<Option<(Rule, usize)>, Box<dyn std::error::Error>> {
    if start_index >= lines.len() {
        return Ok(None);
    }
    let line = lines[start_index].trim();
    if line.starts_with("//") || line.is_empty() {
        return Ok(None);
    }
    let Some(rule_name) = find_rule_start(line) else {
        return Ok(None);
    };
    let (definition, end_index) = collect_rule_definition(lines, start_index)?;
    let rule = build_rule(rule_name, definition, start_index + 1);
    Ok(Some((rule, end_index)))
}

pub fn collect_rule_definition(lines: &[&str], start_index: usize) -> Result<(String, usize), Box<dyn std::error::Error>> {
    validate_collection_input(lines, start_index)?;
    let mut state = initialize_collection_state(start_index);
    collect_lines_until_complete(lines, &mut state);
    Ok((state.definition, state.current_index))
}

pub fn validate_collection_input(lines: &[&str], start_index: usize) -> Result<(), Box<dyn std::error::Error>> {
    if start_index >= lines.len() {
        return Err("Invalid start index for rule collection".into());
    }
    Ok(())
}

pub fn initialize_collection_state(start_index: usize) -> CollectionState {
    CollectionState {
        definition: String::new(),
        brace_count: 0,
        in_rule: false,
        current_index: start_index,
    }
}

pub fn collect_lines_until_complete(lines: &[&str], state: &mut CollectionState) {
    while state.current_index < lines.len() {
        let current_line = lines[state.current_index];
        state.definition.push_str(current_line);
        state.definition.push('\n');
        if let Some((new_brace_count, rule_started)) = process_line_braces(current_line, state.brace_count, state.in_rule) {
            state.brace_count = new_brace_count;
            state.in_rule = rule_started;
            if should_complete_collection(state.brace_count, state.in_rule) {
                state.current_index += 1;
                break;
            }
        }
        state.current_index += 1;
    }
}

pub fn should_complete_collection(brace_count: i32, in_rule: bool) -> bool {
    brace_count == 0 && in_rule
}

pub fn process_line_braces(line: &str, mut brace_count: i32, mut in_rule: bool) -> Option<(i32, bool)> {
    let mut found_braces = false;
    for ch in line.chars() {
        match ch {
            '{' => {
                brace_count += 1;
                in_rule = true;
                found_braces = true;
            }
            '}' => {
                brace_count -= 1;
                found_braces = true;
            }
            _ => {}
        }
    }
    if found_braces {
        Some((brace_count, in_rule))
    } else {
        None
    }
}

pub fn build_rule(rule_name: &str, definition: String, line_number: usize) -> Rule {
    let references = extract_rule_references(&definition);
    let inline_patterns = extract_inline_patterns(&definition);
    Rule {
        name: rule_name.to_string(),
        definition,
        line_number,
        references,
        inline_patterns,
    }
}

pub fn find_rule_start(line: &str) -> Option<&str> {
    if let Some(equals_pos) = line.find(" = ") {
        let name_part = line[..equals_pos].trim();
        if is_valid_identifier(name_part) {
            return Some(name_part);
        }
    }
    None
}

// =====================
// 5. Validation Checks and Helpers (restored)
// =====================

/// Checks for duplicate rule patterns in the grammar.
pub fn check_duplicate_patterns(rules: &std::collections::HashMap<String, Rule>, result: &mut ValidationResult) {
    let mut seen = std::collections::HashSet::new();
    for rule in rules.values() {
        if !seen.insert(&rule.definition) {
            result.report_warning(format!("Duplicate pattern in rule '{}': {}", rule.name, rule.definition.trim()));
        }
    }
}

/// Checks for undefined rule references in the grammar.
pub fn check_rule_references(rules: &std::collections::HashMap<String, Rule>, result: &mut ValidationResult) {
    let rule_names: std::collections::HashSet<_> = rules.keys().collect();
    for rule in rules.values() {
        for reference in &rule.references {
            if !rule_names.contains(reference) && !GRAMMAR_CONSTANTS.built_ins.contains(&reference.as_str()) {
                result.report_error(format!("Rule '{}' references undefined rule '{}'.", rule.name, reference));
            }
        }
    }
}

/// Checks for consistency between inline patterns and references.
pub fn check_inline_vs_reference_consistency(rules: &std::collections::HashMap<String, Rule>, result: &mut ValidationResult) {
    for rule in rules.values() {
        for pattern in &rule.inline_patterns {
            if rule.references.contains(pattern) {
                result.report_warning(format!("Rule '{}' uses '{}' as both inline pattern and reference.", rule.name, pattern));
            }
        }
    }
}

/// Checks that all critical rules are present in the grammar.
pub fn check_critical_rule_coverage(rules: &std::collections::HashMap<String, Rule>, result: &mut ValidationResult) {
    for &critical in GRAMMAR_CONSTANTS.critical_rules {
        if !rules.contains_key(critical) {
            result.report_error(format!("Critical rule '{}' is missing from the grammar.", critical));
        }
    }
}

/// Checks for correct usage of spread_arg in the grammar.
pub fn check_spread_arg_usage(rules: &std::collections::HashMap<String, Rule>, result: &mut ValidationResult) {
    for rule in rules.values() {
        if rule.definition.contains("..." ) && !rule.definition.contains("spread_arg") {
            result.report_suggestion(format!("Rule '{}' uses '...' but does not reference 'spread_arg'.", rule.name));
        }
    }
}

/// Returns true if the string is empty or whitespace only.
pub fn is_empty_or_whitespace(s: &str) -> bool {
    s.trim().is_empty()
}

/// Extracts rule references from a rule definition using a simple regex.
pub fn extract_rule_references(definition: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let re = Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
    for cap in re.captures_iter(definition) {
        let name = &cap[1];
        // Heuristic: skip obvious literals and operators
        if !["true", "false", "and", "or", "not", "if", "else", "define", "quote", "do", "..."].contains(&name) {
            refs.push(name.to_string());
        }
    }
    refs
}

/// Extracts inline patterns (literals in braces or quotes) from a rule definition.
pub fn extract_inline_patterns(definition: &str) -> Vec<String> {
    let mut patterns = Vec::new();
    let re = Regex::new(r#""([^"]*)"|'([^']*)'|\{([^}]*)\}"#).unwrap();
    for cap in re.captures_iter(definition) {
        for i in 1..=3 {
            if let Some(m) = cap.get(i) {
                patterns.push(m.as_str().to_string());
            }
        }
    }
    patterns
}

/// Returns true if the string is a valid identifier for a rule name.
pub fn is_valid_identifier(s: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();
    re.is_match(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_grammar() -> &'static str {
        r#"
        program = { expr* }
        expr = { atom | list }
        atom = { number | symbol }
        number = @{ "-"? ~ ASCII_DIGIT+ }
        symbol = @{ ASCII_ALPHA+ }
        list = { "(" ~ expr* ~ ")" }
        "#
    }

    #[test]
    fn test_parse_grammar_rules_valid() {
        let rules = parse_grammar_rules(sample_grammar()).unwrap();
        assert!(rules.contains_key("program"));
        assert!(rules.contains_key("expr"));
        assert!(rules.contains_key("atom"));
    }

    #[test]
    fn test_parse_grammar_rules_empty() {
        assert!(parse_grammar_rules("").is_err());
        assert!(parse_grammar_rules("   \n  ").is_err());
    }

    #[test]
    fn test_parse_single_rule_skips_comments_and_blank() {
        let lines = ["// comment", "   ", "foo = { bar }", "baz = { qux }"];
        let mut idx = 0;
        let mut found = None;
        while idx < lines.len() {
            if let Some((rule, new_idx)) = parse_single_rule(&lines, idx).unwrap() {
                found = Some((rule, new_idx));
                break;
            }
            idx += 1;
        }
        let (rule, idx) = found.expect("Should find a rule");
        assert_eq!(rule.name, "foo");
        assert_eq!(idx, 3);
    }

    #[test]
    fn test_check_duplicate_patterns() {
        let mut rules = HashMap::new();
        rules.insert("a".to_string(), Rule { name: "a".to_string(), definition: "foo".to_string(), line_number: 1, references: vec![], inline_patterns: vec![] });
        rules.insert("b".to_string(), Rule { name: "b".to_string(), definition: "foo".to_string(), line_number: 2, references: vec![], inline_patterns: vec![] });
        let mut result = ValidationResult::new();
        check_duplicate_patterns(&rules, &mut result);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_check_rule_references() {
        let mut rules = HashMap::new();
        rules.insert("a".to_string(), Rule { name: "a".to_string(), definition: "b".to_string(), line_number: 1, references: vec!["b".to_string(), "BUILTIN".to_string()], inline_patterns: vec![] });
        rules.insert("b".to_string(), Rule { name: "b".to_string(), definition: "".to_string(), line_number: 2, references: vec![], inline_patterns: vec![] });
        let mut result = ValidationResult::new();
        check_rule_references(&rules, &mut result);
        // BUILTIN is not in built_ins, so should error
        assert!(!result.errors.is_empty());
    }

    #[test]
    fn test_check_inline_vs_reference_consistency() {
        let mut rules = HashMap::new();
        rules.insert("a".to_string(), Rule { name: "a".to_string(), definition: "".to_string(), line_number: 1, references: vec!["foo".to_string()], inline_patterns: vec!["foo".to_string()] });
        let mut result = ValidationResult::new();
        check_inline_vs_reference_consistency(&rules, &mut result);
        assert!(!result.warnings.is_empty());
    }

    #[test]
    fn test_check_critical_rule_coverage() {
        let mut rules = HashMap::new();
        rules.insert("program".to_string(), Rule { name: "program".to_string(), definition: "".to_string(), line_number: 1, references: vec![], inline_patterns: vec![] });
        let mut result = ValidationResult::new();
        check_critical_rule_coverage(&rules, &mut result);
        assert!(!result.errors.is_empty()); // missing others
    }

    #[test]
    fn test_check_spread_arg_usage() {
        let mut rules = HashMap::new();
        rules.insert("foo".to_string(), Rule { name: "foo".to_string(), definition: "...".to_string(), line_number: 1, references: vec![], inline_patterns: vec![] });
        let mut result = ValidationResult::new();
        check_spread_arg_usage(&rules, &mut result);
        assert!(!result.suggestions.is_empty());
    }

    #[test]
    fn test_is_empty_or_whitespace() {
        assert!(is_empty_or_whitespace("   "));
        assert!(!is_empty_or_whitespace("foo"));
    }

    #[test]
    fn test_extract_rule_references() {
        let refs = extract_rule_references("foo bar true ...");
        assert!(refs.contains(&"foo".to_string()));
        assert!(refs.contains(&"bar".to_string()));
        assert!(!refs.contains(&"true".to_string()));
        assert!(!refs.contains(&"...".to_string()));
    }

    #[test]
    fn test_extract_inline_patterns() {
        let pats = extract_inline_patterns("'foo' \"bar\" {baz}");
        assert!(pats.contains(&"foo".to_string()));
        assert!(pats.contains(&"bar".to_string()));
        assert!(pats.contains(&"baz".to_string()));
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("foo"));
        assert!(!is_valid_identifier("123foo"));
        assert!(!is_valid_identifier("foo-bar"));
    }

    #[test]
    fn test_validate_grammar_str_valid() {
        let result = validate_grammar_str(sample_grammar());
        assert!(result.unwrap().is_valid());
    }

    #[test]
    fn test_validate_grammar_str_errors() {
        let bad_grammar = "expr = { foo }"; // missing critical rules
        let result = validate_grammar_str(bad_grammar).unwrap();
        assert!(!result.is_valid());
        assert!(!result.errors.is_empty());
    }
}