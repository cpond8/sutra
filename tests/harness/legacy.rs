//! # Legacy Compatibility Layer
//!
//! This module provides compatibility for legacy positional expectation forms and old test harness quirks.
//! It is strictly separated from the new tagged, compositional system.
//!
//! ## Philosophy Alignment
//! - **Encapsulation:** Legacy logic is isolated and can be removed cleanly in the future.
//! - **Transparency:** All legacy conversions are explicit and documented.
//! - **Pragmatism:** Supports migration without breaking existing tests.

use super::Expectation;
use crate::ast::value::Value;

/// Convert a legacy positional (expect ...) form to tagged expectations.
///
/// # Arguments
/// * `expr` - The legacy AST node representing the (expect ...) form.
///
/// # Returns
/// * `Ok(Vec<Expectation>)` on success, or an error surfaced via miette on failure.
///
/// # Example
/// ```lisp
/// (expect 42) ; → Value(42)
/// (expect-error "msg") ; → Error { ... }
/// ```
pub fn convert_legacy_expect(expr: &crate::ast::Expr) -> miette::Result<Vec<Expectation>> {
    // TODO: Implement conversion
    Ok(vec![])
}