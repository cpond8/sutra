use miette::{Diagnostic, NamedSource, SourceSpan};
use thiserror::Error;
use std::collections::HashMap;

/// Unified error type for all Sutra engine failures with native miette diagnostics
///
/// This enum follows a consistent structural pattern for homoiconicity:
/// 1. Data fields first (message, names, values)
/// 2. Source context (#[source_code] src, #[label] span)
/// 3. Optional diagnostic fields (#[help], suggestions, related spans)
/// 4. Error chaining (#[source] for underlying errors)
#[derive(Error, Diagnostic, Debug)]
pub enum SutraError {
    // ============================================================================
    // PARSE ERRORS - Syntax and structural parsing failures
    // ============================================================================

    #[error("Parse error: missing {element}")]
    #[diagnostic(
        code(sutra::parse::missing),
        help("Add the required syntax element")
    )]
    ParseMissing {
        element: String, // "parameter list", "body", "function name", etc.
        #[source_code]
        src: NamedSource<String>,
        #[label("missing element")]
        span: SourceSpan,
    },

    #[error("Parse error: malformed {construct}")]
    #[diagnostic(code(sutra::parse::malformed))]
    ParseMalformed {
        construct: String, // "spread", "quote", "define", etc.
        #[source_code]
        src: NamedSource<String>,
        #[label("malformed construct")]
        span: SourceSpan,
        #[help]
        suggestion: Option<String>,
    },

    #[error("Parse error: invalid {item_type} '{value}'")]
    #[diagnostic(code(sutra::parse::invalid_value))]
    ParseInvalidValue {
        item_type: String, // "number", "boolean", "path"
        value: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("invalid value")]
        span: SourceSpan,
        #[help]
        suggestion: Option<String>,
    },

    #[error("Parse error: empty expression")]
    #[diagnostic(
        code(sutra::parse::empty),
        help("Expressions must contain content")
    )]
    ParseEmpty {
        #[source_code]
        src: NamedSource<String>,
        #[label("empty expression")]
        span: SourceSpan,
    },

    #[error("Parse error: unexpected token")]
    #[diagnostic(code(sutra::parse::unexpected_token))]
    ParseUnexpected {
        #[source_code]
        src: NamedSource<String>,
        #[label("unexpected token")]
        span: SourceSpan,
        #[help]
        suggestion: Option<String>,
    },

    #[error("Parse error: required parameter after rest parameter")]
    #[diagnostic(
        code(sutra::parse::param_order),
        help("Rest parameters must come last")
    )]
    ParseParameterOrder {
        #[source_code]
        src: NamedSource<String>,
        #[label("required parameter")]
        span: SourceSpan,
        #[label("rest parameter here")]
        rest_span: SourceSpan,
    },

    // ============================================================================
    // VALIDATION ERRORS - Semantic validation failures
    // ============================================================================

    #[error("Validation error: unknown symbol '{symbol}'")]
    #[diagnostic(code(sutra::validation::unknown_symbol))]
    ValidationUnknownSymbol {
        symbol: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("undefined symbol")]
        span: SourceSpan,
        #[help]
        suggestions: Option<String>, // "Did you mean: symbol1, symbol2?"
    },

    #[error("Validation error: arity mismatch - expected {expected}, got {actual}")]
    #[diagnostic(
        code(sutra::validation::arity),
        help("Check the function signature and provide the correct number of arguments")
    )]
    ValidationArity {
        expected: String, // "2", "1-3", "at least 2"
        actual: usize,
        #[source_code]
        src: NamedSource<String>,
        #[label("incorrect argument count")]
        span: SourceSpan,
    },

    #[error("Validation error: duplicate name '{name}'")]
    #[diagnostic(
        code(sutra::validation::duplicate),
        help("Use unique names for parameters, variables, and definitions")
    )]
    ValidationDuplicate {
        name: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("duplicate name")]
        span: SourceSpan,
        #[label("first defined here")]
        first_definition: SourceSpan,
    },

    #[error("Validation error: {message}")]
    #[diagnostic(code(sutra::validation::general))]
    ValidationGeneral {
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("validation error")]
        span: SourceSpan,
        #[help]
        suggestion: Option<String>,
    },

    // ============================================================================
    // MACRO ERRORS - Macro expansion and template failures
    // ============================================================================

    #[error("Macro error: duplicate macro name '{name}'")]
    #[diagnostic(
        code(sutra::macros::duplicate),
        help("Use unique names for macro definitions")
    )]
    MacroDuplicate {
        name: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("duplicate macro name")]
        span: SourceSpan,
        #[label("first defined here")]
        first_definition: SourceSpan,
    },

    #[error("Macro error: invalid macro definition - {reason}")]
    #[diagnostic(
        code(sutra::macros::invalid_definition),
        help("Use proper macro definition syntax")
    )]
    MacroInvalidDefinition {
        reason: String, // "requires exactly 3 elements", "must be list expression", etc.
        actual_count: Option<usize>,
        #[source_code]
        src: NamedSource<String>,
        #[label("invalid macro definition")]
        span: SourceSpan,
    },

    #[error("Macro error: invalid macro call - {reason}")]
    #[diagnostic(
        code(sutra::macros::invalid_call),
        help("Check macro call syntax and usage")
    )]
    MacroInvalidCall {
        reason: String, // "must be list expression", "requires AST transformation", etc.
        macro_name: Option<String>,
        #[source_code]
        src: NamedSource<String>,
        #[label("invalid macro call")]
        span: SourceSpan,
    },

    #[error("Macro error: expansion failed for '{macro_name}' - {details}")]
    #[diagnostic(
        code(sutra::macros::expansion_failed),
        help("Check macro template and arguments")
    )]
    MacroExpansionFailed {
        macro_name: String,
        details: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("expansion failure")]
        span: SourceSpan,
    },

    // ============================================================================
    // RUNTIME ERRORS - Evaluation and execution failures
    // ============================================================================

    #[error("Runtime error: undefined symbol '{symbol}'")]
    #[diagnostic(
        code(sutra::runtime::undefined_symbol),
        help("Ensure the symbol is defined before use")
    )]
    RuntimeUndefinedSymbol {
        symbol: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("undefined symbol")]
        span: SourceSpan,
    },

    #[error("Runtime error: {function} expects {expected} arguments, got {actual}")]
    #[diagnostic(
        code(sutra::runtime::arity),
        help("Check the function signature and provide the correct arguments")
    )]
    RuntimeArity {
        function: String,
        expected: String,
        actual: usize,
        #[source_code]
        src: NamedSource<String>,
        #[label("incorrect call")]
        span: SourceSpan,
    },

    #[error("Runtime error: {operation} on empty list")]
    #[diagnostic(
        code(sutra::runtime::empty_list),
        help("Ensure the list is not empty before calling this operation")
    )]
    RuntimeEmptyList {
        operation: String, // "car", "cdr"
        #[source_code]
        src: NamedSource<String>,
        #[label("empty list operation")]
        span: SourceSpan,
    },

    #[error("Runtime error: invalid {construct} - {reason}")]
    #[diagnostic(
        code(sutra::runtime::invalid_construct),
        help("Check the syntax and usage of this construct")
    )]
    RuntimeInvalidConstruct {
        construct: String, // "let binding", "special form call", etc.
        reason: String, // "name must be symbol", "requires direct dispatch", etc.
        #[source_code]
        src: NamedSource<String>,
        #[label("invalid construct")]
        span: SourceSpan,
    },

    #[error("Runtime error: {message}")]
    #[diagnostic(code(sutra::runtime::general))]
    RuntimeGeneral {
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("runtime error")]
        span: SourceSpan,
        #[help]
        suggestion: Option<String>,
    },

    // ============================================================================
    // TYPE ERRORS - Type system violations
    // ============================================================================

    #[error("Type error: expected {expected}, got {actual}")]
    #[diagnostic(
        code(sutra::types::mismatch),
        help("Check the types of your arguments and ensure they match expected types")
    )]
    TypeMismatch {
        expected: String,
        actual: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("type mismatch")]
        span: SourceSpan,
    },

    #[error("Type error: cannot apply {operation} to {operand_type}")]
    #[diagnostic(
        code(sutra::types::invalid_operation),
        help("Ensure operands are compatible with the operation")
    )]
    TypeInvalidOperation {
        operation: String,
        operand_type: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("invalid operation")]
        span: SourceSpan,
    },

    // ============================================================================
    // ARITHMETIC ERRORS - Mathematical operation failures
    // ============================================================================

    #[error("Arithmetic error: {operation} by zero")]
    #[diagnostic(
        code(sutra::arithmetic::division_by_zero),
        help("Ensure the operand is not zero")
    )]
    ArithmeticDivisionByZero {
        operation: String, // "division", "modulo"
        #[source_code]
        src: NamedSource<String>,
        #[label("division by zero")]
        span: SourceSpan,
    },

    // ============================================================================
    // RESOURCE ERRORS - File I/O and resource access failures
    // ============================================================================

    #[error("Resource error: {operation} failed for '{path}' - {reason}")]
    #[diagnostic(code(sutra::resource::operation_failed))]
    ResourceOperation {
        operation: String, // "read", "write", "access"
        path: String,
        reason: String, // "file not found", "permission denied", "invalid filename"
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
        #[help]
        suggestion: Option<String>,
    },

    // ============================================================================
    // TEST ERRORS - Test execution and assertion failures
    // ============================================================================

    #[error("Test assertion failed: {message}")]
    #[diagnostic(
        code(sutra::test::assertion_failed),
        help("Check your test expectations and actual values")
    )]
    TestAssertion {
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("test failure")]
        span: SourceSpan,
        expected: Option<String>,
        actual: Option<String>,
    },

    #[error("Test error: duplicate test name '{name}'")]
    #[diagnostic(
        code(sutra::test::duplicate),
        help("Use unique names for each test")
    )]
    TestDuplicate {
        name: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("duplicate test name")]
        span: SourceSpan,
        #[label("first defined here")]
        first_definition: SourceSpan,
    },

    #[error("Test error: {issue} in test '{test_name}'")]
    #[diagnostic(code(sutra::test::structure))]
    TestStructure {
        issue: String, // "missing expect form", "invalid syntax", etc.
        test_name: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("test structure error")]
        span: SourceSpan,
        #[help]
        suggestion: Option<String>,
    },

    // ============================================================================
    // LIMIT ERRORS - Resource and computational limits exceeded
    // ============================================================================

    #[error("Limit exceeded: {limit_type} - current {current}, maximum {maximum}")]
    #[diagnostic(code(sutra::limit::exceeded))]
    LimitExceeded {
        limit_type: String, // "stack depth", "recursion depth", "memory usage"
        current: usize,
        maximum: usize,
        #[source_code]
        src: Option<NamedSource<String>>,
        #[label("limit exceeded")]
        span: Option<SourceSpan>,
        #[help]
        suggestion: Option<String>,
    },

    // ============================================================================
    // CONFIGURATION ERRORS - Setup and compatibility issues
    // ============================================================================

    #[error("Configuration error: {issue} - {details}")]
    #[diagnostic(code(sutra::config::error))]
    Configuration {
        issue: String, // "version mismatch", "missing required", "invalid setup"
        details: String,
        #[help]
        suggestion: Option<String>,
    },

    // ============================================================================
    // STORY ERRORS - Narrative logic and consistency failures
    // ============================================================================

    #[error("Story error: {problem} in '{element}'")]
    #[diagnostic(code(sutra::story::logic_error))]
    StoryLogic {
        problem: String, // "circular dependency", "unreachable content", "state inconsistency"
        element: String,
        details: Option<String>,
        dependency_chain: Option<Vec<String>>,
        #[source_code]
        src: NamedSource<String>,
        #[label("story logic error")]
        span: SourceSpan,
        #[help]
        suggestion: Option<String>,
    },

    // ============================================================================
    // INTERNAL ERRORS - Engine bugs and system failures
    // ============================================================================

    #[error("Internal error: {issue} - {details}")]
    #[diagnostic(
        code(sutra::internal::error),
        help("This is an internal engine error. Please report this as a bug.")
    )]
    Internal {
        issue: String, // "unexpected state", "unsupported rule", "empty parser tree", "general error"
        details: String,
        context: Option<HashMap<String, String>>,
        #[source_code]
        src: Option<NamedSource<String>>,
        #[label("internal error")]
        span: Option<SourceSpan>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    },

    // ============================================================================
    // COMPOSITE ERRORS - Multiple related errors
    // ============================================================================

    #[error("Multiple errors: {operation} failed with {count} errors")]
    #[diagnostic(
        code(sutra::composite::multiple),
        help("Review individual error details below")
    )]
    Composite {
        operation: String, // "validation", "batch operation", "compilation"
        count: usize,
        errors: Vec<SutraError>,
    },

    // ============================================================================
    // WARNINGS - Non-fatal issues
    // ============================================================================

    #[error("Warning: {category} - {message}")]
    #[diagnostic(code(sutra::warning::issue))]
    Warning {
        category: String, // "deprecated feature", "performance", "style"
        message: String,
        #[source_code]
        src: NamedSource<String>,
        #[label("warning")]
        span: SourceSpan,
        #[help]
        suggestion: Option<String>,
    },
}

/// Error type classification enum for backwards compatibility with existing tests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    Parse,
    Validation,
    Eval,
    TypeError,
    Internal,
    TestFailure,
}

impl ErrorType {
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

impl SutraError {
    /// Returns the error type classification for backwards compatibility with tests
    pub fn error_type(&self) -> ErrorType {
        match self {
            // Parse Errors
            Self::ParseMissing { .. }
            | Self::ParseMalformed { .. }
            | Self::ParseInvalidValue { .. }
            | Self::ParseEmpty { .. }
            | Self::ParseUnexpected { .. }
            | Self::ParseParameterOrder { .. } => ErrorType::Parse,

            // Validation Errors (including Macro errors for compatibility)
            Self::ValidationUnknownSymbol { .. }
            | Self::ValidationArity { .. }
            | Self::ValidationDuplicate { .. }
            | Self::ValidationGeneral { .. }
            | Self::MacroDuplicate { .. }
            | Self::MacroInvalidDefinition { .. }
            | Self::MacroInvalidCall { .. }
            | Self::MacroExpansionFailed { .. } => ErrorType::Validation,

            // Runtime Errors (including Arithmetic for compatibility)
            Self::RuntimeUndefinedSymbol { .. }
            | Self::RuntimeArity { .. }
            | Self::RuntimeEmptyList { .. }
            | Self::RuntimeInvalidConstruct { .. }
            | Self::RuntimeGeneral { .. }
            | Self::ArithmeticDivisionByZero { .. } => ErrorType::Eval,

            // Type Errors
            Self::TypeMismatch { .. }
            | Self::TypeInvalidOperation { .. } => ErrorType::TypeError,

            // Test Errors
            Self::TestAssertion { .. }
            | Self::TestDuplicate { .. }
            | Self::TestStructure { .. } => ErrorType::TestFailure,

            // All other errors -> Internal
            Self::ResourceOperation { .. }
            | Self::LimitExceeded { .. }
            | Self::Configuration { .. }
            | Self::StoryLogic { .. }
            | Self::Internal { .. }
            | Self::Composite { .. }
            | Self::Warning { .. } => ErrorType::Internal,
        }
    }

    /// Check if this is a warning (non-fatal error)
    pub fn is_warning(&self) -> bool {
        matches!(self, Self::Warning { .. })
    }

    /// Check if this is a fatal error (cannot continue execution)
    pub fn is_fatal(&self) -> bool {
        match self {
            Self::LimitExceeded { limit_type, .. } => {
                limit_type.contains("stack") || limit_type.contains("memory")
            },
            Self::Internal { issue, .. } => {
                issue.contains("unexpected") || issue.contains("empty tree")
            },
            _ => false,
        }
    }

    /// Check if this is a resource-related error
    pub fn is_resource_error(&self) -> bool {
        matches!(self, Self::ResourceOperation { .. })
    }

    /// Check if this is a story logic error
    pub fn is_story_error(&self) -> bool {
        matches!(self, Self::StoryLogic { .. })
    }

    /// Get the error category for structured error handling
    pub fn category(&self) -> &'static str {
        match self {
            Self::ParseMissing { .. }
            | Self::ParseMalformed { .. }
            | Self::ParseInvalidValue { .. }
            | Self::ParseEmpty { .. }
            | Self::ParseUnexpected { .. }
            | Self::ParseParameterOrder { .. } => "parse",

            Self::ValidationUnknownSymbol { .. }
            | Self::ValidationArity { .. }
            | Self::ValidationDuplicate { .. }
            | Self::ValidationGeneral { .. } => "validation",

            Self::MacroDuplicate { .. }
            | Self::MacroInvalidDefinition { .. }
            | Self::MacroInvalidCall { .. }
            | Self::MacroExpansionFailed { .. } => "macro",

            Self::RuntimeUndefinedSymbol { .. }
            | Self::RuntimeArity { .. }
            | Self::RuntimeEmptyList { .. }
            | Self::RuntimeInvalidConstruct { .. }
            | Self::RuntimeGeneral { .. } => "runtime",

            Self::TypeMismatch { .. }
            | Self::TypeInvalidOperation { .. } => "type",

            Self::ArithmeticDivisionByZero { .. } => "arithmetic",

            Self::ResourceOperation { .. } => "resource",

            Self::TestAssertion { .. }
            | Self::TestDuplicate { .. }
            | Self::TestStructure { .. } => "test",

            Self::LimitExceeded { .. } => "limit",
            Self::Configuration { .. } => "configuration",
            Self::StoryLogic { .. } => "story",
            Self::Internal { .. } => "internal",
            Self::Composite { .. } => "composite",
            Self::Warning { .. } => "warning",
        }
    }
}

impl SutraError {
    /// Helper for creating parse errors with source and span
    pub fn parse_error(
        construct: impl Into<String>,
        src: NamedSource<String>,
        span: SourceSpan,
    ) -> Self {
        Self::ParseMalformed {
            construct: construct.into(),
            src,
            span,
            suggestion: None,
        }
    }

    /// Helper for creating type mismatch errors
    pub fn type_mismatch(
        expected: impl Into<String>,
        actual: impl Into<String>,
        src: NamedSource<String>,
        span: SourceSpan,
    ) -> Self {
        Self::TypeMismatch {
            expected: expected.into(),
            actual: actual.into(),
            src,
            span,
        }
    }

    /// Helper for creating runtime arity errors
    pub fn runtime_arity_error(
        function: impl Into<String>,
        expected: impl Into<String>,
        actual: usize,
        src: NamedSource<String>,
        span: SourceSpan,
    ) -> Self {
        Self::RuntimeArity {
            function: function.into(),
            expected: expected.into(),
            actual,
            src,
            span,
        }
    }

    /// Helper for creating validation arity errors
    pub fn validation_arity_error(
        expected: impl Into<String>,
        actual: usize,
        src: NamedSource<String>,
        span: SourceSpan,
    ) -> Self {
        Self::ValidationArity {
            expected: expected.into(),
            actual,
            src,
            span,
        }
    }

    /// Helper for creating resource operation errors
    pub fn resource_error(
        operation: impl Into<String>,
        path: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::ResourceOperation {
            operation: operation.into(),
            path: path.into(),
            reason: reason.into(),
            source: None,
            suggestion: None,
        }
    }

    /// Helper for creating internal errors
    pub fn internal_error(
        issue: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::Internal {
            issue: issue.into(),
            details: details.into(),
            context: None,
            src: None,
            span: None,
            source: None,
        }
    }

    /// Helper for creating division by zero errors
    pub fn division_by_zero(
        operation: impl Into<String>,
        src: NamedSource<String>,
        span: SourceSpan,
    ) -> Self {
        Self::ArithmeticDivisionByZero {
            operation: operation.into(),
            src,
            span,
        }
    }
}
