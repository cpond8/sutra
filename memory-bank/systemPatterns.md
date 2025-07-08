# Sutra Engine - System Patterns

## Overview

This document captures the canonical architectural and design patterns for the Sutra Engine. It serves as the reference for all contributors and must be kept synchronized with the codebase.

## Core Architectural Patterns

### 1. Pipeline Separation

The engine enforces a strict `parse → macroexpand → validate → evaluate` pipeline:

- Each stage is independently testable and documented
- No hidden state or side effects between layers
- All transformations are inspectable and reversible
- Debugging available at every stage

### 2. Registry Pattern

- All atoms and macros registered via canonical builder functions
- Single source of truth for both production and test environments
- Ensures extensibility, test/production parity, prevents duplication
- Test atoms feature-gated (`cfg(debug_assertions)`, `cfg(test)`, `test-atom` feature)

### 3. Pure Function Architecture

- All core logic implemented as pure functions with no global state
- State propagated explicitly through immutable data structures
- Only atoms can produce side effects on world state
- All mutations return new world state, preserving original

### 4. Macro-Driven Extensibility

- All higher-level features implemented as macros, not core engine code
- Macro system supports variadic, recursive, and hygienic macros
- Macro expansion is fully transparent and testable
- Single source of truth: macro library defines all surface constructs

### 5. Error Handling and Transparency

- All errors structured, span-carrying, and contextual
- `EvalError` and two-phase enrichment pattern standard for user-facing errors
- Clear, actionable error messages with debugging information

### 8. Modern Rust Idioms and Code Quality (2025-07-08)

- Direct function calls preferred over macro indirection where appropriate
- Helper functions for common patterns (error construction, context management)
- Type aliases for improved readability and consistency
- Clean separation between foundational functions and implementation details
- Macros used only where they provide genuine abstraction benefits

**Atom Implementation Pattern**:

- Error construction via centralized helper functions
- Evaluation helpers for common operation types (binary, unary, n-ary)
- Direct function calls with explicit parameter passing
- Maintained span information for precise error reporting

### 6. Minimalism and Compositionality

- Engine exposes minimal set of irreducible operations (atoms)
- All complexity composed via macros and user-defined constructs
- No privileged engine code in macro layer

## Test Suite Protocol

**All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted.**

**Test Organization:**

- Rust integration/unit tests: `tests/rust/`
- Protocol-compliant integration tests: `tests/scripts/` (Sutra scripts + expected output)
- Inline tests for small modules

## File Hierarchy (2025-07-07)

Modular directory structure:

- `src/syntax/` - parser, CST, error handling, validation
- `src/ast/` - AST builder, value types
- `src/atoms/` - core atom implementations
- `src/macros/` - macro system and standard library
- `src/runtime/` - evaluation, registry, world state
- `src/cli/` - CLI logic, args, output
- Entry points: `src/lib.rs`, `src/main.rs`

All directory-based modules use explicit `mod.rs` files per Rust idiom.

## Engine Architecture

### Data Flow

```
World State (immutable) → Atoms (mutation) → New World State
    ↑                                              ↓
Parse → Macroexpand → Validate → Evaluate → Output
```

### Core Components

**Atoms (Irreducible Core):**

- `core/set!`, `core/del!` - state mutation
- `+`, `-`, `*`, `/`, `mod` - pure math operations
- `eq?`, `gt?`, `lt?`, `gte?`, `lte?`, `not` - predicates
- `do` - sequential evaluation
- `print` - output

**Macros (Author-Facing Layer):**

- `cond` - primary variadic conditional macro
- `if` - simple 3-arity macro expanding to `cond`
- State mutation: `set!`, `del!`, `add!`, `sub!`, `inc!`, `dec!`
- Predicates: `is?`, `over?`, `under?` (with auto-get functionality)

### World State Management

- Single, serializable, deeply immutable data structure
- All data accessible by path (e.g., `player.hp`, `world.npcs[0].hunger`)
- No hidden or duplicated state
- PRNG state tracked explicitly for deterministic randomness

## Architectural Constraints

### Required Patterns

- All state changes through explicit atoms
- All higher-level features as macros
- Full pipeline transparency and debuggability
- Deterministic execution with explicit randomness
- Pure functional programming throughout

### Forbidden Patterns

- Global state or singletons
- Mutation in place (except through explicit atoms)
- Privileged engine code in macro layer
- Coupling between syntax and semantics
- Hidden side effects or magic

## Registry Reliability Strategies

Implemented reliability measures:

- **Integration Tests**: End-to-end pipeline validation (Priority: High)
- **Registry Hashing**: SHA256 fingerprinting of macro definitions (Priority: High)
- **Feature-Gated Test Atoms**: Optional test atoms for development builds

Future considerations (incremental adoption):

- Phantom types for canonical registry
- Sealed/immutable registry pattern
- Mutation linting and smoke testing

## Code Quality Protocols

### Audit Protocol (2025-07-05)

- Automated/search-based tools paired with explicit manual review
- All complexity audits require explicit enumeration of every function
- Protocol-driven, batch-based, test-driven development
- Never rely solely on semantic search for completeness

### Performance Patterns

- Persistent data structures for efficient immutable updates
- Tail-call optimization for unbounded recursion
- Minimal copying and allocation
- Lazy evaluation where appropriate

## Cross-References

- See `docs/architecture/parsing-pipeline-plan.md` for pipeline implementation details
- See `memory-bank/activeContext.md` for current work focus and priorities
- See `memory-bank/progress.md` for completed work and status
- See `memory-bank/techContext.md` for technical stack and constraints
- See `system-reference.md` for detailed system reference

## Changelog

- **2025-07-07**: File hierarchy reorganized, test organization updated, feature-gated test atoms
- **2025-07-06**: Test suite protocol established, integration test runner bootstrapped
- **2025-07-05**: Code audit protocol updated, proper-list-only architecture migration complete
- **2025-07-04**: Modular parsing pipeline established as canonical pattern
- **2025-07-02**: Registry hashing implemented, reliability strategies documented
