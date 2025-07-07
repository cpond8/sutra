# Sutra Engine - Technical Context

## Technology Stack

### Core Language: Rust
- Memory safety without garbage collection
- Excellent performance for game engines
- Strong type system prevents many runtime errors
- Rich ecosystem for parsing and data structures
- Zero-cost abstractions align with minimalism philosophy

### Key Dependencies
- **im**: Persistent, immutable data structures (`im = "15.1.0"`)
- **rand**: Random number generation traits/utilities (`rand = "0.8.5"`)
- **rand_xoshiro**: High-performance, seedable PRNG (`rand_xoshiro = "0.6.0"`)
- **pest**: PEG parser generator (`pest = "2.7.10"`)
- **pest_derive**: Derive macro for `pest` (`pest_derive = "2.7.10"`)
- **serde**: Serialization/deserialization (`serde = { version = "1.0", features = ["derive"] }`)
- **serde_json**: JSON support (`serde_json = "1.0"`)
- **clap**: CLI argument parsing (`clap = { version = "4.5.4", features = ["derive"] }`)
- **termcolor**: Colored terminal output (`termcolor = "1.4.1"`)
- **difference**: Text diffs (`difference = "2.0.0"`)
- **walkdir**: Recursive directory traversal (`walkdir = "2.5.0"`)

## Development Environment

### Project Structure
```
sutra/
├── Cargo.toml          # Rust project configuration
├── src/
│   ├── lib.rs          # Core library API
│   ├── main.rs         # Main CLI entry point
│   ├── ast.rs          # AST types and span tracking
│   ├── value.rs        # Runtime data values
│   ├── world.rs        # Persistent world state
│   ├── error.rs        # Unified error types
│   ├── sutra.pest      # Formal PEG grammar for all syntaxes
│   ├── parser.rs       # Unified PEG-based parser
│   ├── atom.rs         # Irreducible operations
│   ├── eval.rs         # Evaluation engine
│   ├── macros.rs       # Macro expansion engine
│   ├── macros_std.rs   # Standard macro library
│   ├── validate.rs     # Validation passes (planned)
│   └── cli/            # CLI module
│       ├── mod.rs      # Main CLI logic and dispatch
│       ├── args.rs     # CLI argument definitions
│       └── output.rs   # Output formatting and printing
├── tests/              # Integration tests
└── docs/               # Design documentation
```

### Build System
- **Cargo**: Standard Rust build tool
- **Tests**: Unit tests per module, integration tests for pipeline
- **CI/CD**: GitHub Actions for automated testing
- **Documentation**: Rust docs + custom markdown

## Technical Constraints

### Performance Requirements
- Handle worlds with thousands of entities/storylets
- Sub-millisecond evaluation for single storylets
- Minimal memory allocation during evaluation
- Efficient serialization for save/load

### Platform Support
- **Primary**: macOS, Linux, Windows (via Rust's cross-platform support)
- **Future**: WASM for web deployment
- **Architecture**: Pure library design enables any frontend

## Data Structure Choices

### AST Representation
- Recursive enum for s-expressions
- Span information for error reporting
- No optimization or transformation in AST layer

### World State
- Persistent HashMap from `im` crate
- Path-based addressing with type safety
- Immutable updates with structural sharing
- Explicit PRNG state for determinism

### Value System
- Tagged union of primitive types
- No object-oriented features
- Direct mapping to JSON for serialization
- Type coercion handled explicitly

## Parsing Strategy

### Unified PEG Parser
- Single, formal PEG (Parsing Expression Grammar) in `src/sutra.pest` is the source of truth for all syntaxes.
- Built using the `pest` library for performance and error reporting.
- Handles both canonical s-expression and author-friendly brace-block syntax.
- Transforms source text into canonical `Expr` AST, with no semantic interpretation.
- Ensures maintainability, consistency, and transparency.

## Evaluation Model

### Tail-Call Optimization
- Evaluator structured as iterative loop
- Enables unbounded recursion for simulation
- No stack overflow for well-formed programs
- Critical for agent-based systems

### Error Handling
- Result types throughout
- Span preservation for user feedback
- Multiple error collection (not fail-fast)
- Clear separation of error categories

## Extension Architecture

### Registry Pattern
- Dynamic atom and macro registration
- Runtime introspection capabilities
- Clean plugin boundaries
- No recompilation for new features

### Output Abstraction
- Trait-based output system
- Injectable for testing and UI integration
- No hardcoded I/O or rendering
- Multiple output streams supported

## Testing Strategy

### Unit Testing
- Every module tested in isolation
- Property-based testing for core operations
- Golden file tests for parsing and expansion
- Mock injection for all external dependencies

### Integration Testing
- Full pipeline tests with realistic examples
- Regression testing on all example scripts
- Performance benchmarking on larger worlds
- Cross-platform compatibility testing

- **Protocol Requirement:** All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted. A full test suite rewrite is required. See `memory-bank/README.md` and `memory-bank/activeContext.md` for details.

- **Integration Test Runner Bootstrapped (2025-07-06):**
  - `tests/scripts/` directory created for protocol-compliant integration tests.
  - First `.sutra` test script (`hello_world.sutra`) and expected output (`hello_world.expected`) added. See `activeContext.md` and `progress.md`.

## Deployment Considerations

### Library-First Design
- Core engine as Rust library crate
- CLI as thin wrapper over library API
- No global state or initialization requirements
- Thread-safe by design (immutable data)

### Future UI Integration
- Pure API enables any frontend
- WebAssembly compilation supported
- Real-time introspection and debugging
- Hot-reload for development workflows

## Alignment with Current Codebase

- All technical patterns and constraints described above are implemented and enforced in the current codebase.
- The project structure, dependencies, and architecture are up-to-date and match the live system.

## Cross-References

- See `memory-bank/projectbrief.md` for project vision and aspirations.
- See `memory-bank/productContext.md` for product rationale and user needs.
- See `memory-bank/systemPatterns.md` for architectural and design patterns.
- See `memory-bank/activeContext.md` for current work focus and priorities.
- See `memory-bank/progress.md` for completed work and next steps.
- See `.cursor/rules/memory-bank.mdc` for update protocol and overlays.

## Parsing Pipeline: Technical Rationale (2025-07-04)

- The new parsing pipeline is designed for Rust ergonomics: enums for core types, trait objects for extensibility, and serde-compatible serialization for all public types and errors.
- Diagnostics are unified and serializable, supporting CLI tools and editor integration.
- The architecture supports incremental/partial parsing and golden tests with real-world content.

See `docs/architecture/parsing-pipeline-plan.md` for the full plan and technical context.

## Changelog

- 2025-07-03: Updated to resolve all audit TODOs, clarify technical context, and align with current codebase and guidelines.
- 2025-06-30: Initial synthesis from legacy documentation.
- 2025-07-04: Added section on parsing pipeline technical rationale and requirements.
- 2025-07-06: Batch refactor for Rust idiom compliance (implicit/explicit return style), match exhaustiveness, and error handling. Explicit returns for early exits restored. All match arms for Expr variants in eval_expr restored. Protocol-driven, batch-based, test-first approach enforced. All tests pass. Lesson: Always enumerate all functions for audit, not just those surfaced by search.
- 2025-07-07: Macro/atom registry and test system are now fully Rust-idiomatic, with anti-nesting audits and iterator combinator refactors complete. Feature-gated (test-atom) and debug-assertion-based test atom registration is in place; integration tests that require test-only atoms are now feature-gated and optional. Protocol for feature-gated/optional integration tests is documented in systemPatterns.md. All code, tests, and documentation are up to date and compliant as of this session.
