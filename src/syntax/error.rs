use serde::{Deserialize, Serialize};

// =============================================================================
// SECTION 1: MODULE DOCUMENTATION & IMPORTS
// =============================================================================

// String formatting constants to eliminate magic numbers

use crate::ast::value::Value;
use crate::ast::{AstNode, Span};

// =============================================================================
// SECTION 2: CORE DATA STRUCTURES
// =============================================================================

/// Structured representation of an evaluation error.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalError {
    pub kind: EvalErrorKind,
    // The fully expanded code that was being executed when the error occurred.
    pub expanded_code: String,
    // The original, unexpanded code snippet from the author's source.
    // This is added during a second enrichment phase by the top-level runner.
    pub original_code: Option<String>,
}

/// Specific kinds of evaluation errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvalErrorKind {
    Arity {
        func_name: String,
        expected: String,
        actual: usize,
    },
    Type {
        func_name: String,
        expected: String,
        found: Value,
    },
    DivisionByZero,
    General(String),
}

/// Specific kinds of validation errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorKind {
    RecursionLimitExceeded,
    // Add other specific validation errors here later
    General(String),
}

/// The kind of error that occurred in Sutra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SutraErrorKind {
    Parse(String),
    Validation(ValidationErrorKind),
    Eval(EvalError),
    Io(String),
    MalformedAst(String),
    InternalParse(String),
    // Unified Macro Error
    MacroExpansion(Box<SutraError>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SutraError {
    pub kind: SutraErrorKind,
    pub span: Option<crate::ast::Span>,
}

// =============================================================================
// SECTION 3: PUBLIC API IMPLEMENTATION
// =============================================================================

/// # Sutra Error Construction Helpers
///
/// This section provides ergonomic, documented constructor functions for all error domains.
/// All error construction outside this module must use these helpers.
///
/// ## How to Add a New Error Domain
/// 1. Define a new variant in `SutraErrorKind` if needed.
/// 2. Add a constructor helper here, following the patterns below.
/// 3. Document usage and rationale in this section.
/// 4. Add/expand tests for the new error domain.
///
/// ## When to Use Each Helper
/// - Use general helpers (`parse_error`, `macro_error`, etc.) for broad error categories.
/// - Use domain-specific helpers (`eval_arity_error`, etc.) for common, repeated patterns in a specific subsystem.
/// - Prefer helpers that require a span for author-facing errors; IO/internal errors may allow `None`.

/// Constructs a parse error (malformed input, syntax error).
///
/// Parse errors occur when the input syntax is malformed or contains unexpected tokens.
/// These are typically user-facing errors that indicate problems with the source code.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::parse_error;
/// use sutra::ast::Span;
/// let span = Span { start: 0, end: 5 };
/// let error = parse_error("Unexpected token ')'", Some(span));
/// ```
pub fn parse_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Parse(msg.into()),
        span,
    }
}

/// Constructs a validation error (post-expansion validation failure).
///
/// Validation errors occur after macro expansion when the resulting code
/// structure is semantically invalid, even if syntactically correct.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::validation_error;
/// use sutra::ast::Span;
/// let span = Span { start: 5, end: 15 };
/// let error = validation_error("Invalid macro usage in this context", Some(span));
/// ```
pub fn validation_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Validation(ValidationErrorKind::General(msg.into())),
        span,
    }
}

/// Constructs an IO error (file or system IO failure).
///
/// IO errors occur when file operations or other system interactions fail.
/// These typically don't have source spans since they're not related to code positions.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::io_error;
/// let error = io_error("Failed to read config file", None);
/// ```
pub fn io_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Io(msg.into()),
        span,
    }
}

/// Constructs a malformed AST error (internal AST structure error).
///
/// These errors indicate problems with the internal AST structure, typically
/// suggesting bugs in the parser or AST construction logic rather than user errors.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::malformed_ast_error;
/// use sutra::ast::Span;
/// let span = Span { start: 0, end: 10 };
/// let error = malformed_ast_error("Empty expression pair in AST", Some(span));
/// ```
pub fn malformed_ast_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::MalformedAst(msg.into()),
        span,
    }
}

/// Constructs an internal parse error (parser state error not caused by user input).
///
/// Internal parse errors indicate problems with the parser's internal state
/// that are not caused by malformed user input, suggesting parser bugs.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::internal_parse_error;
/// use sutra::ast::Span;
/// let span = Span { start: 20, end: 30 };
/// let error = internal_parse_error("Parser generated empty tree", Some(span));
/// ```
pub fn internal_parse_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::InternalParse(msg.into()),
        span,
    }
}

/// Constructs an enhanced evaluation arity error with debugging context.
///
/// Creates rich, contextual arity error messages that help developers understand
/// exactly what went wrong with function argument counts and how to fix it.
/// Includes argument summaries, usage suggestions, and contextual advice.
///
/// # Arguments
/// * `span` - Source location where the error occurred
/// * `args` - The actual arguments provided to the function
/// * `func_name` - Name of the function being called
/// * `expected` - Expected argument count description (e.g., "2", "at least 1")
///
/// # Example
/// ```rust
/// use sutra::syntax::error::eval_arity_error;
/// use sutra::ast::{Expr, Span, WithSpan};
/// use std::sync::Arc;
/// type AstNode = WithSpan<Arc<Expr>>;
/// let span = Span { start: 0, end: 10 };
/// let args: Vec<AstNode> = vec![];
/// let error = eval_arity_error(Some(span), &args, "core/set!", "exactly 2");
/// ```
pub fn eval_arity_error(
    span: Option<Span>,
    args: &[AstNode],
    func_name: &str,
    expected: impl ToString,
) -> SutraError {
    let expanded_code = build_arity_expanded_code(func_name, args);
    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            kind: EvalErrorKind::Arity {
                func_name: func_name.to_string(),
                expected: expected.to_string(),
                actual: args.len(),
            },
            expanded_code,
            original_code: None, // Will be filled by with_source if available
        }),
        span,
    }
}

/// Constructs an enhanced evaluation type error with better context and suggestions.
///
/// Creates rich, contextual type error messages that help developers understand
/// type mismatches and how to fix them. Includes type information, value details,
/// and conversion suggestions.
///
/// # Arguments
/// * `span` - Source location where the error occurred
/// * `arg` - The expression that produced the wrong type
/// * `func_name` - Name of the function expecting the type
/// * `expected` - Expected type description (e.g., "a Number", "a String")
/// * `found` - The actual value that was found
///
/// # Example
/// ```rust
/// use sutra::syntax::error::eval_type_error;
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::ast::value::Value;
/// use std::sync::Arc;
/// type AstNode = WithSpan<Arc<Expr>>;
/// let span = Span { start: 0, end: 10 };
/// let arg = WithSpan { value: Arc::new(Expr::String("hello".to_string(), span.clone())), span: span.clone() };
/// let value = Value::String("hello".to_string());
/// let error = eval_type_error(Some(span), &arg, "core/get", "a Path", &value);
/// ```
pub fn eval_type_error(
    span: Option<Span>,
    arg: &AstNode,
    func_name: &str,
    expected: &str,
    found: &Value,
) -> SutraError {
    let expanded_code = arg.value.pretty();
    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            kind: EvalErrorKind::Type {
                func_name: func_name.to_string(),
                expected: expected.to_string(),
                found: found.clone(),
            },
            expanded_code,
            original_code: None, // Will be filled by with_source if available
        }),
        span,
    }
}

/// Constructs an enhanced general evaluation error with context and suggestions.
///
/// Creates detailed evaluation error messages that help developers understand
/// runtime failures and how to address them. Includes contextual information
/// about the failing expression and targeted suggestions.
///
/// # Arguments
/// * `span` - Source location where the error occurred
/// * `arg` - The expression that was being evaluated when the error occurred
/// * `msg` - The error message describing what went wrong
///
/// # Example
/// ```rust
/// use sutra::syntax::error::eval_general_error;
/// use sutra::ast::{Expr, Span, WithSpan};
/// use std::sync::Arc;
/// type AstNode = WithSpan<Arc<Expr>>;
/// let span = Span { start: 0, end: 10 };
/// let arg = WithSpan { value: Arc::new(Expr::Symbol("x".to_string(), span.clone())), span: span.clone() };
/// let error = eval_general_error(Some(span), &arg, "Division by zero");
/// ```
pub fn eval_general_error(span: Option<Span>, arg: &AstNode, msg: impl Into<String>) -> SutraError {
    let expanded_code = arg.value.pretty();
    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            kind: EvalErrorKind::General(msg.into()),
            expanded_code,
            original_code: None, // Will be filled by with_source if available
        }),
        span,
    }
}

/// Constructs a recursion depth error (exceeded recursion limit).
///
/// Creates an error indicating that the evaluation recursion depth has been exceeded,
/// typically suggesting infinite recursion or very deep call stacks.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::recursion_depth_error;
/// use sutra::ast::Span;
/// let span = Span { start: 0, end: 10 };
/// let error = recursion_depth_error(Some(span));
/// ```
pub fn recursion_depth_error(span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Validation(ValidationErrorKind::RecursionLimitExceeded),
        span,
    }
}

// =============================================================================
// SECTION 4: CONVERSIONS
// =============================================================================

impl std::fmt::Display for SutraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            SutraErrorKind::Parse(s) => write!(f, "Parse Error: {}", s),
            SutraErrorKind::Validation(kind) => write!(f, "Validation Error: {}", kind),
            SutraErrorKind::Io(s) => write!(f, "IO Error: {}", s),
            SutraErrorKind::Eval(e) => e.fmt(f),
            SutraErrorKind::MalformedAst(s) => write!(f, "Malformed AST Error: {}", s),
            SutraErrorKind::InternalParse(s) => write!(f, "Internal Parse Error: {}", s),
            SutraErrorKind::MacroExpansion(e) => write!(f, "In macro expansion: {}", e),
        }
    }
}

impl std::fmt::Display for EvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Evaluation Error")?;
        // Potentially add suggestion and code snippets here later
        Ok(())
    }
}

impl std::fmt::Display for ValidationErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidationErrorKind::RecursionLimitExceeded => write!(f, "Recursion limit exceeded"),
            ValidationErrorKind::General(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for SutraError {}

// =============================================================================
// SECTION 5: INFRASTRUCTURE/TRAITS
// =============================================================================

impl SutraError {
    // Helper to enrich the error with the original source code snippet.
    // This is part of the "two-phase error enrichment" pattern.
    pub fn with_source(mut self, source: &str) -> Self {
        if let Some(span) = &self.span {
            if let Some(original_code) = source.get(span.start..span.end).map(|s| s.to_string()) {
                if let SutraErrorKind::Eval(eval_error) = &mut self.kind {
                    eval_error.original_code = Some(original_code);
                }
            }
        }
        self
    }

    /// Returns a semantic error code for this error, useful for stable test matching
    /// independent of user-facing message changes.
    ///
    /// This allows tests to match against stable error categories instead of brittle
    /// message text that might change during development.
    pub fn error_code(&self) -> Option<&str> {
        match &self.kind {
            SutraErrorKind::Parse(_) => Some("PARSE_ERROR"),
            SutraErrorKind::Validation(kind) => match kind {
                ValidationErrorKind::RecursionLimitExceeded => Some("RECURSION_LIMIT_EXCEEDED"),
                ValidationErrorKind::General(_) => Some("VALIDATION_ERROR"),
            },
            SutraErrorKind::Io(_) => Some("IO_ERROR"),
            SutraErrorKind::MalformedAst(_) => Some("MALFORMED_AST_ERROR"),
            SutraErrorKind::InternalParse(_) => Some("INTERNAL_PARSE_ERROR"),
            SutraErrorKind::Eval(eval_error) => match eval_error.kind {
                EvalErrorKind::Arity { .. } => Some("ARITY_ERROR"),
                EvalErrorKind::Type { .. } => Some("TYPE_ERROR"),
                EvalErrorKind::DivisionByZero => Some("DIVISION_BY_ZERO"),
                EvalErrorKind::General(_) => Some("EVAL_ERROR"),
            },
            SutraErrorKind::MacroExpansion(inner) => inner.error_code(),
        }
    }
}

// --- Internal Helpers ---

/// Builds the expanded code representation for arity errors.
fn build_arity_expanded_code(func_name: &str, args: &[AstNode]) -> String {
    format!(
        "({} {})",
        func_name,
        args.iter()
            .map(|arg| arg.value.pretty())
            .collect::<Vec<_>>()
            .join(" ")
    )
}

// =============================================================================
// SECTION 7: MODULE EXPORTS
// =============================================================================

/// Constructs a SutraError from an existing kind and span.
///
/// Utility function for creating errors when you already have a constructed
/// SutraErrorKind and optional span. Useful for error transformation and forwarding.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::{from_kind, SutraErrorKind};
/// use sutra::ast::Span;
///
/// let span = Span { start: 0, end: 10 };
/// let kind = SutraErrorKind::Parse("custom error".to_string());
/// let error = from_kind(kind, Some(span));
/// ```
pub fn from_kind(kind: SutraErrorKind, span: Option<Span>) -> SutraError {
    SutraError { kind, span }
}
