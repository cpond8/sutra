//! Error handling for macro operations.
//!
//! Provides `SutraMacroError` for structured macro error reporting, preserving context and actionable suggestions. Errors flow upward, maintaining span and error chain information for debugging and user feedback.

use crate::ast::Span;
use crate::syntax::error::{SutraError, SutraErrorKind};
use serde::{Deserialize, Serialize};

// =============================
// Macro error type and variants
// =============================

/// Macro expansion errors with contextual information.
///
/// Preserves structured error information and context for debugging macro failures.
/// Use for reporting expansion errors and recursion limit violations.
///
/// Example:
/// ```rust
/// use sutra::macros::SutraMacroError;
/// use sutra::ast::Span;
/// let err = SutraMacroError::RecursionLimit { span: Span { start: 0, end: 1 }, macro_name: "foo".to_string() };
/// match err {
///     SutraMacroError::Expansion { .. } => {},
///     SutraMacroError::RecursionLimit { .. } => {},
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SutraMacroError {
    /// Error during macro expansion with preserved context
    Expansion {
        /// Span of the macro call that failed
        span: Span,
        /// Name of the macro that failed
        macro_name: String,
        /// Human-readable error message
        message: String,
        /// Preserved source error for structured information access.
        /// Contains the original error kind that caused the macro expansion failure.
        source_error_kind: Option<SutraErrorKind>,
        /// Optional suggestion from the original error for better debugging.
        /// Extracted from EvalError when available to provide actionable guidance.
        suggestion: Option<String>,
        /// Original span from the source error, if different from macro call span.
        /// Helps pinpoint the exact location where the underlying error occurred.
        source_span: Option<Span>,
    },
    /// Macro recursion limit exceeded
    RecursionLimit {
        /// Span where recursion limit was exceeded
        span: Span,
        /// Name of the macro that exceeded the limit
        macro_name: String,
    },
}

impl std::fmt::Display for SutraMacroError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SutraMacroError::Expansion {
                macro_name,
                message,
                suggestion,
                ..
            } => {
                write!(f, "Macro '{}' expansion failed: {}", macro_name, message)?;
                if let Some(suggestion) = suggestion {
                    write!(f, "\nSuggestion: {}", suggestion)?;
                }
                Ok(())
            }
            SutraMacroError::RecursionLimit { macro_name, .. } => {
                write!(
                    f,
                    "Macro '{}' recursion limit ({}) exceeded",
                    macro_name,
                    crate::macros::types::MAX_MACRO_RECURSION_DEPTH
                )
            }
        }
    }
}

impl std::error::Error for SutraMacroError {}

// =============================
// Error conversion utilities
// =============================

/// Converts a `SutraError` into a macro expansion error, preserving context and suggestions.
/// Use when macro expansion fails due to a parsing or evaluation error.
pub fn expansion_error_from_sutra_error(
    span: &Span,
    macro_name: &str,
    error: SutraError,
) -> SutraMacroError {
    // Extract suggestion if available
    let suggestion = match &error.kind {
        SutraErrorKind::Eval(eval_error) => eval_error.suggestion.clone(),
        _ => None,
    };

    // Compose human-readable message
    let enhanced_message = format!(
        "Macro expansion failed: {}{}",
        error,
        error
            .span
            .as_ref()
            .map_or(String::new(), |s| format!(" (at {}:{})", s.start, s.end))
    );

    SutraMacroError::Expansion {
        span: span.clone(),
        macro_name: macro_name.to_string(),
        message: enhanced_message,
        source_error_kind: Some(error.kind),
        suggestion,
        source_span: error.span,
    }
}

/// Constructs a `RecursionLimit` error for macro expansion.
/// Used when a macro exceeds the allowed recursion depth.
pub fn macro_recursion_limit_error(span: &Span, macro_name: &str) -> SutraMacroError {
    SutraMacroError::RecursionLimit {
        span: span.clone(),
        macro_name: macro_name.to_string(),
    }
}

// =============================
// Arity error helpers
// =============================

/// Creates a detailed arity error for macro calls, with context and suggestions.
pub fn enhanced_macro_arity_error(
    args_len: usize,
    params: &crate::ast::ParamList,
    span: &Span,
) -> SutraError {
    let required_len = params.required.len();
    let has_variadic = params.rest.is_some();

    let main_message = "Macro arity mismatch";
    let context_message = build_arity_context_message(args_len, required_len, has_variadic);
    let param_info = build_param_info_string(params);
    let suggestion = build_arity_suggestion(args_len, required_len, has_variadic, &param_info);

    let full_message = format!("{}\n\n{}\n\n{}", main_message, context_message, param_info);

    use crate::syntax::error::{EvalError, SutraError, SutraErrorKind};
    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            message: full_message,
            expanded_code: format!("<macro call with {} arguments>", args_len),
            original_code: None,
            suggestion: Some(suggestion),
        }),
        span: Some(span.clone()),
    }
}

// =============================
// Private arity helpers
// =============================

// Builds arity error context message.
fn build_arity_context_message(args_len: usize, required_len: usize, has_variadic: bool) -> String {
    // Handle variadic case early
    if has_variadic {
        return format!(
            "Expected at least {} arguments, but received {}. This macro accepts additional arguments via '...' parameter.",
            required_len, args_len
        );
    }

    // Exact argument count required
    format!(
        "Expected exactly {} arguments, but received {}. This macro requires a specific number of arguments.",
        required_len, args_len
    )
}

// Builds parameter info string for macro arity errors.
fn build_param_info_string(params: &crate::ast::ParamList) -> String {
    format!(
        "Macro parameters: {}{}",
        params.required.join(", "),
        if let Some(rest) = &params.rest {
            format!(" ...{}", rest)
        } else {
            String::new()
        }
    )
}

// Helper for pluralizing argument count messages.
fn pluralize_args(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

// Generates arity error suggestions for macro calls.
fn build_arity_suggestion(
    args_len: usize,
    required_len: usize,
    has_variadic: bool,
    param_info: &str,
) -> String {
    // Too few arguments
    if args_len < required_len {
        let missing = required_len - args_len;
        return format!(
            "Add {} more argument{} to match the macro definition: {}",
            missing,
            pluralize_args(missing),
            param_info
        );
    }

    // Too many arguments (non-variadic)
    if args_len > required_len && !has_variadic {
        let extra = args_len - required_len;
        return format!(
            "Remove {} argument{} - this macro only accepts {} arguments: {}",
            extra,
            pluralize_args(extra),
            required_len,
            param_info
        );
    }

    // General mismatch
    format!(
        "Check the macro definition and ensure arguments match: {}",
        param_info
    )
}

// =============================
// Recursion depth helpers
// =============================

/// Checks recursion depth limit and returns an error if exceeded (generic).
///
/// Used for enforcing recursion limits in macro expansion and error reporting.
pub fn check_recursion_depth_generic<E>(
    depth: usize,
    span: &Span,
    error_fn: impl FnOnce(&Span) -> E,
) -> Result<(), E> {
    if depth > crate::macros::types::MAX_MACRO_RECURSION_DEPTH {
        return Err(error_fn(span));
    }
    Ok(())
}

/// Checks recursion depth for `SutraError` context.
/// Returns `Err(SutraError)` if limit exceeded.
pub fn check_recursion_depth(depth: usize, span: &Span, context: &str) -> Result<(), SutraError> {
    check_recursion_depth_generic(depth, span, |span| {
        crate::syntax::error::macro_error(
            format!(
                "{} recursion limit ({}) exceeded.",
                context,
                crate::macros::types::MAX_MACRO_RECURSION_DEPTH
            ),
            Some(span.clone()),
        )
    })
}

/// Checks recursion depth for `SutraMacroError` context.
/// Returns `Err(SutraMacroError)` if limit exceeded.
pub fn check_macro_recursion_depth(
    depth: usize,
    span: &Span,
    macro_name: Option<&str>,
) -> Result<(), SutraMacroError> {
    check_recursion_depth_generic(depth, span, |span| {
        macro_recursion_limit_error(span, macro_name.unwrap_or("<unknown>"))
    })
}
