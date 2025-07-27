//! Sutra Error Handling - Unified Encapsulated API
//!
//! This module provides the ONLY way to create and interact with Sutra errors.
//! All internal implementation is completely hidden to prevent misuse.

mod builders;
mod context;
mod internal;

use miette::{Diagnostic, SourceSpan};
use std::fmt;
use miette::{LabeledSpan, NamedSource};
use std::sync::Arc;

// Instead, we now use the dedicated struct.
pub use crate::runtime::source::SourceContext;

// Re-export only the error type enum for backwards compatibility
#[deprecated(since = "0.8.0", note = "Please use `ErrorKind` instead")]
pub use internal::ErrorType;

/// Opaque error type that wraps the internal error implementation.
///
/// This type cannot be constructed directly - it must be created through
/// the constructor functions provided by this module. This ensures all
/// errors have proper source context and prevents construction errors.
#[derive(Debug)]
#[deprecated(since = "0.8.0", note = "Please use the new `SutraError` and its context-based constructors")]
pub struct OldSutraError(internal::InternalSutraError);

// Implement required traits by delegating to internal error
impl fmt::Display for OldSutraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for OldSutraError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

impl Diagnostic for OldSutraError {
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

impl OldSutraError {
    /// Get the error type for backwards compatibility
    #[deprecated(since = "0.8.0", note = "Use `SutraError::kind` and `ErrorKind::category` instead")]
    pub fn error_type(&self) -> ErrorType {
        self.0.error_type()
    }

    /// Check if this is a warning-level error
    #[deprecated(since = "0.8.0", note = "This is replaced by `SutraError`'s `diagnostic_info.is_warning` field")]
    pub fn is_warning(&self) -> bool {
        self.0.is_warning()
    }

    /// Check if this is a fatal error
    #[deprecated(since = "0.8.0", note = "Fatal errors are now the default. Non-fatal errors are warnings.")]
    pub fn is_fatal(&self) -> bool {
        self.0.is_fatal()
    }

    /// Get the error category as a string
    #[deprecated(since = "0.8.0", note = "Use `ErrorKind::category()` which returns an enum `ErrorCategory`")]
    pub fn category(&self) -> &'static str {
        self.0.category()
    }
}

// ============================================================================
// PUBLIC CONSTRUCTOR FUNCTIONS - The only way to create errors
// ============================================================================

/// Create a parse error for missing syntax elements
#[deprecated(since = "0.8.0", note = "Use context.missing_element() instead")]
pub fn parse_missing(
    element: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> OldSutraError {
    builders::build_parse_missing(element.into(), source.to_named_source(), span.into())
}

/// Create a parse error for malformed syntax constructs
#[deprecated(since = "0.8.0", note = "Use context.report() with ErrorKind::MalformedConstruct instead")]
pub fn parse_malformed(
    construct: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> OldSutraError {
    builders::build_parse_malformed(construct.into(), source.to_named_source(), span.into())
}

/// Create a parse error for invalid values
#[deprecated(since = "0.8.0", note = "Use context.report() with ErrorKind::InvalidLiteral instead")]
pub fn parse_invalid_value(
    item_type: impl Into<String>,
    value: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> OldSutraError {
    builders::build_parse_invalid_value(
        item_type.into(),
        value.into(),
        source.to_named_source(),
        span.into(),
    )
}

/// Create a parse error for empty expressions
#[deprecated(since = "0.8.0", note = "Use context.report() with ErrorKind::EmptyExpression instead")]
pub fn parse_empty(source: &SourceContext, span: impl Into<SourceSpan>) -> OldSutraError {
    builders::build_parse_empty(source.to_named_source(), span.into())
}

/// Create a parse error for parameter ordering
#[deprecated(since = "0.8.0", note = "Use context.report() with ErrorKind::ParameterOrderViolation instead")]
pub fn parse_parameter_order(
    source: &SourceContext,
    span: impl Into<SourceSpan>,
    rest_span: impl Into<SourceSpan>,
) -> OldSutraError {
    builders::build_parse_parameter_order(source.to_named_source(), span.into(), rest_span.into())
}

/// Create a runtime error for undefined symbols
#[deprecated(since = "0.8.0", note = "Use context.undefined_symbol() instead")]
pub fn runtime_undefined_symbol(
    symbol: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> OldSutraError {
    builders::build_runtime_undefined_symbol(symbol.into(), source.to_named_source(), span.into())
}

/// Create a general runtime error
#[deprecated(since = "0.8.0", note = "This is a general-purpose error and should be replaced with a more specific error type.")]
pub fn runtime_general(
    message: impl Into<String>,
    label: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> OldSutraError {
    builders::build_runtime_general(
        message.into(),
        label.into(),
        source.to_named_source(),
        span.into(),
    )
}

/// Create a validation error for incorrect arity
#[deprecated(since = "0.8.0", note = "Use context.arity_mismatch() instead")]
pub fn validation_arity(
    expected: impl Into<String>,
    actual: usize,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> OldSutraError {
    builders::build_validation_arity(
        expected.into(),
        actual,
        source.to_named_source(),
        span.into(),
    )
}

/// Create a type mismatch error
#[deprecated(since = "0.8.0", note = "Use context.type_mismatch() instead")]
pub fn type_mismatch(
    expected: impl Into<String>,
    actual: impl Into<String>,
    source: &SourceContext,
    span: impl Into<SourceSpan>,
) -> OldSutraError {
    builders::build_type_mismatch(
        expected.into(),
        actual.into(),
        source.to_named_source(),
        span.into(),
    )
}

/// Create a test assertion error
#[deprecated(since = "0.8.0", note = "Use context.report() with ErrorKind::AssertionFailure instead")]
pub fn test_assertion(
    message: impl Into<String>,
    test_name: impl Into<String>,
    src: &SourceContext,
    span: impl Into<SourceSpan>,
) -> OldSutraError {
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

impl OldSutraError {
    /// Add a suggestion to help fix the error
    #[deprecated(since = "0.8.0", note = "Enhancement is now automatic based on context")]
    pub fn with_suggestion(self, suggestion: impl Into<String>) -> Self {
        OldSutraError(self.0.with_suggestion(suggestion.into()))
    }

    /// Add test context (test file name and test name)
    #[deprecated(since = "0.8.0", note = "Enhancement is now automatic based on context")]
    pub fn with_test_context(self, file: impl Into<String>, test_name: impl Into<String>) -> Self {
        OldSutraError(self.0.with_test_context(file.into(), test_name.into()))
    }

    /// Add a related span for multi-location diagnostics
    #[deprecated(since = "0.8.0", note = "Enhancement is now automatic based on context")]
    pub fn with_related_span(self, span: impl Into<SourceSpan>, label: impl Into<String>) -> Self {
        OldSutraError(self.0.with_related_span(span.into(), label.into()))
    }

    /// Mark this error as a warning instead of fatal
    #[deprecated(since = "0.8.0", note = "Enhancement is now automatic based on context")]
    pub fn as_warning(self) -> Self {
        OldSutraError(self.0.as_warning())
    }
}

/// The single error type - no wrapper, no variants, just essential data
#[derive(Debug)]
pub struct SutraError {
    /// What went wrong (type-specific data)
    pub kind: ErrorKind,
    /// Where it happened (context-specific source information)
    pub source_info: SourceInfo,
    /// How to help (auto-populated based on context)
    pub diagnostic_info: DiagnosticInfo,
}

/// All error types as a clean enum - no duplicate fields
#[derive(Debug, Clone)]
pub enum ErrorKind {
    // Parse errors - structural and syntactic issues
    MissingElement { element: String },
    MalformedConstruct { construct: String },
    InvalidLiteral { literal_type: String, value: String },
    EmptyExpression,
    ParameterOrderViolation { rest_span: SourceSpan },
    UnexpectedToken { expected: String, found: String },

    // Runtime errors - evaluation failures
    UndefinedSymbol { symbol: String },
    TypeMismatch { expected: String, actual: String },
    ArityMismatch { expected: String, actual: usize },
    InvalidOperation { operation: String, operand_type: String },
    RecursionLimit,
    StackOverflow,

    // Validation errors - semantic analysis issues
    InvalidMacro { macro_name: String, reason: String },
    InvalidPath { path: String },
    DuplicateDefinition { symbol: String, original_location: SourceSpan },
    ScopeViolation { symbol: String, scope: String },

    // Test errors
    AssertionFailure { message: String, test_name: String },
}

/// Context-specific source information
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub source: Arc<NamedSource<String>>,
    pub primary_span: SourceSpan,
    pub file_context: FileContext,
}

#[derive(Debug, Clone)]
pub enum FileContext {
    ParseTime { parser_state: String },
    Runtime { test_info: Option<(String, String)> },
    Validation { phase: String },
}

/// Diagnostic enhancement data
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    pub help: Option<String>,
    pub related_spans: Vec<LabeledSpan>,
    pub error_code: String,
    pub is_warning: bool,
}

/// Context-aware error creation - each context knows how to create appropriate errors
pub trait ErrorReporting {
    /// Create an error with context-appropriate enhancements
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError;

    /// Convenience methods for common error types
    fn missing_element(&self, element: &str, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::MissingElement { element: element.into() }, span)
    }

    fn type_mismatch(&self, expected: &str, actual: &str, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::TypeMismatch { expected: expected.into(), actual: actual.into() }, span)
    }

    fn undefined_symbol(&self, symbol: &str, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::UndefinedSymbol { symbol: symbol.into() }, span)
    }

    fn arity_mismatch(&self, expected: &str, actual: usize, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::ArityMismatch { expected: expected.into(), actual }, span)
    }

    fn invalid_operation(&self, operation: &str, operand_type: &str, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::InvalidOperation {
            operation: operation.into(),
            operand_type: operand_type.into()
        }, span)
    }
}


impl ErrorKind {
    /// Get the error category for test assertions
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::MissingElement { .. } | Self::MalformedConstruct { .. } |
            Self::InvalidLiteral { .. } | Self::EmptyExpression |
            Self::ParameterOrderViolation { .. } | Self::UnexpectedToken { .. } => ErrorCategory::Parse,

            Self::UndefinedSymbol { .. } | Self::TypeMismatch { .. } |
            Self::ArityMismatch { .. } | Self::InvalidOperation { .. } |
            Self::RecursionLimit | Self::StackOverflow => ErrorCategory::Runtime,

            Self::InvalidMacro { .. } | Self::InvalidPath { .. } |
            Self::DuplicateDefinition { .. } | Self::ScopeViolation { .. } => ErrorCategory::Validation,

            Self::AssertionFailure { .. } => ErrorCategory::Test,
        }
    }

    /// Get error code suffix for diagnostic codes
    /// Uses const evaluation for zero-cost error code generation
    pub const fn code_suffix(&self) -> &'static str {
        match self {
            Self::MissingElement { .. } => "missing_element",
            Self::MalformedConstruct { .. } => "malformed_construct",
            Self::InvalidLiteral { .. } => "invalid_literal",
            Self::EmptyExpression => "empty_expression",
            Self::ParameterOrderViolation { .. } => "parameter_order_violation",
            Self::UnexpectedToken { .. } => "unexpected_token",
            Self::UndefinedSymbol { .. } => "undefined_symbol",
            Self::TypeMismatch { .. } => "type_mismatch",
            Self::ArityMismatch { .. } => "arity_mismatch",
            Self::InvalidOperation { .. } => "invalid_operation",
            Self::RecursionLimit => "recursion_limit",
            Self::StackOverflow => "stack_overflow",
            Self::InvalidMacro { .. } => "invalid_macro",
            Self::InvalidPath { .. } => "invalid_path",
            Self::DuplicateDefinition { .. } => "duplicate_definition",
            Self::ScopeViolation { .. } => "scope_violation",
            Self::AssertionFailure { .. } => "assertion_failure",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    Parse,
    Runtime,
    Validation,
    Test,
}

impl std::error::Error for SutraError {}

impl fmt::Display for SutraError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::MissingElement { element } => {
                write!(f, "Parse error: missing {}", element)
            }
            ErrorKind::MalformedConstruct { construct } => {
                write!(f, "Parse error: malformed {}", construct)
            }
            ErrorKind::InvalidLiteral { literal_type, value } => {
                write!(f, "Parse error: invalid {} '{}'", literal_type, value)
            }
            ErrorKind::EmptyExpression => {
                write!(f, "Parse error: empty expression")
            }
            ErrorKind::ParameterOrderViolation { .. } => {
                write!(f, "Parse error: parameter order violation")
            }
            ErrorKind::UnexpectedToken { expected, found } => {
                write!(f, "Parse error: expected {}, found {}", expected, found)
            }
            ErrorKind::UndefinedSymbol { symbol } => {
                write!(f, "Runtime error: undefined symbol '{}'", symbol)
            }
            ErrorKind::TypeMismatch { expected, actual } => {
                write!(f, "Type error: expected {}, got {}", expected, actual)
            }
            ErrorKind::ArityMismatch { expected, actual } => {
                write!(f, "Runtime error: incorrect arity, expected {}, got {}", expected, actual)
            }
            ErrorKind::InvalidOperation { operation, operand_type } => {
                write!(f, "Runtime error: invalid operation '{}' on {}", operation, operand_type)
            }
            ErrorKind::RecursionLimit => {
                write!(f, "Runtime error: recursion limit exceeded")
            }
            ErrorKind::StackOverflow => {
                write!(f, "Runtime error: stack overflow")
            }
            ErrorKind::InvalidMacro { macro_name, reason } => {
                write!(f, "Validation error: invalid macro '{}': {}", macro_name, reason)
            }
            ErrorKind::InvalidPath { path } => {
                write!(f, "Validation error: invalid path '{}'", path)
            }
            ErrorKind::DuplicateDefinition { symbol, .. } => {
                write!(f, "Validation error: duplicate definition of '{}'", symbol)
            }
            ErrorKind::ScopeViolation { symbol, scope } => {
                write!(f, "Validation error: '{}' not accessible in {} scope", symbol, scope)
            }
            ErrorKind::AssertionFailure { message, test_name } => {
                write!(f, "Test assertion failed in '{}': {}", test_name, message)
            }
        }
    }
}

impl Diagnostic for SutraError {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(&self.diagnostic_info.error_code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.diagnostic_info.help.as_ref().map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let mut labels = vec![
            LabeledSpan::new_with_span(Some(self.primary_label()), self.source_info.primary_span)
        ];
        labels.extend(self.diagnostic_info.related_spans.clone());
        Some(Box::new(labels.into_iter()))
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&*self.source_info.source)
    }
}

impl SutraError {
    fn primary_label(&self) -> String {
        match &self.kind {
            ErrorKind::MissingElement { .. } => "missing here".into(),
            ErrorKind::MalformedConstruct { .. } => "malformed syntax".into(),
            ErrorKind::InvalidLiteral { .. } => "invalid literal".into(),
            ErrorKind::EmptyExpression => "empty expression".into(),
            ErrorKind::ParameterOrderViolation { .. } => "parameter order error".into(),
            ErrorKind::UnexpectedToken { .. } => "unexpected token".into(),
            ErrorKind::UndefinedSymbol { .. } => "undefined symbol".into(),
            ErrorKind::TypeMismatch { .. } => "type mismatch".into(),
            ErrorKind::ArityMismatch { .. } => "arity mismatch".into(),
            ErrorKind::InvalidOperation { .. } => "invalid operation".into(),
            ErrorKind::RecursionLimit => "recursion limit exceeded".into(),
            ErrorKind::StackOverflow => "stack overflow".into(),
            ErrorKind::InvalidMacro { .. } => "invalid macro".into(),
            ErrorKind::InvalidPath { .. } => "invalid path".into(),
            ErrorKind::DuplicateDefinition { .. } => "duplicate definition".into(),
            ErrorKind::ScopeViolation { .. } => "scope violation".into(),
            ErrorKind::AssertionFailure { .. } => "assertion failed here".into(),
        }
    }
}

/// Creates a placeholder span for errors not tied to a specific source code
/// location, such as I/O errors or internal application state failures.
/// This makes the intent of using an empty span explicit and searchable.
pub fn unspanned() -> miette::SourceSpan {
    miette::SourceSpan::from(0..0)
}
