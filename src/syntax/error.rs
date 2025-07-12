use serde::{Deserialize, Serialize};

// =============================================================================
// SECTION 1: MODULE DOCUMENTATION & IMPORTS
// =============================================================================

// String formatting constants to eliminate magic numbers
const SHORT_STRING_LIMIT: usize = 20;
const LONG_STRING_LIMIT: usize = 30;
const TRUNCATION_SUFFIX: &str = "...";

use crate::ast::{AstNode, Expr, Span};
use crate::ast::value::Value;

// =============================================================================
// SECTION 2: CORE DATA STRUCTURES
// =============================================================================

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
    build_simple_error(msg, span, SutraErrorKind::Parse)
}

/// Constructs a macro error (expansion, definition, invocation).
///
/// Macro errors occur during macro expansion, when defining new macros, or when
/// invoking macros with incorrect parameters or invalid contexts.
///
/// # Example
/// ```rust
/// use sutra::syntax::error::macro_error;
/// use sutra::ast::Span;
/// let span = Span { start: 10, end: 20 };
/// let error = macro_error("Duplicate macro name 'my-macro'", Some(span));
/// ```
pub fn macro_error(msg: impl Into<String>, span: Option<Span>) -> SutraError {
    build_simple_error(msg, span, SutraErrorKind::Macro)
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
    build_simple_error(msg, span, SutraErrorKind::Validation)
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
    build_simple_error(msg, span, SutraErrorKind::Io)
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
    build_simple_error(msg, span, SutraErrorKind::MalformedAst)
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
    build_simple_error(msg, span, SutraErrorKind::InternalParse)
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
    let expected_str = expected.to_string();
    let actual_count = args.len();

    let main_message = build_arity_main_message(func_name);
    let context_message = build_arity_context_message(&expected_str, actual_count);
    let args_summary = build_arity_args_summary(args);
    let suggestion = generate_arity_suggestion(func_name, &expected_str, actual_count);
    let expanded_code = build_arity_expanded_code(func_name, args);
    let full_message = combine_arity_message_parts(&main_message, &context_message, &args_summary);

    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            message: full_message,
            expanded_code,
            original_code: None, // Will be filled by with_source if available
            suggestion: Some(suggestion),
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
    let main_message = build_type_main_message(func_name);
    let context_message = build_type_context_message(expected, found);
    let value_info = build_type_value_info(found, arg);
    let suggestion = generate_type_suggestion(func_name, expected, found);
    let expanded_code = arg.value.pretty();
    let full_message = combine_type_message_parts(&main_message, &context_message, &value_info);

    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            message: full_message,
            expanded_code,
            original_code: None, // Will be filled by with_source if available
            suggestion: Some(suggestion),
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
pub fn eval_general_error(
    span: Option<Span>,
    arg: &AstNode,
    msg: impl Into<String>,
) -> SutraError {
    let message = msg.into();

    let main_message = build_general_main_message(&message);
    let context_message = build_general_context_message(arg);
    let suggestion = generate_general_error_suggestion(&message, &arg.value);
    let expanded_code = arg.value.pretty();
    let full_message = combine_general_message_parts(&main_message, &context_message);

    SutraError {
        kind: SutraErrorKind::Eval(EvalError {
            message: full_message,
            expanded_code,
            original_code: None, // Will be filled by with_source if available
            suggestion: Some(suggestion),
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
        kind: SutraErrorKind::Validation("Recursion depth limit exceeded".to_string()),
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

// =============================================================================
// SECTION 5: INFRASTRUCTURE/TRAITS
// =============================================================================

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

// =============================================================================
// SECTION 6: INTERNAL HELPERS (GROUPED BY FUNCTION)
// =============================================================================

// --- String Formatting Utilities ---

/// Truncates strings with consistent ellipsis formatting for error display.
fn truncate_string_for_display(s: &str, limit: usize) -> String {
    if s.len() > limit {
        let truncate_at = limit.saturating_sub(TRUNCATION_SUFFIX.len());
        format!("{}{}", &s[..truncate_at], TRUNCATION_SUFFIX)
    } else {
        s.to_string()
    }
}

/// Formats string values with quotes and truncation for error messages.
fn format_string_value(s: &str, limit: usize) -> String {
    let truncated = truncate_string_for_display(s, limit);
    format!("string \"{}\"", truncated)
}

// --- Type Handling Utilities ---

/// Maps canonical type names for consistent display.
static TYPE_NAME_MAP: &[(&str, &str)] = &[
    ("a Number", "Number"),
    ("a String", "String"),
    ("a Path", "Path"),
    ("a Bool", "Boolean"),
    ("a Boolean", "Boolean"),
    ("a List", "List"),
];

/// Converts type names to unified display format.
fn get_unified_type_name(type_name: &str) -> String {
    // Check the type name map first
    if let Some((_, unified)) = TYPE_NAME_MAP.iter().find(|(original, _)| *original == type_name) {
        return unified.to_string();
    }

    // Handle generic "a " and "an " prefixes
    if type_name.starts_with("a ") {
        type_name[2..].to_string()
    } else if type_name.starts_with("an ") {
        type_name[3..].to_string()
    } else {
        type_name.to_string()
    }
}

/// Gets unified type name for Value types.
fn get_value_unified_type_name(value: &Value) -> String {
    match value {
        Value::Number(_) => "Number".to_string(),
        Value::String(_) => "String".to_string(),
        Value::Bool(_) => "Boolean".to_string(),
        Value::List(_) => "List".to_string(),
        Value::Map(_) => "Map".to_string(),
        Value::Path(_) => "Path".to_string(),
        Value::Nil => "Nil".to_string(),
    }
}

/// Formats values for error display with truncation.
fn format_any_value_for_error(value: &Value) -> String {
    match value {
        Value::Number(n) => format!("number {}", n),
        Value::String(s) => format_string_value(s, LONG_STRING_LIMIT),
        Value::Bool(b) => format!("boolean {}", b),
        Value::List(items) => format!("list with {} elements", items.len()),
        Value::Map(map) => format!("map with {} keys", map.len()),
        Value::Path(path) => format!("path '{}'", path),
        Value::Nil => "nil".to_string(),
    }
}

// --- Error Construction Helpers ---

/// Unified constructor helper to eliminate repetition across simple error types.
fn build_simple_error<F>(msg: impl Into<String>, span: Option<Span>, kind_constructor: F) -> SutraError
where
    F: FnOnce(String) -> SutraErrorKind,
{
    SutraError {
        kind: kind_constructor(msg.into()),
        span,
    }
}

// --- Arity Error Helpers ---

/// Creates main message for arity errors.
fn build_arity_main_message(func_name: &str) -> String {
    format!("Arity mismatch in call to '{}'", func_name)
}

/// Builds context message explaining arity requirements.
fn build_arity_context_message(expected_str: &str, actual_count: usize) -> String {
    let expected_str = expected_str.to_string();
    if expected_str.contains("at least") {
        let min_args = expected_str.split_whitespace().nth(2).unwrap_or("?");
        format!(
            "Expected at least {} arguments, but received {}. This function accepts a variable number of arguments.",
            min_args, actual_count
        )
    } else if expected_str.contains("exactly") {
        let exact_args = expected_str.split_whitespace().nth(1).unwrap_or("?");
        format!(
            "Expected exactly {} arguments, but received {}. This function requires a specific number of arguments.",
            exact_args, actual_count
        )
    } else {
        format!(
            "Expected {} arguments, but received {}.",
            expected_str, actual_count
        )
    }
}

/// Builds the argument summary for arity errors.
fn build_arity_args_summary(args: &[AstNode]) -> String {
    if args.is_empty() {
        "No arguments provided".to_string()
    } else {
        format!(
            "Arguments provided ({}): {}",
            args.len(),
            args.iter()
                .enumerate()
                .map(|(i, arg)| format!("  {}: {}", i + 1, summarize_expr(&arg.value)))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

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

/// Combines message parts into the final arity error message.
fn combine_arity_message_parts(main_message: &str, context_message: &str, args_summary: &str) -> String {
    format!(
        "{}\n\n{}\n\n{}",
        main_message,
        context_message,
        args_summary
    )
}

// --- Type Error Helpers ---

/// Creates the main type error message.
fn build_type_main_message(func_name: &str) -> String {
    format!("Type mismatch in call to '{}'", func_name)
}

/// Builds the detailed context message for type errors.
fn build_type_context_message(expected: &str, found: &Value) -> String {
    let expected_type = get_unified_type_name(expected);
    let found_type = get_value_unified_type_name(found);
    format!(
        "Expected argument of type {}, but received {}.",
        expected_type,
        found_type
    )
}

/// Builds the value information for type errors.
fn build_type_value_info(found: &Value, arg: &AstNode) -> String {
    format!(
        "Argument provided: {} (from expression: {})",
        format_any_value_for_error(found),
        summarize_expr(&arg.value)
    )
}

/// Combines message parts into the final type error message.
fn combine_type_message_parts(main_message: &str, context_message: &str, value_info: &str) -> String {
    format!(
        "{}\n\n{}\n\n{}",
        main_message,
        context_message,
        value_info
    )
}

// --- General Error Helpers ---

/// Creates the main general error message.
fn build_general_main_message(message: &str) -> String {
    format!("Evaluation error: {}", message)
}

/// Builds the context message for general errors.
fn build_general_context_message(arg: &AstNode) -> String {
    format!(
        "Error occurred while evaluating: {}",
        summarize_expr(&arg.value)
    )
}

/// Combines message parts into the final general error message.
fn combine_general_message_parts(main_message: &str, context_message: &str) -> String {
    format!(
        "{}\n\n{}",
        main_message,
        context_message
    )
}

// --- Expression Utilities ---

/// Creates a concise summary of an expression for error messages.
fn summarize_expr(expr: &Expr) -> String {
    match expr {
        Expr::List(items, _) => format!("(list with {} elements)", items.len()),
        Expr::Symbol(name, _) => format!("symbol '{}'", name),
        Expr::Path(path, _) => format!("path '{}'", path),
        Expr::String(s, _) => format_string_value(s, SHORT_STRING_LIMIT),
        Expr::Number(n, _) => format!("number {}", n),
        Expr::Bool(b, _) => format!("boolean {}", b),
        Expr::If { .. } => "if expression".to_string(),
        Expr::Quote(_, _) => "quoted expression".to_string(),
        Expr::ParamList(_) => "parameter list".to_string(),
        Expr::Spread(_) => "spread argument".to_string(),
    }
}

// --- Suggestion System ---

// Structured suggestion data to replace hard-coded match statements
struct FunctionUsage {
    pattern: &'static str,
    description: &'static str,
}

static ARITY_SUGGESTIONS: &[(&str, FunctionUsage)] = &[
    // Core functions
    ("core/set!", FunctionUsage { pattern: "(core/set! path value)", description: "Set a value at a path" }),
    ("core/get", FunctionUsage { pattern: "(core/get path)", description: "Get value at a path" }),
    ("core/del!", FunctionUsage { pattern: "(core/del! path)", description: "Delete value at a path" }),

    // Arithmetic functions
    ("+", FunctionUsage { pattern: "(+ a b) or (+ a b c ...)", description: "These arithmetic functions require at least 2 arguments" }),
    ("-", FunctionUsage { pattern: "(- a b) or (- a b c ...)", description: "These arithmetic functions require at least 2 arguments" }),
    ("*", FunctionUsage { pattern: "(* a b) or (* a b c ...)", description: "These arithmetic functions require at least 2 arguments" }),
    ("/", FunctionUsage { pattern: "(/ a b) or (/ a b c ...)", description: "These arithmetic functions require at least 2 arguments" }),
    ("mod", FunctionUsage { pattern: "(mod dividend divisor)", description: "Modulo operation requires exactly 2 arguments" }),

    // Comparison functions
    ("eq?", FunctionUsage { pattern: "(eq? a b)", description: "Comparison functions require exactly 2 arguments" }),
    ("gt?", FunctionUsage { pattern: "(gt? a b)", description: "Comparison functions require exactly 2 arguments" }),
    ("lt?", FunctionUsage { pattern: "(lt? a b)", description: "Comparison functions require exactly 2 arguments" }),
    ("gte?", FunctionUsage { pattern: "(gte? a b)", description: "Comparison functions require exactly 2 arguments" }),
    ("lte?", FunctionUsage { pattern: "(lte? a b)", description: "Comparison functions require exactly 2 arguments" }),
    ("not", FunctionUsage { pattern: "(not value)", description: "Logical negation requires exactly 1 argument" }),

    // List functions
    ("list", FunctionUsage { pattern: "(list item1 item2 ...)", description: "Create list from arguments" }),
    ("len", FunctionUsage { pattern: "(len collection)", description: "Get length requires exactly 1 argument" }),
    ("apply", FunctionUsage { pattern: "(apply function arg1 arg2 ... arg-list)", description: "Apply requires at least 2 arguments" }),

    // String functions
    ("core/str+", FunctionUsage { pattern: "(core/str+ string1 string2 ...)", description: "String concatenation accepts any number of arguments" }),

    // Control flow
    ("do", FunctionUsage { pattern: "(do expr1 expr2 ...)", description: "Sequential evaluation accepts any number of arguments" }),
    ("error", FunctionUsage { pattern: "(error message)", description: "Error requires exactly 1 string argument" }),

    // I/O functions
    ("print", FunctionUsage { pattern: "(print value)", description: "Print requires exactly 1 argument" }),
    ("core/print", FunctionUsage { pattern: "(core/print value)", description: "Print requires exactly 1 argument" }),
];

static TYPE_SUGGESTIONS: &[(&str, &str, &str)] = &[
    // Core path operations
    ("core/get", "a Path", "Paths are created with symbols like 'x' or nested like 'player/name'"),
    ("core/set!", "a Path", "First argument must be a path. Use a symbol or nested path like 'items/0'"),
    ("core/del!", "a Path", "Provide a path to delete. Use a symbol like 'x' or nested path like 'config/debug'"),

    // Arithmetic operations
    ("+", "a Number", "This operation requires numeric arguments"),
    ("-", "a Number", "This operation requires numeric arguments"),
    ("*", "a Number", "This operation requires numeric arguments"),
    ("/", "a Number", "This operation requires numeric arguments"),
    ("mod", "a Number", "This operation requires numeric arguments"),

    // String operations
    ("core/str+", "a String", "String concatenation requires all arguments to be strings"),

    // List operations
    ("len", "a List", "This function measures the length of lists. Provide a list argument"),

    // Boolean operations
    ("not", "a Bool", "Logical negation requires a boolean value (true or false)"),
    ("eq?", "a Bool", "Comparison functions work with numbers, strings, or booleans"),
    ("gt?", "a Bool", "Comparison functions work with numbers, strings, or booleans"),
    ("lt?", "a Bool", "Comparison functions work with numbers, strings, or booleans"),
    ("gte?", "a Bool", "Comparison functions work with numbers, strings, or booleans"),
    ("lte?", "a Bool", "Comparison functions work with numbers, strings, or booleans"),
];

static GENERAL_ERROR_SUGGESTIONS: &[(&str, &str)] = &[
    ("division by zero", "Check that the divisor is not zero before performing division. Consider using conditional logic."),
    ("index out of bounds", "Verify that the index is within the bounds of the collection. Use length checks."),
    ("key not found", "Ensure the key exists in the map before accessing it. Consider using default values."),
    ("overflow", "The calculation resulted in a value too large to represent. Check input ranges."),
    ("underflow", "The calculation resulted in a value too small to represent. Check input ranges."),
    ("invalid", "Check that all inputs are valid and in the expected format."),
];

/// Generates helpful suggestions for fixing arity errors.
fn generate_arity_suggestion(func_name: &str, expected: &str, actual_count: usize) -> String {
    let base_suggestion = ARITY_SUGGESTIONS
        .iter()
        .find(|(name, _)| *name == func_name)
        .map(|(_, usage)| format!("Usage: {} - {}", usage.pattern, usage.description))
        .unwrap_or_else(|| "Check the function documentation for correct usage".to_string());

    let arity_advice = build_arity_advice(expected, actual_count);
    format!("{}{}", base_suggestion, arity_advice)
}

/// Builds specific arity advice based on mismatch type.
fn build_arity_advice(expected: &str, actual_count: usize) -> &'static str {
    if expected.contains("at least") && actual_count == 0 {
        " You need to provide at least one argument."
    } else if expected.contains("exactly") && actual_count > expected.split_whitespace().nth(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(0) {
        " You provided too many arguments - remove the extra ones."
    } else if expected.contains("exactly") && actual_count < expected.split_whitespace().nth(1).and_then(|s| s.parse::<usize>().ok()).unwrap_or(0) {
        " You need to provide more arguments."
    } else {
        ""
    }
}

/// Generates helpful suggestions for fixing type errors.
fn generate_type_suggestion(func_name: &str, expected: &str, found: &Value) -> String {
    let base_suggestion = find_type_suggestion(func_name, expected, found);
    let conversion_hint = build_conversion_hint(expected, found);
    format!("{}{}", base_suggestion, conversion_hint)
}

/// Finds the appropriate type suggestion from structured data.
fn find_type_suggestion(func_name: &str, expected: &str, found: &Value) -> String {
    // Handle special cases first
    if matches!(found, Value::String(_)) && matches!(func_name, "+" | "-" | "*" | "/" | "mod") && expected.contains("Number") {
        return "Arithmetic operations require numbers. Convert strings to numbers if needed".to_string();
    }

    if func_name == "core/str+" && expected.contains("String") {
        return if let Value::Number(n) = found {
            format!("Convert the number {} to a string, or ensure all arguments are strings", n)
        } else {
            "String concatenation requires all arguments to be strings".to_string()
        };
    }

    // Check structured data
    TYPE_SUGGESTIONS
        .iter()
        .find(|(name, exp, _)| *name == func_name && *exp == expected)
        .map(|(_, _, suggestion)| suggestion.to_string())
        .unwrap_or_else(|| "Check the function documentation for expected argument types".to_string())
}

/// Builds type conversion hints based on found and expected types.
fn build_conversion_hint(expected: &str, found: &Value) -> &'static str {
    match found {
        Value::String(_) if expected.contains("Number") =>
            " Consider parsing the string to a number if it contains numeric data.",
        Value::Number(_) if expected.contains("String") =>
            " Consider converting the number to a string representation.",
        Value::List(_) if !expected.contains("List") =>
            " If you meant to access a list element, use indexing or list operations.",
        _ => ""
    }
}

/// Generates helpful suggestions for general evaluation errors.
fn generate_general_error_suggestion(message: &str, expr: &Expr) -> String {
    let base_suggestion = find_general_error_suggestion(message);
    let expr_hint = build_expression_hint(expr);
    format!("{}{}", base_suggestion, expr_hint)
}

/// Finds the appropriate general error suggestion from structured data.
fn find_general_error_suggestion(message: &str) -> &'static str {
    let message_lower = message.to_lowercase();
    GENERAL_ERROR_SUGGESTIONS
        .iter()
        .find(|(pattern, _)| message_lower.contains(pattern))
        .map(|(_, suggestion)| *suggestion)
        .unwrap_or("Review the operation and ensure all preconditions are met.")
}

/// Builds expression-specific hints for general errors.
fn build_expression_hint(expr: &Expr) -> &'static str {
    match expr {
        Expr::List(items, _) if items.is_empty() => " Empty lists may cause issues in some operations.",
        Expr::List(items, _) if items.len() > 100 => " Very large lists may cause performance issues.",
        Expr::Number(n, _) if n.is_infinite() => " Infinite values can cause mathematical errors.",
        Expr::Number(n, _) if n.is_nan() => " NaN (Not a Number) values can propagate through calculations.",
        Expr::String(s, _) if s.is_empty() => " Empty strings may not be accepted by some operations.",
        _ => ""
    }
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








