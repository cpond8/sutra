# Sutra Engine - Active Context

## Current Work Focus

**Phase:** Macro System Canonicalization, Loader/Parser Hardening, and Test Modernization

**Priority:**
- Debug and resolve all macro loader and macro expansion test failures, especially those related to parameter list handling and macro definition parsing.
- Ensure the parser, PEG grammar, and macro loader are fully aligned and robust to all valid and invalid forms.
- Complete the migration to explicit, structured parameter list handling (`ParamList` struct) throughout the parser, AST, macro loader, and all tests.
- Harden error handling and error message checks to match canonical spec and test suite.
- Maintain strict test/production parity: all registry, loader, and macro expansion logic must be identical in both environments.
- Use a batch-based, test-driven approach: after each change, re-run the test suite and only proceed when the current batch is passing.

## Macro System Bootstrapping Roadmap (2025-07-02, updated 2025-07-04)

1. Migrate all standard macros to the native macro language (pending full test pass).
2. Design and implement macro hygiene.
3. Expand the standard macro library (Tier 2+).
4. Validation and author feedback.
5. Documentation, CLI, and tooling.

## Current Complexities and Context

- The macro system and parser have been refactored for explicit, robust parameter list handling, but legacy test cases and loader logic may not be fully updated.
- Many macro loader and macro expansion tests are failing due to mismatches in expected AST structure, error messages, or parameter list representation.
- The PEG grammar, parser, and loader must be audited and updated in lockstep to ensure all valid macro definitions are parsed and loaded correctly, and all invalid forms are rejected with clear, author-facing errors.
- The test suite is being modernized in batches: each batch updates a set of tests or logic, then re-runs the suite to confirm correctness before proceeding.
- All error handling must be robust, contextual, and span-aware, with error messages matching the canonical spec and test suite.
- Documentation and memory bank files must be updated after every significant change or insight.

## Next Steps (Immediate Priority)

1. Continue batch-based modernization of macro loader and macro expansion tests, updating for new parameter list struct and error handling.
2. Audit and update the PEG grammar and parser logic for full alignment with loader/test expectations.
3. Update documentation and memory bank after each significant change.
4. Maintain strict test/production parity and round-trippability.
5. Run the full test suite after each batch and only proceed when all updated tests pass.
6. Update code review protocols and checklists to require explicit function enumeration for complexity/nesting audits, not just semantic search. (2025-07-05)

## Key Lessons and Guidance for Future Contributors

- Always ensure the parser, grammar, loader, and tests are updated together when making changes to core syntax or AST structure.
- Use batch-based, test-driven development to isolate and resolve failures incrementally.
- Maintain strict separation of concerns: parser only parses, macro loader only loads, macro expander only expands.
- All error messages must be clear, actionable, and span-carrying.
- Update documentation and memory bank files after every significant change.
- Review this file and system-reference.md before making or reviewing changes.

## Changelog

- 2025-07-04: Updated for batch-based macro loader/parser/test modernization, parameter list struct migration, and current debugging context.
- 2025-07-03: Updated to resolve all audit TODOs, clarify active context, and align with current codebase and guidelines.
- 2025-06-30: Initial synthesis from legacy documentation.
- 2025-07-04: Updated for parsing pipeline refactor as current focus and added cross-reference to plan.
- 2025-07-04: src/atoms_std.rs is now fully span-carrying compliant. All error macros and atom logic use WithSpan<Expr> throughout. All linter/type errors resolved. Canonical error macro pattern enforced. See parsing-pipeline-plan.md and macroexpander migration for context.
- 2025-07-05: Codebase, tests, and documentation are now fully compliant with the proper-list-only and ...rest-only architecture. All legacy dotted/improper list handling and legacy variadic syntax have been removed.
- 2025-07-05: Macro system, CLI, and test harness refactor completed. Session summary and next steps added.

## Rationale for Order

- Macro migration and hygiene must precede library expansion to avoid subtle bugs.
- Validation and documentation are most effective once the macro system is stable and self-hosted.

## Status and Recent Changes

- **Variadic/Recursive Macro Support Complete:** MacroTemplate now supports variadic and mixed-arity macros. Argument binding and error handling are robust and explicit. Comprehensive tests for all edge cases (including recursion depth, arity, and parameter validation) are in place and passing.
- **Macro System Ready for Self-Hosting:** The macro system is now ready for migration of all standard macros to the native engine language.
- **Parser Refactor Complete:** The parser has been decomposed into per-rule helpers, with robust error handling and explicit dotted list validation. All unreachable!()s replaced with structured errors. Dotted list parsing now asserts and errors on malformed shapes.
- **Test Suite Run:** The parser compiles and passes borrow checker, but several macro loader and macro expansion tests (especially for variadic macros and cond) are failing. Parser and macro system are now fully decoupled and testable.

## Active Decisions and Considerations

- Parser and macro system must be fully round-trippable and robust to malformed input.
- All error returns must include rule, input, and span for debugging.
- Dotted list parsing is now strict and canonical.
- Macro system migration to native macros is on hold until parser/macro test failures are resolved.

## Confirmed Design Decisions

- **Professional CLI Architecture:** The CLI is a pure orchestrator of the core library, with a strict separation of concerns. All user-facing output is centralized in its own module.
- **Rich, Contextual Errors:** The `EvalError` struct and two-phase enrichment pattern is the new standard for all user-facing errors.
- **Minimal, Pure, and Robust Atoms:** All core atoms, especially `get`, are now implemented as pure, minimal, and robust functions, with explicit handling for all edge cases and consistent return values.
- **Keyword-Avoidant Naming:** The `macro` module was renamed to `macros` to improve clarity and avoid confusing `r#` syntax.
- **Strict pipeline separation:** `parse -> expand -> eval`.
- **Path Canonicalization:** The macro system is the sole authority for converting user-facing path syntax into canonical `Expr::Path` nodes. This is the primary architectural pattern.
- **Strict Pipeline Separation:** The `parse -> expand -> eval` pipeline is strictly enforced. The evaluator will reject any non-canonical AST forms (like bare symbols), ensuring the macro expander has done its job.
- **Span-aware `AtomFn`:** The signature `fn(..., parent_span: &Span)` is the standard for all atoms.
- **Unified Registry Pattern:** All atom and macro registration is now centralized in canonical builder functions. No duplication exists between test and production setup; all code paths use the same logic for registry construction.
- **Doctest Documentation Pattern:** All public API functions that can be meaningfully demonstrated in isolation (e.g., registry builders) have doc examples that compile and run. Internal or context-dependent functions (e.g., atoms) are documented with comments explaining why doctests are not appropriate.

## Current Design Questions

- **Macro hygiene:** How sophisticated to make the hygiene system for user-defined macros.
- **Performance optimization:** When to implement lazy evaluation or other optimizations.
- **`get` atom completion:** The `get` atom needs to be extended to support collection access (lists, maps) in addition to world paths.
- **`cond`-first architecture:** The decision has been made to make `cond` the primary, variadic conditional macro, which expands into `if`. `if` is a simple, 3-arity macro that expands to the `Expr::If` primitive. This maintains a minimal core while providing a powerful and ergonomic authoring experience.

## Important Patterns and Preferences

- **No explicit `get` operations:** Automatic value resolution in all contexts.
- **Clear mutation marking:** All state changes use `!` suffix (`set!`, `add!`, etc.).
- **Consistent predicate naming:** All boolean checks use `?` suffix (`is?`, `has?`, etc.).
- **Readable aliases:** Comparison operators have both canonical (`gt?`) and readable (`over?`) forms.

## Technical Architecture Principles

- **Library-first design:** Core as pure library, CLI as thin wrapper.
- **No global state:** Everything flows through explicit parameters.
- **Pure functions everywhere:** Except for explicit atom mutations on world state.
- **Testability at every level:** Each module independently testable.
- **Transparent debugging:** Macro expansion and world state changes always inspectable.

## Learnings and Project Insights

- **Canonicalization Contract Fully Enforced:** The macro system now guarantees that all atom path arguments are in the canonical flat `(list ...)` form, with a single source of truth and comprehensive test coverage.
- **Immutability requires careful state propagation:** When working with immutable data structures like `World`, it is critical to ensure that the _new_ state from one operation is correctly threaded as the input to the _next_ operation.
- **Registries are a critical architectural component:** A single source of truth for standard libraries is non-negotiable for a robust, maintainable system.
- **Test environments must mirror production:** Test setup must be identical to the production pipeline to be reliable.
- **Minimalism enables power:** Small atom set + macro composition provides unlimited expressiveness.
- **Syntax flexibility matters:** Dual syntax removes adoption barriers while preserving power.
- **Transparency is crucial:** Authors must be able to understand and debug their content.
- **Immutability simplifies:** Pure functional approach eliminates many bug classes.
- **Doctest Documentation Pattern:** Add doctests to all public APIs where feasible, and document with comments where not. This ensures clarity for future contributors and robust, example-driven documentation.

## Implementation Strategy Insights

- **Staged approach is critical:** Each stage validates previous decisions before proceeding.
- **Documentation-driven development:** Comprehensive design docs prevent architectural drift.
- **Test-driven from start:** TDD approach ensures reliability and debuggability.
- **Registry pattern scales:** Enables extension without core modifications.

## Narrative Design Insights

- **QBN patterns are achievable:** All Emily Short patterns can be expressed as macros.
- **Emergence from composition:** Complex narrative behaviors arise from simple building blocks.
- **Author ergonomics matter:** Syntax and debugging tools are as important as functionality.
- **Modularity enables reuse:** Storylets, pools, and threads compose cleanly.

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

The highest-value, lowest-cost techniques (integration tests, registry hashing) are prioritized for immediate implementation. Others (phantom types, sealed registry, mutation linting, smoke mode) are recommended for incremental adoption as the codebase and team grow. Advanced/forensic techniques (provenance, logging, opt-out API, fuzzing, singleton, metrics) are deferred unless future needs arise.

**Rationale:** This approach maximizes reliability and maintainability while upholding Sutra's principles of minimalism, compositionality, and single source of truth. All decisions and ratings are archived for future reference and onboarding.

- [2025-07-02] Registry hashing/fingerprinting implemented: MacroRegistry now computes a SHA256 hash of all macro names and definitions, with a canonical test in macro_expansion_tests.rs. See system-reference.md for details.

## Alignment with Current Codebase

- All active context, decisions, and priorities described above are implemented and enforced in the current codebase.
- The roadmap, design decisions, and patterns are up-to-date and match the live system.

## Cross-References

- See `memory-bank/projectbrief.md` for project vision and aspirations.
- See `memory-bank/productContext.md` for product rationale and user needs.
- See `memory-bank/systemPatterns.md` for architectural and design patterns.
- See `memory-bank/techContext.md` for technical stack and constraints.
- See `memory-bank/progress.md` for completed work and next steps.
- See `.cursor/rules/memory-bank.mdc` for update protocol and overlays.

## Parsing Pipeline Refactor: Current Focus (2025-07-04)

- The modular parsing pipeline refactor is now the primary focus. All work should proceed in alignment with the canonical plan in `docs/architecture/parsing-pipeline-plan.md`.
- Immediate next steps: Ship interfaces and trivial implementations, write golden tests, document contracts, and review each module in isolation before integration.

## Macroexpander Refactor and AST Invariant Migration (2025-07-04)

- Migrated Expr::List to Vec<WithSpan<Expr>> in all core modules: ast.rs, macros_std.rs, macros.rs, parser.rs, eval.rs, validate.rs.
- All macroexpander logic, helpers, and registry now operate on WithSpan<Expr> throughout the pipeline.
- Issues encountered:
  - Linter/type errors due to mixed Expr/WithSpan<Expr> usage, especially in pattern matches and list construction.
  - Macro_rules! and error helper macros in atoms_std.rs require explicit, line-by-line fixes for delimiter and type safety.
  - Some macro contexts and test helpers still need a final audit for span-carrying compliance.
- Current status: Macroexpander and helpers are type-safe and span-carrying. Atoms_std.rs and some macro contexts need a final audit.
- Remaining work:
  - Complete audit and fix in atoms_std.rs (especially macro_rules! and error helpers).
  - Update all tests and doc examples for new AST invariant.
  - Perform a final integration test of the pipeline.
- Context for future contributors:
  - All new code must use WithSpan<Expr> for AST lists and macroexpander logic.
  - Legacy API is deprecated and should not be used.
  - See parsing-pipeline-plan.md for canonical contracts and migration rationale.

## 2025-07-05: Documentation Audit

- All major documentation (architecture, parsing pipeline, language spec, authoring patterns, storylet spec) was reviewed on July 5, 2025.
- All docs are up to date, accurate, and fully aligned with the current codebase and recent progress.
- Canonical parsing pipeline, language spec, macro/atom boundaries, and authoring patterns are all current and use proper-list-only and ...rest conventions.
- Changelogs and review dates are present and recent in all docs.
- No legacy or deprecated patterns remain in the main documentation.
- See: `docs/architecture/parsing-pipeline-plan.md`, `docs/specs/language-spec.md`, `docs/architecture/authoring-patterns.md`, `docs/specs/storylet-spec.md`, `docs/architecture/architecture.md`.

## Macro System Refactor (Incremental Phase)

- Completed incremental refactor of `expand_template` in `src/macros.rs`.
    - Arity checking and parameter binding are now handled by dedicated helper functions.
    - All error cases are explicit and robust.
    - The function is clearer, easier to maintain, and fully tested.
- No changes to public API or behavior.
- All tests pass, including edge cases.

**Next step:**
- Explore and prototype a radical, layered, provenance-aware macro system in a new branch.

## Planned: Radical, Layered, Provenance-Aware Macro System

- **Motivation:**
    - Enable advanced macro hygiene, debugging, and modularity for complex authoring scenarios and future game engine extensibility.
    - Support provenance tracking for macro definitions and expansions, making macro origin and transformation history inspectable.
- **Key Features:**
    - **Layered Macro Registry:** Macros are registered in layers (e.g., core, standard library, user, scenario), supporting shadowing and modular extension.
    - **Provenance Metadata:** Every macro definition and expansion carries metadata about its origin (file, line, author, etc.) and transformation path.
    - **Context-Aware Expansion:** Macro expansion context includes provenance, hygiene scope, and layer, enabling advanced features like macro hygiene and selective expansion.
    - **Inspectable Expansion Trace:** The system records a trace of macro expansions, allowing authors to debug and audit macro transformations.
- **Next Step:**
    - Prototype this architecture in a new branch, iterating on design and integration with the existing pipeline after validating incremental improvements.

## 2025-07-05: Macro System, CLI, and Test Harness Refactor (Session Summary)

- Completed a comprehensive refactor and modernization of the macro system, CLI, and test harness.
- Removed all legacy macroexpander logic (`MacroExpander`, `SutraMacroContext`, `SutraMacroExpander`) from the codebase.
- Refactored the CLI to use the new `MacroEnv` and `expand_macros` API, and updated the output module for the new macro expansion trace format.
- Updated all tests to use the new macro system, introduced the `must_expand_ok` helper, and enforced correct error/result handling.
- Fixed architectural issues with recursion depth tracking in macro expansion, enforcing a strict limit (128) on all expansion paths.
- Pruned and updated the test suite, ensuring all tests pass and removing outdated or irrelevant tests.
- Performed a full documentation and memory bank audit, confirming protocol compliance and identifying areas for update.

**Next Steps:**
- Continue to update documentation and memory bank files after any further significant changes.
- Maintain strict protocol compliance and batch-based, test-driven development.
