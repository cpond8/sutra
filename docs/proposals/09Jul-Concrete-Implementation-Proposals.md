# [OBSOLETE] Error Handling Enhancement: Quick Wins Implementation Plan

**ATTN:** This plan was superceded by 13Jul-Error-Handling-Proposal.md, and so is non-canonical.

Based on the architectural evaluation, this document focuses exclusively on the error handling enhancement (Rating: 7/10) as the only viable recommendation. The memory optimization proposal was rejected due to extreme implementation complexity (8/10 difficulty) and uncertain benefits.

This plan prioritizes incremental "easy wins" with clear cost-benefit analysis, allowing for quick implementation and validation before considering any larger refactors.

## Why Error Handling Only?

**Cost-Benefit Analysis:**

- **Error Enhancement**: 6-8 weeks, low risk (2/10), high developer benefit
- **Memory Optimization**: 10-16 weeks, very high risk (9/10), uncertain benefit

The error handling improvement offers immediate, measurable benefits with minimal architectural disruption.

## Error Handling Enhancement: Incremental Implementation

### Current State Analysis

**Strengths (Keep):**

- Centralized error construction in `src/syntax/error.rs`
- Rich `EvalError` type with context and suggestions
- Domain-specific constructors (`eval_arity_error`, `eval_type_error`, etc.)

**Pain Points (Fix):**

- Manual `Display` implementations create inconsistency
- Error construction could be more ergonomic
- CLI error reporting lacks error chain traversal

### Quick Wins Strategy: 4 Phases

Each phase is a standalone improvement that can be implemented, tested, and delivered independently.

---

## Phase 1: Add `thiserror` Dependencies (Week 1)

**Scope**: Zero-risk dependency addition with immediate validation

### Implementation

**File: `Cargo.toml`**

```toml
[dependencies]
# ...existing dependencies...
thiserror = "1.0"
```

**File: `src/syntax/error.rs`** - Add derive only to existing types:

```rust
use thiserror::Error;

// STEP 1: Just add derives, keep everything else identical
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[error("Evaluation error: {message}")]
pub struct EvalError {
    pub message: String,
    pub expanded_code: String,
    pub original_code: Option<String>,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
pub enum SutraErrorKind {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Macro error: {0}")]
    Macro(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("{0}")]
    Eval(#[from] EvalError),
    #[error("IO error: {0}")]
    Io(String),
    #[error("Malformed AST error: {0}")]
    MalformedAst(String),
    #[error("Internal parse error: {0}")]
    InternalParse(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[error("{kind}")]
pub struct SutraError {
    #[source]
    pub kind: SutraErrorKind,
    pub span: Option<crate::ast::Span>,
}

// Keep ALL existing helper functions unchanged
// ...existing code...
```

### Cost-Benefit Analysis

- **Cost**: 2-3 hours, zero breaking changes
- **Benefit**: Automatic `Display` and `Error` trait implementations
- **Risk**: Near zero - purely additive
- **Validation**: Compile and run existing tests

---

## Phase 2: Enhanced CLI Error Reporting (Week 2)

**Scope**: Leverage `thiserror`'s error chaining for better diagnostics

### Implementation

**File: `src/cli/output.rs`** - Enhance error display:

```rust
impl OutputSink for StdoutSink {
    fn emit(&mut self, message: &str, span: Option<&Span>) {
        // Standard error output with span info
        if let Some(span) = span {
            eprintln!("[{}:{}] {}", span.start, span.end, message);
        } else {
            eprintln!("{}", message);
        }

        // NEW: Enhanced error chain reporting
        if let Ok(error) = message.parse::<SutraError>() {
            self.print_error_chain(&error);
        }
    }
}

// NEW: Helper method for error chain traversal
impl StdoutSink {
    fn print_error_chain(&self, error: &SutraError) {
        let mut source = error.source();
        while let Some(err) = source {
            eprintln!("  └─ Caused by: {}", err);
            source = err.source();
        }
    }
}
```

### Cost-Benefit Analysis

- **Cost**: 4-6 hours implementation
- **Benefit**: Much better error diagnostics for users
- **Risk**: Low - only affects CLI output
- **Validation**: Test with existing error cases

---

## Phase 3: Error Construction Ergonomics (Week 3)

**Scope**: Add convenience methods without changing existing patterns

### Implementation

**File: `src/syntax/error.rs`** - Add ergonomic builders:

```rust
// NEW: Fluent builder for complex error scenarios
pub struct SutraErrorBuilder {
    kind: SutraErrorKind,
    span: Option<Span>,
}

impl SutraErrorBuilder {
    pub fn new(kind: SutraErrorKind) -> Self {
        Self { kind, span: None }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn build(self) -> SutraError {
        SutraError {
            kind: self.kind,
            span: self.span,
        }
    }
}

// NEW: Convenience methods for common patterns
impl SutraError {
    pub fn parse_with_span(message: impl Into<String>, span: Span) -> Self {
        SutraErrorBuilder::new(SutraErrorKind::Parse(message.into()))
            .with_span(span)
            .build()
    }

    pub fn eval_with_context(
        message: impl Into<String>,
        expanded_code: impl Into<String>,
        span: Option<Span>,
    ) -> Self {
        let eval_error = EvalError {
            message: message.into(),
            expanded_code: expanded_code.into(),
            original_code: None,
            suggestion: None,
        };
        SutraError {
            kind: SutraErrorKind::Eval(eval_error),
            span,
        }
    }
}

// Keep ALL existing helper functions for backward compatibility
// ...existing code...
```

### Cost-Benefit Analysis

- **Cost**: 6-8 hours implementation and testing
- **Benefit**: Better developer ergonomics, cleaner error construction
- **Risk**: Low - purely additive, doesn't break existing code
- **Validation**: Use new patterns in 2-3 places, ensure old patterns still work

---

## Phase 4: Gradual Migration and Cleanup (Week 4)

**Scope**: Selectively migrate high-value call sites to new patterns

### Implementation Strategy

**Target 3-5 high-impact locations for new patterns:**

1. **Parser error construction** (high frequency)
2. **CLI command error handling** (user-facing)
3. **Evaluation errors** (complex context)

**File: `src/syntax/parser.rs`** - Example migration:

```rust
// OLD pattern (keep working)
// return Err(parse_error("Expected symbol", Some(span)));

// NEW pattern (migrate selectively)
return Err(SutraError::parse_with_span("Expected symbol", span));
```

**Migration criteria:**

- ✅ High-frequency error sites
- ✅ Complex error construction
- ❌ Simple, working error sites
- ❌ Low-value or rarely hit paths

### Cost-Benefit Analysis

- **Cost**: 8-12 hours selective migration
- **Benefit**: Demonstrate new patterns, validate ergonomics
- **Risk**: Very low - keep old patterns working
- **Validation**: Performance and error message quality should be identical or better

---

## Implementation Summary

### Total Timeline: 4 weeks

- **Week 1**: Dependencies and derives (2-3 hours)
- **Week 2**: CLI error reporting (4-6 hours)
- **Week 3**: Builder patterns (6-8 hours)
- **Week 4**: Selective migration (8-12 hours)

### Risk Mitigation Strategy

1. **All existing error helpers remain functional**
2. **Each phase is independently testable**
3. **No breaking changes to public APIs**
4. **Gradual adoption - teams can use old or new patterns**

### Success Metrics

- **Phase 1**: Compilation success with new derives
- **Phase 2**: Better CLI error output with chaining
- **Phase 3**: New builder patterns available and tested
- **Phase 4**: 3-5 call sites successfully migrated

### What We're NOT Doing

- ❌ Large-scale refactoring of all error sites
- ❌ Breaking changes to existing error patterns
- ❌ Memory optimization with `Cow<str>` (rejected)
- ❌ Complex architectural changes

This approach prioritizes **quick wins** with **immediate benefits** while preserving all existing functionality and minimizing implementation risk.
