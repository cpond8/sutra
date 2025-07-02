# Sutra Engine - System Patterns

## Core Architecture

### Engine Pipeline

Sutra operates as a sequence of pure, compositional, strictly layered modules:

```
parse → macro-expand → validate → evaluate → output/presentation
```

**Key Properties:**

- Each layer is decoupled, testable, and extensible
- No hidden state or side effects between layers
- All transformations are inspectable and reversible
- Debugging available at every stage

### Data Flow and State Management

**Single Source of Truth**

- World state is a single, serializable, deeply immutable data structure
- All data accessible by path (e.g., `player.hp`, `world.npcs[0].hunger`)
- No hidden or duplicated state anywhere in the system

**Pure State Transitions**

- Macros never mutate world state - only atoms can produce side-effects
- All mutations return new world state, preserving original
- PRNG state tracked explicitly for deterministic randomness

## Core Technical Patterns

### Atoms and Macros Architecture

**Atoms (Irreducible Core)**

- `set!`, `add!`, `del!`, `push!`, `pull!` - state mutation
- `+`, `-`, `*`, `/` - pure math operations
- `eq?`, `gt?`, `lt?`, `has?` - predicates
- `cond`, `do` - control flow
- `print` - output
- `rand` - randomness

**Macros (Author-Facing Layer)**

- All higher-level constructs built as macros
- `storylet`, `choice`, `pool`, `select` - narrative patterns
- `is?`, `over?`, `at-least?` - readable predicates
- `inc!`, `dec!` - common mutations

### Syntax System

**Unified PEG Parser**

- A single, formal PEG (Parsing Expression Grammar) defined in `src/sutra.pest` serves as the single source of truth for all supported syntaxes.
- The parser supports both s-expression and brace-block syntaxes.
- All input is parsed into a single canonical AST (`Expr`), ensuring perfect consistency.
- This approach provides superior error reporting and long-term maintainability.

**Auto-Resolution ("Auto-Get")**

- **Status**: Implemented in `src/macros.rs` and `src/macros_std.rs`.
- **Method**: The "auto-get" feature is now handled entirely during the macro expansion phase.
- **Process**:
  1. Authors write code using bare symbols (e.g., `(is? player.hp 100)`).
  2. During macro expansion, macros like `is?` use a `wrap_in_get` helper function.
  3. This helper transforms any bare symbol argument into an explicit `(get ...)` call (e.g., `(get player.hp)`).
  4. The final AST passed to the evaluator contains only explicit `(get ...)` calls for all world lookups.
- **Canonicalization Contract**: All macro-generated atom path arguments are now strictly required to be in the canonical flat `(list ...)` form, enforced by a single canonicalization helper and comprehensive tests.
- **Evaluator Role**: The evaluator (`src/eval.rs`) has been simplified and now throws a semantic error if it encounters a bare symbol, ensuring no implicit lookups occur during evaluation.
- **Benefit**: This enforces a stricter separation of concerns, making the pipeline more transparent and easier to debug. The evaluator is only responsible for executing atoms, not for resolving symbols.

## Module Boundaries

### Core Modules

- **ast.rs** - AST types and span tracking
- **value.rs** - Runtime data values
- **world.rs** - Persistent world state
- **sutra.pest** - Formal PEG grammar for all syntaxes
- **parser.rs** - Unified PEG-based parser
- **atom.rs** - Irreducible operations
- **eval.rs** - Evaluation engine with TCO
- **macros.rs** - Macro expansion system
- **macros_std.rs** - Standard macro library
- **validate.rs** - Structural and semantic validation (planned)

### CLI Module

- **cli/** - The command-line interface, which acts as a user-facing wrapper around the core library.
  - **mod.rs** - Main CLI logic and command dispatch.
  - **args.rs** - CLI argument and subcommand definitions.
  - **output.rs** - All user-facing output formatting (errors, traces, etc.).

## Design Patterns

### Registry Pattern

- **Status**: Implemented in `src/atom.rs` and `src/macros.rs`.
- Atoms and macros stored in inspectable registries.
- The `AtomFn` signature is `fn(args: &[Expr], context: &mut EvalContext, parent_span: &Span) -> Result<(Value, World), SutraError>`, ensuring all evaluation context and location information is passed explicitly for high-quality error reporting.
- Runtime introspection of available operations.
- Clean extension point for new functionality.

### Output Injection

- **Status**: Implemented in `src/atom.rs`.
- All output handled through injectable traits (`OutputSink`).
- Enables testing, UI integration, and custom rendering.
- No global or hardcoded I/O.

### Error Handling

- **Rich, Contextual Errors**: The `SutraError` system is designed for maximum author feedback. Evaluation errors (`EvalError`) are captured with the original code, the fully expanded code, and a helpful suggestion.
- **Two-Phase Enrichment**: Errors are created with immediate context within their pipeline stage (e.g., `eval` creates an error with the expanded code). A top-level runner then "enriches" this error with further context (like the original source code) before displaying it to the user. This keeps each module's responsibility clean.
- **Span-based**: All errors retain source span information, allowing the CLI to point directly to the source of the problem.

## Architectural Constraints

### What's Forbidden

- No global state or singletons
- No mutation in place (except through explicit atoms)
- No privileged engine code in macro layer
- No coupling between syntax and semantics
- No hidden side effects or magic

### What's Required

- All state changes through explicit atoms
- All higher-level features as macros
- Full pipeline transparency and debuggability
- Deterministic execution with explicit randomness
- Pure functional programming throughout

## Scalability Patterns

### Performance Considerations

- Persistent data structures for efficient immutable updates
- Tail-call optimization for unbounded recursion
- Lazy evaluation where appropriate
- Minimal copying and allocation

### Extensibility Patterns

- Macro libraries as separate modules
- User-defined macros (future)
- Plugin architecture through registries
- No core engine modifications required for new features

### Macro Path Canonicalization Pattern (2025-07-01)

- All macro-generated atom calls with path arguments must use `canonicalize_path`.
- No path normalization logic is allowed outside the macro layer.
- Macro expansion tests enforce this contract for all path forms and error cases.
- This pattern ensures single source of truth, minimalism, and strict separation of concerns between macro and atom layers.

_Last Updated: 2025-07-01_
