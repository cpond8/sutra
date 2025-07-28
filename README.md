# Sutra Engine

**Sutra is a compositional, emergent narrative engine and language designed to empower the creation of deeply modular, emergent, and replayable interactive stories.**

Its core aim is to enable authors and designers to build games where narrative is not a fixed branching tree, but a living system: stories are composed from small, self-contained units (storylets, threads) that become available, interact and recombine dynamically based on the evolving state of the world and its characters to create emergent stories.

Sutra is inspired by the best practices of quality-based narrative (QBN), storylet-driven design, and salience/waypoint-based progression. It provides a foundation for games where:

- **Player choices and world state drive the unfolding of narrative,** not pre-scripted paths.
- **Content is modular and extensible:** new storylets, events, and systems can be added without rewriting or breaking existing stories.
- **Emergence and replayability are first-class goals:** each playthrough can yield a unique, coherent story arc, shaped by the interplay of player actions, system-driven events and authorial design.
- **Authors can express complex, interlocking systems of narrative logic, resources, relationships, and pacing**, without sacrificing clarity or maintainability.

Sutra achieves this by:

- Providing a minimal, compositional language that unifies s-expression and block syntax for both authors and the engine.
- Enforcing a strict separation of concerns: parsing, macro-expansion, validation, evaluation, and presentation are all modular and inspectable.
- Making all computation, macro expansion, and world state changes transparent and debuggable.
- Supporting canonical patterns for modular content (storylets), flexible narrative flows (threads), and dynamic, state-driven event selection (pools, salience, history).
- Enabling both menu-driven and system-driven (AI director, salience) narrative progression, supporting a wide range of game genres, from interactive fiction to simulation-heavy emergent worlds.

**In short:**
Sutra is a toolkit for building games where narrative is systemic, modular and alive, where every playthrough can be different, and where authors can focus on designing meaningful, interconnected story systems rather than wrestling with branching complexity.

---

## Philosophy

Sutra is designed for:

- **Compositionality:** All computation is built from a minimal set of irreducible "atoms" and a macro system, enabling maximal extensibility.
- **Transparency:** All computation, macro expansion, and world state changes are inspectable and debuggable.
- **Separation of Concerns:** Parsing, macro-expansion, validation, evaluation, and presentation are strictly separated for maintainability and testability.
- **Single Source of Truth:** Eliminates documentation and implementation drift by enforcing a single source of truth for all concepts and patterns.

For a detailed statement of philosophy and guiding principles, see [`docs/philosophy.md`](docs/philosophy.md).

---

## Directory Structure

```
.
├── src/
│   ├── atoms.rs, atoms/           # Atom system: primitive operations, domain modules
│   │   ├── collections.rs         # List and collection operations
│   │   ├── execution.rs           # Control flow and execution atoms
│   │   ├── external.rs            # External I/O and system operations
│   │   ├── logic.rs               # Boolean logic and comparison atoms
│   │   ├── math.rs                # Mathematical operations
│   │   ├── special_forms.rs       # Special forms (if, let, lambda, etc.)
│   │   ├── string.rs              # String manipulation operations
│   │   ├── test.rs                # Testing framework atoms
│   │   └── world.rs               # World state management atoms
│   ├── cli.rs                     # Command-line interface and subcommands
│   ├── discovery.rs               # Test discovery and harness
│   ├── errors.rs                  # Error types, diagnostics, and reporting
│   ├── grammar/                   # Grammar definition files
│   │   └── grammar.pest           # PEG grammar specification
│   ├── grammar_validation.rs      # Grammar validation and rule checking
│   ├── lib.rs                     # Library entry point, module exports
│   ├── macros.rs                  # Macro system: expansion, registry, definitions
│   ├── main.rs                    # Binary entry point (CLI launcher)
│   ├── parser.rs                  # Parsing implementation and AST construction
│   ├── repl.rs                    # Interactive REPL (Read-Eval-Print Loop)
│   ├── runtime.rs                 # Evaluation engine and world state management
│   ├── semantic_validation.rs     # Semantic validation for expanded AST
│   ├── syntax.rs                  # Core AST types and value representations
│   ├── test.rs                    # Test framework types and utilities
│   ├── test_runner.rs             # Test execution and harness implementation
│   └── validation.rs              # Validation module coordination and re-exports
├── tests/                         # Test suite: organized by functional domain
│   ├── builtins/                  # Built-in function tests
│   ├── control/                   # Control flow and execution tests
│   ├── core/                      # Core language feature tests
│   ├── io/                        # Input/output operation tests
│   ├── syntax/                    # Parsing and syntax tests
│   └── world/                     # World state management tests
├── docs/                          # Canonical documentation
│   ├── canonical-language-reference.md
│   ├── philosophy.md
│   └── references/
├── scripts/                       # Utility scripts (e.g., grammar checks)
├── Cargo.toml                     # Rust package manifest
├── Cargo.lock                     # Cargo dependency lockfile
└── ...
```

---

## Core Architecture

### 1. **Syntax & AST Layer (`src/syntax.rs`)**

Defines the core data structures for representing Sutra expressions, including:

- `Expr`, `AstNode`, `Span`, `Spanned` for AST representation with source location tracking
- `ParamList` for function parameter handling
- Value representations with `Value` enum supporting `Nil`, `Number`, `String`, `Bool`, `List`, `Map`, `Path`, and `Lambda`

### 2. **Parser (`src/parser.rs`)**

- PEG grammar-based parser (`src/grammar/grammar.pest`) with comprehensive rule coverage
- Robust error reporting with source location tracking
- Supports both s-expression `()` and brace-block `{}` syntaxes
- Handles quotes, defines, lambdas, spread arguments, and parameter lists

### 3. **Atoms System (`src/atoms.rs`, `src/atoms/`)**

Atoms are the primitive operations of the engine, organized into domain modules:

- `math.rs`: Arithmetic operations (`+`, `-`, `*`, `/`, `mod`, `abs`, `min`, `max`)
- `logic.rs`: Boolean logic and comparisons (`eq?`, `gt?`, `lt?`, `not`, etc.)
- `collections.rs`: List and collection operations (`list`, `len`, `car`, `cdr`, `cons`, `map`)
- `execution.rs`: Control flow atoms (`do`, `apply`, `if`, `let`, `cond`)
- `external.rs`: I/O and system operations (`print`, `output`, `rand`)
- `string.rs`: String manipulation (`str`, `str+`, `display`)
- `world.rs`: World state management (`get`, `set!`, `del!`, `exists?`, `path`)
- `special_forms.rs`: Special syntax forms (`lambda`, `define`, `quote`)
- `test.rs`: Testing framework support

Atoms are registered with three calling conventions: `Pure`, `Stateful`, and `SpecialForm`

### 4. **Macro System (`src/macros.rs`)**

- Purely syntactic transformation of the AST before evaluation
- Supports both native Rust macro functions and declarative macro templates
- User and standard macros with variadic support (`...args`)
- Template-based macro definitions with parameter substitution

### 5. **Validation (`src/grammar_validation.rs`, `src/semantic_validation.rs`)**

- **Grammar validation**: Comprehensive rule checking for PEG grammar correctness
- **Semantic validation**: AST correctness validation after macro expansion
- Coordinated through `src/validation.rs` with unified error reporting
- Ensures scripts are well-formed and semantically correct before evaluation

### 6. **Runtime & Evaluation (`src/runtime.rs`)**

- Evaluation engine with lexical scoping and recursion control
- World state management with immutable state updates
- Deterministic execution with controlled side effects
- Function call handling and lambda evaluation

### 7. **Testing Framework (`src/test.rs`, `src/test_runner.rs`)**

- **Test Discovery**: Automatic discovery and parsing of `.sutra` test files (`src/discovery.rs`)
- **Test Execution**: Comprehensive test runner with progress tracking
- **Test Types**: Support for value expectations, error testing, and output validation
- **Test Forms**: `(test "name" (expect value) body...)` syntax

### 8. **CLI & REPL (`src/cli.rs`, `src/repl.rs`)**

- Command-line interface with comprehensive subcommands
- Interactive REPL with persistent state and multi-line expression support
- Direct code evaluation for quick testing
- Macro expansion tracing and AST inspection
- Test execution and progress reporting

### 9. **Error Handling (`src/errors.rs`)**

- Unified error system with rich diagnostic information
- Source location tracking and error context preservation
- Multiple error types for different validation and runtime phases
- Integration with `miette` for rich error display

---

## Language Features

### Syntax

- **Unified Syntax:** Both s-expressions `()` and brace blocks `{}` produce identical AST
- **Quotes:** `'expr` for unevaluated expressions
- **Defines:** `(define (name params...) body)` for function definitions
- **Lambdas:** `(lambda (params...) body)` for anonymous functions
- **Spread Arguments:** `...args` for variadic parameters and calls
- **Paths:** `set!` for hierarchical symbol resolution

### Core Operations

- **Math:** `+`, `-`, `*`, `/`, `mod`, `abs`, `min`, `max`
- **Logic:** `eq?`, `gt?`, `lt?`, `gte?`, `lte?`, `not`
- **Collections:** `list`, `len`, `has?`, `car`, `cdr`, `cons`, `push!`, `pull!`, `map`
- **Strings:** `str`, `str+`, `display`
- **World State:** `get`, `set!`, `del!`, `exists?`, `path`, `add!`, `sub!`, `inc!`, `dec!`
- **Execution:** `do`, `apply`, `error`, `if`, `let`, `lambda`, `cond`
- **External:** `print`, `output`, `rand`

### Macro System

- **Template Macros:** Declarative macro definitions with parameter lists
- **Native Macros:** Rust function macros for complex transformations
- **Variadic Support:** `...args` forwarding for Lisp-style macros
- **Standard Library:** Rich set of built-in macros in `std_macros.sutra`

### Testing Framework

- **Test Discovery:** Automatic discovery of `.sutra` test files in the `tests/` directory
- **Test Forms:** `(test "name" (expect value) body...)` syntax for clear test definitions
- **Error Testing:** `(expect-error error-type)` for testing error conditions and edge cases
- **Output Testing:** `(expect-output "text")` for testing printed output
- **Progress Tracking:** Real-time test execution progress with pass/fail statistics
- **Test Organization:** Tests organized by functional domain (builtins, control, core, io, syntax, world)

### Interactive Development

- **REPL:** Interactive shell with persistent state across evaluations
- **Direct Evaluation:** `eval` command for quick code testing
- **Multi-line Support:** Automatic expression completion detection
- **Context Management:** State clearing and session control

---

## Recent Changes

**Code Organization (July 2025)**: The codebase has been recently reorganized to improve modularity and discoverability:

- Parser moved from `src/syntax/parser.rs` to `src/parser.rs` for top-level visibility
- Grammar files consolidated in `src/grammar/` directory
- Validation modules promoted: `validation/grammar.rs` → `grammar_validation.rs`, `validation/semantic.rs` → `semantic_validation.rs`
- Test runner moved from `src/test/runner.rs` to `src/test_runner.rs`
- Eliminated unnecessary directory nesting while maintaining backward compatibility through re-exports

This reorganization reduces module indirection, improves code discoverability, and maintains clean separation of concerns.

---

## Building and Running

This project uses [Cargo](https://doc.rust-lang.org/cargo/) for building and testing.

**Build:**

```sh
cargo build
```

**Run the CLI:**

```sh
cargo run -- <command> [args]
```

**Run the test suite:**

```sh
cargo test
```

**Validate the grammar:**

```sh
cargo run -- validate-grammar
```

---

## CLI Commands

Key commands:

- `run <file>`: Full pipeline (parse, expand, validate, eval, output)
- `eval [code]`: Evaluate Sutra code directly from command line or stdin
- `repl`: Start an interactive REPL (Read-Eval-Print Loop) session
- `macroexpand <file>`: Print fully macro-expanded code
- `macrotrace <file>`: Show stepwise macro expansion trace with diffs
- `validate-grammar`: Validate the PEG grammar for errors
- `format <file>`: Pretty-print and normalize a script
- `test [path]`: Discover and run all test scripts in a directory (default: `tests`)
- `list-macros`: List all available macros with documentation
- `list-atoms`: List all available atoms with documentation
- `ast <file>`: Show the Abstract Syntax Tree (AST) for a script

### Interactive Development

**REPL (Read-Eval-Print Loop):**

```sh
cargo run -- repl
```

The REPL provides an interactive shell for evaluating Sutra expressions with persistent state:

- Variables and functions persist across input lines
- Multi-line expressions are supported with automatic completion detection
- Special commands: `:help`, `:quit`, `:clear` (with short aliases `:h`, `:q`, `:c`)
- Rich error diagnostics with source location information

Example REPL session:

```
sutra> (define x 42)
42
sutra> (define (square n) (* n n))
<lambda>
sutra> (square x)
1764
sutra> :clear
Context cleared.
sutra> x
Error: Undefined symbol 'x'
sutra> :quit
Goodbye!
```

**Direct Code Evaluation:**

```sh
# Evaluate code from command line
cargo run -- eval '(+ 1 2 3)'

# Evaluate code from stdin
echo '(define x 42) x' | cargo run -- eval

# Interactive evaluation (without persistent state)
cargo run -- eval
```

---

## Test Suite

Tests are organized by functional domain in the `tests/` directory:

- `tests/builtins/`: Built-in function tests (arithmetic, comparison, list, logic, random, string)
- `tests/control/`: Control flow and execution tests (conditionals, consistency, execution)
- `tests/core/`: Core language feature tests (collections, literals, scoping, special forms)
- `tests/io/`: Input/output operation tests
- `tests/syntax/`: Parsing and syntax validation tests
- `tests/world/`: World state management tests (assignment, persistence)

Each `.sutra` file contains test scripts using the `(test "name" (expect value) body...)` syntax. Run tests with:

```sh
# Run all tests
cargo run -- test

# Run tests in a specific directory
cargo run -- test tests/core

# Run a specific test file
cargo run -- test tests/core/literals.sutra
```

---

## Documentation

Canonical documentation is maintained in the `docs/` directory:

- Language reference
- Philosophy and design principles
- Narrative and storylet system references
- Threading and execution model

---

## Contributing

- All code must pass formatting, linting (`clippy`), and tests.
- Public APIs must be documented with clear examples and rationale.
- See `clippy.toml` and project memories for code style and review rules.

---

## License

Sutra is released under the MIT License.

---

If you need further details on any subsystem, see the in-code documentation or the `docs/` directory.
