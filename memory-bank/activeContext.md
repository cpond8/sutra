# TOP PRIORITY: IMPLEMENT NATIVE .sutra FILE LOADING AND INTERPRETATION
# The engine must first be able to load, parse, and interpret native `.sutra` files (written in the Sutra engine language, not Rust). This is the absolute prerequisite for all further macro migration, test suite rewrite, or native language support. No other work may proceed until this is complete.

# TEST SUITE REWRITE REQUIRED
# All tests must be rewritten to use only user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted. See systemPatterns.md and README.md for protocol.

# BLOCKERS: Native-Language Test Suite (2025-07-06)

The following explicit gaps must be resolved before a fully native-language (s-expr and brace/block) test suite is possible:

## A. Macro System
- [ ] Migrate all standard macros to native macro language (no Rust-native macro implementations).
- [ ] Complete macro loader and macro expansion test modernization (parameter list, error handling, edge cases).
- [ ] Implement macro hygiene (gensym, hygiene scope, provenance tracking).

## B. Parser and Syntax
- [ ] Audit and update PEG grammar and parser logic for full alignment with macro loader/test expectations.
- [ ] Ensure both s-expr and brace/block forms are parsed identically into canonical AST.
- [ ] Finalize parameter list struct migration in parser, AST, macro loader, and all tests.

## C. Atoms and Core Engine
- [ ] Audit and fix all atoms for span-carrying compliance (esp. macro_rules! and error helpers in atoms_std.rs).
- [ ] Ensure all core atoms and macro expansion patterns match the canonical spec (see language-spec.md).

## D. Error Handling and Validation
- [ ] Harden error handling and error message checks to match canonical spec and test suite.
- [ ] Implement structural and semantic validation before/after macro expansion.

## E. Test Suite and Documentation
- [ ] Rewrite all tests as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors.
- [ ] Update all tests and doc examples for new AST invariant (WithSpan<Expr>).
- [ ] Audit and update documentation and memory bank after each significant change.

# PRIORITIZED ACTION PLAN: Native-Language Test Suite Blockers (2025-07-06)

This is the canonical, dependency-ordered plan for resolving all blockers to a fully native-language (s-expr and brace/block) test suite. Each step must be completed before the next can proceed. See progress.md and system-reference.md for cross-references and rationale.

## 1. Macro System Modernization (Highest Priority)
- Migrate all standard macros to native macro language (remove Rust-native macro implementations).
- Complete macro loader and macro expansion test modernization (parameter list, error handling, edge cases).
- Implement macro hygiene (gensym, hygiene scope, provenance tracking).

## 2. Parser and Syntax Alignment
- Audit and update PEG grammar and parser logic for full alignment with macro loader/test expectations.
- Finalize parameter list struct migration in parser, AST, macro loader, and all tests.
- Ensure both s-expr and brace/block forms are parsed identically into canonical AST.

## 3. Atoms and Core Engine Audit
- Audit and fix all atoms for span-carrying compliance (esp. macro_rules! and error helpers in atoms_std.rs).
- Ensure all core atoms and macro expansion patterns match the canonical spec (see language-spec.md).

## 4. Error Handling and Validation
- Harden error handling and error message checks to match canonical spec and test suite.
- Implement structural and semantic validation before/after macro expansion.

## 5. Test Suite Rewrite and Documentation
- Rewrite all tests as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors.
- Update all tests and doc examples for new AST invariant (WithSpan<Expr>).
- Audit and update documentation and memory bank after each significant change.

Batch-based, test-driven iteration is required: after each batch, re-run the full test suite and only proceed when all updated tests pass. Documentation and memory bank must be updated after each batch.

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
- 2025-07-06: Batch refactor completed for Rust idiom compliance (implicit/explicit return style), match exhaustiveness, and error handling. All explicit returns for early exits restored where required. All match arms for Expr variants in eval_expr restored. Protocol-driven, batch-based, test-first approach enforced. All tests pass. Lesson: Always enumerate all functions for audit, not just those surfaced by search.
- 2025-07-06: Macro system helpers refactored for protocol compliance (pure, linear, documented, no deep nesting). Full protocol-driven audit performed. All tests and docs updated. Memory bank changelogs updated per protocol.
- **Integration Test Runner Bootstrapped (2025-07-06):**
  - Created `tests/scripts/` directory and added first `.sutra` test script (`hello_world.sutra`) with expected output (`hello_world.expected`).
  - This is the foundation for protocol-compliant, user-facing integration tests. See `progress.md` and `systemPatterns.md` for details.

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

## Active Context Update: Parser Refactor

- `build_param_list` in `src/parser.rs` is now a maximally functional, type-driven, protocol-compliant example.
- Uses a custom enum (`ParamListParts`) to encode invariants and eliminate illegal states.
- All logic is expressed with iterator combinators, pattern matching, and pure helpers.
- No mutation, flags, or unreachable! macros remain.
- This is now the reference pattern for parameter parsing and similar logic in the project.

- Macro expansion engine is now refactored for purity, composability, and protocol compliance.
- Macro expansion is handled by pure, single-responsibility functions.
- All tests and lints pass; documentation is up to date and follows @Rust best practices.

## Step-by-Step Plan: Native `.sutra` File Loading and Interpretation

1. **Requirements & Scope Definition**
   - Clarify MVP: engine must load a `.sutra` file, parse it, and execute it through the full pipeline (`parse → macro-expand → validate → evaluate → output`).
   - All macro and user code must be loaded from `.sutra` source, not Rust.
   - Document all interfaces and contracts before coding.

2. **CLI & API Input Pathways**
   - Add CLI argument (e.g., `sutra run <file.sutra>`) to specify a `.sutra` file to load and run.
   - Support reading from stdin for piping scripts.
   - Expose a public Rust API for loading and running `.sutra` source from a string or file.

3. **File Reading & Error Handling**
   - Implement robust file reading with clear, span-carrying error messages for missing/unreadable files.
   - Test: Attempt to load a non-existent file and verify user-facing error output.

4. **Parsing Pipeline Integration**
   - Parse the loaded file using the canonical PEG parser, producing a `Vec<WithSpan<Expr>>` AST.
   - All parse errors must include file name, line/col, and a clear message.
   - Test: Parse a minimal valid and invalid `.sutra` file, verify error output.

5. **Macro Loader from Source**
   - Implement a macro loader that scans the parsed AST for macro definitions, registers them, and handles errors for duplicates/invalid forms.
   - Test: Load a `.sutra` file with valid/invalid macro definitions, verify registry and error output.

6. **User Code Extraction**
   - Separate macro definitions from user code in the parsed AST.
   - Ensure only user code is passed to macro expansion and evaluation.
   - Test: Run a `.sutra` file with both macro definitions and user code, verify correct execution.

7. **Pipeline Wiring**
   - Wire up the full pipeline: `parse → macro-expand → validate → evaluate → output`.
   - Ensure all stages are called in order, with clear error propagation and reporting.
   - Test: Run a `.sutra` file that defines and uses a macro, verify correct expansion and evaluation.

8. **Output & Diagnostics**
   - Ensure all output is routed through the CLI output module, with clear formatting.
   - Support tracing macro expansion and evaluation steps for debugging and transparency.
   - Test: Run a `.sutra` file and verify output and trace diagnostics.

9. **Documentation & Protocol Compliance**
   - Update all relevant documentation (memory bank, CLI help, README, architecture docs).
   - Add usage examples for running `.sutra` files.
   - Document all new/changed interfaces and error messages.

10. **Batch-Based, Test-Driven Iteration**
    - After each batch: run the full test suite, update documentation and memory bank, only proceed when all tests pass and docs are current.

11. **Quality & Protocol Audits**
    - Run full protocol-driven audits: linting, formatting, test coverage, manual review for compliance, update memory bank with lessons learned.

12. **Final Review & Release**
    - Conduct a final review: ensure all requirements are met, docs are up to date, and the system is robust. Tag and release the version with native `.sutra` file support.

### Guiding Principles (applied at every step):
- Minimalism, Transparency, Compositionality, Test/Production Parity, Documentation-Driven, Batch-Based/Test-Driven.

## Integration Test Runner Plan (for Native `.sutra` Scripts)

### Purpose
- Build a robust, protocol-compliant integration test runner for `.sutra` scripts.
- Ensure all tests are discoverable, automatable, and produce clear, colorized, and auditable output.

### Scope
- The runner must:
  - Execute all `.sutra` scripts in a test directory (e.g., `tests/scripts/`).
  - Compare actual output/errors to expected results (from comments or sidecar files).
  - Report pass/fail with clear diagnostics.
  - Be extensible for future test types (macro expansion, error cases, world state queries).

### Plan (Selected: Rust Integration Test Runner)
1. **Create `tests/scripts/` directory** for `.sutra` test scripts.
2. **Add sample `.sutra` scripts** with expected output (in comments or `.expected` files).
3. **Implement `tests/script_runner.rs`**:
   - Discover all `.sutra` files in `tests/scripts/`.
   - For each file:
     - Read the script and expected output.
     - Run the script using the public API (`run_sutra_source`).
     - Capture and compare output/errors.
     - Report pass/fail with colorized output.
   - Fail the test suite if any script fails.
4. **Integrate with `cargo test`** for automation.
5. **Document and cross-link** all test scripts and runner logic in the memory bank and `progress.md`.

### Risks & Considerations
- Output capture and comparison must be robust.
- Standardize expected output format (top-of-file comment or `.expected` file).
- Distinguish expected vs. unexpected errors.

### Next Actions
1. Create `tests/scripts/` and add at least one `.sutra` test script with expected output.
2. Draft `tests/script_runner.rs` to discover and run all `.sutra` scripts.
3. Implement output capture and comparison logic.
4. Integrate colorized, protocol-compliant diagnostics and reporting.
5. Document the test runner and scripts in the memory bank and cross-link in `progress.md`.

### Rationale
- Plan A (Rust integration test runner) is selected for protocol, audit, and Rust ecosystem alignment.
- All steps are diagram-governed and protocol-compliant.
- This plan is canonical and must be resumed in the next session.

## Error Handling Refactor Plan (2025-07-06):
  - Initiated a full audit and refactor of error handling across the Sutra engine.
  - Goal: Standardize all public APIs to use `SutraError`, ensure all conversions from internal error types are explicit, and match all enum variant signatures exactly.
  - Rationale: Persistent build/test errors traced to inconsistent error propagation and enum usage.
  - See `progress.md` for stepwise plan and next actions.

- Decision: Proceeding with the error handling refactor proposal as discussed in recent reviews and architectural notes.
- Plan:
  - Replace all error construction macros (e.g., eval_err!) with ergonomic, domain-specific and general constructor functions in error.rs.
  - Centralize all error construction helpers in error.rs; remove scattered logic from other modules.
  - Document the distinction between general-purpose and domain-specific error constructors, and provide onboarding guidance in error.rs.
  - Only introduce a ContextualError trait if more than one structured error type (like EvalError) emerges, to avoid premature abstraction.
  - Enforce usage of constructor helpers (and ban direct struct construction of SutraError outside error.rs) via a CI lint/test, with a possible future custom Clippy lint.
- Rationale: This approach improves maintainability, onboarding, type safety, and user-facing error quality, and aligns with Rust best practices.
- Next Steps: Begin implementation as per the proposal, update documentation and tests accordingly.

# [2025-07-07] Error Handling Refactor Audit & Preparation (Batch 1)

## Summary

**This section records the protocol-driven audit and preparation for the error handling refactor, as required by the error-refactor-plan.md and memory bank protocol.**

### Files/Locations with Direct `SutraError` Construction or Error Macro Usage
- Direct `SutraError { ... }` construction found in:
  - src/parser.rs (parsing errors, malformed AST, internal parse errors)
  - src/macros.rs (macro expansion errors, duplicate macro names, parameter validation)
  - src/eval.rs (evaluation errors, recursion depth, type/arity errors)
  - src/atoms_std.rs (atom contract errors, type/arity errors)
  - src/macros_std.rs (macro helpers, path canonicalization errors)
  - src/cli/output.rs (output formatting, error printing)
  - src/cli/mod.rs (CLI error handling)
  - src/validate.rs (validation errors)
- Error construction macros (`eval_err!`, etc.) found in:
  - src/atoms_std.rs (extensive use for arity/type/general errors)
  - Possibly other modules (to be confirmed in next batch)

### Identified Error Domains & Patterns
- **General:** parse, macro, validation, IO, malformed AST, internal parse
- **Domain-specific:** eval_arity_error, eval_type_error, eval_general_error, macro expansion errors, atom contract errors
- **Patterns:**
  - Most errors constructed with kind, message, and optional span
  - Macros wrap common error patterns for brevity (to be replaced by helpers)

### Rationale & Protocol
- All error handling must be robust, ergonomic, and span-carrying, with clear, actionable messages
- Centralize all error construction in error.rs using ergonomic, domain-specific and general helpers
- Remove all error macros and direct struct construction outside error.rs
- Enforce via CI/lint and update all documentation and onboarding
- Batch-based, test-driven modernization: after each batch, re-run tests and update memory bank

### Next Steps
- Implement constructor helpers in error.rs (Step B)
- Remove macros and update call sites (Step C)
- Update tests and documentation after each batch

**See also:** docs/architecture/error-refactor-plan.md, memory-bank/progress.md, memory-bank/systemPatterns.md, memory-bank/projectPatterns.md

# [2024-07-07] Error Handling Refactor Progress Update

## Summary

- **Preparation, constructor helpers, macro removal, and call site migration (esp. src/atoms_std.rs) are complete.**
- All findings, rationale, and protocol are documented in error-refactor-plan.md and the memory bank.
- **Outstanding issues:**
  - Duplicate import of `Span` in `src/error.rs` (remove one)
  - Missing imports for error helpers in `src/eval.rs` (add `recursion_depth_error`, `eval_arity_error`, `eval_type_error`, `eval_general_error`)
  - Unused imports in `src/eval.rs` and `src/parser.rs` (remove `EvalError`, `SutraErrorKind`, etc.)
  - Incorrect use of `Default::default()` in error helpers in `src/error.rs` (remove or replace)
  - Trait implementation missing for `EvalError` (implement `Display` or use `{:?}` in `src/lib.rs`)
  - Type mismatch in closure arguments in `src/lib.rs` (ensure error types are consistent)
- See error-refactor-plan.md for full plan, status, and next steps.
- See progress.md for project-wide status and blockers.

---

## [2025-07-07] Debugging and Validator Refactor Context

### Validator Registry Refactor (Critical Blocker)
- **Issue:** Integration test failures persist due to the validator not recognizing atoms (e.g., `core/print`) that are present in the canonical atom registry but not in macro registries.
- **Root Cause:** The validator currently only checks macro registries (`user_macros`, `core_macros`) and does not receive or use the canonical atom registry.
- **Required Change:** Refactor the validator to always receive and use the canonical atom registry, passed from the main code path. The validator must check the atom registry for symbols not found in macro registries before reporting errors.
- **Rationale:** This aligns with the registry pattern (see `systemPatterns.md` and `error-refactor-plan.md`), ensures test/production parity, and resolves the current integration test blocker.
- **Status:** Debugging confirmed that macro expansion and atom registration are correct; the validator is the point of failure. Refactor is now top priority.
- **Next Steps:**
  1. Refactor validator and all call sites to accept and use the atom registry.
  2. Update tests and documentation.
  3. Re-run integration tests to confirm resolution.

## [2024-07-07] Registry Invariant Regression Test
- The single source of truth atom registry invariant is now enforced by a dedicated regression test (`test/echo`).
- All pipeline stages are guaranteed to use the canonical registry.

## [2024-07-07] Registry Invariant and Output Pipeline Complete
- Registry invariant is fully enforced and tested; output emission is correct and robust.
- All integration tests pass.

## [2024-07-08] Final State: Error Handling & Registry Invariant
- Error handling refactor complete: all helpers used, all direct struct construction removed.
- All tests pass with and without test-atom feature.
- CI runs both test modes to enforce registry invariant.
- Registry invariant is enforced and regression-tested.
- All requirements from error-refactor-plan.md are satisfied.

# [2024-07-08] Error Handling Refactor & Linter/Clippy Cleanup Complete

- All outstanding linter and clippy warnings/errors have been resolved.
- All error handling is now routed through ergonomic, documented helpers; no direct struct construction or macros remain outside error.rs.
- All function signatures and test runners have been updated for protocol compliance.
- The full test suite passes with all features enabled.
- The codebase is now fully compliant with the error-refactor plan, registry invariant, and Rust protocol.
- See progress.md for project-wide status and blockers.

---

# 2025-07-07: Macro/Atom Registry, Test System, and Rust Idiom Audit Complete

- Macro/atom registry and test system are now fully Rust-idiomatic, with explicit anti-nesting audits and iterator combinator refactors complete.
- Feature-gated (`test-atom`) and debug-assertion-based test atom registration is in place; integration tests that require test-only atoms are now feature-gated and optional.
- Protocol for feature-gated/optional integration tests is documented in systemPatterns.md.
- All code, tests, and documentation are up to date and compliant as of this session.

# FILE HIERARCHY AND MODULE ORGANIZATION UPDATE (2025-07-07)

## New Modular Structure

- The Rust codebase has been reorganized for maximal modularity and maintainability:
  - `src/syntax/` (parser.rs, cst_parser.rs, error.rs, validate.rs, validator.rs, grammar.pest, mod.rs)
  - `src/ast/` (builder.rs, value.rs, mod.rs)
  - `src/atoms/` (std.rs, mod.rs)
  - `src/macros/` (std.rs, mod.rs)
  - `src/runtime/` (eval.rs, path.rs, registry.rs, world.rs, mod.rs)
  - `src/cli/` (args.rs, output.rs, mod.rs)
  - `src/lib.rs`, `src/main.rs` as entry points
- All directory-based modules use explicit `mod.rs` files (per Rust idiom) except where a single root file suffices.
- God files have been eliminated; each module is focused and minimal.
- Test placement protocol:
  - Inline tests for small modules
  - `tests.rs` only for large/shared test suites
  - Rust integration/unit tests in `tests/rust/`
  - Protocol-compliant integration tests in `tests/scripts/` (Sutra scripts + expected output)

## Rationale
- Prevents god files and over-merging
- Follows modern Rust best practices (explicit root files, minimal use of mod.rs)
- Enables clear ownership, testability, and onboarding
- Supports future growth and modular refactor

## Changelog
- 2025-07-07: Major file hierarchy and module organization refactor. Modular directories created in src/, god files removed, explicit mod.rs usage, and new test organization. All documentation and memory bank files must be updated to reflect this canonical structure.
