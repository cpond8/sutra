# Sutra Engine - System Patterns

## Overview

This document captures the canonical architectural and design patterns, system-wide decisions, and implementation strategies for the Sutra Engine. It is the reference for all contributors and must be kept in sync with the codebase and other memory-bank files.

## Recent Updates (2025-07-02)

- **Parser Refactor:** The parser is now decomposed into per-rule helpers, with robust error handling and strict, canonical dotted list validation. All unreachable!()s are replaced with structured errors that include rule, input, and span. Dotted list parsing asserts and errors on malformed shapes.
- **Parser/Macro System Decoupling:** The parser and macro system are now fully decoupled and testable. Round-trippability and robust error handling are enforced at every stage.
- **Current Focus:** Debugging macro system test failures, especially for variadic macro parameter parsing and cond macro expansion. Ensuring parser and macro system are in sync for all edge cases.

## Canonical Patterns

### 1. Registry Pattern
- All atoms and macros are registered via canonical builder functions in `src/registry.rs`.
- There is a single source of truth for both production and test environments.
- This ensures extensibility, test/production parity, and prevents duplication.

### 2. Macro-Driven Extensibility
- All higher-level features are implemented as macros, not as core engine code.
- The macro system supports variadic, recursive, and hygienic macros.
- Macro expansion is fully transparent and testable.

### 3. Pure Function Architecture
- All core logic is implemented as pure functions, with no global state or hidden side effects.
- State is propagated explicitly through immutable data structures.

### 4. Pipeline Separation
- The engine enforces a strict `parse -> expand -> eval` pipeline.
- Each stage is independently testable and documented.

### 5. Error Handling and Transparency
- All errors are structured, span-carrying, and contextual.
- The `EvalError` and two-phase enrichment pattern is standard for user-facing errors.

### 6. Minimalism and Compositionality
- The engine exposes only a minimal set of irreducible operations (atoms).
- All complexity is composed via macros and user-defined constructs.

### 7. Test/Production Parity
- Test and production environments use the same registry and loader logic.
- All tests are run against the canonical pipeline.

### Test Suite Protocol (2025-07-06)

> **Protocol Requirement:** All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted. A full test suite rewrite is required. See `memory-bank/README.md` and `memory-bank/activeContext.md` for details.

- **Integration Test Runner Bootstrapped (2025-07-06):**
  - `tests/scripts/` directory created for protocol-compliant integration tests.
  - First `.sutra` test script (`hello_world.sutra`) and expected output (`hello_world.expected`) added. See `activeContext.md` and `progress.md`.

### 8. Modular Parsing Pipeline (2025-07-04)
- The parsing pipeline is now a canonical, interface-driven, modular architecture. Each stage (CST parser, AST builder, macroexpander, validator, etc.) is a pure, swappable module with a documented contract.
- Rationale: Ensures maintainability, testability, and future extensibility. Supports robust diagnostics, editor integration, and authoring ergonomics.
- Best practices: Use enums for core types, trait objects only for extensibility, unified diagnostics, and serialization for all public types.
- See `docs/architecture/parsing-pipeline-plan.md` for the full plan and context.

### 9. Code Audit Protocol (2025-07-05)
- Automated or search-based code review tools must be paired with explicit, protocol-driven manual review for critical code quality checks (e.g., never-nester, complexity). All code audit protocols should require explicit enumeration and review of every function in a file, not just those surfaced by search.

## Alignment with Current Codebase

- All patterns described above are implemented and enforced in the current codebase.
- The registry, macro system, and error handling are fully aligned with these patterns.
- The parser and macro system are decoupled and testable.

## Cross-References

- See `memory-bank/projectbrief.md` for project vision and aspirations.
- See `memory-bank/productContext.md` for product rationale and user needs.
- See `memory-bank/techContext.md` for technical stack and constraints.
- See `memory-bank/activeContext.md` for current work focus and priorities.
- See `memory-bank/progress.md` for completed work and next steps.
- See `.cursor/rules/memory-bank.mdc` for update protocol and overlays.

## Changelog

- 2025-07-03: Updated to resolve all audit TODOs, clarify patterns, and align with current codebase and guidelines.
- 2025-06-30: Initial synthesis from legacy documentation.
- 2025-07-04: Added modular parsing pipeline as a canonical system pattern.
- 2025-07-05: Migration to proper-list-only and ...rest-only architecture complete. All legacy code, tests, and documentation for improper/dotted lists and legacy variadic syntax have been removed.
- 2025-07-05: Macro system, CLI, and test harness refactor system patterns and changelog updated.
- 2025-07-06: Batch refactor for Rust idiom compliance (implicit/explicit return style), match exhaustiveness, and error handling. Explicit returns for early exits restored. All match arms for Expr variants in eval_expr restored. Protocol-driven, batch-based, test-first approach enforced. All tests pass. Lesson: Always enumerate all functions for audit, not just those surfaced by search. Macro system helpers refactored for protocol compliance (pure, linear, early-return, documented, no deep nesting). Protocol-driven audit and batch-based, test-driven modernization enforced. Memory bank updated per protocol.
- 2024-07-07: Added registry invariant regression test note.
- 2025-07-07: Macro/atom registry and test system are now fully Rust-idiomatic, with anti-nesting audits and iterator combinator refactors complete. Feature-gated (test-atom) and debug-assertion-based test atom registration is in place; integration tests that require test-only atoms are now feature-gated and optional. Protocol for feature-gated/optional integration tests is documented here and in activeContext.md. All code, tests, and documentation are up to date and compliant as of this session.

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

### Macro System Architecture

- Macro expansion is a purely syntactic, pure transformation pipeline stage.
- All macroexpander logic, macro functions, and recursive expansion operate on `WithSpan<Expr>` (never bare `Expr`).
- Macro system is layered, runs after parsing and before validation/evaluation.
- Expansion process is inspectable and traceable.
- Incremental refactor: `expand_template` now uses helpers for arity and parameter binding, with explicit error handling.
- Next: Explore a layered, provenance-aware macro system in a new branch.

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
- **Error Handling Policy (2025-07-06):**
  - All public APIs must use `SutraError` as the canonical error type.
  - All conversions from internal error types (e.g., `EvalError`) must be explicit and auditable.
  - All enum variant signatures must be matched exactly in both construction and pattern matching.
  - See `activeContext.md` and `progress.md` for rationale and implementation plan.

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

## Registry/Expander Reliability: Advanced Strategies (2025-07-02)

A full audit and review of macro registry/expander reliability strategies was conducted. The following techniques were considered and rated (1-5 scale):

| Technique                 | Necessity | Alignment | Payoff/Cost |
|---------------------------|-----------|-----------|-------------|
| Phantom types             | 3         | 5         | 4           |
| Registry hashing          | 4         | 5         | 5           |
| Sealed registry           | 3         | 5         | 4           |
| Loader/expansion logging  | 2         | 4         | 3           |
| Integration tests         | 5         | 5         | 5           |
| Test-in-prod smoke mode   | 3         | 5         | 4           |
| Provenance reporting      | 2         | 4         | 3           |
| Mutation linting          | 3         | 5         | 4           |
| Opt-out API               | 2         | 4         | 3           |
| Fuzzing                   | 2         | 4         | 3           |
| Singleton pattern         | 2         | 4         | 3           |
| Metrics                   | 2         | 4         | 3           |

**Immediate priorities:** Integration tests and registry hashing are to be implemented now. Phantom types, sealed registry, mutation linting, and smoke mode are recommended for incremental adoption. Advanced/forensic techniques are deferred unless future needs arise.

**Rationale:** This policy ensures maximal reliability, maintainability, and alignment with Sutra's principles. See activeContext.md for current implementation status and rationale.

_Last Updated: 2025-07-01_

- [2025-07-02] Registry hashing/fingerprinting implemented: MacroRegistry now computes a SHA256 hash of all macro names and definitions, with a canonical test in macro_expansion_tests.rs. See system-reference.md for details.

## Planned: Radical, Layered Macro System

- Macro registry will be layered (core, stdlib, user, scenario) for modularity and shadowing.
- Provenance tracking will attach origin metadata to all macro definitions and expansions.
- Expansion context will include provenance, hygiene, and layer for advanced features.
- Expansion trace will be recorded and inspectable for debugging and auditing.
- To be prototyped in a new branch after incremental improvements are validated.

## 2025-07-05: Macro System, CLI, and Test Harness Refactor (Session System Patterns)

- Macro expansion now enforces a strict recursion depth limit (128) on every expansion step.
- CLI output and macro expansion trace format have been updated for clarity and protocol compliance.
- Test harness and error handling patterns have been modernized and documented.
- All recursive macro expansion logic must increment and check recursion depth, with a default limit of 128.

### Registry Invariant Regression Test (2024-07-07)
- Milestone complete: registry invariant is enforced, output pipeline is robust, and all integration tests pass.

## Test Atom Registration Policy

Test atoms (e.g., `test/echo`) are always registered in dev/debug builds (`cfg(debug_assertions)`), as well as for tests and the `test-atom` feature. This ensures integration tests and dev builds always have test atoms available, but they are not present in release builds. This policy is intentional to support seamless integration testing without requiring feature flags.

## 2025-07-07: Feature-Gated and Optional Integration Test Protocol

- Macro/atom registry and test system are now fully Rust-idiomatic, with explicit anti-nesting audits and iterator combinator refactors complete.
- Test atoms (e.g., `test/echo`) are registered in dev/debug builds (`cfg(debug_assertions)`), for tests (`cfg(test)`), and when the `test-atom` feature is enabled.
- Integration tests that require test-only atoms are now feature-gated and optional, using `#[cfg(feature = "test-atom")]` on the test function or file.
- If the feature is not enabled, the test is skipped and does not fail.
- This protocol is documented here and in activeContext.md, and is now the canonical approach for optional, feature-gated integration tests in the Sutra project.

<!-- AUDIT ANNOTATIONS BEGIN -->

<!-- TODO: Review "Core Technical Patterns" and "Macro System Architecture" for currency and alignment with the latest language spec and codebase. -->

<!-- TODO: Confirm the current status of `cond` as a macro vs. core construct, and update all references for consistency. -->

<!-- TODO: Check for overlap/redundancy with projectbrief.md and productContext.md regarding architecture, patterns, and philosophy. -->

<!-- TODO: Add explicit cross-reference to memory-bank/README.md for memory-bank structure and usage. -->

<!-- TODO: Add changelog/versioning section to track future updates. -->

<!-- AUDIT ANNOTATIONS END -->
