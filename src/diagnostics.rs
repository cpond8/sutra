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
//! - **Use `err_msg!` for message-only errors (no context):**
//!   - Example: `return Err(err_msg!(Parse, "Unexpected token"));`
//!
//! - **Use `err_ctx!` for errors with context (source, span, help, or pre-built context):**
//!   - Example: `return Err(err_ctx!(Parse, "Unexpected token", src, span));`
//!   - Example: `return Err(err_ctx!(Validation, "Invalid input", ctx));` (where `ctx` is an `ErrorContext`)
//!   - Example: `return Err(err_ctx!(TypeError, "Expected number", src, span, "Numbers only allowed here"));`
//!   - Example: `return Err(err_ctx!(DivisionByZero, src, span));`
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
//! ## Example Usage
//!
//! ```rust
//! // Message only (no context)
//! return Err(err_msg!(Parse, "Unexpected token"));
//!
//! // Message + source + span
//! return Err(err_ctx!(Parse, "Unexpected token", src, span));
//!
//! // Message + context (context must have .source field)
//! return Err(err_ctx!(Validation, "Invalid input", ctx));
//!
//! // Context only (no message)
//! return Err(err_ctx!(DivisionByZero, src, span));
//!
//! // Message + help + context
//! return Err(err_ctx!(TypeError, "Expected number", src, span, "Numbers only allowed here"));
//!
//! // Internal error, message only
//! return Err(err_msg!(Internal, "Invariant violated"));
//! ```
//!
//! See the macro and type docs below for details.

use std::sync::Arc;
use miette::Diagnostic;
use thiserror::Error;
use crate::ast::Span;

/// Encapsulates all diagnostic context for a Sutra error, including optional source, span: *span, and help message.
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Optional source code for error highlighting.
    pub src: Option<Arc<String>>,
    /// Optional span within the source for precise error location.
    pub span: Option<Span>,
    /// Optional help message for user guidance.
    pub help: Option<String>,
}

impl ErrorContext {
    /// Returns an empty error context (no source, span: *span, or help).
    pub fn none() -> Self {
        Self { src: None, span: None, help: None }
    }
    /// Creates a context with only a source.
    pub fn with_src(src: Arc<String>) -> Self {
        Self { src: Some(src), span: None, help: None }
    }
    /// Creates a context with only a span.
    pub fn with_span(span: Span) -> Self {
        Self { src: None, span: Some(span), help: None }
    }
    /// Creates a context with both source and span.
    pub fn with_src_and_span(src: Arc<String>, span: Span) -> Self {
        Self { src: Some(src), span: Some(span), help: None }
    }
    /// Creates a context with source, span: *span, and help message.
    pub fn with_all(src: Arc<String>, span: Span, help: String) -> Self {
        Self { src: Some(src), span: Some(span), help: Some(help) }
    }
}

/// Unified error type for all Sutra engine failure modes, supporting rich diagnostics and ergonomic construction via the err_msg! and err_ctx! macros.
#[derive(Debug, Error, Diagnostic)]
pub enum SutraError {
    #[error("Parse error: {message}")]
    #[diagnostic(code(sutra::parse))]
    Parse {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Validation error: {message}")]
    #[diagnostic(code(sutra::validation))]
    Validation {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Evaluation error: {message}")]
    #[diagnostic(code(sutra::eval))]
    Eval {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Type error: {message}")]
    #[diagnostic(code(sutra::type_error))]
    TypeError {
        message: String,
        ctx: ErrorContext,
    },
    #[error("Division by zero")]
    #[diagnostic(code(sutra::division_by_zero))]
    DivisionByZero {
        ctx: ErrorContext,
    },
    #[error("Internal error: {message}")]
    #[diagnostic(code(sutra::internal))]
    Internal {
        message: String,
        ctx: ErrorContext,
    },
    // Add more variants as needed
}

/// Trait for extracting a string source from various types for error context.
pub trait AsErrorSource {
    fn as_error_source(&self) -> String;
}

impl AsErrorSource for String {
    fn as_error_source(&self) -> String { self.clone() }
}
impl AsErrorSource for &str {
    fn as_error_source(&self) -> String { self.to_string() }
}
impl AsErrorSource for std::sync::Arc<String> {
    fn as_error_source(&self) -> String { self.as_ref().clone() }
}

impl<T: AsErrorSource + ?Sized> AsErrorSource for &mut T {
    fn as_error_source(&self) -> String {
        (**self).as_error_source()
    }
}

/// Converts any AsErrorSource to Arc<String> for use in error context.
pub fn to_error_src<S: AsErrorSource>(src: S) -> std::sync::Arc<String> {
    std::sync::Arc::new(src.as_error_source())
}

/// Constructs a SutraError variant with a formatted message and no context.
///
/// Use this macro for errors that do not require source, span, or help context. Supports formatting with multiple arguments.
///
/// # Example
/// ```rust
/// return Err(err_msg!(Parse, "Unexpected token: {}", token));
/// ```
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

/// Constructs a SutraError variant with context (source, span, help, or pre-built context).
///
/// Use this macro for errors that require additional diagnostic context, such as source code, span, help messages, or a pre-built ErrorContext. The macro automatically wraps and converts source types as needed.
///
/// # Example
/// ```rust
/// return Err(err_ctx!(Parse, "Unexpected token", src, span));
/// return Err(err_ctx!(TypeError, "Expected number", src, span, "Numbers only allowed here"));
/// ```
#[macro_export]
macro_rules! err_ctx {
    // Message, src, span, help
    ($variant:ident, $msg:expr, $src:expr, $span:expr, $help:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                src: Some($crate::diagnostics::to_error_src($src)),
                span: Some($span),
                help: Some(format!("{}", $help)),
            },
        }
    };
    // Message, src, help (uses Span::default())
    ($variant:ident, $msg:expr, $src:expr, $help:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                src: Some($crate::diagnostics::to_error_src($src)),
                span: Some($crate::ast::Span::default()),
                help: Some(format!("{}", $help)),
            },
        }
    };
    // Message, src, span
    ($variant:ident, $msg:expr, $src:expr, $span:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                src: Some($crate::diagnostics::to_error_src($src)),
                span: Some($span),
                help: None,
            },
        }
    };
    // Message, src
    ($variant:ident, $msg:expr, $src:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                src: Some($crate::diagnostics::to_error_src($src)),
                span: None,
                help: None,
            },
        }
    };
    // Pre-built context
    ($variant:ident, $ctx:expr) => {
        $crate::SutraError::$variant {
            message: String::new(),
            ctx: $ctx,
        }
    };
    // Unit variant (no message, no context)
    ($variant:ident) => {
        $crate::SutraError::$variant {
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
}
