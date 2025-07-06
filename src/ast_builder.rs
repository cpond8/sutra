//! # AST Builder Module
//!
//! ## Purpose
//! Transforms the CST into a canonical AST, normalizing forms and discarding syntactic sugar. All nodes and errors carry spans.
//!
//! ## Core Principles
//! - Pure, stateless, composable
//! - All data/errors carry spans
//! - Serde-compatible, testable, minimal
//!
//! ## Invariants
//! - Never mutates input
//! - Output AST is canonical and normalized
//!
//! ## Changelog
//! - 2025-07-05: Initial stub by AI. Rationale: Canonical modular pipeline contract.

use serde::{Serialize, Deserialize};

/// Main trait for the AST builder stage.
pub trait SutraAstBuilder {
    /// Builds a canonical AST from a CST node. Returns AST or build error.
    fn build_ast(&self, cst: &SutraCstNode) -> Result<WithSpan<SutraAstNode>, SutraAstBuildError>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WithSpan<T> {
    pub value: T,
    pub span: SutraSpan,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraAstNode {
    List(Vec<WithSpan<SutraAstNode>>),
    Symbol(String),
    Number(f64),
    String(String),
    Bool(bool),
    Path(Vec<String>),
    If {
        condition: Box<WithSpan<SutraAstNode>>,
        then_branch: Box<WithSpan<SutraAstNode>>,
        else_branch: Box<WithSpan<SutraAstNode>>,
    },
    // ... extend as needed ...
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraAstBuildError {
    pub span: SutraSpan,
    pub message: String, // Must start with rule name and describe expected vs. found
}

/// # Example
/// ```rust
/// let builder = MyAstBuilder::default();
/// let ast = builder.build_ast(&cst)?;
/// assert!(matches!(ast.value, SutraAstNode::List(_)));
/// ```