# Architectural Improvement Proposal: Technical Appendices

## Appendix A: Detailed Code Examples

### A.1: Registry Token System Implementation

```rust
//! Complete implementation of compile-time registry enforcement

/// Zero-sized token proving canonical construction
/// Cannot be constructed outside this module
#[derive(Debug, Clone)]
pub struct RegistryToken(());

/// Wrapper ensuring canonical construction
pub struct CanonicalAtomRegistry {
    registry: AtomRegistry,
    _token: RegistryToken, // Compile-time proof, zero runtime cost
}

impl CanonicalAtomRegistry {
    /// Access inner registry - only possible with valid token
    pub fn inner(&self) -> &AtomRegistry {
        &self.registry
    }

    /// Consume wrapper to extract registry
    pub fn into_inner(self) -> AtomRegistry {
        self.registry
    }
}

/// Canonical builder - the ONLY way to create valid registries
pub fn build_canonical_atom_registry() -> CanonicalAtomRegistry {
    let mut registry = AtomRegistry::new();

    // All standard registration logic centralized here
    crate::atoms::std::register_std_atoms(&mut registry);

    #[cfg(any(test, feature = "test-atom"))]
    {
        crate::atoms::std::register_test_atoms(&mut registry);
    }

    CanonicalAtomRegistry {
        registry,
        _token: RegistryToken(()), // Unreachable from outside
    }
}

// Usage in evaluation context
pub fn evaluate_with_registry(
    expr: &WithSpan<Expr>,
    registry: &CanonicalAtomRegistry, // Guaranteed canonical
    world: &World,
) -> Result<(Value, World), SutraError> {
    // Implementation uses registry.inner() to access atoms
    // Compile-time guarantee of canonical construction
}
```

### A.2: Pure Function Validation Pipeline

```rust
//! Composable validation system using pure functions

/// Validation result with error accumulation
pub type ValidationResult = Result<(), Vec<SutraError>>;

/// Pure validator function type
pub type Validator = fn(&WithSpan<Expr>) -> ValidationResult;

/// Core composition mechanism
pub fn validate_with_pipeline(
    expr: &WithSpan<Expr>,
    validators: &[Validator],
) -> ValidationResult {
    let errors: Vec<SutraError> = validators
        .iter()
        .filter_map(|validator| validator(expr).err())
        .flatten()
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

/// Individual validators as pure functions
pub fn validate_no_bare_symbols(expr: &WithSpan<Expr>) -> ValidationResult {
    match &expr.value {
        Expr::Symbol(_, span) => Err(vec![validation_error(
            "Bare symbols not allowed in canonical AST",
            Some(span.clone()),
        )]),
        Expr::List(items, _) => {
            // Recursively validate all list items
            let results: Vec<ValidationResult> = items
                .iter()
                .map(validate_no_bare_symbols)
                .collect();
            combine_validation_results(results)
        }
        _ => Ok(()), // Other expressions are valid
    }
}

pub fn validate_canonical_paths(expr: &WithSpan<Expr>) -> ValidationResult {
    match &expr.value {
        Expr::Path(path, span) => {
            if path.0.is_empty() {
                Err(vec![validation_error(
                    "Empty paths are not allowed",
                    Some(span.clone()),
                )])
            } else {
                Ok(())
            }
        }
        Expr::List(items, _) => {
            let results: Vec<ValidationResult> = items
                .iter()
                .map(validate_canonical_paths)
                .collect();
            combine_validation_results(results)
        }
        _ => Ok(()),
    }
}

/// Pre-defined pipelines for common validation scenarios
pub const PRE_MACRO_VALIDATORS: &[Validator] = &[
    validate_param_list_placement,
];

pub const POST_MACRO_VALIDATORS: &[Validator] = &[
    validate_no_bare_symbols,
    validate_canonical_paths,
    validate_param_list_placement,
];

pub const PRE_EVAL_VALIDATORS: &[Validator] = &[
    validate_no_bare_symbols,
    validate_canonical_paths,
];

/// Convenience functions for standard validation points
pub fn validate_pre_macro(expr: &WithSpan<Expr>) -> ValidationResult {
    validate_with_pipeline(expr, PRE_MACRO_VALIDATORS)
}

pub fn validate_post_macro(expr: &WithSpan<Expr>) -> ValidationResult {
    validate_with_pipeline(expr, POST_MACRO_VALIDATORS)
}

pub fn validate_pre_eval(expr: &WithSpan<Expr>) -> ValidationResult {
    validate_with_pipeline(expr, PRE_EVAL_VALIDATORS)
}
```

### A.3: Data-Driven Metadata System

```rust
//! Metadata system using simple data structures

/// Flexible arity specification
#[derive(Debug, Clone, PartialEq)]
pub enum Arity {
    Exact(usize),           // Exactly N arguments
    AtLeast(usize),         // At least N arguments
    Range(usize, usize),    // Between min and max (inclusive)
    Any,                    // Any number of arguments
}

impl Arity {
    /// Check if argument count satisfies arity requirement
    pub fn check(&self, arg_count: usize) -> bool {
        match self {
            Arity::Exact(n) => arg_count == *n,
            Arity::AtLeast(n) => arg_count >= *n,
            Arity::Range(min, max) => arg_count >= *min && arg_count <= *max,
            Arity::Any => true,
        }
    }

    /// Generate helpful error message for arity mismatch
    pub fn error_message(&self, actual: usize) -> String {
        match self {
            Arity::Exact(expected) => {
                format!("Expected exactly {} arguments, got {}", expected, actual)
            }
            Arity::AtLeast(min) => {
                format!("Expected at least {} arguments, got {}", min, actual)
            }
            Arity::Range(min, max) => {
                format!("Expected {}-{} arguments, got {}", min, max, actual)
            }
            Arity::Any => unreachable!("Any arity should never fail"),
        }
    }
}

/// Rich metadata for atoms
#[derive(Debug, Clone)]
pub struct AtomMetadata {
    pub name: String,
    pub arity: Arity,
    pub description: String,
    pub is_pure: bool,          // Separates pure from side-effecting
    pub examples: Vec<String>,
    pub see_also: Vec<String>,  // Related atoms/macros
    pub since_version: String,  // API versioning
}

/// Atom entry combining function and metadata
#[derive(Clone)]
pub struct AtomEntry {
    pub func: AtomFn,
    pub metadata: AtomMetadata,
}

/// Enhanced registry with full metadata support
#[derive(Default)]
pub struct MetaAtomRegistry {
    entries: HashMap<String, AtomEntry>,
}

impl MetaAtomRegistry {
    /// Register atom with metadata
    pub fn register(&mut self, entry: AtomEntry) {
        self.entries.insert(entry.metadata.name.clone(), entry);
    }

    /// Get function for execution
    pub fn get_function(&self, name: &str) -> Option<&AtomFn> {
        self.entries.get(name).map(|entry| &entry.func)
    }

    /// Get metadata for introspection
    pub fn get_metadata(&self, name: &str) -> Option<&AtomMetadata> {
        self.entries.get(name).map(|entry| &entry.metadata)
    }

    /// List all atom names
    pub fn list_names(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// Filter by purity for optimization
    pub fn list_pure_atoms(&self) -> Vec<String> {
        self.entries
            .values()
            .filter(|entry| entry.metadata.is_pure)
            .map(|entry| entry.metadata.name.clone())
            .collect()
    }

    /// List side-effecting atoms
    pub fn list_side_effect_atoms(&self) -> Vec<String> {
        self.entries
            .values()
            .filter(|entry| !entry.metadata.is_pure)
            .map(|entry| entry.metadata.name.clone())
            .collect()
    }

    /// Validate arity for function call
    pub fn check_arity(&self, name: &str, arg_count: usize) -> Result<(), String> {
        if let Some(metadata) = self.get_metadata(name) {
            if metadata.arity.check(arg_count) {
                Ok(())
            } else {
                Err(metadata.arity.error_message(arg_count))
            }
        } else {
            Err(format!("Unknown atom: {}", name))
        }
    }
}

/// Helper function for creating atom entries
pub fn atom_entry(
    name: &str,
    func: AtomFn,
    arity: Arity,
    is_pure: bool,
    description: &str,
) -> AtomEntry {
    AtomEntry {
        func,
        metadata: AtomMetadata {
            name: name.to_string(),
            arity,
            description: description.to_string(),
            is_pure,
            examples: Vec::new(),
            see_also: Vec::new(),
            since_version: "1.0".to_string(),
        },
    }
}

/// Example registration using metadata system
pub fn register_math_atoms(registry: &mut MetaAtomRegistry) {
    registry.register(atom_entry(
        "+",
        atom_add,
        Arity::AtLeast(2),
        true, // Pure function
        "Adds two or more numbers together",
    ));

    registry.register(atom_entry(
        "print",
        atom_print,
        Arity::Exact(1),
        false, // Side-effecting
        "Prints a value to output",
    ));
}
```

### A.4: Isolated Path Canonicalization

```rust
//! Complete path canonicalization module

/// The ONLY function that interprets path syntax
/// Contract: All modules must use this for path conversion
pub fn canonicalize_path(expr: &WithSpan<Expr>) -> Result<Path, SutraError> {
    match &expr.value {
        // Dotted symbol: "player.score" -> ["player", "score"]
        Expr::Symbol(s, _) => {
            let segments: Vec<String> = s.split('.').map(String::from).collect();
            validate_path_segments(&segments, &expr.span)?;
            Ok(Path(segments))
        }

        // List form: (player score) -> ["player", "score"]
        Expr::List(items, _) => {
            if items.is_empty() {
                return Err(validation_error(
                    "Path lists cannot be empty",
                    Some(expr.span.clone()),
                ));
            }

            let segments: Result<Vec<String>, SutraError> = items
                .iter()
                .map(extract_path_segment)
                .collect();

            let segments = segments?;
            validate_path_segments(&segments, &expr.span)?;
            Ok(Path(segments))
        }

        // Already canonical
        Expr::Path(path, _) => {
            validate_path_segments(&path.0, &expr.span)?;
            Ok(path.clone())
        }

        _ => Err(validation_error(
            "Invalid path format: expected symbol, list, or existing path",
            Some(expr.span.clone()),
        )),
    }
}

/// Extract string from path segment expression
fn extract_path_segment(item: &WithSpan<Expr>) -> Result<String, SutraError> {
    match &item.value {
        Expr::Symbol(s, _) | Expr::String(s, _) => Ok(s.clone()),
        _ => Err(validation_error(
            "Path segments must be symbols or strings",
            Some(item.span.clone()),
        )),
    }
}

/// Comprehensive path segment validation
fn validate_path_segments(segments: &[String], span: &Span) -> Result<(), SutraError> {
    if segments.is_empty() {
        return Err(validation_error(
            "Paths cannot be empty",
            Some(span.clone()),
        ));
    }

    for (i, segment) in segments.iter().enumerate() {
        if segment.is_empty() {
            return Err(validation_error(
                &format!(
                    "Path segment {} is empty (check for double dots like 'player..score')",
                    i + 1
                ),
                Some(span.clone()),
            ));
        }

        if !is_valid_identifier(segment) {
            return Err(validation_error(
                &format!(
                    "Invalid path segment '{}': must be a valid identifier",
                    segment
                ),
                Some(span.clone()),
            ));
        }

        // Additional validation for reserved words
        if is_reserved_word(segment) {
            return Err(validation_error(
                &format!(
                    "Path segment '{}' is a reserved word",
                    segment
                ),
                Some(span.clone()),
            ));
        }
    }

    Ok(())
}

/// Validate identifier format
fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars().next().unwrap().is_alphabetic()
        && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

/// Check for reserved words
fn is_reserved_word(s: &str) -> bool {
    matches!(s, "if" | "do" | "set!" | "get" | "del!" | "core")
}

/// Helper: wrap expression in (core/get path) call
pub fn wrap_in_get_call(expr: &WithSpan<Expr>) -> Result<WithSpan<Expr>, SutraError> {
    let path = canonicalize_path(expr)?;

    let get_symbol = WithSpan {
        value: Expr::Symbol("core/get".to_string(), expr.span.clone()),
        span: expr.span.clone(),
    };

    let path_expr = WithSpan {
        value: Expr::Path(path, expr.span.clone()),
        span: expr.span.clone(),
    };

    Ok(WithSpan {
        value: Expr::List(vec![get_symbol, path_expr], expr.span.clone()),
        span: expr.span.clone(),
    })
}

/// Helper: create path expression from Path object
pub fn path_to_expr(path: &Path, span: Span) -> WithSpan<Expr> {
    WithSpan {
        value: Expr::Path(path.clone(), span.clone()),
        span,
    }
}

/// Helper: check if expression represents a valid path
pub fn is_path_like(expr: &WithSpan<Expr>) -> bool {
    matches!(
        &expr.value,
        Expr::Symbol(_, _) | Expr::Path(_, _) | Expr::List(_, _)
    ) && canonicalize_path(expr).is_ok()
}
```

## Appendix B: Performance Analysis

### B.1: Compile-Time vs Runtime Costs

| **Component**         | **Current Approach** | **Proposed Approach** | **Performance Impact**                      |
| --------------------- | -------------------- | --------------------- | ------------------------------------------- |
| Registry Construction | Runtime validation   | Compile-time tokens   | **Zero cost** - compile-time only           |
| Validation            | Ad-hoc checks        | Systematic pipeline   | **Minimal** - structured error accumulation |
| Metadata Access       | None available       | Rich data structures  | **Zero cost** - direct field access         |
| Path Canonicalization | Centralized function | Enhanced validation   | **Negligible** - better error messages      |

### B.2: Memory Impact

- **Registry Tokens**: Zero-sized types, no runtime memory overhead
- **Metadata Structures**: Constant overhead per atom/macro (estimated 200-500 bytes)
- **Validation Pipeline**: Temporary allocation for error accumulation only
- **Path Validation**: No additional memory, enhanced validation logic only

### B.3: Benchmark Projections

Based on similar pure-function architectures:

- **Registry access**: No change (same HashMap lookup)
- **Validation overhead**: < 5% for complex expressions, negligible for simple ones
- **Metadata queries**: Direct field access, faster than current approaches
- **Path canonicalization**: Same algorithmic complexity, better error handling

## Appendix C: Migration Strategy

### C.1: Phase 1 Migration (Registry Enforcement)

**Step 1**: Implement token system

```bash
# Create new registry_builder.rs module
# Implement CanonicalAtomRegistry and CanonicalMacroRegistry
# Add build_canonical_* functions
```

**Step 2**: Update call sites

```bash
# Find all AtomRegistry::new() and MacroRegistry::new() calls
grep -r "Registry::new()" src/
# Update CLI module: src/cli/mod.rs
# Update library module: src/lib.rs
```

**Step 3**: Enforce restrictions

```bash
# Make original constructors pub(crate)
# Add regression tests
# Verify compilation fails for direct construction
```

### C.2: Phase 2 Migration (Validation Pipeline)

**Step 1**: Implement validation functions

```bash
# Create validation_pipeline.rs module
# Implement core validators as pure functions
# Add error accumulation logic
```

**Step 2**: Add pipeline checkpoints

```bash
# Update parsing pipeline to include validation
# Add validation calls to CLI and library entry points
# Implement error reporting integration
```

**Step 3**: Define standard pipelines

```bash
# Create pre-macro, post-macro, pre-eval pipelines
# Add convenience functions for common use cases
# Update documentation and examples
```

### C.3: Backward Compatibility Strategy

- **Registry**: Existing code continues to work during transition
- **Validation**: New validation is additive, doesn't break existing functionality
- **Metadata**: Enhanced registries are backward compatible with current usage
- **Paths**: Existing path canonicalization behavior is preserved

### C.4: Testing Strategy

Each phase includes comprehensive testing:

- **Unit tests**: Every pure function tested independently
- **Integration tests**: Full pipeline testing with realistic examples
- **Regression tests**: Ensure no existing functionality breaks
- **Performance tests**: Benchmark critical paths before and after changes

## Appendix D: Future Extensions

### D.1: Parallel Validation

With pure function validators, parallel validation becomes trivial:

```rust
// Future: parallel validation for large ASTs
pub fn validate_parallel(expr: &WithSpan<Expr>, validators: &[Validator]) -> ValidationResult {
    use rayon::prelude::*;

    let results: Vec<ValidationResult> = validators
        .par_iter()
        .map(|validator| validator(expr))
        .collect();

    combine_validation_results(results)
}
```

### D.2: Plugin System

Metadata-driven approach enables dynamic plugin loading:

```rust
// Future: dynamic atom/macro loading
pub trait AtomPlugin {
    fn entries(&self) -> Vec<AtomEntry>;
}

pub fn load_plugin(plugin: Box<dyn AtomPlugin>, registry: &mut MetaAtomRegistry) {
    for entry in plugin.entries() {
        registry.register(entry);
    }
}
```

### D.3: Advanced Introspection

Rich metadata enables sophisticated tooling:

```rust
// Future: advanced introspection for IDE support
pub fn generate_documentation(registry: &MetaAtomRegistry) -> String {
    registry.list_names()
        .iter()
        .map(|name| {
            let meta = registry.get_metadata(name).unwrap();
            format!(
                "## {}\n{}\n**Arity:** {:?}\n**Pure:** {}\n",
                meta.name, meta.description, meta.arity, meta.is_pure
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```
