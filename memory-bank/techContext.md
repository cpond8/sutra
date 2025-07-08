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

- **Language**: Rust (memory safety, performance, type system)
- **Key dependencies**: im (immutable data), pest (PEG parser), rand, serde, clap, termcolor, difference, walkdir

## Technical Constraints

- Pure functions and immutability
- Modular, testable codebase
- No global state except explicit world mutations

## Project Structure

- `src/` – core modules (syntax, ast, atoms, macros, runtime, cli)
- `tests/` – protocol-compliant .sutra scripts, test runner utilities
- `debug/` – macro investigation files
- `docs/` – architecture and design docs
- `memory-bank/` – living documentation

## Reference

- See `systemPatterns.md` for architecture
- See `progress.md` for implementation status
