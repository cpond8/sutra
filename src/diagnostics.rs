//!
//! ****************************************************************************************
//! ** CRITICAL: BOILERPLATE ELIMINATION RULES FOR sutra_err! MACRO USAGE                 **
//! ****************************************************************************************
//!
//! - **Do not construct `ErrorContext` manually unless absolutely necessary.**
//!   The macro is designed to eliminate the need for explicit context construction in almost all cases.
//!
//! - **Pass `src` and `span` directly:**
//!   Use `sutra_err!(..., ..., src, span)` whenever you have both.
//!
//! - **Pass a context struct only if it has a `.source` field:**
//!   Use `sutra_err!(..., ..., ctx)` if you have a context.
//!
//! - **Never use `.clone()` or `.into()` on `span` or `src` in a `sutra_err!` call.**
//!   The macro will handle cloning and conversion internally. Just pass the variable.
//!
//! - **Never pass a `usize`, integer, or primitive as a span.**
//!   Always construct and pass a `Span` struct: `Span { start: pos, end: pos }` or similar.
//!
//! - **Never pass an `ErrorContext` unless you are using a context-only macro arm.**
//!   If you have both `src` and `span`, always use the direct macro arm: `sutra_err!(..., ..., src, span)`.
//!
//! - **If you need to attach a help message, use the macro arm that accepts it.**
//!   Example: `sutra_err!(..., ..., src, span: *span, help)`
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
//! This module defines the unified, `miette`-based diagnostic system for the Sutra
//! engine. All errors produced by any stage of the compilation or evaluation
//! pipeline are represented by the types in this module.
//!
//! ## Canonical Unified Error System (15 Jul 2025)
//!
//! - All error construction should use the `sutra_err!` macro for minimalism and compositionality.
//! - The `ErrorContext` struct encapsulates all diagnostic context (source, span: *span, help).
//! - The macro automatically handles `Arc`, cloning, and context extraction—users do not need to wrap or clone sources themselves.
//! - Example usage:
//!   // Message only (no context)
//!   return Err(sutra_err!(Parse, "Unexpected token"));
//!
//!   // Message + source + span
//!   return Err(sutra_err!(Parse, "Unexpected token", src, span));
//!
//!   // Message + context (context must have .source field)
//!   return Err(sutra_err!(Validation, "Invalid input", ctx));
//!
//!   // Context only (no message)
//!   return Err(sutra_err!(DivisionByZero, src, span));
//!
//!   // Message + help + context
//!   return Err(sutra_err!(TypeError, "Expected number", src, span: *span, "Numbers only allowed here"));
//!
//!   // Internal error, message only
//!   return Err(sutra_err!(Internal, "Invariant violated"));
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

/// Unified error type for all Sutra engine failure modes, supporting rich diagnostics and ergonomic construction via the sutra_err! macro.
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

/// Macro for ergonomic construction of SutraError variants.
///
/// # Usage patterns
///
/// ```rust
/// sutra_err!(Variant, "msg", args...);         // message with format args
/// sutra_err!(Variant, "msg", src, span: *span, help); // src: any AsErrorSource (string/&str/context)
/// sutra_err!(Variant, "msg", src, span);       // src: any AsErrorSource (string/&str/context)
/// sutra_err!(Variant, "msg", ctx);             // ctx: ErrorContext
/// sutra_err!(Variant, "msg");                  // message only
/// sutra_err!(Variant, src, span);              // for variants without message field
/// sutra_err!(Variant);                         // for variants without message field
/// ```
///
/// The macro automatically handles string formatting and uses trait-based dispatch for source types.
#[macro_export]
macro_rules! sutra_err {
    // message with 3+ format arguments (must come first)
    ($variant:ident, $msg:expr, $arg1:expr, $arg2:expr, $arg3:expr, $($arg:expr),*) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg1, $arg2, $arg3, $($arg),*),
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
    // message with exactly 2 format arguments
    ($variant:ident, $msg:expr, $arg1:expr, $arg2:expr) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg1, $arg2),
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
    // message, src, span: *span, help - src can be any AsErrorSource
    ($variant:ident, $msg:expr, $src:expr, $span:expr, $help:expr) => {
        $crate::SutraError::$variant {
            message: $msg.to_string(),
            ctx: $crate::diagnostics::ErrorContext {
                src: Some($crate::diagnostics::to_error_src($src)),
                span: Some($span),
                help: Some($help.to_string()),
            },
        }
    };
    // message, src, span - src can be any AsErrorSource (strings OR context structs)
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
    // message with single format argument
    ($variant:ident, $msg:expr, $arg:expr) => {
        $crate::SutraError::$variant {
            message: format!($msg, $arg),
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
    // message only
    ($variant:ident, $msg:expr) => {
        $crate::SutraError::$variant {
            message: $msg, // note that .to_string() is not called here
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
    // no message, no context - for DivisionByZero only
    ($variant:ident) => {
        $crate::SutraError::$variant {
            ctx: $crate::diagnostics::ErrorContext::none(),
        }
    };
}
