//!
//! ****************************************************************************************
//! ** BOILERPLATE ELIMINATION RULES FOR Sutra Error Macros (`err_msg!`, `err_ctx!`)      **
//! ****************************************************************************************
//!
//! # Overview
//!
//! This module defines the unified, `miette`-based diagnostic system for the Sutra engine. All errors produced by any stage of the compilation or evaluation pipeline are represented by the types in this module. Error construction is streamlined and ergonomic via the `err_msg!` and `err_ctx!` macros, which eliminate nearly all manual boilerplate.
//!
//! # Error Construction Macros
//!
//! - **Use `err_msg!` for simple, message-only errors.**
//!   - `err_msg!(Parse, "Unexpected token")`
//!
//! - **Use `err_ctx!` for errors with a string-based source and span.**
//!   - `err_ctx!(Validation, "Invalid input", src, span)`
//!
//! - **Use `err_src!` for errors with a pre-built `NamedSource`.**
//!   - `err_src!(TypeError, "Mismatched types", named_source, span)`
//!
//! # Best Practices and Rules
//!
//! - **Do not construct `ErrorContext` manually unless absolutely necessary.**
//!   The macros handle context construction for almost all cases.
//!
//! - **Pass `src` and `span` directly:**
//!   Use `err_ctx!(..., ..., src, span)` whenever you have both. The macro will handle conversion and wrapping.
//!
//! - **Pass a context struct only if it has a `.source` field:**
//!   Use `err_ctx!(..., ..., ctx)` if you have a pre-built context.
//!
//! - **Never use `.clone()` or `.into()` on `span` or `src` in a macro call.**
//!   The macros handle cloning and conversion internally. Just pass the variable.
//!
//! - **Never pass a `usize`, integer, or primitive as a span.**
//!   Always construct and pass a `Span` struct: `Span { start: pos, end: pos }` or similar.
//!
//! - **Never pass an `ErrorContext` unless you are using a context-only macro arm.**
//!   If you have both `src` and `span`, always use the direct macro arm: `err_ctx!(..., ..., src, span)`.
//!
//! - **If you need to attach a help message, use the macro arm that accepts it.**
//!   Example: `err_ctx!(..., ..., src, span, help)`
//!
//! - **If you have a context struct with a `.source` field, pass it directly.**
//!   Do not manually extract or wrap the source from a context struct; the macro will do this for you.
//!
//! - **Do not manually wrap or construct `Arc<String>` for sources.**
//!   Pass a `String`, `&str`, or context struct; the macro will handle wrapping as needed.
//!
//! - **Manual context construction is almost never needed.**
//!   Only do this for rare, advanced scenarios (e.g., custom help, no source, or nonstandard context).
//!
//! ****************************************************************************************

use std::sync::Arc;

use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};
use thiserror::Error;

use crate::Span;

// Type aliases for clarity and brevity
pub type SourceArc = Arc<NamedSource<String>>;

/// Type-safe error classification enum that corresponds to SutraError variants.
/// This replaces fragile string-based error type matching in test code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    /// Parse errors: Invalid syntax, unmatched delimiters, bad escapes
    Parse,
    /// Validation errors: Unknown macros/atoms, arity errors, invalid paths
    Validation,
    /// Runtime evaluation errors, arity mismatches, division by zero
    Eval,
    /// Type mismatches (e.g., string + number)
    TypeError,
    /// Internal engine errors
    Internal,
    /// Test assertion failures
    TestFailure,
}

impl ErrorType {
    /// Returns the string representation used in legacy test files.
    /// This maintains backward compatibility with existing test expectations.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorType::Parse => "Parse",
            ErrorType::Validation => "Validation",
            ErrorType::Eval => "Eval",
            ErrorType::TypeError => "TypeError",
            ErrorType::Internal => "Internal",
            ErrorType::TestFailure => "TestFailure",
        }
    }
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A single additional label for multi-span diagnostics.
#[derive(Debug)]
pub struct RelatedLabel {
    pub source: SourceArc,
    pub span: Span,
    pub label: String,
}

/// Minimal, composable error context for diagnostics.
#[derive(Debug, Default)]
pub struct ErrorContext {
    /// The primary source for this error (if any).
    pub source: Option<SourceArc>,
    /// The primary span for this error (if any).
    pub span: Option<Span>,
    /// An optional help message.
    pub help: Option<String>,
    /// Additional labeled spans for multi-label diagnostics.
    pub related: Vec<RelatedLabel>,
}

impl ErrorContext {
    /// Returns an empty error context (no source, span, or help).
    pub fn none() -> Self {
        Self {
            source: None,
            span: None,
            help: None,
            related: vec![],
        }
    }

    /// Creates a context with only a source.
    pub fn with_source(source: SourceArc) -> Self {
        Self {
            source: Some(source),
            span: None,
            help: None,
            related: vec![],
        }
    }

    /// Creates a context with only a span.
    pub fn with_span(span: Span) -> Self {
        Self {
            source: None,
            span: Some(span),
            help: None,
            related: vec![],
        }
    }

    /// Creates a context with both source and span.
    pub fn with_source_and_span(source: SourceArc, span: Span) -> Self {
        Self {
            source: Some(source),
            span: Some(span),
            help: None,
            related: vec![],
        }
    }

    /// Creates a context with source, span, and help message.
    pub fn with_all(source: SourceArc, span: Span, help: String) -> Self {
        Self {
            source: Some(source),
            span: Some(span),
            help: Some(help),
            related: vec![],
        }
    }
}

/// Unified error type for all Sutra engine failure modes, supporting error chaining and multi-label diagnostics.
#[derive(Debug, Error)]
pub enum SutraError {
    #[error("Parse error: {message}")]
    Parse {
        message: String,
        ctx: ErrorContext,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        ctx: ErrorContext,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
    #[error("Evaluation error: {message}")]
    Eval {
        message: String,
        ctx: ErrorContext,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
    #[error("Type error: {message}")]
    TypeError {
        message: String,
        ctx: ErrorContext,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
    #[error("Division by zero")]
    DivisionByZero {
        ctx: ErrorContext,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        ctx: ErrorContext,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
    #[error("Test failure: {message}")]
    TestSyntaxFailure {
        message: String,
        ctx: ErrorContext,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },
}

impl SutraError {
    fn get_ctx(&self) -> &ErrorContext {
        match self {
            SutraError::Parse { ctx, .. } => ctx,
            SutraError::Validation { ctx, .. } => ctx,
            SutraError::Eval { ctx, .. } => ctx,
            SutraError::TypeError { ctx, .. } => ctx,
            SutraError::DivisionByZero { ctx, .. } => ctx,
            SutraError::Internal { ctx, .. } => ctx,
            SutraError::TestSyntaxFailure { ctx, .. } => ctx,
        }
    }

    /// Returns the type-safe error classification for this error.
    /// This replaces the fragile string-based error type extraction.
    pub fn error_type(&self) -> ErrorType {
        match self {
            SutraError::Parse { .. } => ErrorType::Parse,
            SutraError::Validation { .. } => ErrorType::Validation,
            SutraError::Eval { .. } => ErrorType::Eval,
            SutraError::TypeError { .. } => ErrorType::TypeError,
            SutraError::DivisionByZero { .. } => ErrorType::Eval, // Division by zero is a runtime eval error
            SutraError::Internal { .. } => ErrorType::Internal,
            SutraError::TestSyntaxFailure { .. } => ErrorType::TestFailure,
        }
    }
}

impl Diagnostic for SutraError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        None
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.get_ctx()
            .help
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn std::fmt::Display + 'a>)
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        self.get_ctx()
            .source
            .as_ref()
            .map(|s| s.as_ref() as &dyn SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let ctx = self.get_ctx();
        let mut labels = Vec::new();
        // Primary label
        if let Some(span) = ctx.span {
            let text = match self {
                SutraError::Parse { message, .. } => Some(message.clone()),
                SutraError::Validation { message, .. } => Some(message.clone()),
                SutraError::Eval { message, .. } => Some(message.clone()),
                SutraError::TypeError { message, .. } => Some(message.clone()),
                SutraError::DivisionByZero { .. } => Some("division by zero".to_string()),
                SutraError::Internal { message, .. } => Some(message.clone()),
                SutraError::TestSyntaxFailure { message, .. } => Some(message.clone()),
            };
            let len = if span.end > span.start {
                span.end - span.start
            } else {
                1
            };
            labels.push(LabeledSpan::new(text, span.start, len));
        }
        // Related labels
        for rel in &ctx.related {
            let len = if rel.span.end > rel.span.start {
                rel.span.end - rel.span.start
            } else {
                1
            };
            labels.push(LabeledSpan::new(
                Some(rel.label.clone()),
                rel.span.start,
                len,
            ));
        }
        if labels.is_empty() {
            None
        } else {
            Some(Box::new(labels.into_iter()))
        }
    }
}

/// Converts a source string into an `Arc<NamedSource<String>>` for use in error contexts.
pub fn to_error_source<S: AsRef<str>>(source: S) -> SourceArc {
    Arc::new(NamedSource::new("source", source.as_ref().to_string()))
}

/// Constructs a SutraError variant with a formatted message and no context.
///
/// Use this macro for errors that do not require source, span, or help context. Supports formatting with multiple arguments.
#[macro_export]
macro_rules! err_msg {
    // Message with 3+ format arguments
    ($variant:ident, $msg:expr, $arg1:expr, $arg2:expr, $arg3:expr, $($arg:expr),*) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg1, $arg2, $arg3, $($arg),*),
            ctx: $crate::ErrorContext { source: None, span: None, help: None, related: vec![] },
            source: None,
        }
    };
    // Message with exactly 2 format arguments
    ($variant:ident, $msg:expr, $arg1:expr, $arg2:expr) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg1, $arg2),
            ctx: $crate::ErrorContext { source: None, span: None, help: None, related: vec![] },
            source: None,
        }
    };
    // Message with single format argument
    ($variant:ident, $msg:expr, $arg:expr) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg),
            ctx: $crate::ErrorContext { source: None, span: None, help: None, related: vec![] },
            source: None,
        }
    };
    // Message only
    ($variant:ident, $msg:expr) => {
        $crate::SutraError::$variant {
            message: format!("{}", $msg),
            ctx: $crate::ErrorContext { source: None, span: None, help: None, related: vec![] },
            source: None,
        }
    };
}

/// Constructs a SutraError variant with a formatted message and context (source, span, help, related labels, or context struct).
///
/// Use this macro for errors that require string-based source, span, optional help context, and optional related labels.
///
/// Example with related labels:
///   err_ctx!(Validation, "Invalid input", src, span, help, related_labels)
#[macro_export]
macro_rules! err_ctx {
    // Message, src, span, help, related
    ($variant:ident, $msg:expr, $src:expr, $span:expr, $help:expr, $related:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::ErrorContext {
                source: Some($crate::diagnostics::SourceArc::clone($src)),
                span: Some($span),
                help: Some(format!("{}", $help)),
                related: $related,
            },
            source: None,
        }
    };
    // Message, src, span, help
    ($variant:ident, $msg:expr, $src:expr, $span:expr, $help:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::ErrorContext {
                source: Some($crate::diagnostics::SourceArc::clone($src)),
                span: Some($span),
                help: Some(format!("{}", $help)),
                related: vec![],
            },
            source: None,
        }
    };
    // Message, src, span
    ($variant:ident, $msg:expr, $src:expr, $span:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::ErrorContext {
                source: Some($crate::diagnostics::SourceArc::clone($src)),
                span: Some($span),
                help: None,
                related: vec![],
            },
            source: None,
        }
    };
    // Message, src, help (uses Span::default())
    ($variant:ident, $msg:expr, $src:expr, $help:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::ErrorContext {
                source: Some($crate::diagnostics::SourceArc::clone($src)),
                span: Some($Span::default()),
                help: Some(format!("{}", $help)),
                related: vec![],
            },
            source: None,
        }
    };
    // Message, src
    ($variant:ident, $msg:expr, $src:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::ErrorContext {
                source: Some($crate::diagnostics::SourceArc::clone($src)),
                span: None,
                help: None,
                related: vec![],
            },
            source: None,
        }
    };
}

/// Constructs a SutraError variant with a pre-built `NamedSource` and optional related labels.
#[macro_export]
macro_rules! err_src {
    // Message, pre-built source, span, related
    ($variant:ident, $msg:expr, $source:expr, $span:expr, $related:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::ErrorContext {
                source: Some(std::sync::Arc::clone($source)),
                span: Some($span),
                help: None,
                related: $related,
            },
            source: None,
        }
    };
    // Message, pre-built source, span
    ($variant:ident, $msg:expr, $source:expr, $span:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::ErrorContext {
                source: Some(std::sync::Arc::clone($source)),
                span: Some($span),
                help: None,
                related: vec![],
            },
            source: None,
        }
    };
}

#[cfg(test)]
mod diagnostics_tests {
    use miette::{NamedSource, Report};
    use Arc;

    use super::*;

    #[test]
    fn test_multilabel_diagnostics() {
        let src1 = Arc::new(NamedSource::new("file1.sutra", "abc def ghi".to_string()));
        let src2 = Arc::new(NamedSource::new("file2.sutra", "xyz 123 456".to_string()));
        let span1 = Span { start: 0, end: 3 };
        let span2 = Span { start: 4, end: 7 };
        let related = vec![
            RelatedLabel {
                source: src1.clone(),
                span: span1,
                label: "first label".to_string(),
            },
            RelatedLabel {
                source: src2.clone(),
                span: span2,
                label: "second label".to_string(),
            },
        ];
        let ctx = ErrorContext {
            source: Some(src1.clone()),
            span: Some(span1),
            help: Some("This is a help message.".to_string()),
            related,
        };
        let err = SutraError::Validation {
            message: "Validation failed".to_string(),
            ctx,
            source: None,
        };
        let report = Report::new(err);
        let output = format!("{report:?}");
        assert!(output.contains("first label"));
        assert!(output.contains("second label"));
        assert!(output.contains("This is a help message."));
    }

    #[test]
    fn test_error_chaining() {
        let src = Arc::new(NamedSource::new("file.sutra", "abc def".to_string()));
        let span = Span { start: 0, end: 3 };
        let ctx1 = ErrorContext {
            source: Some(src.clone()),
            span: Some(span),
            help: None,
            related: vec![],
        };
        let cause = SutraError::Parse {
            message: "Parse error".to_string(),
            ctx: ctx1,
            source: None,
        };
        let ctx2 = ErrorContext {
            source: Some(src.clone()),
            span: Some(span),
            help: Some("Top-level help".to_string()),
            related: vec![],
        };
        let err = SutraError::Validation {
            message: "Validation failed".to_string(),
            ctx: ctx2,
            source: Some(Box::new(cause)),
        };
        let report = Report::new(err);
        let output = format!("{report:?}");
        assert!(output.contains("Validation failed"));
        assert!(output.contains("Parse error"));
        assert!(output.contains("Top-level help"));
    }

    #[test]
    fn test_edge_cases() {
        // No help, no related, no cause
        let src = Arc::new(NamedSource::new("file.sutra", "abc def".to_string()));
        let span = Span { start: 0, end: 3 };
        let ctx = ErrorContext {
            source: Some(src.clone()),
            span: Some(span),
            help: None,
            related: vec![],
        };
        let err = SutraError::Eval {
            message: "Eval error".to_string(),
            ctx,
            source: None,
        };
        let report = Report::new(err);
        let output = format!("{report:?}");
        assert!(output.contains("Eval error"));
        // With help, no related, no cause
        let ctx = ErrorContext {
            source: Some(src.clone()),
            span: Some(span),
            help: Some("Help!".to_string()),
            related: vec![],
        };
        let err = SutraError::Eval {
            message: "Eval error".to_string(),
            ctx,
            source: None,
        };
        let report = Report::new(err);
        let output = format!("{report:?}");
        assert!(output.contains("Help!"));
        // With related, no help, no cause
        let related = vec![RelatedLabel {
            source: src.clone(),
            span,
            label: "label".to_string(),
        }];
        let ctx = ErrorContext {
            source: Some(src.clone()),
            span: Some(span),
            help: None,
            related,
        };
        let err = SutraError::Eval {
            message: "Eval error".to_string(),
            ctx,
            source: None,
        };
        let report = Report::new(err);
        let output = format!("{report:?}");
        assert!(output.contains("label"));
    }
}
