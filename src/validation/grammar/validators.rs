use std::collections::{HashMap, HashSet};

use crate::errors;
use crate::validation::{Rule, ValidationReporter, ValidationResult, GRAMMAR_CONSTANTS};

/// Validates grammar rules for various correctness issues
/// Each validator focuses on a single validation concern
pub struct GrammarValidators;

impl GrammarValidators {
    /// Checks for duplicate rule patterns in the grammar.
    pub fn check_duplicate_patterns(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
        let mut seen = HashSet::new();

        for rule in rules.values() {
            if seen.insert(&rule.definition) {
                continue;
            }

            let message = format!(
                "Rule '{}' has duplicate pattern: {}",
                rule.name,
                rule.definition.trim()
            );
            let warning = errors::grammar_validation_error(message, &rule.definition, true);
            result.report_warning(warning);
        }
    }

    /// Checks for undefined rule references in the grammar.
    pub fn check_rule_references(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
        let rule_names: HashSet<&String> = rules.keys().collect();

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
            if rule_names.contains(reference) {
                continue;
            }

            if GRAMMAR_CONSTANTS.built_ins.contains(&reference.as_str()) {
                continue;
            }

            let message = format!(
                "Rule '{}' references undefined rule '{}'",
                rule.name, reference
            );
            let error = errors::grammar_validation_error(message, &rule.definition, false);
            result.report_error(error);
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
            if !rule.references.contains(pattern) {
                continue;
            }

            let message = format!(
                "Rule '{}' uses '{}' as both inline pattern and reference",
                rule.name, pattern
            );
            let warning = errors::grammar_validation_error(message, &rule.definition, true);
            result.report_warning(warning);
        }
    }

    /// Checks that all critical rules are present in the grammar.
    pub fn check_critical_rule_coverage(
        rules: &HashMap<String, Rule>,
        result: &mut ValidationResult,
    ) {
        for &critical in GRAMMAR_CONSTANTS.critical_rules {
            if rules.contains_key(critical) {
                continue;
            }

            let message = format!("Critical rule '{}' is missing from the grammar", critical);
            // For a missing rule, the definition context is empty.
            let error = errors::grammar_validation_error(message, "", false);
            result.report_error(error);
        }
    }

    /// Checks for correct usage of spread_arg in the grammar.
    pub fn check_spread_arg_usage(rules: &HashMap<String, Rule>, result: &mut ValidationResult) {
        for rule in rules.values() {
            if !rule.definition.contains("...") {
                continue;
            }

            if rule.definition.contains("spread_arg") {
                continue;
            }

            result.report_suggestion(format!(
                "Rule '{}' uses '...' but does not reference 'spread_arg'",
                rule.name
            ));
        }
    }
}
