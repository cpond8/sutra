# Sutra Engine - System Patterns

## Core Architectural Patterns

### Pipeline Separation

Strict `parse → macroexpand → validate → evaluate` pipeline with independent, testable stages.

### Registry Pattern

All atoms and macros registered via canonical builder functions. Single source of truth for production and test environments.

### Pure Function Architecture

All core logic implemented as pure functions with no global state. State propagated explicitly through immutable data structures.

### Macro-Driven Extensibility

All higher-level features implemented as macros. Variadic argument splicing via `...` spread operator. Macro expansion fully transparent and testable.

### Error Handling and Transparency

All errors structured, span-carrying, and contextual. `EvalError` and two-phase enrichment pattern for user-facing errors.

### Modern Rust Idioms

Direct function calls preferred over macro indirection. Helper functions for common patterns. Type aliases for readability.

### Minimalism and Compositionality

Engine exposes minimal set of irreducible operations (atoms). All complexity composed via macros and user-defined constructs.

## Core Components

**Atoms (Irreducible Core)**: `core/set!`, `core/del!`, `+`, `-`, `*`, `/`, `mod`, `eq?`, `gt?`, `lt?`, `do`, `print`, `core/str+`, `apply`

**Macros (Author-Facing Layer)**: `cond`, `if`, `set!`, `del!`, `add!`, `sub!`, `inc!`, `dec!`, `is?`, `over?`, `under?`, `str+`

## World State Management

Single, serializable, deeply immutable data structure. All data accessible by path. No hidden or duplicated state. PRNG state tracked explicitly.

## Architectural Constraints

**Required Patterns**: State changes through explicit atoms, higher-level features as macros, full pipeline transparency, deterministic execution, pure functional programming.

**Forbidden Patterns**: Global state, mutation in place, privileged engine code in macro layer, coupling between syntax and semantics, hidden side effects.
