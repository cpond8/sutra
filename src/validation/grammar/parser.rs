use std::collections::HashMap;

use regex::Regex;

// Domain modules with aliases
use crate::validation::{CollectionState, Rule};

// ============================================================================
// TYPE ALIASES - Reduce verbosity in parser functions
// ============================================================================

/// Type alias for grammar parser results
type GrammarResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Type alias for rule collections
type RuleMap = HashMap<String, Rule>;

/// Parser for grammar rule definitions
/// Handles the complex state machine logic for collecting multi-line rule definitions
pub struct GrammarParser {
    rule_regex: Regex,
    identifier_regex: Regex,
}

impl GrammarParser {
    pub fn new() -> GrammarResult<Self> {
        Ok(Self {
            rule_regex: Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)")
                .map_err(|e| format!("Failed to compile rule regex: {e}"))?,
            identifier_regex: Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$")
                .map_err(|e| format!("Failed to compile identifier regex: {e}"))?,
        })
    }
}

impl Default for GrammarParser {
    fn default() -> Self {
        Self::new().expect("GrammarParser::new() should not fail with valid regex patterns")
    }
}

impl GrammarParser {
    pub fn parse_rules(&self, content: &str) -> GrammarResult<RuleMap> {
        if self.is_empty_or_whitespace(content) {
            return Err("Empty grammar content".into());
        }

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Err("No lines found in grammar content".into());
        }

        let mut rules = HashMap::new();
        let mut i = 0;

        while i < lines.len() {
            if let Some((rule, new_index)) = self.parse_single_rule(&lines, i)? {
                rules.insert(rule.name.clone(), rule);
                i = new_index;
            } else {
                i += 1;
            }
        }

        Ok(rules)
    }

    fn parse_single_rule(
        &self,
        lines: &[&str],
        start_index: usize,
    ) -> GrammarResult<Option<(Rule, usize)>> {
        if start_index >= lines.len() {
            return Ok(None);
        }

        let line = lines[start_index].trim();
        if line.starts_with("//") || line.is_empty() {
            return Ok(None);
        }

        let Some(rule_name) = self.find_rule_start(line) else {
            return Ok(None);
        };

        let (definition, end_index) = self.collect_rule_definition(lines, start_index)?;
        let rule = self.build_rule(rule_name, definition, start_index + 1);

        Ok(Some((rule, end_index)))
    }

    fn collect_rule_definition(
        &self,
        lines: &[&str],
        start_index: usize,
    ) -> GrammarResult<(String, usize)> {
        fn validate_collection_input(lines: &[&str], start_index: usize) -> GrammarResult<()> {
            if start_index >= lines.len() {
                return Err("Invalid start index for rule collection".into());
            }
            Ok(())
        }

        fn initialize_collection_state(start_index: usize) -> CollectionState {
            CollectionState {
                definition: String::new(),
                brace_count: 0,
                in_rule: false,
                current_index: start_index,
            }
        }

        validate_collection_input(lines, start_index)?;
        let mut state = initialize_collection_state(start_index);
        self.collect_lines_until_complete(lines, &mut state);
        Ok((state.definition, state.current_index))
    }

    fn collect_lines_until_complete(&self, lines: &[&str], state: &mut CollectionState) {
        while state.current_index < lines.len() {
            let current_line = lines[state.current_index];
            let cleaned_line = self.remove_inline_comment(current_line);
            state.definition.push_str(&cleaned_line);
            state.definition.push('\n');
            if self.line_completes_rule(&cleaned_line, state) {
                state.current_index += 1;
                break;
            }
            state.current_index += 1;
        }
    }

    fn line_completes_rule(&self, cleaned_line: &str, state: &mut CollectionState) -> bool {
        fn should_complete_collection(brace_count: i32, in_rule: bool) -> bool {
            brace_count == 0 && in_rule
        }

        if let Some((new_brace_count, rule_started)) =
            self.process_line_braces(cleaned_line, state.brace_count, state.in_rule)
        {
            state.brace_count = new_brace_count;
            state.in_rule = rule_started;
            should_complete_collection(state.brace_count, state.in_rule)
        } else {
            false
        }
    }

    fn process_line_braces(
        &self,
        line: &str,
        mut brace_count: i32,
        mut in_rule: bool,
    ) -> Option<(i32, bool)> {
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

    fn build_rule(&self, rule_name: &str, definition: String, line_number: usize) -> Rule {
        let references = self.extract_rule_references(&definition);
        let inline_patterns = self.extract_inline_patterns(&definition);

        Rule {
            name: rule_name.to_string(),
            definition,
            line_number,
            references,
            inline_patterns,
        }
    }

    fn find_rule_start<'a>(&self, line: &'a str) -> Option<&'a str> {
        if let Some(equals_pos) = line.find(" = ") {
            let name_part = line[..equals_pos].trim();
            if self.is_valid_identifier(name_part) {
                return Some(name_part);
            }
        }
        None
    }

    fn extract_rule_references(&self, definition: &str) -> Vec<String> {
        let mut refs = Vec::new();

        for cap in self.rule_regex.captures_iter(definition) {
            let name = &cap[1];
            // Heuristic: skip obvious literals and operators
            if ![
                "true", "false", "and", "or", "not", "if", "else", "define", "quote", "do", "...",
            ]
            .contains(&name)
            {
                refs.push(name.to_string());
            }
        }

        refs
    }

    fn extract_inline_patterns(&self, grammar_definition: &str) -> Vec<String> {
        /// Extracts non-empty pattern strings from regex capture groups
        fn extract_non_empty_patterns_from_capture(
            capture_groups: &regex::Captures,
        ) -> Vec<String> {
            let capture_group_indices = 1..=3; // Groups 1, 2, 3 correspond to quoted strings, single quotes, and braces
            capture_group_indices
                .filter_map(|group_index| {
                    capture_groups
                        .get(group_index)
                        .map(|matched_text| matched_text.as_str().to_string())
                })
                .collect()
        }

        let inline_pattern_regex = match Regex::new(r#"([^"]*)"|'([^']*)'|\{([^}]*)\}"#) {
            Ok(regex) => regex,
            Err(_) => return Vec::new(), // Return empty vector on regex compilation failure
        };

        inline_pattern_regex
            .captures_iter(grammar_definition)
            .flat_map(|capture_groups| extract_non_empty_patterns_from_capture(&capture_groups))
            .collect()
    }

    fn is_valid_identifier(&self, s: &str) -> bool {
        self.identifier_regex.is_match(s)
    }

    fn is_empty_or_whitespace(&self, s: &str) -> bool {
        s.trim().is_empty()
    }

    fn remove_inline_comment(&self, line: &str) -> String {
        if let Some(comment_pos) = line.find("//") {
            line[..comment_pos].trim_end().to_string()
        } else {
            line.to_string()
        }
    }
}
