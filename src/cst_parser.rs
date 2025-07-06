//! # CST Parser Module
//!
//! ## Purpose
//! Parses source input into a concrete syntax tree (CST) using a PEG grammar. Provides a read-only, traversable CST for downstream processing and tooling.
//!
//! ## Core Principles
//! - Pure, stateless, composable
//! - All data/errors carry spans
//! - Serde-compatible, testable, minimal
//!
//! ## Invariants
//! - Never mutates input
//! - All spans are valid and non-overlapping
//!
//! ## Changelog
//! - 2025-07-05: Initial stub by AI. Rationale: Canonical modular pipeline contract.

use serde::{Serialize, Deserialize};

/// Main trait for the CST parser stage.
pub trait SutraCstParser {
    /// Parses input source into a CST. Returns a root node or a parse error.
    fn parse(&self, input: &str) -> Result<SutraCstNode, SutraCstParseError>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraCstNode {
    pub rule: SutraRule,
    pub children: Vec<SutraCstNode>,
    pub span: SutraSpan,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraRule {
    Program,
    // ... extend as needed ...
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraCstParseError {
    pub span: SutraSpan,
    pub message: String, // Must start with rule name and describe expected vs. found
}

/// # Example
/// ```rust
/// let parser = MyCstParser::default();
/// let source = "(print \"Hello\")";
/// match parser.parse(source) {
///     Ok(cst) => assert_eq!(cst.rule, SutraRule::Program),
///     Err(e) => eprintln!("Parse error: {} at {:?}", e.message, e.span),
/// }
/// ```

//! ## Anti-Patterns Checklist
//! - Never mutate input or global state.
//! - Never expose internal fields.
//! - Never use .unwrap(), .expect(), or panic! in production.
//! - Never use trait objects unless required.
//! - Never use macros for main logic (except error helpers, with justification).