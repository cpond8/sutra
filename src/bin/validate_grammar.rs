//! Grammar Validation Tool for Sutra Engine
//!
//! Validates grammar.pest for consistency issues that could lead to parsing bugs.
//! Provides comprehensive validation including rule reference checking, pattern
//! analysis, and critical rule coverage validation.
//!
//! ## Usage
//! ```bash
//! cargo run --bin validate_grammar
//! ```
//!
//! ## Features
//! - Parses grammar.pest and validates rule consistency
//! - Detects undefined rule references and duplicate patterns
//! - Validates critical rule coverage and spread_arg usage
//! - Reports detailed warnings, errors, and suggestions

// ============================================================================
// 1. Module docs & imports
// ============================================================================

use std::collections::{HashMap, HashSet};
use std::fs;
use std::process;

// ============================================================================
// 2. Core data structures
// ============================================================================

/// Represents a parsed grammar rule with metadata and analysis
#[derive(Debug, Clone)]
struct Rule {
    name: String,
    definition: String,
    line_number: usize,
    references: Vec<String>,
    inline_patterns: Vec<String>,
}

/// Validation results with categorized issues and reporting functionality
#[derive(Debug)]
struct ValidationResult {
    errors: Vec<String>,
    warnings: Vec<String>,
    suggestions: Vec<String>,
}

/// Consolidated grammar knowledge to reduce coupling and centralize domain logic
struct GrammarConstants {
    built_ins: &'static [&'static str],
    critical_rules: &'static [&'static str],
}

/// State structure for rule collection process
#[derive(Debug)]
struct CollectionState {
    definition: String,
    brace_count: i32,
    in_rule: bool,
    current_index: usize,
}

/// Centralized grammar domain knowledge
const GRAMMAR_CONSTANTS: GrammarConstants = GrammarConstants {
    built_ins: &[
        "SOI", "EOI", "WHITESPACE", "COMMENT", "ANY",
        "ASCII_DIGIT", "ASCII_ALPHA", "ASCII_ALPHANUMERIC",
        "POP", "PUSH", "PEEK", "PEEK_ALL", "DROP",
        "define", "quote",
    ],
    critical_rules: &["program", "expr", "list", "atom", "symbol"],
};

// ============================================================================
// 3. Public API implementation
// ============================================================================

/// Main entry point for the validation tool
fn main() {
    let grammar_path = "src/syntax/grammar.pest";

    println!("🔍 Validating grammar file: {}", grammar_path);

    let validation_result = match validate_grammar(grammar_path) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to validate grammar: {}", e);
            process::exit(1);
        }
    };

    validation_result.print_report();

    if !validation_result.is_valid() {
        process::exit(1);
    }
}

/// Validates a grammar file and returns comprehensive results
///
/// # Arguments
/// * `path` - Path to the grammar.pest file to validate
///
/// # Returns
/// * `Ok(ValidationResult)` - Validation results with errors, warnings, suggestions
/// * `Err(Box<dyn std::error::Error>)` - File reading or parsing errors
///
/// # Example
/// ```rust
/// let result = validate_grammar("src/syntax/grammar.pest")?;
/// if !result.is_valid() {
///     eprintln!("Grammar validation failed");
/// }
/// ```
fn validate_grammar(path: &str) -> Result<ValidationResult, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path)?;
    let mut result = ValidationResult::new();

    // Parse grammar rules
    let rules = parse_grammar_rules(&content)?;

    println!("📋 Parsed {} grammar rules", rules.len());

    // Run comprehensive validation checks
    check_duplicate_patterns(&rules, &mut result);
    check_rule_references(&rules, &mut result);
    check_inline_vs_reference_consistency(&rules, &mut result);
    check_critical_rule_coverage(&rules, &mut result);
    check_spread_arg_usage(&rules, &mut result);

    Ok(result)
}

// ============================================================================
// 4. Conversions & ValidationResult implementation
// ============================================================================

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
    fn new() -> Self {
        Self {
            errors: Vec::new(),
            warnings: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    fn print_report(&self) {
        self.print_section(&self.errors, "❌", "GRAMMAR VALIDATION ERRORS");
        self.print_section(&self.warnings, "⚠️ ", "GRAMMAR WARNINGS");
        self.print_section(&self.suggestions, "💡", "GRAMMAR SUGGESTIONS");

        if self.is_valid() && self.warnings.is_empty() && self.suggestions.is_empty() {
            println!("✅ Grammar validation passed - no issues found");
        }
    }

    /// Helper to print a validation section with consistent formatting
    ///
    /// Eliminates repeated section printing patterns across error/warning/suggestion output.
    /// Uses guard clause pattern for early return on empty collections.
    ///
    /// # Arguments
    /// * `items` - Collection of validation messages to print
    /// * `emoji` - Emoji prefix for the section header
    /// * `title` - Section title text
    fn print_section(&self, items: &[String], emoji: &str, title: &str) {
        // Guard clause - skip empty sections
        if items.is_empty() {
            return;
        }

        // Happy path - print section with consistent formatting
        eprintln!("{} {}:", emoji, title);
        for item in items {
            eprintln!("  • {}", item);
        }
        eprintln!();
    }
}

// ============================================================================
// 5. Infrastructure/traits
// ============================================================================

/// DRY trait for consistent error reporting across validation functions
trait ValidationReporter {
    fn report_error(&mut self, message: impl Into<String>);
    fn report_warning(&mut self, message: impl Into<String>);
    fn report_suggestion(&mut self, message: impl Into<String>);
}

// ============================================================================
// 6. Internal helpers (grouped by function)
// ============================================================================

// === Grammar parsing helpers ===
//
// Core parsing logic for extracting rule definitions from grammar text.
// Functions handle line-by-line parsing, brace tracking, and rule construction.
//
// Main flow: parse_grammar_rules -> parse_single_rule -> collect_rule_definition
//            -> process_line_braces + build_rule + find_rule_start

/// Parses grammar content into a map of Rule structs with comprehensive validation
fn parse_grammar_rules(content: &str) -> Result<HashMap<String, Rule>, Box<dyn std::error::Error>> {
    // Guard clause - validate input early
    if is_empty_or_whitespace(content) {
        return Err("Empty grammar content".into());
    }

    let lines: Vec<&str> = content.lines().collect();
    if lines.is_empty() {
        return Err("No lines found in grammar content".into());
    }

    let mut rules = HashMap::new();
    let mut i = 0;

    // Happy path - main parsing loop unindented at the end
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

/// Extracts a single rule starting at the given line index
fn parse_single_rule(lines: &[&str], start_index: usize) -> Result<Option<(Rule, usize)>, Box<dyn std::error::Error>> {
    // Guard clause - validate index bounds
    if start_index >= lines.len() {
        return Ok(None);
    }

    let line = lines[start_index].trim();

    // Guard clause - skip comments and empty lines
    if line.starts_with("//") || line.is_empty() {
        return Ok(None);
    }

    // Guard clause - check if this line starts a rule
    let Some(rule_name) = find_rule_start(line) else {
        return Ok(None);
    };

    // Happy path - collect rule definition and build rule
    let (definition, end_index) = collect_rule_definition(lines, start_index)?;
    let rule = build_rule(rule_name, definition, start_index + 1);

    Ok(Some((rule, end_index)))
}

/// Collects a complete rule definition by tracking brace pairs
fn collect_rule_definition(lines: &[&str], start_index: usize) -> Result<(String, usize), Box<dyn std::error::Error>> {
    // Guard clause - validate input early
    validate_collection_input(lines, start_index)?;

    // Initialize collection state
    let mut state = initialize_collection_state(start_index);

    // Happy path - collect lines until rule is complete
    collect_lines_until_complete(lines, &mut state);

    Ok((state.definition, state.current_index))
}

/// Validates input parameters for rule collection
fn validate_collection_input(lines: &[&str], start_index: usize) -> Result<(), Box<dyn std::error::Error>> {
    if start_index >= lines.len() {
        return Err("Invalid start index for rule collection".into());
    }
    Ok(())
}



/// Initializes the collection state for rule processing
fn initialize_collection_state(start_index: usize) -> CollectionState {
    CollectionState {
        definition: String::new(),
        brace_count: 0,
        in_rule: false,
        current_index: start_index,
    }
}

/// Collects lines until rule definition is complete
///
/// Processes lines one by one, tracking brace pairs to determine
/// when a complete rule definition has been collected.
///
/// # Arguments
/// * `lines` - Array of lines from the grammar file
/// * `state` - Mutable collection state to update
fn collect_lines_until_complete(lines: &[&str], state: &mut CollectionState) {
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

/// Determines if rule collection should be completed
fn should_complete_collection(brace_count: i32, in_rule: bool) -> bool {
    brace_count == 0 && in_rule
}

/// Processes braces in a line and returns updated counts
fn process_line_braces(line: &str, mut brace_count: i32, mut in_rule: bool) -> Option<(i32, bool)> {
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

/// Constructs a Rule struct from parsed components
fn build_rule(rule_name: &str, definition: String, line_number: usize) -> Rule {
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

/// Identifies rule start patterns in a line

fn find_rule_start(line: &str) -> Option<&str> {
    if let Some(equals_pos) = line.find(" = ") {
        let name_part = line[..equals_pos].trim();
        if is_valid_identifier(name_part) {
            return Some(name_part);
        }
    }
    None
}

// === Rule reference extraction helpers ===
//
// Text processing utilities for extracting rule references from definitions.
// Handles comment removal, string literal filtering, and reference validation.
//
// Main flow: extract_rule_references -> remove_comments -> remove_string_literals
//            -> validate_and_clean_reference (per token)

/// DRY helper for consistent empty text validation
fn is_empty_or_whitespace(text: &str) -> bool {
    text.trim().is_empty()
}

/// DRY helper for consistent identifier validation
fn is_valid_identifier(text: &str) -> bool {
    text.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Extracts rule references from a grammar rule definition
fn extract_rule_references(definition: &str) -> Vec<String> {
    // Guard clause - validate input early
    if is_empty_or_whitespace(definition) {
        return Vec::new();
    }

    // Guard clause - process definition text and validate result
    let Some(processed_text) = process_definition_text(definition) else {
        return Vec::new();
    };

    // Happy path - extract and filter references unindented at the end
    let words = extract_words_from_text(&processed_text);
    filter_valid_references(words)
}

/// Processes definition text by removing comments and string literals
fn process_definition_text(definition: &str) -> Option<String> {
    // Remove comments and validate result
    let clean_definition = remove_comments(definition);
    if is_empty_or_whitespace(&clean_definition) {
        return None;
    }

    // Remove string literals and validate result
    let without_strings = remove_string_literals(&clean_definition);
    if is_empty_or_whitespace(&without_strings) {
        return None;
    }

    Some(without_strings)
}

/// Extracts words from processed definition text
fn extract_words_from_text(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// Filters and validates word tokens to extract valid rule references
fn filter_valid_references(words: Vec<&str>) -> Vec<String> {
    let mut references: Vec<String> = words
        .into_iter()
        .filter_map(|word| validate_and_clean_reference(word))
        .collect();

    references.sort();
    references.dedup();
    references
}

/// Removes comments from definition text

fn remove_comments(definition: &str) -> String {
    let mut clean_definition = String::new();
    for line in definition.lines() {
        if let Some(comment_pos) = line.find("//") {
            clean_definition.push_str(&line[..comment_pos]);
        } else {
            clean_definition.push_str(line);
        }
        clean_definition.push(' ');
    }
    clean_definition
}

/// Validates and cleans a token to determine if it's a valid rule reference
fn validate_and_clean_reference(word: &str) -> Option<String> {
    let clean_word = word.trim_matches(|c| "(){}[]~*+?|&!\"'.,;_".contains(c));

    // Guard clauses for invalid references
    if clean_word.is_empty() { return None; }
    if !is_valid_identifier(clean_word) { return None; }
    if clean_word.chars().all(|c| c.is_uppercase()) { return None; } // Skip constants like SOI, EOI
    if clean_word == "true" || clean_word == "false" { return None; } // Skip literals
    if clean_word.len() <= 1 { return None; } // Skip single characters

    // Happy path - valid reference
    Some(clean_word.to_string())
}

/// Removes string literals from text to avoid parsing their contents
fn remove_string_literals(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            // Skip string literal content using helper
            process_quoted_content(&mut chars, false);
            result.push(' '); // Replace string with space
        } else {
            result.push(ch);
        }
    }

    result
}

// === Inline pattern extraction helpers ===
//
// Utilities for identifying and extracting quoted string patterns from rules.
// Used to detect literal tokens and analyze pattern usage across grammar.
//
// Main flow: extract_inline_patterns -> extract_single_quoted_pattern (per quote)

/// DRY helper for consistent quoted string processing
fn process_quoted_content(chars: &mut std::iter::Peekable<std::str::Chars>, collect_content: bool) -> String {
    let mut content = String::new();

    while let Some(inner_ch) = chars.next() {
        if collect_content {
            content.push(inner_ch);
        }
        if inner_ch == '"' && chars.peek() != Some(&'\\') {
            break;
        }
    }

    content
}

/// Extracts inline quoted patterns from grammar rule definitions
fn extract_inline_patterns(definition: &str) -> Vec<String> {
    // Guard clause - validate input early
    if is_empty_or_whitespace(definition) {
        return Vec::new();
    }

    // Guard clause - check for quotes at all
    if !definition.contains('"') {
        return Vec::new();
    }

    // Happy path - extract quoted patterns unindented at the end
    let mut patterns = Vec::new();
    let mut chars = definition.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            if let Some(pattern) = extract_single_quoted_pattern(ch, &mut chars) {
                patterns.push(pattern);
            }
        }
    }

    patterns
}

/// Extracts a single quoted pattern from character iterator
fn extract_single_quoted_pattern(start_quote: char, chars: &mut std::iter::Peekable<std::str::Chars>) -> Option<String> {
    let mut pattern = String::new();
    pattern.push(start_quote);

    // Collect quoted content using helper
    let content = process_quoted_content(chars, true);
    pattern.push_str(&content);

    // Guard clause - skip empty or minimal patterns
    if pattern.len() <= 2 {
        return None;
    }

    // Happy path - valid pattern
    Some(pattern)
}

// === Validation check helpers ===
//
// High-level validation functions that analyze the parsed grammar for issues.
// Each function focuses on a specific type of validation concern.
//
// Available checks:
// - check_duplicate_patterns: Finds repeated inline patterns
// - check_rule_references: Validates rule reference integrity
// - check_inline_vs_reference_consistency: Suggests reference improvements
// - check_critical_rule_coverage: Ensures essential rules exist
// - check_spread_arg_usage: Validates variadic parameter handling

/// DRY helper for consistent rules validation across all check functions

fn validate_rules_not_empty(
    rules: &HashMap<String, Rule>,
    result: &mut ValidationResult,
    error_context: Option<&str>
) -> bool {
    if rules.is_empty() {
        if let Some(context) = error_context {
            result.report_error(format!("No rules found - {}", context));
        }
        return false;
    }
    true
}

/// Checks for duplicate inline patterns across rules

fn check_duplicate_patterns(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
    // Guard clause - early return on empty rules
    if !validate_rules_not_empty(rules, result, None) { return; }

    let mut pattern_to_rules: HashMap<String, Vec<String>> = HashMap::new();

    // Group rules by their inline patterns
    for rule in rules.values() {
        for pattern in &rule.inline_patterns {
            pattern_to_rules.entry(pattern.clone())
                .or_insert_with(Vec::new)
                .push(rule.name.clone());
        }
    }

    // Happy path - report duplicates unindented at the end
    for (pattern, rule_names) in pattern_to_rules {
        if rule_names.len() > 1 {
            result.report_warning(format!(
                "Pattern {} appears in multiple rules: {}. Consider extracting to a shared rule.",
                pattern,
                rule_names.join(", ")
            ));
        }
    }
}

/// Validates that all rule references point to defined rules

fn check_rule_references(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
    // Guard clause - early return on empty rules
    if !validate_rules_not_empty(rules, result, Some("to validate references")) { return; }

    let rule_names: HashSet<String> = rules.keys().cloned().collect();
    let built_ins = GRAMMAR_CONSTANTS.built_ins.to_vec();

    // Happy path - validate rule references unindented at the end
    for rule in rules.values() {
        for reference in &rule.references {
            if should_report_undefined_reference(reference, &rule_names, &built_ins) {
                result.report_error(format!(
                    "Rule '{}' (line {}) references undefined rule '{}'",
                    rule.name, rule.line_number, reference
                ));
            }
        }
    }
}

/// Determines if a reference should be reported as undefined
fn should_report_undefined_reference(reference: &str, rule_names: &HashSet<String>, built_ins: &[&str]) -> bool {
    !rule_names.contains(reference)
        && !built_ins.contains(&reference)
        && reference != "_" // Silent rule marker
        && reference.len() > 1 // Skip single characters
}

/// Checks for inline patterns that could be rule references instead
fn check_inline_vs_reference_consistency(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
    // Guard clause - early return on empty rules
    if !validate_rules_not_empty(rules, result, None) { return; }

    let rule_names: HashSet<String> = rules.keys().cloned().collect();

    // Happy path - check for consistency issues unindented at the end
    for rule in rules.values() {
        for pattern in &rule.inline_patterns {
            let clean_pattern = pattern.trim_matches('"');
            if rule_names.contains(clean_pattern) {
                result.report_suggestion(format!(
                    "Rule '{}' uses inline pattern '{}' which matches rule name '{}'. Consider using rule reference instead.",
                    rule.name, pattern, clean_pattern
                ));
            }
        }
    }
}

/// Validates that all critical grammar rules are present and properly structured
fn check_critical_rule_coverage(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
    // Guard clause - early return on empty rules
    if !validate_rules_not_empty(rules, result, Some("critical rules missing")) { return; }

    let critical_rules = GRAMMAR_CONSTANTS.critical_rules.to_vec();

    // Check for missing critical rules
    for critical_rule in critical_rules {
        if !rules.contains_key(critical_rule) {
            result.report_error(format!(
                "Missing critical rule: '{}'. This rule is essential for the grammar.",
                critical_rule
            ));
        }
    }

    // Happy path - validate program rule anchoring unindented at the end
    if let Some(program_rule) = rules.get("program") {
        if !program_rule.definition.contains("SOI") || !program_rule.definition.contains("EOI") {
            result.report_warning(
                "Program rule should anchor with SOI and EOI for complete input consumption"
            );
        }
    }
}

/// Validates proper spread_arg usage - addresses the specific variadic macro bug
fn check_spread_arg_usage(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
    // Guard clause - early return on empty rules
    if !validate_rules_not_empty(rules, result, Some("spread_arg validation cannot proceed")) { return; }

    // Guard clause - check if spread_arg rule exists first
    let Some(spread_arg_rule) = rules.get("spread_arg") else {
        result.report_error(
            "Missing spread_arg rule. This is essential for variadic parameter parsing"
        );
        return;
    };

    // Validate spread_arg rule definition
    if !spread_arg_rule.inline_patterns.contains(&"\"...\"".to_string()) {
        result.report_warning(
            "spread_arg rule should contain '...' pattern for variadic argument parsing"
        );
    }

    // Happy path - validate param_items usage unindented at the end
    if let Some(param_items_rule) = rules.get("param_items") {
        if param_items_rule.definition.contains("\"...\"") {
            result.report_error(
                "param_items rule contains inline '...' pattern. Should use spread_arg rule reference for consistency"
            );
        }

        if !param_items_rule.references.contains(&"spread_arg".to_string()) {
            result.report_warning(
                "param_items rule should reference spread_arg rule for variadic parameter handling"
            );
        }
    }
}