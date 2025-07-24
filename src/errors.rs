//! Sutra Error Handling - Unified Encapsulated API
//!
//! This module provides the ONLY way to create and interact with Sutra errors.
//! All internal implementation is completely hidden to prevent misuse.

mod builders;
mod context;
mod internal;

use miette::{Diagnostic, SourceSpan};
use std::fmt;

// Instead, we now use the dedicated struct.
pub use crate::runtime::source::SourceContext;

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
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_parse_missing(element.into(), source.to_named_source(), span.into())
}

/// Create a parse error for malformed syntax constructs
pub fn parse_malformed(
    construct: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_parse_malformed(construct.into(), source.to_named_source(), span.into())
}

/// Create a parse error for invalid values
pub fn parse_invalid_value(
    item_type: impl Into<String>,
    value: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_parse_invalid_value(
        item_type.into(),
        value.into(),
        source.to_named_source(),
        span.into(),
    )
}

/// Create a parse error for empty expressions
pub fn parse_empty(source: &SourceContext, span: impl Into<SourceSpan>) -> SutraError {
    builders::build_parse_empty(source.to_named_source(), span.into())
}

/// Create a parse error for parameter ordering
pub fn parse_parameter_order(
    source: &SourceContext,
    span: impl Into<SourceSpan>,
    rest_span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_parse_parameter_order(source.to_named_source(), span.into(), rest_span.into())
}

/// Create a runtime error for undefined symbols
pub fn runtime_undefined_symbol(
    symbol: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_runtime_undefined_symbol(symbol.into(), source.to_named_source(), span.into())
}

/// Create a general runtime error
pub fn runtime_general(
    message: impl Into<String>,
    label: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_runtime_general(
        message.into(),
        label.into(),
        source.to_named_source(),
        span.into(),
    )
}

/// Create a validation error for incorrect arity
pub fn validation_arity(
    expected: impl Into<String>,
    actual: usize,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_validation_arity(
        expected.into(),
        actual,
        source.to_named_source(),
        span.into(),
    )
}

/// Create a type mismatch error
pub fn type_mismatch(
    expected: impl Into<String>,
    actual: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_type_mismatch(
        expected.into(),
        actual.into(),
        source.to_named_source(),
        span.into(),
    )
}

/// Create a test assertion error
pub fn test_assertion(
    message: impl Into<String>,
    test_name: impl Into<String>,
    src: &SourceContext,
    span: impl Into<SourceSpan>,
) -> SutraError {
    builders::build_test_assertion(
        message.into(),
        test_name.into(),
        src.to_named_source(),
        span.into(),
    )
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
    pub fn with_related_span(self, span: impl Into<SourceSpan>, label: impl Into<String>) -> Self {
        SutraError(self.0.with_related_span(span.into(), label.into()))
    }

    /// Mark this error as a warning instead of fatal
    pub fn as_warning(self) -> Self {
        SutraError(self.0.as_warning())
    }
}
