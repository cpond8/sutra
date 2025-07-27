//! This module defines the canonical `SpannedValue` type, which is the standard
//! result of any successful evaluation in the Sutra engine.

use crate::ast::{Span, Value};
use crate::errors::SutraError;

/// A canonical value paired with its source span. By carrying the span with the
/// value, we ensure that any subsequent errors related to this value (e.g.,
/// type mismatches) can be reported with precise source location information.
#[derive(Debug, Clone)]
pub struct SpannedValue {
    pub value: Value,
    pub span: Span,
}

/// The canonical result type for any operation that produces a value.
pub type SpannedResult = Result<SpannedValue, SutraError>;