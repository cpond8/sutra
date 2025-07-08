use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalError {
    pub message: String,
    // The fully expanded code that was being executed when the error occurred.
    pub expanded_code: String,
    // The original, unexpanded code snippet from the author's source.
    // This is added during a second enrichment phase by the top-level runner.
    pub original_code: Option<String>,
    pub suggestion: Option<String>,
}

/// The kind of error that occurred in Sutra.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SutraErrorKind {
    Parse(String), // User-facing parse errors (malformed input, syntax error)
    Macro(String),
    Validation(String),
    Eval(EvalError),
    Io(String),
    // New error kinds for parser internal logic errors
    MalformedAst(String), // Unexpected AST structure, likely a bug or grammar mismatch
    InternalParse(String), // Internal parser state error, not user input
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SutraError {
    pub kind: SutraErrorKind,
    pub span: Option<crate::ast::Span>,
}

impl SutraError {
    // Helper to enrich the error with the original source code snippet.
    // This is part of the "two-phase error enrichment" pattern.
    pub fn with_source(mut self, source: &str) -> Self {
        if let Some(span) = &self.span {
            let original_code = source.get(span.start..span.end).map(|s| s.to_string());

            if let SutraErrorKind::Eval(eval_error) = &mut self.kind {
                eval_error.original_code = original_code;
            }
        }
        self
    }
}

impl std::fmt::Display for SutraError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            SutraErrorKind::Parse(s) => write!(f, "Parse Error: {}", s),
            SutraErrorKind::Macro(s) => write!(f, "Macro Error: {}", s),
            SutraErrorKind::Validation(s) => write!(f, "Validation Error: {}", s),
            SutraErrorKind::Io(s) => write!(f, "IO Error: {}", s),
            SutraErrorKind::Eval(e) => {
                writeln!(f, "Evaluation Error: {}", e.message)?;
                if let Some(suggestion) = &e.suggestion {
                    writeln!(f, "\nSuggestion: {}", suggestion)?;
                }
                if let Some(original) = &e.original_code {
                    writeln!(f, "\nOriginal Code:")?;
                    writeln!(f, "  {}", original)?;
                }
                writeln!(f, "\nExpanded Code:")?;
                write!(f, "  {}", e.expanded_code)
            }
            SutraErrorKind::MalformedAst(s) => write!(f, "Malformed AST Error: {}", s),
            SutraErrorKind::InternalParse(s) => write!(f, "Internal Parse Error: {}", s),
        }
    }
}

impl std::error::Error for SutraError {}

use crate::ast::value::Value;
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
use crate::ast::{Expr, Span, WithSpan};

/// Constructs a parse error (malformed input, syntax error).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::parse_error;
/// use sutra::ast::Span;
/// let span = Span::default();
/// let _ = parse_error("Unexpected token", Some(span.clone()));
/// ```
pub fn parse_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Parse(msg.into()),
        span,
    }
}

/// Constructs a macro error (expansion, definition, invocation).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::macro_error;
/// use sutra::ast::Span;
/// let span = Span::default();
/// let _ = macro_error("Duplicate macro name", Some(span.clone()));
/// ```
pub fn macro_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Macro(msg.into()),
        span,
    }
}

/// Constructs a validation error (post-expansion validation failure).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::validation_error;
/// use sutra::ast::Span;
/// let span = Span::default();
/// let _ = validation_error("Invalid macro usage", Some(span.clone()));
/// ```
pub fn validation_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Validation(msg.into()),
        span,
    }
}

/// Constructs an IO error (file or system IO failure).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::io_error;
/// let _ = io_error("Failed to read file", None);
/// ```
pub fn io_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Io(msg.into()),
        span,
    }
}

/// Constructs a malformed AST error (internal AST structure error).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::malformed_ast_error;
/// use sutra::ast::Span;
/// let span = Span::default();
/// let _ = malformed_ast_error("Empty expr pair", Some(span.clone()));
/// ```
pub fn malformed_ast_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::MalformedAst(msg.into()),
        span,
    }
}

/// Constructs an internal parse error (parser state error not caused by user input).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::internal_parse_error;
/// use sutra::ast::Span;
/// let span = Span::default();
/// let _ = internal_parse_error("Parser generated an empty tree", Some(span.clone()));
/// ```
pub fn internal_parse_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::InternalParse(msg.into()),
        span,
    }
}

// --- Domain-Specific (Evaluation) Error Helpers ---

/// Constructs an evaluation arity error (wrong number of arguments).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::eval_arity_error;
/// use sutra::ast::{Expr, Span, WithSpan};
/// let span = Span::default();
/// let args: Vec<WithSpan<Expr>> = vec![];
/// let _ = eval_arity_error(Some(span.clone()), &args, "core/set!", "2");
/// ```
pub fn eval_arity_error(
    span: Option<Span>,
    args: &[WithSpan<Expr>],
    func_name: &str,
    expected: impl ToString,
) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Validation(format!(
            "{}: expected {} argument(s), got {}",
            func_name,
            expected.to_string(),
            args.len()
        )),
        span,
    }
}

/// Constructs an evaluation type error (type mismatch).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::eval_type_error;
/// use sutra::ast::{Expr, Span, WithSpan};
/// use sutra::ast::value::Value;
/// let span = Span::default();
/// let arg = WithSpan { value: Expr::List(vec![], Span::default()), span: span.clone() };
/// let val = Value::Nil;
/// let _ = eval_type_error(Some(span.clone()), &arg, "core/get", "a Path", &val);
/// ```
pub fn eval_type_error(
    span: Option<Span>,
    _arg: &WithSpan<Expr>,
    func_name: &str,
    expected: &str,
    found: &Value,
) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Validation(format!(
            "{}: expected {} for argument {:?}, found {:?}",
            func_name, expected, _arg, found
        )),
        span,
    }
}

/// Constructs a general evaluation error (other evaluation failures).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::eval_general_error;
/// use sutra::ast::{Expr, Span, WithSpan};
/// let span = Span::default();
/// let arg = WithSpan { value: Expr::List(vec![], Span::default()), span: span.clone() };
/// let _ = eval_general_error(Some(span.clone()), &arg, "Division by zero");
/// ```
pub fn eval_general_error(
    span: Option<Span>,
    _arg: &WithSpan<Expr>,
    msg: impl Into<String>,
) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Validation(msg.into()),
        span,
    }
}

/// Constructs a recursion depth error (exceeded recursion limit).
///
/// # Example
/// ```rust
/// use sutra::syntax::error::recursion_depth_error;
/// use sutra::ast::Span;
/// let span = Span::default();
/// let _ = recursion_depth_error(Some(span.clone()));
/// ```
pub fn recursion_depth_error(span: Option<Span>) -> SutraError {
    SutraError {
        kind: SutraErrorKind::Validation("Recursion depth limit exceeded".to_string()),
        span,
    }
}

/// Constructs a SutraError from an existing kind and span.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::{from_kind, SutraErrorKind};
/// use sutra::ast::Span;
/// let span = Span::default();
/// let _ = from_kind(SutraErrorKind::Parse("custom".to_string()), Some(span.clone()));
/// ```
pub fn from_kind(kind: SutraErrorKind, span: Option<Span>) -> SutraError {
    SutraError { kind, span }
}
