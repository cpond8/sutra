//! Sutra Error Handling - Unified Encapsulated API
//!
//! This module provides the ONLY way to create and interact with Sutra errors.
//! All internal implementation is completely hidden to prevent misuse.

mod internal;
mod builders;
mod context;

use miette::{Diagnostic, SourceSpan};
use std::fmt;

// Re-export only the error type enum for backwards compatibility
pub use internal::ErrorType;

/// Opaque error type that wraps the internal error implementation.
///
/// This type cannot be constructed directly - it must be created through
/// the constructor functions provided by this module. This ensures all
/// errors have proper source context and prevents construction errors.
#[derive(Debug)]
pub struct SutraError(internal::InternalSutraError);

// Implement required traits by delegating to internal error
impl fmt::Display for SutraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for SutraError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl Diagnostic for SutraError {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.0.code()
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.0.help()
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        self.0.labels()
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.0.source_code()
    }
}

impl SutraError {
    /// Get the error type for backwards compatibility
    pub fn error_type(&self) -> ErrorType {
        self.0.error_type()
    }

    /// Check if this is a warning-level error
    pub fn is_warning(&self) -> bool {
        self.0.is_warning()
    }

    /// Check if this is a fatal error
    pub fn is_fatal(&self) -> bool {
        self.0.is_fatal()
    }

    /// Get the error category as a string
    pub fn category(&self) -> &'static str {
        self.0.category()
    }
}

// ============================================================================
// PUBLIC CONSTRUCTOR FUNCTIONS - The only way to create errors
// ============================================================================

/// Create a parse error for missing syntax elements
pub fn parse_missing(
    element: impl Into<String>,
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
) -> SutraError {
    builders::build_parse_missing(element.into(), source_name.into(), source_code.into(), span)
}

/// Create a parse error for malformed syntax constructs
pub fn parse_malformed(
    construct: impl Into<String>,
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
) -> SutraError {
    builders::build_parse_malformed(construct.into(), source_name.into(), source_code.into(), span)
}

/// Create a parse error for invalid values
pub fn parse_invalid_value(
    item_type: impl Into<String>,
    value: impl Into<String>,
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
) -> SutraError {
    builders::build_parse_invalid_value(item_type.into(), value.into(), source_name.into(), source_code.into(), span)
}

/// Create a parse error for empty expressions
pub fn parse_empty(
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
) -> SutraError {
    builders::build_parse_empty(source_name.into(), source_code.into(), span)
}

/// Create a parse error for parameter ordering
pub fn parse_parameter_order(
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
    rest_span: SourceSpan,
) -> SutraError {
    builders::build_parse_parameter_order(source_name.into(), source_code.into(), span, rest_span)
}

/// Create a runtime error for undefined symbols
pub fn runtime_undefined_symbol(
    symbol: impl Into<String>,
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
) -> SutraError {
    builders::build_runtime_undefined_symbol(symbol.into(), source_name.into(), source_code.into(), span)
}

/// Create a general runtime error
pub fn runtime_general(
    message: impl Into<String>,
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
) -> SutraError {
    builders::build_runtime_general(message.into(), source_name.into(), source_code.into(), span)
}

/// Create a validation error for incorrect arity
pub fn validation_arity(
    expected: impl Into<String>,
    actual: usize,
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
) -> SutraError {
    builders::build_validation_arity(expected.into(), actual, source_name.into(), source_code.into(), span)
}

/// Create a type mismatch error
pub fn type_mismatch(
    expected: impl Into<String>,
    actual: impl Into<String>,
    source_name: impl Into<String>,
    source_code: impl Into<String>,
    span: SourceSpan,
) -> SutraError {
    builders::build_type_mismatch(expected.into(), actual.into(), source_name.into(), source_code.into(), span)
}

// ============================================================================
// ENHANCEMENT METHODS - Fluent API for adding details
// ============================================================================

impl SutraError {
    /// Add a suggestion to help fix the error
    pub fn with_suggestion(self, suggestion: impl Into<String>) -> Self {
        SutraError(self.0.with_suggestion(suggestion.into()))
    }

    /// Add test context (test file name and test name)
    pub fn with_test_context(self, file: impl Into<String>, test_name: impl Into<String>) -> Self {
        SutraError(self.0.with_test_context(file.into(), test_name.into()))
    }

    /// Add a related span for multi-location diagnostics
    pub fn with_related_span(
        self,
        source_name: impl Into<String>,
        source_code: impl Into<String>,
        span: SourceSpan,
        label: impl Into<String>,
    ) -> Self {
        SutraError(self.0.with_related_span(source_name.into(), source_code.into(), span, label.into()))
    }

    /// Mark this error as a warning instead of fatal
    pub fn as_warning(self) -> Self {
        SutraError(self.0.as_warning())
    }
}
