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
- **Status**: Implemented in `src/eval.rs`.
- Authors never write explicit `get` operations.
- Path references (e.g., `player.hp`) are automatically resolved to values from the world state during evaluation.
- This provides a clean, spreadsheet-like authoring experience.

## Module Boundaries

### Core Modules
- **ast.rs** - AST types and span tracking
- **value.rs** - Runtime data values
- **world.rs** - Persistent world state
- **sutra.pest** - Formal PEG grammar for all syntaxes
- **parser.rs** - Unified PEG-based parser
- **atom.rs** - Irreducible operations
- **eval.rs** - Evaluation engine with TCO
- **macro.rs** - Macro expansion system
- **validate.rs** - Structural and semantic validation

### Extension Modules
- **macros_std.rs** - Standard narrative/gameplay macros
- **cli.rs** - Command-line interface

## Design Patterns

### Registry Pattern
- **Status**: Implemented in `src/atom.rs`. Foundational structures for the macro registry have been created in `src/macro.rs`.
- Atoms and macros stored in inspectable registries.
- The `AtomFn` signature is `fn(args: &[Expr], context: &mut EvalContext) -> Result<(Value, World), SutraError>`, ensuring all evaluation context is passed explicitly.
- Runtime introspection of available operations.
- Clean extension point for new functionality.

### Output Injection
- **Status**: Implemented in `src/atom.rs`.
- All output handled through injectable traits (`OutputSink`).
- Enables testing, UI integration, and custom rendering.
- No global or hardcoded I/O.

### Error Handling
- Span-based error reporting throughout pipeline
- Multiple error collection (not fail-fast)
- Clear separation of parse, macro, validation, and runtime errors

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

*Last Updated: 2025-07-01*
