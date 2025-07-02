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

- `core/set!`, `core/del!` - state mutation
- `+`, `-`, `*`, `/`, `mod` - pure math operations
- `eq?`, `gt?`, `lt?`, `gte?`, `lte?`, `not` - predicates
- `do` - sequential evaluation
- `print` - output (planned)
- `rand` - randomness (planned)

**Macros (Author-Facing Layer)**

- All higher-level constructs are built as macros.
- **Conditional Logic:** `cond` is the primary, variadic conditional macro. `if` is a simpler, fixed-arity macro that expands to `cond`.
- **State Mutation:** `set!`, `del!`, `add!`, `sub!`, `inc!`, `dec!`.
- **Predicates:** `is?`, `over?`, `under?` (provide auto-get functionality).
- **Future:** `storylet`, `choice`, `pool`, `select`.

### Registry Pattern (Unified, DRY)

- **Canonical Registry Builder**: Both production and test code use a single, canonical builder function for atom and macro registries (`build_default_atom_registry`, `build_default_macro_registry` in `registry.rs`).
- **No Duplication**: All standard atoms/macros are registered in one place; tests and production always share the same logic.
- **Extensible**: Tests may further mutate the registry after construction for custom scenarios, but always start from the canonical builder.
- **Contract**: All registration logic is centralized; new atoms/macros are added in one place and available everywhere.

### Syntax System

**Unified PEG Parser**

- A single, formal PEG (Parsing Expression Grammar) defined in `src/sutra.pest` serves as the single source of truth for all supported syntaxes.
- The parser supports both s-expression and brace-block syntaxes.
- All input is parsed into a single canonical AST (`Expr`), ensuring perfect consistency.
- This approach provides superior error reporting and long-term maintainability.

**Path Canonicalization**

- **Status**: Fully implemented in `src/macros_std.rs`.
- **Method**: The macro expansion layer is the sole authority for interpreting user-facing path syntax and converting it into a canonical, unambiguous `Expr::Path` AST node. This replaces the older, less explicit "auto-get" system.
- **Process**:

  ```mermaid
  graph TD
      subgraph User Code
          A["(inc! player.hp)"]
      end

      subgraph Stage 1: Parse
          B(Parser)
          C["Expr::List(\n  Expr::Symbol(\"inc!\"),\n  Expr::Symbol(\"player.hp\")\n)"]
      end

      subgraph Stage 2: Macro Expand
          D(Macro Expander)
          E["expr_to_path(\"player.hp\") -> Path([\"player\", \"hp\"])"]
          F["Expanded AST:\n(set! \n  (path player hp) \n  (+ (get (path player hp)) 1))"]
      end

      subgraph Stage 3: Evaluate
          G(Evaluator)
          H["Atoms (`set!`, `get`, `+`) operate on canonical `Value::Path`"]
      end

      A --> B --> C --> D --> E --> F --> G --> H
  ```

- **Key Principles**:
  1. **Single Source of Truth**: The `expr_to_path` function in `src/macros_std.rs` is the only place in the engine that understands and parses path syntax (e.g., `player.hp` or `(list "player" "hp")`).
  2. **Canonical AST**: The macro expander's primary responsibility is to produce a canonical AST where all paths are explicit `Expr::Path` nodes and all value lookups are explicit `(get ...)` calls.
  3. **Simplified Evaluator**: The evaluator (`src/eval.rs`) is simplified and hardened. It operates only on the canonical AST and will throw a semantic error if it encounters a bare symbol, enforcing the contract with the macro layer.
- **Benefit**: This architecture creates a highly predictable, transparent, and robust pipeline. It fully decouples syntax from evaluation, making the system easier to debug, test, and extend.

### Macro System Architecture (The "Sutra Way")

After extensive analysis, the Sutra engine has adopted a pragmatic, two-tiered macro system that balances author ergonomics with architectural purity. This system is built on one core primitive: the `Expr::If` special form.

-   **`Expr::If` (The Primitive):** This is the one true conditional construct in the engine. It is a special form in the AST, not a macro, and is handled directly by the evaluator. It is the bedrock on which all other conditional logic is built.

-   **`MacroFn` (Native Macros):**
    -   **Mechanism:** A native Rust function that receives the raw AST of the macro call and has the full power of Rust to transform it.
    -   **Use Case:** Reserved for core language features that require complex, procedural logic during expansion. The primary example is **`cond`**, which is implemented as a native, recursive `MacroFn` that expands into a series of nested `if` macro calls.

-   **`MacroTemplate` (Declarative Macros):**
    -   **Mechanism:** A simple, data-driven substitution system. Macros are defined as templates with named parameters (including a variadic `&rest` parameter) and an AST body.
    -   **Use Case:** The primary way for authors to create new syntactic abstractions. Ideal for simple wrappers, logging utilities, and domain-specific language constructs.
    -   **Limitation:** Purely syntactic. Cannot perform conditional logic or complex computations during expansion. This is a deliberate trade-off for safety and simplicity.

This layered approach is a perfect example of the engine's philosophy: the powerful, variadic `cond` macro is built upon the simpler `if` macro, which in turn is a direct gateway to the single, minimal `Expr::If` primitive. This provides a safe and easy-to-use system for authors while retaining the power needed for the core language, all without compromising the strict `parse -> expand -> evaluate` pipeline.

## Module Boundaries

### Core Modules

- **ast.rs** - AST types and span tracking
- **value.rs** - Runtime data values
- **world.rs** - Persistent world state
- **sutra.pest** - Formal PEG grammar for all syntaxes
- **parser.rs** - Unified PEG-based parser
- **atom.rs** - Irreducible operations
- **eval.rs** - Evaluation engine (handles the `Expr::If` special form)
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

## Macro System as Sole Authority
- All author-facing language constructs (including `cond`) are implemented as macros in the macro library (`src/macros_std.rs`).
- Only `if` exists as a primitive in the AST; all other control flow (e.g., `cond`) is macro sugar, expanded to nested `if`.
- Macro system is pure, stateless, and does not mutate the AST or world in place.
- Strict layering: macro system is the only surface for author constructs; evaluator and AST remain minimal and pure.
- Single source of truth: macro library defines all surface constructs.
- All error and edge cases for `cond` macro are now tested and documented.
- Macro system remains the single source of truth for all author-facing constructs.

## Migration Path
- `cond` macro is currently implemented in Rust for variadic support, but this is temporary.
- As soon as the macro system supports variadic/recursive user macros, `cond` must be ported to a user macro and the Rust implementation removed.
- This ensures no privileged logic or hidden complexity in the engine.

_Last Updated: 2025-07-01_
