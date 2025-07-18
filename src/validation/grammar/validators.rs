use crate::validation::grammar::{Rule, ValidationReporter, ValidationResult, GRAMMAR_CONSTANTS};
use std::collections::{HashMap, HashSet};

/// Validates grammar rules for various correctness issues
/// Each validator focuses on a single validation concern
pub struct GrammarValidators;

impl GrammarValidators {
    /// Checks for duplicate rule patterns in the grammar.
    pub fn check_duplicate_patterns(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
        let mut seen = HashSet::new();

        for rule in rules.values() {
            if !seen.insert(&rule.definition) {
                result.report_warning(format!(
                    "Duplicate pattern in rule '{}': {}",
                    rule.name,
                    rule.definition.trim()
                ));
            }
        }
    }

    /// Checks for undefined rule references in the grammar.
    pub fn check_rule_references(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
        let rule_names: HashSet<_> = rules.keys().collect();
        for rule in rules.values() {
            Self::report_undefined_references(rule, &rule_names, result);
        }
    }

    fn report_undefined_references(
        rule: &Rule,
        rule_names: &HashSet<&String>,
        result: &mut ValidationResult,
    ) {
        for reference in &rule.references {
            if !rule_names.contains(reference)
                && !GRAMMAR_CONSTANTS.built_ins.contains(&reference.as_str())
            {
                result.report_error(format!(
                    "Rule '{}' references undefined rule '{}'.",
                    rule.name, reference
                ));
            }
        }
    }

    /// Checks for consistency between inline patterns and references.
    pub fn check_inline_vs_reference_consistency(
        rules: &HashMap<String, Rule>,
        result: &mut ValidationResult,
    ) {
        for rule in rules.values() {
            Self::report_inline_reference_overlap(rule, result);
        }
    }

    fn report_inline_reference_overlap(rule: &Rule, result: &mut ValidationResult) {
        for pattern in &rule.inline_patterns {
            if rule.references.contains(pattern) {
                result.report_warning(format!(
                    "Rule '{}' uses '{}' as both inline pattern and reference.",
                    rule.name, pattern
                ));
            }
        }
    }

    /// Checks that all critical rules are present in the grammar.
    pub fn check_critical_rule_coverage(
        rules: &HashMap<String, Rule>,
        result: &mut ValidationResult,
    ) {
        for &critical in GRAMMAR_CONSTANTS.critical_rules {
            if !rules.contains_key(critical) {
                result.report_error(format!(
                    "Critical rule '{critical}' is missing from the grammar."
                ));
            }
        }
    }

    /// Checks for correct usage of spread_arg in the grammar.
    pub fn check_spread_arg_usage(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
        for rule in rules.values() {
            if rule.definition.contains("...") && !rule.definition.contains("spread_arg") {
                result.report_suggestion(format!(
                    "Rule '{}' uses '...' but does not reference 'spread_arg'.",
                    rule.name
                ));
            }
        }
    }
}
