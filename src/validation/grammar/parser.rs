use std::collections::HashMap;
use regex::Regex;
use crate::validation::grammar::{Rule, CollectionState};

/// Parser for grammar rule definitions
/// Handles the complex state machine logic for collecting multi-line rule definitions
pub struct GrammarParser {
    rule_regex: Regex,
    identifier_regex: Regex,
}

impl GrammarParser {
    pub fn new() -> Self {
        Self {
            rule_regex: Regex::new(r"([a-zA-Z_][a-zA-Z0-9_]*)").unwrap(),
            identifier_regex: Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap(),
        }
    }

    pub fn parse_rules(&self, content: &str) -> Result<HashMap<String, Rule>, Box<dyn std::error::Error>> {
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

    fn parse_single_rule(&self, lines: &[&str], start_index: usize) -> Result<Option<(Rule, usize)>, Box<dyn std::error::Error>> {
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

    fn collect_rule_definition(&self, lines: &[&str], start_index: usize) -> Result<(String, usize), Box<dyn std::error::Error>> {
        self.validate_collection_input(lines, start_index)?;
        let mut state = self.initialize_collection_state(start_index);
        self.collect_lines_until_complete(lines, &mut state);
        Ok((state.definition, state.current_index))
    }

    fn validate_collection_input(&self, lines: &[&str], start_index: usize) -> Result<(), Box<dyn std::error::Error>> {
        if start_index >= lines.len() {
            return Err("Invalid start index for rule collection".into());
        }
        Ok(())
    }

    fn initialize_collection_state(&self, start_index: usize) -> CollectionState {
        CollectionState {
            definition: String::new(),
            brace_count: 0,
            in_rule: false,
            current_index: start_index,
        }
    }

    fn collect_lines_until_complete(&self, lines: &[&str], state: &mut CollectionState) {
        while state.current_index < lines.len() {
            let current_line = lines[state.current_index];
            state.definition.push_str(current_line);
            state.definition.push('\n');

            if let Some((new_brace_count, rule_started)) = self.process_line_braces(current_line, state.brace_count, state.in_rule) {
                state.brace_count = new_brace_count;
                state.in_rule = rule_started;

                if self.should_complete_collection(state.brace_count, state.in_rule) {
                    state.current_index += 1;
                    break;
                }
            }

            state.current_index += 1;
        }
    }

    fn should_complete_collection(&self, brace_count: i32, in_rule: bool) -> bool {
        brace_count == 0 && in_rule
    }

    fn process_line_braces(&self, line: &str, mut brace_count: i32, mut in_rule: bool) -> Option<(i32, bool)> {
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
            if !["true", "false", "and", "or", "not", "if", "else", "define", "quote", "do", "..."].contains(&name) {
                refs.push(name.to_string());
            }
        }

        refs
    }

    fn extract_inline_patterns(&self, definition: &str) -> Vec<String> {
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

    fn is_valid_identifier(&self, s: &str) -> bool {
        self.identifier_regex.is_match(s)
    }

    fn is_empty_or_whitespace(&self, s: &str) -> bool {
        s.trim().is_empty()
    }
}