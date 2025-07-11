# Sutra Engine - Technical Context

## Technology Stack

**Core Language**: Rust - Memory safety, performance, cross-platform support, zero-cost abstractions.

**Key Dependencies**: `im` (persistent data structures), `pest` (PEG parser), `rand` (RNG), `serde` (serialization), `clap` (CLI), `termcolor` (terminal output), `difference` (text diffs), `walkdir` (directory traversal).

## Project Structure

```
sutra/
├── src/
│   ├── syntax/      # Parsing, error handling, validation
│   ├── ast/         # AST types and span tracking
│   ├── atoms/       # Core atom implementations
│   ├── macros/      # Macro system and stdlib
│   ├── runtime/     # Evaluation engine and world state
│   ├── cli/         # CLI logic and argument parsing
│   └── test_utils.rs # Centralized test utilities
├── tests/
│   ├── scripts/     # Protocol-compliant .sutra test scripts
│   └── *.rs         # Rust integration tests
```

## Technical Architecture

**Data Structures**: AST as recursive enum with span information, World State as persistent HashMap with path-based addressing, Values as tagged union with direct JSON mapping.

**Parsing**: Unified PEG parser with `pest`, handles s-expression and brace-block syntax.

**Evaluation**: Tail-call optimized iterative loop, enables unbounded recursion.

**Performance**: Sub-millisecond evaluation, minimal allocation, efficient serialization.

**Testing**: Protocol-compliant `.sutra` scripts in `tests/scripts/` with file-based test runner in `tests/common.rs`.

**Development**: Standard Rust/Cargo toolchain with GitHub Actions CI/CD.
