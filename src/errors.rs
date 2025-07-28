//! Sutra Error Handling - Unified Encapsulated API
//!
//! All internal implementation is completely hidden to prevent misuse.

use miette::{Diagnostic, SourceSpan};
use miette::{LabeledSpan, NamedSource};
use std::fmt;
use std::sync::Arc;

// ============================================================================
// SOURCE CONTEXT - Error reporting infrastructure
// ============================================================================

/// Represents source context for error reporting with explicit hierarchy
/// between real sources (preferred) and fallbacks (tolerated when necessary)
#[derive(Debug, Clone)]
pub struct SourceContext {
    pub name: String,
    pub content: String,
}

impl SourceContext {
    /// Create a source context from real file content
    /// This is the preferred method for error reporting
    pub fn from_file(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
        }
    }

    /// Create a fallback when real source is unavailable
    /// Use only when real source cannot be obtained
    pub fn fallback(context: &str) -> Self {
        Self {
            name: "fallback".to_string(),
            content: format!("// {}", context),
        }
    }

    /// Convert to NamedSource for use with miette error reporting
    pub fn to_named_source(&self) -> Arc<NamedSource<String>> {
        Arc::new(NamedSource::new(self.name.clone(), self.content.clone()))
    }
}

impl Default for SourceContext {
    fn default() -> Self {
        Self::fallback("default context")
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
#[derive(Debug, Clone, PartialEq)]
pub enum ErrorKind {
    // Parse errors - structural and syntactic issues
    MissingElement {
        element: String,
    },
    MalformedConstruct {
        construct: String,
    },
    InvalidLiteral {
        literal_type: String,
        value: String,
    },
    EmptyExpression,
    ParameterOrderViolation {
        rest_span: SourceSpan,
    },
    UnexpectedToken {
        expected: String,
        found: String,
    },

    // Runtime errors - evaluation failures
    UndefinedSymbol {
        symbol: String,
    },
    TypeMismatch {
        expected: String,
        actual: String,
    },
    ArityMismatch {
        expected: String,
        actual: usize,
    },
    InvalidOperation {
        operation: String,
        operand_type: String,
    },
    RecursionLimit,
    StackOverflow,

    // Validation errors - semantic analysis issues
    InvalidMacro {
        macro_name: String,
        reason: String,
    },
    InvalidPath {
        path: String,
    },
    DuplicateDefinition {
        symbol: String,
        original_location: SourceSpan,
    },
    ScopeViolation {
        symbol: String,
        scope: String,
    },
    GeneralValidation {
        message: String,
    },

    // Test errors
    AssertionFailure {
        message: String,
        test_name: String,
    },
}

/// Context-specific source information
#[derive(Debug, Clone)]
pub struct SourceInfo {
    pub source: Arc<NamedSource<String>>,
    pub primary_span: SourceSpan,
    pub phase: String,
}

/// Diagnostic enhancement data
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    pub help: Option<String>,
    pub error_code: String,
}

/// Context-aware error creation - each context knows how to create appropriate errors
pub trait ErrorReporting {
    /// Create an error with context-appropriate enhancements
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError;

    /// Convenience methods for common error types
    fn missing_element(&self, element: &str, span: SourceSpan) -> SutraError {
        self.report(
            ErrorKind::MissingElement {
                element: element.into(),
            },
            span,
        )
    }

    fn type_mismatch(&self, expected: &str, actual: &str, span: SourceSpan) -> SutraError {
        self.report(
            ErrorKind::TypeMismatch {
                expected: expected.into(),
                actual: actual.into(),
            },
            span,
        )
    }

    fn undefined_symbol(&self, symbol: &str, span: SourceSpan) -> SutraError {
        self.report(
            ErrorKind::UndefinedSymbol {
                symbol: symbol.into(),
            },
            span,
        )
    }

    fn arity_mismatch(&self, expected: &str, actual: usize, span: SourceSpan) -> SutraError {
        self.report(
            ErrorKind::ArityMismatch {
                expected: expected.into(),
                actual,
            },
            span,
        )
    }

    fn invalid_operation(
        &self,
        operation: &str,
        operand_type: &str,
        span: SourceSpan,
    ) -> SutraError {
        self.report(
            ErrorKind::InvalidOperation {
                operation: operation.into(),
                operand_type: operand_type.into(),
            },
            span,
        )
    }

    /// Creates an internal error - these indicate engine bugs, not user errors.
    /// Use this for situations that should never happen in correct engine operation.
    fn internal_error(&self, message: &str, span: SourceSpan) -> SutraError {
        let mut error = self.report(
            ErrorKind::InvalidOperation {
                operation: "internal engine operation".into(),
                operand_type: format!("INTERNAL ERROR: {}", message),
            },
            span,
        );
        error.diagnostic_info.help =
            Some("This is an internal engine error. Please report this as a bug.".into());
        error
    }
}

impl ErrorKind {
    /// Get the error category for test assertions
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::MissingElement { .. }
            | Self::MalformedConstruct { .. }
            | Self::InvalidLiteral { .. }
            | Self::EmptyExpression
            | Self::ParameterOrderViolation { .. }
            | Self::UnexpectedToken { .. } => ErrorCategory::Parse,

            Self::UndefinedSymbol { .. }
            | Self::TypeMismatch { .. }
            | Self::ArityMismatch { .. }
            | Self::InvalidOperation { .. }
            | Self::RecursionLimit
            | Self::StackOverflow => ErrorCategory::Runtime,

            Self::InvalidMacro { .. }
            | Self::InvalidPath { .. }
            | Self::DuplicateDefinition { .. }
            | Self::ScopeViolation { .. }
            | Self::GeneralValidation { .. } => ErrorCategory::Validation,

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
            Self::GeneralValidation { .. } => "general_validation",
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
            ErrorKind::InvalidLiteral {
                literal_type,
                value,
            } => {
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
                write!(
                    f,
                    "Runtime error: incorrect arity, expected {}, got {}",
                    expected, actual
                )
            }
            ErrorKind::InvalidOperation {
                operation,
                operand_type,
            } => {
                write!(
                    f,
                    "Runtime error: invalid operation '{}' on {}",
                    operation, operand_type
                )
            }
            ErrorKind::RecursionLimit => {
                write!(f, "Runtime error: recursion limit exceeded")
            }
            ErrorKind::StackOverflow => {
                write!(f, "Runtime error: stack overflow")
            }
            ErrorKind::InvalidMacro { macro_name, reason } => {
                write!(
                    f,
                    "Validation error: invalid macro '{}': {}",
                    macro_name, reason
                )
            }
            ErrorKind::InvalidPath { path } => {
                write!(f, "Validation error: invalid path '{}'", path)
            }
            ErrorKind::DuplicateDefinition { symbol, .. } => {
                write!(f, "Validation error: duplicate definition of '{}'", symbol)
            }
            ErrorKind::ScopeViolation { symbol, scope } => {
                write!(
                    f,
                    "Validation error: '{}' not accessible in {} scope",
                    symbol, scope
                )
            }
            ErrorKind::GeneralValidation { message } => {
                write!(f, "Validation error: {}", message)
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
        self.diagnostic_info
            .help
            .as_ref()
            .map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let labels = vec![LabeledSpan::new_with_span(
            Some(self.primary_label()),
            self.source_info.primary_span,
        )];
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
            ErrorKind::GeneralValidation { .. } => "validation issue".into(),
            ErrorKind::AssertionFailure { .. } => "assertion failed here".into(),
        }
    }
}

/// Standalone constructor for grammar validation phase.
///
/// This is a special case. The grammar validator does not have access to an `ErrorReporting`
/// context, so this function encapsulates the logic for creating validation errors,
/// ensuring that `SutraError` structs are not constructed manually in the validators.
pub fn grammar_validation_error(
    message: String,
    rule_definition: &str,
    is_warning: bool,
) -> SutraError {
    let code_suffix = if is_warning { "warning" } else { "error" };
    let source = Arc::new(NamedSource::new(
        "grammar_validation",
        rule_definition.to_string(),
    ));

    SutraError {
        kind: ErrorKind::GeneralValidation { message },
        source_info: SourceInfo {
            source,
            primary_span: (0..rule_definition.len()).into(),
            phase: "Grammar Structure".into(),
        },
        diagnostic_info: DiagnosticInfo {
            help: None,
            error_code: format!("validation.grammar.{}", code_suffix),
        },
    }
}

/// Creates a placeholder span for errors not tied to a specific source code
/// location, such as I/O errors or internal application state failures.
/// This makes the intent of using an empty span explicit and searchable.
pub fn unspanned() -> miette::SourceSpan {
    miette::SourceSpan::from(0..0)
}

/// Converts a Sutra AST Span to a miette SourceSpan.
/// This is a utility function for the new error system to bridge between
/// the AST span representation and the error reporting span format.
pub fn to_source_span(span: crate::syntax::Span) -> miette::SourceSpan {
    miette::SourceSpan::from(span.start..span.end)
}

/// General-purpose error creation context used throughout the codebase
/// for creating properly contextualized SutraError instances
pub struct ValidationContext {
    pub source: SourceContext,
    pub phase: String,
}

impl ValidationContext {
    pub fn new(source: SourceContext, phase: String) -> Self {
        Self { source, phase }
    }
}

impl ErrorReporting for ValidationContext {
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError {
        let error_code = format!("sutra::{}::{}", self.phase, kind.code_suffix());

        SutraError {
            kind,
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                phase: self.phase.clone(),
            },
            diagnostic_info: DiagnosticInfo {
                help: None,
                error_code,
            },
        }
    }
}

// ============================================================================
// ERROR FORMATTING UTILITIES
// ============================================================================

/// Prints a SutraError with full miette diagnostics
///
/// This provides rich error formatting with source spans, suggestions, and context.
/// Use this for user-facing error display in CLI and REPL contexts.
pub fn print_error(error: SutraError) {
    use miette::Report;
    let report = Report::new(error);
    eprintln!("{report:?}");
}
