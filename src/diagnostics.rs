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
//!

use std::sync::Arc;
use miette::{Diagnostic, NamedSource};
use thiserror::Error;
use crate::ast::Span;

/// Encapsulates all diagnostic context for a Sutra error, including optional source, span, and help message.
#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    /// Optional source code for error highlighting.
    pub source: Option<Arc<NamedSource<String>>>,
    /// Optional raw source text for direct access (for error reporting).
    pub source_text: Option<Arc<String>>,
    /// Optional span within the source for precise error location.
    pub span: Option<Span>,
    /// Optional help message for user guidance.
    pub help: Option<String>,
}

impl ErrorContext {
    /// Returns an empty error context (no source, span, or help).
    pub fn none() -> Self {
        Self::default()
    }

    /// Creates a context with only a source.
    pub fn with_source(source: Arc<NamedSource<String>>) -> Self {
        Self { source: Some(source), source_text: None, ..Default::default() }
    }

    /// Creates a context with only a span.
    pub fn with_span(span: Span) -> Self {
        Self { span: Some(span), source_text: None, ..Default::default() }
    }

    /// Creates a context with both source and span.
    pub fn with_source_and_span(source: Arc<NamedSource<String>>, span: Span) -> Self {
        Self { source: Some(source), span: Some(span), source_text: None, ..Default::default() }
    }

    /// Creates a context with source, span, and help message.
    pub fn with_all(source: Arc<NamedSource<String>>, span: Span, help: String) -> Self {
        Self { source: Some(source), span: Some(span), help: Some(help), source_text: None }
    }
}

/// Unified error type for all Sutra engine failure modes.
#[derive(Debug, Error, Clone)]
pub enum SutraError {
    #[error("Parse error: {message}")]
    Parse {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Validation error: {message}")]
    Validation {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Evaluation error: {message}")]
    Eval {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Type error: {message}")]
    TypeError {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Division by zero")]
    DivisionByZero {
        ctx: ErrorContext,
    },
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Test failure: {message}")]
    TestFailure {
        message: String,
        ctx: ErrorContext,
    },
}

impl SutraError {
    fn get_ctx(&self) -> &ErrorContext {
        match self {
            SutraError::Parse { ctx, .. } => ctx,
            SutraError::Validation { ctx, .. } => ctx,
            SutraError::Eval { ctx, .. } => ctx,
            SutraError::TypeError { ctx, .. } => ctx,
            SutraError::DivisionByZero { ctx } => ctx,
            SutraError::Internal { ctx, .. } => ctx,
            SutraError::TestFailure { ctx, .. } => ctx,
        }
    }
}

impl Diagnostic for SutraError {
    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        let code: &str = match self {
            SutraError::Parse { .. } => "sutra::parse",
            SutraError::Validation { .. } => "sutra::validation",
            SutraError::Eval { .. } => "sutra::eval",
            SutraError::TypeError { .. } => "sutra::type_error",
            SutraError::DivisionByZero { .. } => "sutra::division_by_zero",
            SutraError::Internal { .. } => "sutra::internal",
            SutraError::TestFailure { .. } => "sutra::test_failure",
        };
        Some(Box::new(code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        self.get_ctx().help.as_ref().map(|h| Box::new(h) as Box<dyn std::fmt::Display + 'a>)
    }

    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        self.get_ctx().source.as_ref().map(|s| s.as_ref() as &dyn miette::SourceCode)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let ctx = self.get_ctx();
        if let Some(span) = ctx.span {
            let text = match self {
                SutraError::Parse { message, .. } => Some(message.clone()),
                SutraError::Validation { message, .. } => Some(message.clone()),
                SutraError::Eval { message, .. } => Some(message.clone()),
                SutraError::TypeError { message, .. } => Some(message.clone()),
                SutraError::DivisionByZero { .. } => Some("division by zero".to_string()),
                SutraError::Internal { message, .. } => Some(message.clone()),
                SutraError::TestFailure { message, .. } => Some(message.clone()),
            };
            let len = if span.end > span.start { span.end - span.start } else { 1 };
            let labels = vec![miette::LabeledSpan::new(text, span.start, len)];
            return Some(Box::new(labels.into_iter()));
        }
        None
    }
}

/// Converts a source string into an `Arc<NamedSource<String>>` for use in error contexts.
pub fn to_error_source<S: AsRef<str>>(source: S) -> Arc<NamedSource<String>> {
    Arc::new(NamedSource::new("source", source.as_ref().to_string()))
}

/// Constructs a SutraError variant with a formatted message and no context.
///
/// Use this macro for errors that do not require source, span, or help context. Supports formatting with multiple arguments.
///
#[macro_export]
macro_rules! err_msg {
    // Message with 3+ format arguments
    ($variant:ident, $msg:expr, $arg1:expr, $arg2:expr, $arg3:expr, $($arg:expr),*) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg1, $arg2, $arg3, $($arg),*),
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
    // Message with exactly 2 format arguments
    ($variant:ident, $msg:expr, $arg1:expr, $arg2:expr) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg1, $arg2),
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
    // Message with single format argument
    ($variant:ident, $msg:expr, $arg:expr) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg),
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
    // Message only
    ($variant:ident, $msg:expr) => {
        $crate::SutraError::$variant {
            message: format!("{}", $msg),
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
}

/// Constructs a SutraError variant with a formatted message and context (source, span, help, or context struct).
///
/// Use this macro for errors that require string-based source, span, and optional help context.
///
#[macro_export]
macro_rules! err_ctx {
    // Message, src, span, help
    ($variant:ident, $msg:expr, $src:expr, $span:expr, $help:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                source: Some($crate::diagnostics::to_error_source($src)),
                span: Some($span),
                help: Some(format!("{}", $help)),
                source_text: None,
            },
        }
    };
    // Message, src, span
    ($variant:ident, $msg:expr, $src:expr, $span:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                source: Some($crate::diagnostics::to_error_source($src)),
                span: Some($span),
                help: None,
                source_text: None,
            },
        }
    };
    // Message, src, help (uses Span::default())
    ($variant:ident, $msg:expr, $src:expr, $help:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                source: Some($crate::diagnostics::to_error_source($src)),
                span: Some($crate::ast::Span::default()),
                help: Some(format!("{}", $help)),
                source_text: None,
            },
        }
    };
    // Message, src
    ($variant:ident, $msg:expr, $src:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                source: Some($crate::diagnostics::to_error_source($src)),
                span: None,
                help: None,
                source_text: None,
            },
        }
    };
}

/// Constructs a SutraError variant with a pre-built `NamedSource`.
#[macro_export]
macro_rules! err_src {
    // Message, pre-built source, span
    ($variant:ident, $msg:expr, $source:expr, $span:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                source: Some(std::sync::Arc::clone($source)),
                span: Some($span),
                help: None,
                source_text: None,
            },
        }
    };
}
