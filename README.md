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
│   ├── ast.rs, ast/               # Core AST types and value representations
│   ├── atoms.rs, atoms/           # Atom system: primitive operations, domain modules
│   ├── cli.rs                     # Command-line interface and subcommands
│   ├── repl.rs                    # Interactive REPL (Read-Eval-Print Loop)
│   ├── diagnostics.rs             # Error types, diagnostics, and reporting
│   ├── discovery.rs               # Test discovery and harness
│   ├── engine.rs                  # High-level orchestration and pipeline
│   ├── error_messages.rs          # Centralized error message constants
│   ├── lib.rs                     # Library entry point, module exports
│   ├── macros.rs, macros/         # Macro system: expansion, registry, std macros
│   ├── main.rs                    # Binary entry point (CLI launcher)
│   ├── runtime.rs, runtime/       # Evaluation and world state management
│   ├── syntax.rs, syntax/         # Parsing and grammar
│   ├── validation.rs, validation/ # Grammar and semantic validation
│   └── ...
├── tests/                     # Test suite: atoms, macros, runtime, syntax, io
│   ├── atoms/
│   ├── macros/
│   ├── runtime/
│   ├── syntax/
│   ├── io/
│   └── ...
├── docs/                      # Canonical documentation
│   ├── canonical-language-reference.md
│   ├── philosophy.md
│   └── references/
├── scripts/                   # Utility scripts (e.g., grammar checks)
├── Cargo.toml                 # Rust package manifest
├── Cargo.lock                 # Cargo dependency lockfile
└── ...
```

---

## Core Architecture

### 1. **AST Layer (`src/ast.rs`, `src/ast/`)**

Defines the core data structures for representing Sutra expressions, including:

- `Expr`, `AstNode`, `Span`, `ParamList`
- Value representations (`value.rs`) with `Value` enum supporting `Nil`, `Number`, `String`, `Bool`, `List`, `Map`, `Path`, and `Lambda`

### 2. **Atoms System (`src/atoms.rs`, `src/atoms/`)**

Atoms are the primitive operations of the engine, organized into domain modules:

- `math.rs`, `logic.rs`, `collections.rs`, `execution.rs`, `external.rs`, `string.rs`, `world.rs`, `special_forms.rs`, `test.rs`
- `helpers.rs` provides shared infrastructure
- Atoms are registered and managed via the `AtomRegistry` with three calling conventions: `Pure`, `Stateful`, and `SpecialForm`

### 3. **Macro System (`src/macros.rs`, `src/macros/`)**

- Purely syntactic transformation of the AST before evaluation
- Supports both native Rust macro functions and declarative macro templates
- Modularized into `expander.rs`, `loader.rs`, `std_macros.rs`
- User and standard macros loaded from `std_macros.sutra`
- Supports variadic macro forwarding with spread arguments (`...args`)

### 4. **Parsing & Syntax (`src/syntax.rs`, `src/syntax/`)**

- PEG grammar (`grammar.pest`) with comprehensive rule coverage
- Parser implementation (`parser.rs`) with robust error reporting
- Supports both s-expression `()` and brace-block `{}` syntaxes
- Handles quotes, defines, lambdas, spread arguments, and parameter lists

### 5. **Validation (`src/validation.rs`, `src/validation/`)**

- Grammar validation (`grammar/`) with comprehensive rule checking
- Semantic validation (`semantic/`) for expanded AST correctness
- Ensures scripts are well-formed and semantically correct

### 6. **Runtime & Evaluation (`src/runtime.rs`, `src/runtime/`)**

- Evaluation engine (`eval.rs`) with lexical scoping and recursion control
- World state management (`world.rs`) with immutable state updates
- Deterministic PRNG for reproducible execution

### 7. **CLI & REPL (`src/cli.rs`, `src/repl.rs`)**

- Command-line interface with comprehensive subcommands
- Interactive REPL with persistent state and multi-line expression support
- Direct code evaluation (`eval` command) for quick testing
- Test discovery and execution with progress tracking
- Macro expansion tracing and AST inspection

### 8. **Testing (`src/discovery.rs`)**

- Test discovery and harness with AST-based test extraction
- Support for `(test "name" (expect value) body...)` test forms

### 9. **Diagnostics (`src/diagnostics.rs`)**

- Unified error system with `miette`-based diagnostics
- Error construction macros (`err_msg!`, `err_ctx!`, `err_src!`)
- Multi-label diagnostics and error chaining

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

- **Test Discovery:** Automatic discovery of `.sutra` test files
- **Test Forms:** `(test "name" (expect value) body...)` syntax
- **Error Testing:** `(expect-error error-type)` for testing error conditions
- **Progress Tracking:** Real-time test execution progress

### Interactive Development

- **REPL:** Interactive shell with persistent state across evaluations
- **Direct Evaluation:** `eval` command for quick code testing
- **Multi-line Support:** Automatic expression completion detection
- **Context Management:** State clearing and session control

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
- `listmacros`: List all available macros with documentation
- `listatoms`: List all available atoms with documentation
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

Tests are organized by domain:

- `tests/atoms/`: Atom operation tests (math, logic, list, string, etc.)
- `tests/macros/`: Macro expansion and assignment tests
- `tests/runtime/`: Runtime consistency and control flow
- `tests/syntax/`: Parsing and security
- `tests/io/`: Output and IO
- Each `.sutra` file is a test script; see `tests/README.md` for details.

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
