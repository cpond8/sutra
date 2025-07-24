//! Internal error implementation - COMPLETELY PRIVATE
//!
//! This module contains the actual error enum and helper types that are
//! used internally but never exposed to the rest of the application.

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceSpan};
use std::sync::Arc;
use thiserror::Error;

/// The actual error enum that implements all the error functionality.
/// This is wrapped by the public `SutraError` type to prevent direct construction.
#[derive(Error, Diagnostic, Debug)]
#[diagnostic(url(docsrs))]
pub(super) enum InternalSutraError {
    #[error("Parse error: missing {element}")]
    #[diagnostic(code(sutra::parse::missing))]
    ParseMissing {
        element: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("missing here")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics (no #[related] attribute since LabeledSpan doesn't implement Diagnostic)
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Parse error: malformed {construct}")]
    #[diagnostic(code(sutra::parse::malformed))]
    ParseMalformed {
        construct: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("malformed syntax")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics (no #[related] attribute since LabeledSpan doesn't implement Diagnostic)
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Parse error: invalid {item_type} '{value}'")]
    #[diagnostic(code(sutra::parse::invalid_value))]
    ParseInvalidValue {
        item_type: String,
        value: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("invalid value")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics (no #[related] attribute since LabeledSpan doesn't implement Diagnostic)
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Parse error: empty expression")]
    #[diagnostic(code(sutra::parse::empty))]
    ParseEmpty {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("empty expression")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Parse error: parameter order violation")]
    #[diagnostic(code(sutra::parse::parameter_order))]
    ParseParameterOrder {
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("parameter order error")]
        span: SourceSpan,
        #[label("rest parameter found here")]
        rest_span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Runtime error: undefined symbol '{symbol}'")]
    #[diagnostic(code(sutra::runtime::undefined_symbol))]
    RuntimeUndefinedSymbol {
        symbol: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("undefined symbol")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics (no #[related] attribute since LabeledSpan doesn't implement Diagnostic)
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Runtime error: {message}")]
    #[diagnostic(code(sutra::runtime::general))]
    RuntimeGeneral {
        message: String,
        label: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("{label}")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics (no #[related] attribute since LabeledSpan doesn't implement Diagnostic)
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Validation error: incorrect arity, expected {expected}, got {actual}")]
    #[diagnostic(code(sutra::validation::arity))]
    ValidationArity {
        expected: String,
        actual: usize,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("arity mismatch")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics (no #[related] attribute since LabeledSpan doesn't implement Diagnostic)
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Type error: expected {expected}, got {actual}")]
    #[diagnostic(code(sutra::types::mismatch))]
    TypeMismatch {
        expected: String,
        actual: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("type mismatch")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        // Related spans for multi-location diagnostics (no #[related] attribute since LabeledSpan doesn't implement Diagnostic)
        related_spans: Vec<LabeledSpan>,
        test_file: Option<String>,
        test_name: Option<String>,
        is_warning: bool,
    },

    #[error("Test assertion failed: {message}")]
    #[diagnostic(code(sutra::test::assertion))]
    TestAssertion {
        message: String,
        test_name: String,
        test_file: String,
        #[source_code]
        src: Arc<NamedSource<String>>,
        #[label("assertion failed here")]
        span: SourceSpan,
        #[help]
        help: Option<String>,
        related_spans: Vec<LabeledSpan>,
        is_warning: bool,
    },
}

/// Error type enumeration for backwards compatibility
///
/// Some variants are reserved for future use or compatibility with the existing codebase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    Parse,
    Validation,
    Eval,
    TypeError,
    /// Reserved for future internal/system errors
    #[allow(dead_code)]
    Internal,
    /// Reserved for future test framework errors
    #[allow(dead_code)]
    TestFailure,
}

impl InternalSutraError {
    /// Add a suggestion to help fix the error
    pub(super) fn with_suggestion(mut self, suggestion: String) -> Self {
        match &mut self {
            Self::ParseMissing { help, .. }
            | Self::ParseMalformed { help, .. }
            | Self::ParseInvalidValue { help, .. }
            | Self::ParseEmpty { help, .. }
            | Self::ParseParameterOrder { help, .. }
            | Self::RuntimeUndefinedSymbol { help, .. }
            | Self::RuntimeGeneral { help, .. }
            | Self::ValidationArity { help, .. }
            | Self::TypeMismatch { help, .. }
            | Self::TestAssertion { help, .. } => {
                *help = Some(suggestion);
            }
        }
        self
    }

    /// Add test context information
    pub(super) fn with_test_context(mut self, file: String, test_name: String) -> Self {
        match &mut self {
            Self::ParseMissing {
                test_file,
                test_name: tn,
                ..
            }
            | Self::ParseMalformed {
                test_file,
                test_name: tn,
                ..
            }
            | Self::ParseInvalidValue {
                test_file,
                test_name: tn,
                ..
            }
            | Self::ParseEmpty {
                test_file,
                test_name: tn,
                ..
            }
            | Self::ParseParameterOrder {
                test_file,
                test_name: tn,
                ..
            }
            | Self::RuntimeUndefinedSymbol {
                test_file,
                test_name: tn,
                ..
            }
            | Self::RuntimeGeneral {
                test_file,
                test_name: tn,
                ..
            }
            | Self::ValidationArity {
                test_file,
                test_name: tn,
                ..
            }
            | Self::TypeMismatch {
                test_file,
                test_name: tn,
                ..
            } => {
                *test_file = Some(file);
                *tn = Some(test_name);
            }
            Self::TestAssertion {
                test_file,
                test_name: tn,
                ..
            } => {
                *test_file = file;
                *tn = test_name;
            }
        }
        self
    }

    /// Add a related span for multi-location diagnostics
    pub(super) fn with_related_span(mut self, span: SourceSpan, label: String) -> Self {
        let labeled_span = LabeledSpan::new_with_span(Some(label), span);
        match &mut self {
            Self::ParseMissing { related_spans, .. }
            | Self::ParseMalformed { related_spans, .. }
            | Self::ParseInvalidValue { related_spans, .. }
            | Self::ParseEmpty { related_spans, .. }
            | Self::ParseParameterOrder { related_spans, .. }
            | Self::RuntimeUndefinedSymbol { related_spans, .. }
            | Self::RuntimeGeneral { related_spans, .. }
            | Self::ValidationArity { related_spans, .. }
            | Self::TypeMismatch { related_spans, .. }
            | Self::TestAssertion { related_spans, .. } => {
                related_spans.push(labeled_span);
            }
        }
        self
    }

    /// Mark this error as a warning instead of fatal
    pub(super) fn as_warning(mut self) -> Self {
        match &mut self {
            Self::ParseMissing { is_warning, .. }
            | Self::ParseMalformed { is_warning, .. }
            | Self::ParseInvalidValue { is_warning, .. }
            | Self::ParseEmpty { is_warning, .. }
            | Self::ParseParameterOrder { is_warning, .. }
            | Self::RuntimeUndefinedSymbol { is_warning, .. }
            | Self::RuntimeGeneral { is_warning, .. }
            | Self::ValidationArity { is_warning, .. }
            | Self::TypeMismatch { is_warning, .. }
            | Self::TestAssertion { is_warning, .. } => {
                *is_warning = true;
            }
        }
        self
    }

    /// Get the error type for categorization
    pub(super) fn error_type(&self) -> ErrorType {
        match self {
            Self::ParseMissing { .. }
            | Self::ParseMalformed { .. }
            | Self::ParseInvalidValue { .. } => ErrorType::Parse,
            Self::ParseEmpty { .. } | Self::ParseParameterOrder { .. } => ErrorType::Parse,
            Self::RuntimeUndefinedSymbol { .. } | Self::RuntimeGeneral { .. } => ErrorType::Eval,
            Self::ValidationArity { .. } => ErrorType::Validation,
            Self::TypeMismatch { .. } => ErrorType::TypeError,
            Self::TestAssertion { .. } => ErrorType::TestFailure,
        }
    }

    /// Check if this is a warning-level error
    pub(super) fn is_warning(&self) -> bool {
        match self {
            Self::ParseMissing { is_warning, .. }
            | Self::ParseMalformed { is_warning, .. }
            | Self::ParseInvalidValue { is_warning, .. }
            | Self::ParseEmpty { is_warning, .. }
            | Self::ParseParameterOrder { is_warning, .. }
            | Self::RuntimeUndefinedSymbol { is_warning, .. }
            | Self::RuntimeGeneral { is_warning, .. }
            | Self::ValidationArity { is_warning, .. }
            | Self::TypeMismatch { is_warning, .. }
            | Self::TestAssertion { is_warning, .. } => *is_warning,
        }
    }

    /// Check if this is a fatal error
    pub(super) fn is_fatal(&self) -> bool {
        !self.is_warning()
    }

    /// Get the error category as a string
    pub(super) fn category(&self) -> &'static str {
        match self.error_type() {
            ErrorType::Parse => "parse",
            ErrorType::Validation => "validation",
            ErrorType::Eval => "runtime",
            ErrorType::TypeError => "type",
            ErrorType::Internal => "internal",
            ErrorType::TestFailure => "test",
        }
    }
}
