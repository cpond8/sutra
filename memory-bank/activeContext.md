# Sutra Engine - Active Context

## Current Work Focus

**Phase**: Macro System Bootstrapping & Parser Reliability
**Priority**: Ensure parser, macro loader, and macro expander are robust, especially for variadic macro support and canonical macro authoring. Debug and resolve all macro system test failures.

## Macro System Bootstrapping Roadmap (2025-07-02)

This roadmap is the single source of truth for macro system bootstrapping and self-hosting. Each step is ordered to minimize risk, maximize architectural clarity, and ensure a robust, extensible macro system.

### 1. Implement Full Variadic/Recursive Macro Support
- Extend the macro system so user-defined macros can be variadic and recursive.
- Ensure robust error handling and recursion limits.

### 2. Migrate All Standard Macros to the Native Macro Language
- Rewrite all higher-level macros (especially `cond`) as native macros using the new support.
- Remove Rust-native macro implementations.
- Update and expand tests for new macro definitions.

### 3. Design and Implement Macro Hygiene
- Design a hygiene system (e.g., gensym for local bindings) to prevent accidental variable capture.
- Integrate into the macro expander and document its behavior.

### 4. Expand the Standard Macro Library (Tier 2+)
- Implement all higher-level narrative/gameplay macros (`storylet`, `choice`, `pool`, etc.) as native macros.
- Develop comprehensive example scripts and usage patterns.
- Performance test with realistic content.

### 5. Validation and Author Feedback
- Implement structural and semantic validation before and after macro expansion.
- Integrate validation and error reporting with CLI and author tools.

### 6. Documentation, CLI, and Tooling
- Audit and update documentation to reflect the new macro system.
- Ensure CLI exposes macroexpansion, registry, and validation features.

## Rationale for Order
- Variadic/recursive macro support is the key technical blocker for self-hosting.
- Macro migration and hygiene must precede library expansion to avoid subtle bugs.
- Validation and documentation are most effective once the macro system is stable and self-hosted.

_This roadmap supersedes all previous immediate priorities for macro system bootstrapping. All planning and implementation should reference this plan as canonical._

_Last Updated: 2025-07-02_

### Recent Changes (2025-07-02)

1. **Variadic/Recursive Macro Support Complete:** MacroTemplate now supports variadic and mixed-arity macros. Argument binding and error handling are robust and explicit. Comprehensive tests for all edge cases (including recursion depth, arity, and parameter validation) are in place and passing.
2. **Macro System Ready for Self-Hosting:** The macro system is now ready for migration of all standard macros to the native engine language.

## Recent Changes (2025-07-02)

- **Parser Refactor Complete:** The parser has been decomposed into per-rule helpers, with robust error handling and explicit dotted list validation. All unreachable!()s replaced with structured errors. Dotted list parsing now asserts and errors on malformed shapes.
- **Test Suite Run:** The parser compiles and passes borrow checker, but several macro loader and macro expansion tests (especially for variadic macros and cond) are failing. Parser and macro system are now fully decoupled and testable.

## Next Steps (Immediate Priority)

1. **Analyze and debug failing macro system tests.** Focus on variadic macro parameter parsing and cond macro expansion.
2. **Ensure parser and macro system are in sync.** Confirm AST output matches macro loader expectations for all edge cases.
3. **Update documentation and CLI** to reflect parser refactor and macro system changes after all tests pass.
4. **Expand/adjust tests** as needed to cover new edge cases and regression scenarios.

## Active Decisions and Considerations

- Parser and macro system must be fully round-trippable and robust to malformed input.
- All error returns must include rule, input, and span for debugging.
- Dotted list parsing is now strict and canonical.
- Macro system migration to native macros is on hold until parser/macro test failures are resolved.

_Last Updated: 2025-07-02_

### Confirmed Design Decisions

- **Professional CLI Architecture**: The CLI is a pure orchestrator of the core library, with a strict separation of concerns. All user-facing output is centralized in its own module.
- **Rich, Contextual Errors**: The `EvalError` struct and two-phase enrichment pattern is the new standard for all user-facing errors.
- **Minimal, Pure, and Robust Atoms**: All core atoms, especially `get`, are now implemented as pure, minimal, and robust functions, with explicit handling for all edge cases and consistent return values.
- **Keyword-Avoidant Naming**: The `macro` module was renamed to `macros` to improve clarity and avoid confusing `r#` syntax.
- **Strict pipeline separation**: `parse -> expand -> eval`.
- **Path Canonicalization**: The macro system is the sole authority for converting user-facing path syntax into canonical `Expr::Path` nodes. This is the primary architectural pattern.
- **Strict Pipeline Separation**: The `parse -> expand -> eval` pipeline is strictly enforced. The evaluator will reject any non-canonical AST forms (like bare symbols), ensuring the macro expander has done its job.
- **Span-aware `AtomFn`**: The signature `fn(..., parent_span: &Span)` is the standard for all atoms.
- **Unified Registry Pattern**: All atom and macro registration is now centralized in canonical builder functions. No duplication exists between test and production setup; all code paths use the same logic for registry construction.
- **Doctest Documentation Pattern**: All public API functions that can be meaningfully demonstrated in isolation (e.g., registry builders) have doc examples that compile and run. Internal or context-dependent functions (e.g., atoms) are documented with comments explaining why doctests are not appropriate.

### Current Design Questions

- **Macro hygiene**: How sophisticated to make the hygiene system for user-defined macros.
- **Performance optimization**: When to implement lazy evaluation or other optimizations.
- **`get` atom completion**: The `get` atom needs to be extended to support collection access (lists, maps) in addition to world paths.
- **`cond`-first architecture**: The decision has been made to make `cond` the primary, variadic conditional macro, which expands into `if`. `if` is a simple, 3-arity macro that expands to the `Expr::If` primitive. This maintains a minimal core while providing a powerful and ergonomic authoring experience.

## Important Patterns and Preferences

### Cline's Implementation Protocol

1.  **Evaluate Before Writing**: For every file I create or modify, I must first explicitly write a "Code Evaluation" section. This section will analyze the proposed code against the implementation plan and the project's core design principles (Purity, Modularity, Separation of Concerns, etc.) and assign each point a rating from 1-10. I will not proceed with a `write_to_file` or `replace_in_file` operation until this evaluation is complete and confirms alignment.
2.  **Document All Incomplete Work**: For any feature or function that I intentionally leave unimplemented or in a placeholder state, I must leave a clear `// TODO:` comment block. This comment must explain what is missing and what the next steps are for completing the feature. This ensures no work is accidentally forgotten.

### Author Experience Priorities

1. **No explicit `get` operations** - automatic value resolution in all contexts
2. **Clear mutation marking** - all state changes use `!` suffix (`set!`, `add!`, etc.)
3. **Consistent predicate naming** - all boolean checks use `?` suffix (`is?`, `has?`, etc.)
4. **Readable aliases** - comparison operators have both canonical (`gt?`) and readable (`over?`) forms

### Technical Architecture Principles

1. **Library-first design** - core as pure library, CLI as thin wrapper
2. **No global state** - everything flows through explicit parameters
3. **Pure functions everywhere** - except for explicit atom mutations on world state
4. **Testability at every level** - each module independently testable
5. **Transparent debugging** - macro expansion and world state changes always inspectable

## Learnings and Project Insights

### Key Architectural Insights

- **Canonicalization Contract Fully Enforced**: The macro system now guarantees that all atom path arguments are in the canonical flat `(list ...)` form, with a single source of truth and comprehensive test coverage.
- **Immutability requires careful state propagation**: The `ATOM_DO` bug is a perfect example of this. When working with immutable data structures like `World`, it is critical to ensure that the _new_ state from one operation is correctly threaded as the input to the _next_ operation. A single mistake here breaks the chain of state.
- **Registries are a critical architectural component**: The discovery of duplicated and incomplete atom/macro registries highlights that a single source of truth for standard libraries is non-negotiable for a robust, maintainable system.
- **Test environments must mirror production**: The fact that the test suite's macro registry was incomplete but tests were still passing (incidentally) shows the danger of divergent environments. A test setup must be identical to the production pipeline to be reliable.
- **Minimalism enables power**: Small atom set + macro composition provides unlimited expressiveness
- **Syntax flexibility matters**: Dual syntax removes adoption barriers while preserving power
- **Transparency is crucial**: Authors must be able to understand and debug their content
- **Immutability simplifies**: Pure functional approach eliminates many bug classes
- **Doctest Documentation Pattern**: The project now follows a clear pattern: add doctests to all public APIs where feasible, and document with comments where not. This ensures clarity for future contributors and robust, example-driven documentation.

### Implementation Strategy Insights

- **Staged approach is critical**: Each stage validates previous decisions before proceeding
- **Documentation-driven development**: Comprehensive design docs prevent architectural drift
- **Test-driven from start**: TDD approach ensures reliability and debuggability
- **Registry pattern scales**: Enables extension without core modifications

### Narrative Design Insights

- **QBN patterns are achievable**: All Emily Short patterns can be expressed as macros
- **Emergence from composition**: Complex narrative behaviors arise from simple building blocks
- **Author ergonomics matter**: Syntax and debugging tools are as important as functionality
- **Modularity enables reuse**: Storylets, pools, and threads compose cleanly

## 2025-07-02 Active Context Update
- **Variadic Macro System Implemented:** A two-tiered macro system is now in place. `MacroTemplate` supports simple declarative macros, while `MacroFn` supports complex procedural macros.
- **Architectural Plan Finalized:** After extensive analysis, a final plan for the conditional system has been approved. `cond` will be the primary, variadic conditional macro, expanding to the simpler `if` macro, which in turn creates the `Expr::If` primitive.
- **Documentation Updated:** All relevant memory bank files (`systemPatterns.md`, `progress.md`) and the `system-reference.md` have been updated to reflect this final architecture.
- **Next Step:** Implement the `cond`-first architecture.

_Last Updated: 2025-07-01_

## Current Focus
- Macro migration: Begin rewriting standard macros as native macros using the new variadic/recursive system. Ensure all tests and documentation are updated accordingly.

## Next Steps
- Audit CLI/docs for macroexpansion trace and author help.
- Monitor for macro system upgrades to enable migration of `cond` to user macro.

- Adopted new canonical macro definition syntax: macro definitions now require parentheses around the macro name and parameters, e.g. `define (my-list first . rest) { ... }`.
- Language spec updated accordingly; all unrelated content restored after accidental removal.
- Only macro definition section changed; all other sections remain as originally specified.
- Future spec/documentation edits must be surgical and avoid regressions in unrelated content.

## Registry/Expander Reliability: Advanced Strategies (2025-07-02)

A comprehensive review of best-in-class strategies for preventing registry/expander/test drift in macro systems was conducted, including:
- Phantom types for canonical registry
- Registry hashing/fingerprinting
- Sealed/immutable registry
- Loader/expansion logging
- Integration tests for loader/expander parity
- Test-in-prod smoke mode
- Provenance reporting
- Registry mutation linting
- Dangerous opt-out APIs for non-canonical registries
- Fuzzing and order randomization in CI
- Singleton pattern for canonical registry
- Metrics collection

Each was rated for necessity, alignment with project principles, and payoff vs. cost (see systemPatterns.md for full table). The highest-value, lowest-cost techniques (integration tests, registry hashing) are prioritized for immediate implementation. Others (phantom types, sealed registry, mutation linting, smoke mode) are recommended for incremental adoption as the codebase and team grow. Advanced/forensic techniques (provenance, logging, opt-out API, fuzzing, singleton, metrics) are deferred unless future needs arise.

**Rationale:** This approach maximizes reliability and maintainability while upholding Sutra's principles of minimalism, compositionality, and single source of truth. All decisions and ratings are archived for future reference and onboarding.

- [2025-07-02] Registry hashing/fingerprinting implemented: MacroRegistry now computes a SHA256 hash of all macro names and definitions, with a canonical test in macro_expansion_tests.rs. See system-reference.md for details.
