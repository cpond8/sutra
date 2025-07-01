# Sutra Engine - Technical Context

## Technology Stack

### Core Language: Rust
**Why Rust:**
- Memory safety without garbage collection
- Excellent performance for game engines
- Strong type system prevents many runtime errors
- Rich ecosystem for parsing and data structures
- Zero-cost abstractions align with minimalism philosophy

### Key Dependencies
- **im**: For persistent, immutable data structures (`im = "15.1.0"`).
- **rand**: Core random number generation traits and utilities (`rand = "0.8.5"`).
- **rand_xoshiro**: A specific, high-performance, seedable PRNG implementation (`rand_xoshiro = "0.6.0"`).
- **pest**: A powerful, expressive PEG (Parsing Expression Grammar) parser generator (`pest = "2.0"`).
- **pest_derive**: The derive macro for `pest` (`pest_derive = "2.0"`).
- **serde**: (Planned) For serialization of world snapshots and debugging.
- **Standard library**: Used extensively to keep external dependencies minimal.

## Development Environment

### Project Structure
```
sutra/
├── Cargo.toml          # Rust project configuration
├── src/
│   ├── lib.rs          # Core library API
│   ├── ast.rs          # AST types and span tracking
│   ├── value.rs        # Runtime data values
│   ├── world.rs        # Persistent world state
│   ├── sutra.pest      # Formal PEG grammar for all syntaxes
│   ├── parser.rs       # Unified PEG-based parser
│   ├── atom.rs         # Irreducible operations
│   ├── eval.rs         # Evaluation engine
│   ├── macro.rs        # Macro expansion system
│   ├── validate.rs     # Validation passes
│   ├── macros_std.rs   # Standard macro library
│   └── cli.rs          # Command-line interface
├── tests/              # Integration tests
├── examples/           # Example scripts and usage
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
- A single, formal PEG (Parsing Expression Grammar) is defined in `src/sutra.pest`. This file is the single source of truth for all supported syntaxes.
- The parser is built using the `pest` library, which provides excellent performance and rich, precise error reporting.
- It handles both canonical s-expression syntax and the author-friendly brace-block syntax.
- The parser's sole responsibility is to transform source text into the canonical `Expr` AST, with no semantic interpretation.
- This unified approach ensures maximum maintainability, consistency, and transparency.

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

*Last Updated: 2025-06-30*
