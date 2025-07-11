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

use crate::syntax::cst_parser::SutraSpan;
use crate::syntax::parser::SutraCstNode;
use crate::ast::{AstNode};
use serde::{Deserialize, Serialize};

/// Main trait for the AST builder stage.
pub trait SutraAstBuilder {
    /// Builds a canonical AST from a CST node. Returns AST or build error.
    fn build_ast(&self, cst: &SutraCstNode) -> Result<AstNode, SutraAstBuildError>;
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraAstBuildError {
    pub span: SutraSpan,
    pub message: String, // Must start with rule name and describe expected vs. found
}
