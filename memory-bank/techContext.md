# Sutra Engine - Technical Context

## Recent Technical Improvements (2025-07-08)

### Atom Standard Library Modernization

**Technical Implementation**:

- Macro-to-Function Conversion: All atom operations now use direct function calls instead of macros, improving maintainability and debuggability.
- Error construction helpers: `arity_error()`, `type_error()`, `validation_error()`
- Type alias for consistency: `pub type AtomResult = Result<(Value, World), SutraError>`
- Evaluation helpers: `eval_binary_numeric_op()`, `eval_nary_numeric_op()`, etc.
- Preserved context macro: `sub_eval_context!` for safe context management

## Technology Stack

### Core Language: Rust

- Memory safety without garbage collection
- Excellent performance for game engines
- Strong type system prevents runtime errors
- Rich ecosystem for parsing and data structures
- Zero-cost abstractions align with minimalism philosophy

### Key Dependencies

- **im**: Persistent, immutable data structures (`im = "15.1.0"`)
- **pest**: PEG parser generator (`pest = "2.7.10"`, `pest_derive = "2.7.10"`)
- **rand**: Random number generation (`rand = "0.8.5"`, `rand_xoshiro = "0.6.0"`)
- **serde**: Serialization/deserialization (`serde = "1.0"`, `serde_json = "1.0"`)
- **clap**: CLI argument parsing (`clap = "4.5.4"`)
- **termcolor**: Colored terminal output (`termcolor = "1.4.1"`)
- **difference**: Text diffs (`difference = "2.0.0"`)
- **walkdir**: Recursive directory traversal (`walkdir = "2.5.0"`)

## Project Structure (2025-07-07)

```
sutra/
├── Cargo.toml                  # Rust project configuration
├── src/
│   ├── syntax/                 # Parsing, error handling, validation
│   │   ├── cst_parser.rs       # Concrete syntax tree parser
│   │   ├── error.rs            # Syntax/parse error types
│   │   ├── grammar.pest        # PEG grammar definition
│   │   ├── parser.rs           # AST parser
│   │   ├── validate.rs         # Syntax validation passes
│   │   └── validator.rs        # Validation logic
│   ├── ast/                    # AST types and span tracking
│   │   ├── builder.rs          # AST construction utilities
│   │   └── value.rs            # Runtime data values
│   ├── atoms/                  # Core atom implementations
│   ├── macros/                 # Macro system and stdlib
│   ├── runtime/                # Evaluation engine and world state
│   │   ├── eval.rs             # Main evaluation logic
│   │   ├── path.rs             # Path-based addressing
│   │   ├── registry.rs         # Atom/macro registry
│   │   └── world.rs            # Persistent world state
│   ├── cli/                    # CLI logic and argument parsing
│   │   ├── args.rs             # CLI argument definitions
│   │   └── output.rs           # Output formatting and printing
│   ├── lib.rs                  # Library entry point
│   └── main.rs                 # CLI entry point
├── tests/
│   ├── rust/                   # Rust-based integration tests
│   └── scripts/                # Protocol-compliant .sutra test scripts
├── debug/
│   └── macro-testing/          # Debug files for macro investigation
├── docs/                       # Architecture and design documentation
└── memory-bank/                # Living project documentation
```

## Technical Architecture

### Data Structure Choices

**AST Representation:**

- Recursive enum for s-expressions with span information
- No optimization or transformation in AST layer
- Span-carrying nodes for precise error reporting

**World State:**

- Persistent HashMap from `im` crate for structural sharing
- Path-based addressing with type safety
- Immutable updates, explicit PRNG state for determinism

**Value System:**

- Tagged union of primitive types (no object-oriented features)
- Direct mapping to JSON for serialization
- Explicit type coercion handling

### Parsing Strategy

**Unified PEG Parser:**

- Single formal grammar in `src/syntax/grammar.pest`
- Handles both s-expression and brace-block syntax
- Built with `pest` library for performance and error reporting
- Transforms to canonical `Expr` AST with no semantic interpretation

### Evaluation Model

**Tail-Call Optimization:**

- Evaluator structured as iterative loop
- Enables unbounded recursion for simulation
- No stack overflow for well-formed programs

**Error Handling:**

- Result types throughout with span preservation
- Multiple error collection (not fail-fast)
- Clear separation of error categories

## Architecture Patterns

### Extension Architecture

- **Registry Pattern**: Dynamic atom/macro registration, runtime introspection
- **Output Abstraction**: Trait-based system, injectable for testing
- **Library-First Design**: Core as pure library, CLI as thin wrapper

### Performance Characteristics

- Handle worlds with thousands of entities/storylets
- Sub-millisecond evaluation for single storylets
- Minimal memory allocation during evaluation
- Efficient serialization for save/load

### Platform Support

- **Primary**: macOS, Linux, Windows (cross-platform Rust)
- **Future**: WASM for web deployment
- **Thread-Safe**: Immutable data design

## Testing Strategy

### Test Organization

- **Inline Tests**: Small modules
- **Rust Integration Tests**: `tests/rust/` (unit/integration testing)
- **Protocol-Compliant Tests**: `tests/scripts/` (`.sutra` scripts + expected output)

### Test Protocol

**All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors. No direct Rust API manipulation permitted.**

### Quality Assurance

- Every module tested in isolation
- Property-based testing for core operations
- Golden file tests for parsing and expansion
- Mock injection for external dependencies
- Full pipeline tests with realistic examples

## Build and Development

### Build System

- **Cargo**: Standard Rust build tool
- **CI/CD**: GitHub Actions for automated testing
- **Documentation**: Rust docs + custom markdown

### Debug Infrastructure

- **Systematic Debug Files**: `debug/macro-testing/` for macro investigation
- **Integration Test Runner**: Bootstrapped in `tests/scripts/`
- **Macro Tracing**: Available for debugging expansion

## Parsing Pipeline (2025-07-07)

The modular parsing pipeline is designed for Rust ergonomics:

- Enums for core types, trait objects for extensibility
- Serde-compatible serialization for all public types
- Unified, serializable diagnostics supporting CLI and editor integration
- Architecture supports incremental/partial parsing

See `docs/architecture/parsing-pipeline-plan.md` for complete technical context.

## Cross-References

- See `memory-bank/systemPatterns.md` for architectural patterns
- See `memory-bank/activeContext.md` for current work focus
- See `memory-bank/progress.md` for completed work and status
- See `docs/architecture/parsing-pipeline-plan.md` for pipeline details
- See `debug/macro-testing/README.md` for debug infrastructure

## Changelog

- **2025-07-07**: File hierarchy reorganized into modular directories, explicit mod.rs usage
- **2025-07-06**: Integration test runner bootstrapped, protocol-compliant tests established
- **2025-07-04**: Parsing pipeline technical rationale added, modular architecture confirmed
- **2025-07-03**: Technical context aligned with current codebase and guidelines
