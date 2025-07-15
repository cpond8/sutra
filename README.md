# Sutra Engine

Sutra is a compositional, emergent, and narrative-rich game engine implemented in Rust. It provides a minimal, extensible core for authoring interactive fiction, simulations, and system-driven narratives using the Verse language.

---

## Design Philosophy and Purpose

Sutra is designed to provide a minimal, compositional, and extensible foundation for building interactive fiction, simulations, and system-driven narratives.
The engine aims to resolve the following problems, as evidenced by code and documentation:

- **Fragmentation and Redundancy**:
  By enforcing a single source of truth for all concepts and patterns, Sutra eliminates documentation and implementation drift.

- **Complexity and Inflexibility**:
  The engine is built around a minimal set of irreducible "atoms" and a macro system, enabling maximal compositionality and extensibility without feature bloat.

- **Opaque or Rigid Authoring**:
  Sutra's uniform syntax (s-expressions and brace-blocks) and macro system empower authors to define new patterns and abstractions without modifying the engine core.

- **Separation of Concerns**:
  The architecture strictly separates parsing, macro-expansion, validation, evaluation, and presentation, supporting maintainability and testability.

- **Transparency and Traceability**:
  All computation, macro expansion, and world state changes are inspectable and debuggable.

For a detailed statement of philosophy and guiding principles, see [`docs/philosophy/philosophy.md`](docs/philosophy/philosophy.md).

---

## Directory Structure

```
.
├── src/                # Main implementation: engine, CLI, and modules
│   ├── cli/            # Command-line interface (CLI) implementation
│   └── ...             # Core modules: ast, atom, macros, parser, eval, etc.
├── tests/              # Test suite: evaluation, parser, macro expansion
├── docs/               # Modular documentation system (see below)
├── memory-bank/        # Project and system context (not code)
├── Cargo.toml          # Rust package manifest
├── Cargo.lock          # Cargo dependency lockfile
├── test.sutra          # Example/test script
└── ...                 # Standard project/config files and build artifacts
```

---

## Build and Setup

This project uses [Cargo](https://doc.rust-lang.org/cargo/) for building and testing.

To build the project:
```sh
cargo build
```

To run the test suite:
```sh
cargo test
```

---

## Codebase Overview

- **Library Entry Point:**
  `src/lib.rs` re-exports all main modules.

- **CLI Entry Point:**
  `src/main.rs` launches the command-line interface via `sutra::cli::run()`.

- **Main Modules:**
  - `ast.rs`, `atom.rs`, `atoms_std.rs`, `macros.rs`, `macros_std.rs`, `parser.rs`, `eval.rs`, `error.rs`, `value.rs`, `world.rs`, `registry.rs`, `path.rs`, `sutra.pest`
  - `cli/` submodule: `mod.rs`, `args.rs`, `output.rs`

- **Parser:**
  Uses a PEG grammar (`sutra.pest`) to support both s-expression and brace-block syntax.

---

## Test Suite

- `core_eval_tests.rs` — Core evaluation and integration tests
- `parser_tests.rs` — Parser tests (s-expr, brace-block, error handling)
- `macro_expansion_tests.rs` — Macro system expansion and error tests

---

## Documentation

Canonical documentation is maintained in the `docs/` directory.
See `docs/README.md` for structure, status, and navigation.

---

## Usage

The Sutra engine is primarily used via its command-line interface.

To run the CLI:
```sh
cargo run -- <command> [args]
```

Available commands (see `src/cli/args.rs` for full details):

- `run <file>`: Full pipeline (parse, expand, validate, eval, output)
- `macroexpand <file>`: Print fully macro-expanded code
- `macrotrace <file>`: Show stepwise macro expansion trace with diffs
- `validate <file>`: Validate a script and show errors/warnings
- `format <file>`: Pretty-print and normalize a script
- `test [path]`: Discover and run all test scripts in a directory (default: `tests`)
- `listmacros`: List all available macros with documentation
- `listatoms`: List all available atoms with documentation

### Generate Expected Output

Regenerate `.expected` files for a `.sutra` script or all scripts in a directory:

```sh
sutra gen-expected path/to/file.sutra
sutra gen-expected path/to/directory/
```

**Safety:** This command will overwrite existing `.expected` files.

---