pub mod parser;
pub mod validators;

use crate::errors::OldSutraError;

// =====================
// Core Data Structures
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
    pub errors: Vec<OldSutraError>,
    pub warnings: Vec<OldSutraError>,
    pub suggestions: Vec<String>, // Keep suggestions as strings for now
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
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
        // Pest-specific tokens
        "_",
        "n",
        "t",
        "r",
    ],
    critical_rules: &["program", "expr", "list", "atom", "symbol"],
};

// =====================
// Traits
// =====================

pub trait ValidationReporter {
    fn report_error(&mut self, error: OldSutraError);
    fn report_warning(&mut self, warning: OldSutraError);
    fn report_suggestion(&mut self, message: impl Into<String>);
}

impl ValidationReporter for ValidationResult {
    fn report_error(&mut self, error: OldSutraError) {
        self.errors.push(error);
    }
    fn report_warning(&mut self, warning: OldSutraError) {
        self.warnings.push(warning);
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

    /// Convert errors to string format for compatibility with existing code
    pub fn error_messages(&self) -> Vec<String> {
        self.errors.iter().map(|e| format!("{}", e)).collect()
    }

    /// Convert warnings to string format for compatibility with existing code
    pub fn warning_messages(&self) -> Vec<String> {
        self.warnings.iter().map(|w| format!("{}", w)).collect()
    }
}

// =====================
// Public API
// =====================

/// Validates grammar from file path
pub fn validate_grammar(path: &str) -> Result<ValidationResult, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    validate_grammar_str(&content)
}

/// Validates grammar from string content
pub fn validate_grammar_str(content: &str) -> Result<ValidationResult, Box<dyn std::error::Error>> {
    let mut result = ValidationResult::new();
    let parser = parser::GrammarParser::new()?;
    let rules = parser.parse_rules(content)?;

    use validators::GrammarValidators;
    GrammarValidators::check_duplicate_patterns(&rules, &mut result);
    GrammarValidators::check_rule_references(&rules, &mut result);
    GrammarValidators::check_inline_vs_reference_consistency(&rules, &mut result);
    GrammarValidators::check_critical_rule_coverage(&rules, &mut result);
    GrammarValidators::check_spread_arg_usage(&rules, &mut result);

    Ok(result)
}
