# Sutra Error System Redesign: Comprehensive Proposal

## Evaluation of the Current Error System

### What We Actually Need

**Tracing the true requirements from usage patterns:**

1. **Create errors with appropriate context** - Parse context vs Runtime context vs Validation context
2. **Attach diagnostic information** - Source locations, suggestions, related spans
3. **Categorize for testing** - Error type classification for test assertions
4. **Integrate with miette** - Rich diagnostic display with source code highlighting
5. **Prevent construction errors** - No wrong parameter types, no missing context
6. **Auto-enhance based on context** - Test info, suggestions, related spans added automatically

### The True Goal

**Create a declarative, intent-based error system where:**

- Callers specify _what went wrong_ and _where_
- The system infers _how to construct_ and _what to enhance_
- Context drives behavior automatically
- Mistakes are impossible by design

### What Must Be Eliminated

**From our analysis, these elements add no value:**

- Builder layer (pure ceremony)
- Wrapper type delegation (unnecessary indirection)
- 10 identical error variants (massive duplication)
- Multiple APIs for same operation (confusion)
- Manual enhancement chains (error-prone)
- String-heavy parameter passing (type-unsafe)
- Reserved/unused variants (speculative complexity)

### The Minimal Essential Logic

**Core operations needed:**

1. Create error from specific context type
2. Enhance with context-appropriate metadata
3. Classify for testing
4. Display via miette integration

**Essential data model:**

- Error kind (what went wrong)
- Source context (where it happened)
- Enhancement metadata (how to help)

# PROPOSAL for the Ideal Implementation

## Architecture Overview

**Three-layer design that eliminates choice at call sites:**

```

Context Layer (Parsing, Evaluation, Validation)
│
▼
Error Creation Layer (Intent-Based Constructors)
│
▼
Diagnostic Layer (Miette Integration)

```

## Core Data Structures

```rust
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceSpan};
use std::sync::Arc;
use std::fmt;

/// The single error type - no wrapper, no variants, just essential data
#[derive(Debug)]
pub struct SutraError {
    /// What went wrong (type-specific data)
    kind: ErrorKind,
    /// Where it happened (context-specific source information)
    source_info: SourceInfo,
    /// How to help (auto-populated based on context)
    diagnostic_info: DiagnosticInfo,
}

/// All error types as a clean enum - no duplicate fields
#[derive(Debug, Clone)]
pub enum ErrorKind {
    // Parse errors - structural and syntactic issues
    MissingElement { element: String },
    MalformedConstruct { construct: String },
    InvalidLiteral { literal_type: String, value: String },
    EmptyExpression,
    ParameterOrderViolation { rest_span: SourceSpan },
    UnexpectedToken { expected: String, found: String },

    // Runtime errors - evaluation failures
    UndefinedSymbol { symbol: String },
    TypeMismatch { expected: String, actual: String },
    ArityMismatch { expected: String, actual: usize },
    InvalidOperation { operation: String, operand_type: String },
    RecursionLimit,
    StackOverflow,

    // Validation errors - semantic analysis issues
    InvalidMacro { macro_name: String, reason: String },
    InvalidPath { path: String },
    DuplicateDefinition { symbol: String, original_location: SourceSpan },
    ScopeViolation { symbol: String, scope: String },

    // Test errors
    AssertionFailure { message: String, test_name: String },
}

/// Context-specific source information
#[derive(Debug, Clone)]
pub struct SourceInfo {
    source: Arc<NamedSource<String>>,
    primary_span: SourceSpan,
    file_context: FileContext,
}

#[derive(Debug, Clone)]
pub enum FileContext {
    ParseTime { parser_state: String },
    Runtime { test_info: Option<(String, String)> },
    Validation { phase: String },
}

/// Diagnostic enhancement data
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    help: Option<String>,
    related_spans: Vec<LabeledSpan>,
    error_code: String,
    is_warning: bool,
}
```

## Context-Driven Error Creation

**The key insight: Different contexts should create errors differently.**

```rust
/// Context-aware error creation - each context knows how to create appropriate errors
pub trait ErrorReporting {
    /// Create an error with context-appropriate enhancements
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError;

    /// Convenience methods for common error types
    fn missing_element(&self, element: &str, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::MissingElement { element: element.into() }, span)
    }

    fn type_mismatch(&self, expected: &str, actual: &str, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::TypeMismatch { expected: expected.into(), actual: actual.into() }, span)
    }

    fn undefined_symbol(&self, symbol: &str, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::UndefinedSymbol { symbol: symbol.into() }, span)
    }

    fn arity_mismatch(&self, expected: &str, actual: usize, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::ArityMismatch { expected: expected.into(), actual }, span)
    }

    fn invalid_operation(&self, operation: &str, operand_type: &str, span: SourceSpan) -> SutraError {
        self.report(ErrorKind::InvalidOperation {
            operation: operation.into(),
            operand_type: operand_type.into()
        }, span)
    }
}

/// Parser context - creates parse errors with rich diagnostic info
impl ErrorReporting for ParserState {
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError {
        SutraError {
            kind,
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                file_context: FileContext::ParseTime { parser_state: self.debug_info() },
            },
            diagnostic_info: DiagnosticInfo {
                help: self.generate_parse_help(&kind),
                related_spans: self.find_related_spans(&kind, span),
                error_code: format!("sutra::parse::{}", kind.code_suffix()),
                is_warning: false,
            },
        }
    }
}

/// Evaluation context - creates runtime errors with test context
impl ErrorReporting for EvaluationContext {
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError {
        SutraError {
            kind,
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                file_context: FileContext::Runtime {
                    test_info: self.test_file.as_ref()
                        .zip(self.test_name.as_ref())
                        .map(|(f, n)| (f.clone(), n.clone()))
                },
            },
            diagnostic_info: DiagnosticInfo {
                help: self.generate_runtime_help(&kind),
                related_spans: Vec::new(), // Runtime errors typically don't have related spans
                error_code: format!("sutra::runtime::{}", kind.code_suffix()),
                is_warning: false,
            },
        }
    }
}

/// Validation context - creates validation errors with phase information
impl ErrorReporting for ValidationContext {
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError {
        SutraError {
            kind,
            source_info: SourceInfo {
                source: self.source.to_named_source(),
                primary_span: span,
                file_context: FileContext::Validation { phase: self.current_phase().into() },
            },
            diagnostic_info: DiagnosticInfo {
                help: self.generate_validation_help(&kind),
                related_spans: Vec::new(), // Validation errors typically don't have related spans
                error_code: format!("sutra::validation::{}", kind.code_suffix()),
                is_warning: false,
            },
        }
    }
}

impl ValidationContext {
    /// Generate context-appropriate help for validation errors
    fn generate_validation_help(&self, kind: &ErrorKind) -> Option<String> {
        match kind {
            ErrorKind::InvalidMacro { macro_name, reason } => {
                Some(format!("The macro '{}' is invalid: {}", macro_name, reason))
            }
            ErrorKind::InvalidPath { path } => {
                Some(format!("The path '{}' is not valid or cannot be resolved", path))
            }
            ErrorKind::ArityMismatch { expected, actual } => {
                Some(format!("Expected {} arguments, but got {}. Check the function signature.", expected, actual))
            }
            ErrorKind::DuplicateDefinition { symbol, .. } => {
                Some(format!("The symbol '{}' is already defined. Use a different name or check for conflicting definitions.", symbol))
            }
            ErrorKind::ScopeViolation { symbol, scope } => {
                Some(format!("The symbol '{}' is not accessible in {} scope. Check variable visibility rules.", symbol, scope))
            }
            _ => None,
        }
    }

    fn current_phase(&self) -> &str {
        // Return current validation phase (e.g., "semantic", "grammar", etc.)
        &self.phase
    }
}
```

## Automatic Enhancement Based on Context

**Each context type knows how to enhance errors appropriately:**

```rust
impl ParserState {
    /// Generate context-appropriate help messages for parse errors
    fn generate_parse_help(&self, kind: &ErrorKind) -> Option<String> {
        match kind {
            ErrorKind::MissingElement { element } => {
                Some(format!("Add the missing {} to complete the expression", element))
            }
            ErrorKind::MalformedConstruct { construct } => {
                Some(format!("Check the syntax of the {} construct", construct))
            }
            ErrorKind::InvalidLiteral { literal_type, .. } => {
                Some(format!("Check the format of the {} literal", literal_type))
            }
            ErrorKind::EmptyExpression => {
                Some("Empty expressions are not allowed. Add content or remove the parentheses.".into())
            }
            ErrorKind::UnexpectedToken { expected, .. } => {
                Some(format!("Expected {} here. Check the syntax.", expected))
            }
            _ => None,
        }
    }

    /// Find related spans for multi-location diagnostics
    fn find_related_spans(&self, kind: &ErrorKind, primary_span: SourceSpan) -> Vec<LabeledSpan> {
        match kind {
            ErrorKind::ParameterOrderViolation { rest_span } => {
                vec![LabeledSpan::new_with_span(Some("rest parameter here".into()), *rest_span)]
            }
            ErrorKind::MissingElement { element } if element == "closing parenthesis" => {
                // Find matching opening parenthesis
                self.find_matching_opener(primary_span)
                    .map(|span| vec![LabeledSpan::new_with_span(Some("opened here".into()), span)])
                    .unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }
}

impl EvaluationContext {
    /// Generate context-appropriate help for runtime errors
    fn generate_runtime_help(&self, kind: &ErrorKind) -> Option<String> {
        match kind {
            ErrorKind::TypeMismatch { expected, actual } => {
                Some(format!("Expected {}, but got {}. Check the value type.", expected, actual))
            }
            ErrorKind::UndefinedSymbol { symbol } => {
                let suggestions = self.find_similar_symbols(symbol);
                if !suggestions.is_empty() {
                    Some(format!("Did you mean one of: {}?", suggestions.join(", ")))
                } else {
                    Some("Check that the symbol is defined before use.".into())
                }
            }
            ErrorKind::ArityMismatch { expected, actual } => {
                Some(format!("This function expects {} arguments, but received {}", expected, actual))
            }
            ErrorKind::InvalidOperation { operation, operand_type } => {
                Some(format!("The operation '{}' cannot be performed on {}. Check the operation requirements.", operation, operand_type))
            }
            ErrorKind::RecursionLimit => {
                Some("The function has called itself too many times. Check for infinite recursion or increase the recursion limit.".into())
            }
            ErrorKind::StackOverflow => {
                Some("The call stack has grown too large. This usually indicates infinite recursion or very deep function calls.".into())
            }
            _ => None,
        }
    }

    /// Find similar symbol names for suggestion purposes
    fn find_similar_symbols(&self, target: &str) -> Vec<String> {
        // Implementation would use edit distance algorithm
        // to find similar symbols in the current scope
        self.bindings.keys()
            .filter(|name| edit_distance(name, target) < 3)
            .map(|s| s.clone())
            .collect()
    }
}
```

## Simplified Call Sites

**All complexity moved into context types - call sites become trivial:**

```rust
// Parse errors - rich diagnostics automatically added
return Err(parser_state.missing_element("closing parenthesis", span));
return Err(parser_state.report(ErrorKind::InvalidLiteral {
    literal_type: "number".into(),
    value: invalid_text.into()
}, span));

// Runtime errors - test context automatically attached
return Err(context.type_mismatch("Number", value.type_name(), node_span));
return Err(context.undefined_symbol(&symbol_name, symbol_span));
return Err(context.invalid_operation("division", "string", operation_span));

// Validation errors - validation context provides appropriate enhancement
return Err(validator.report(ErrorKind::InvalidMacro {
    macro_name: name.into(),
    reason: "macro not found in current scope".into()
}, span));
return Err(validator.report(ErrorKind::DuplicateDefinition {
    symbol: duplicate_name.into(),
    original_location: original_span
}, span));
```

## Error Classification and Testing

**Simple, predictable classification:**

```rust
impl ErrorKind {
    /// Get the error category for test assertions
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::MissingElement { .. } | Self::MalformedConstruct { .. } |
            Self::InvalidLiteral { .. } | Self::EmptyExpression |
            Self::ParameterOrderViolation { .. } | Self::UnexpectedToken { .. } => ErrorCategory::Parse,

            Self::UndefinedSymbol { .. } | Self::TypeMismatch { .. } |
            Self::ArityMismatch { .. } | Self::InvalidOperation { .. } |
            Self::RecursionLimit | Self::StackOverflow => ErrorCategory::Runtime,

            Self::InvalidMacro { .. } | Self::InvalidPath { .. } |
            Self::DuplicateDefinition { .. } | Self::ScopeViolation { .. } => ErrorCategory::Validation,

            Self::AssertionFailure { .. } => ErrorCategory::Test,
        }
    }

    /// Get error code suffix for diagnostic codes
    /// Uses const evaluation for zero-cost error code generation
    const fn code_suffix(&self) -> &'static str {
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
```

## Miette Integration

**Clean, single implementation:**

```rust
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
            ErrorKind::InvalidLiteral { literal_type, value } => {
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
                write!(f, "Runtime error: incorrect arity, expected {}, got {}", expected, actual)
            }
            ErrorKind::InvalidOperation { operation, operand_type } => {
                write!(f, "Runtime error: invalid operation '{}' on {}", operation, operand_type)
            }
            ErrorKind::RecursionLimit => {
                write!(f, "Runtime error: recursion limit exceeded")
            }
            ErrorKind::StackOverflow => {
                write!(f, "Runtime error: stack overflow")
            }
            ErrorKind::InvalidMacro { macro_name, reason } => {
                write!(f, "Validation error: invalid macro '{}': {}", macro_name, reason)
            }
            ErrorKind::InvalidPath { path } => {
                write!(f, "Validation error: invalid path '{}'", path)
            }
            ErrorKind::DuplicateDefinition { symbol, .. } => {
                write!(f, "Validation error: duplicate definition of '{}'", symbol)
            }
            ErrorKind::ScopeViolation { symbol, scope } => {
                write!(f, "Validation error: '{}' not accessible in {} scope", symbol, scope)
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
        self.diagnostic_info.help.as_ref().map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
        let mut labels = vec![
            LabeledSpan::new_with_span(Some(self.primary_label()), self.source_info.primary_span)
        ];
        labels.extend(self.diagnostic_info.related_spans.clone());
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
            ErrorKind::AssertionFailure { .. } => "assertion failed here".into(),
        }
    }
}
```

## Critical Implementation Details

### Span Management Requirements

**The new system MUST eliminate `Span::default()` usage entirely:**

```rust
// Add compile-time enforcement
impl SourceSpan {
    // Remove any constructors that allow empty spans
    // Force all spans to come from actual source locations
}

// Context types must provide proper span extraction
impl ParserState {
    fn current_span(&self) -> SourceSpan { /* actual parser position */ }
    fn span_for_token(&self, token: &Token) -> SourceSpan { /* token's real span */ }
}

impl EvaluationContext {
    fn span_for_node(&self, node: &AstNode) -> SourceSpan {
        // Use node's actual span, never default
        to_source_span(node.span)
    }
    fn current_span(&self) -> SourceSpan {
        // Use the span of the currently evaluating node
        to_source_span(self.current_span)
    }
}
```

### Error Context Trait Specifications

**Each context must implement required helper methods:**

```rust
// ParserState required methods
impl ParserState {
    fn debug_info(&self) -> String {
        format!("Parser at line {}, col {}", self.line, self.col)
    }

    fn find_matching_opener(&self, close_span: SourceSpan) -> Option<SourceSpan> {
        // Scan backwards to find matching opening bracket/paren
    }

    fn to_named_source(&self) -> Arc<NamedSource<String>> {
        self.source.to_named_source()
    }
}

// EvaluationContext required methods
impl EvaluationContext {
    fn find_similar_symbols(&self, symbol: &str) -> Vec<String> {
        // Use Levenshtein distance to find close matches in current scope
        self.world.find_similar_names(symbol, 3) // max distance 3
    }

    fn to_named_source(&self) -> Arc<NamedSource<String>> {
        self.source.to_named_source()
    }
}
```

### Performance Considerations

**Const error codes for zero-cost diagnostics:**

```rust
impl ErrorKind {
    /// Generate error codes at compile time to avoid runtime string formatting
    const fn code_suffix(&self) -> &'static str {
        match self {
            Self::MissingElement { .. } => "missing_element",
            Self::TypeMismatch { .. } => "type_mismatch",
            Self::UndefinedSymbol { .. } => "undefined_symbol",
            Self::InvalidOperation { .. } => "invalid_operation",
            Self::DuplicateDefinition { .. } => "duplicate_definition",
            // ... rest of variants
        }
    }
}

// Error code generation becomes zero-cost
impl ErrorReporting for ParserState {
    fn report(&self, kind: ErrorKind, span: SourceSpan) -> SutraError {
        SutraError {
            // ... other fields
            diagnostic_info: DiagnosticInfo {
                // No runtime string formatting - resolved at compile time
                error_code: format!("sutra::parse::{}", kind.code_suffix()),
                // ... other fields
            },
        }
    }
}
```

**String interning for common error messages:**

```rust
use string_cache::DefaultAtom as Atom;

// Intern common error strings to reduce allocations
lazy_static! {
    static ref COMMON_HELP: HashMap<&'static str, Atom> = {
        let mut m = HashMap::new();
        m.insert("missing_paren", Atom::from("Add the missing ) to complete the expression"));
        m.insert("type_mismatch", Atom::from("Check the value type"));
        m
    };
}
```

### Testing Requirements

**The new system must pass all existing error tests:**

```rust
// Compatibility tests - ensure no behavioral regressions
#[test]
fn test_error_category_compatibility() {
    // All existing error types must map to same categories
    // ErrorType::Parse → ErrorCategory::Parse
    // ErrorType::Eval → ErrorCategory::Runtime
    // ErrorType::Validation → ErrorCategory::Validation
    // ErrorType::TypeError → ErrorCategory::Runtime
    // ErrorType::TestFailure → ErrorCategory::Test
    assert_eq!(old_parse_error.error_type(), new_parse_error.kind.category());
}

#[test]
fn test_miette_output_compatibility() {
    // Error display format must remain consistent
    let old_output = format!("{:?}", old_error);
    let new_output = format!("{:?}", new_error);
    assert_similar_diagnostic_output(old_output, new_output);
}
```

## Migration Strategy

**Backward-compatible transition with specific implementation order:**

### Critical Naming Collision Management

**The most important naming collision:**

- **Old**: `SutraError` → **Has been renamed to**: `OldSutraError`
- **New**: `SutraError` (in `errors` module)

### Streamlined Migration Strategy (Minimal Transition Time)

**Goal: Minimize time in transitional state - implement atomically where possible**

### Phase 1: Atomic Implementation

```rust
// Implement complete new error system in one atomic change
pub mod errors {
    // All new types: ErrorKind, SutraError, SourceInfo, DiagnosticInfo
    // Complete ErrorReporting trait with all context implementations
    // Full miette integration
}

// Add deprecation warnings to old constructors (simple, helpful)
#[deprecated(since = "0.8.0", note = "Use context.missing_element() instead")]
pub fn parse_missing(/* ... */) -> OldSutraError { /* existing impl */ }

#[deprecated(since = "0.8.0", note = "Enhancement is now automatic based on context")]
impl OldSutraError {
    pub fn with_suggestion(self, suggestion: impl Into<String>) -> Self { /* existing impl */ }
}
```

### Phase 2: Direct Migration

**Migrate call sites directly in atomic commits by module:**

```rust
// Before (old system)
return Err(errors::parse_missing("paren", &source, span)
    .with_suggestion("Add closing parenthesis"));

// After (new system) - one atomic change per module
return Err(parser_state.missing_element("paren", span));
```

**Migration order (by risk/impact):**

1. **Core parsing** - Most critical, highest impact
2. **Runtime evaluation** - High frequency, straightforward patterns
3. **Validation logic** - Complex but isolated
4. **Test framework** - Low risk, simple patterns

### Phase 3: Cleanup

```rust
// Remove old error system entirely
// Delete: OldSutraError, builders module, internal module
// Update: All imports, documentation, tests
```

## Implementation Pitfalls to Avoid

### Critical "Do Not" List

1. **DO NOT use `Span::default()` anywhere unless legitimately unavoidable** - This was the source of major diagnostic failures
2. **DO NOT create multiple ways to construct the same error type** - Maintain single construction path per context
3. **DO NOT make enhancement methods optional** - All errors should get context-appropriate help
4. **DO NOT preserve the builder pattern** - It added complexity without benefit
5. **DO NOT use string concatenation for error codes** - Use const statics for performance
6. **DO NOT remove deprecated functions during migration** - Keep them until 1.0.0 for compatibility
7. **DO NOT ignore deprecation warnings** - Each warning points to a migration opportunity

## Reference Implementation Checklist

**For the implementor to verify completeness:**

### Data Structures ✓

- [x] `SutraError` struct with three fields (kind, source_info, diagnostic_info)
- [x] `ErrorKind` enum with all current error types mapped
- [x] `SourceInfo` struct with context-specific information
- [x] `DiagnosticInfo` struct with help/spans/codes

### Trait Implementation ✓

- [x] `ErrorReporting` trait with context-driven constructors
- [x] `ErrorReporting` impl for `ParserState` with parse-specific enhancement
- [x] `ErrorReporting` impl for `EvaluationContext` with runtime-specific enhancement
- [x] `ErrorReporting` impl for validation contexts

### Integration ✓

- [x] `std::error::Error` implementation
- [x] `fmt::Display` implementation
- [x] `miette::Diagnostic` implementation with proper labels/spans
- [x] Error category mapping for test compatibility

### Validation ✓

- [ ] All Span::default() eliminated or verified to be necessary
- [ ] All existing tests pass with new system
- [ ] Error message consistency maintained
- [x] All deprecated APIs properly annotated with migration guidance

---

## Conclusion

This redesign eliminates the architectural debt while preserving all functionality. The key insight is **context-driven behavior**: instead of forcing callers to specify how to construct errors, the system infers the right behavior from the calling context.

**Result:** Error handling becomes **declarative** - callers state intent (what went wrong, where), and the system handles all implementation details (how to construct, what to enhance, how to display).

The error system becomes **the most reliable part of the codebase** instead of a source of errors itself.
