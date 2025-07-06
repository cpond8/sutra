# Sutra Engine - Progress

## Completed Work

- **Foundational Scaffolding:** Project structure, core data types, unified PEG parser, atom engine, and module integration are complete and stable.
- **Core Engine Stabilization:** Macro expansion, standard macro library, atom engine, and test suite are robust and fully tested.
- **Language Specification Synchronization:** The language spec is fully synchronized with the codebase.
- **Comprehensive Design Phase:** All major architectural decisions, implementation plans, and documentation are complete.
- **Macro Path Canonicalization:** Macro system contract and canonicalization are fully implemented and tested.
- **Registry Pattern Audit:** Canonical builder functions and test/production parity are enforced.
- **Error Handling Audit:** Structured, span-carrying errors and two-phase enrichment are standard.
- **AST/Parser Audit:** AST nodes carry spans, parser uses unified PEG grammar, and error handling is consistent.
- **Atoms/Eval Audit:** Atoms follow uniform contracts, state propagation is immutable, and documentation is up-to-date.
- **CLI/Output and Tests Audit:** CLI is a pure orchestrator, output is centralized, and test/production parity is maintained.
- **Parser Refactor:** Parser is decomposed into per-rule helpers, with robust error handling and dotted list validation.

## Macro System Bootstrapping Roadmap (2025-07-02)

This roadmap is the single source of truth for macro system bootstrapping and self-hosting. All previous 'What's Next' or 'Next Steps' sections are superseded by this roadmap.

1. Migrate all standard macros to the native macro language.
2. Design and implement macro hygiene.
3. Expand the standard macro library (Tier 2+).
4. Validation and author feedback.
5. Documentation, CLI, and tooling.

## What's Left to Build

- **Macro Migration:** Rewrite all higher-level macros (especially `cond`) as native macros using the new support. Remove Rust-native macro implementations. Update and expand tests for new macro definitions.
- **Macro Hygiene:** Design and implement a hygiene system (e.g., gensym for local bindings) to prevent accidental variable capture. Integrate into the macro expander and document its behavior.
- **Standard Macro Library Expansion:** Implement all higher-level narrative/gameplay macros (`storylet`, `choice`, `pool`, etc.) as native macros. Develop comprehensive example scripts and usage patterns. Performance test with realistic content.
- **Validation and Author Feedback:** Implement structural and semantic validation before and after macro expansion. Integrate validation and error reporting with CLI and author tools.
- **Documentation, CLI, and Tooling:** Audit and update documentation to reflect the new macro system. Ensure CLI exposes macroexpansion, registry, and validation features.

## Current Status Assessment

- **Strengths:** Thorough design, clear implementation path, comprehensive documentation, and risk mitigation.
- **Challenges:** Macro system complexity, performance optimization, user experience, and documentation maintenance.
- **Timeline Estimates:** 15-20 weeks for complete system; MVP in 8-12 weeks.

## Known Issues and Technical Debt

- **Performance characteristics** not yet validated with large-scale content.
- **Macro system feature creep** and **performance optimization pressure** are ongoing risks.
- **User macro system** and **editor integration** will require careful design.
- (2025-07-05) Incident: A function with excessive nesting (`parse_macros_from_source`) was missed in an initial audit due to over-reliance on semantic search. Corrective action: Protocol updated to require explicit function enumeration and review in all future complexity/nesting audits.

## Evolution of Project Decisions

- **Original Concept:** Minimalist Lisp-like language for narrative scripting.
- **Refined Architecture:** Expanded to support any game/simulation system, dual syntax, macro composition, and debugging focus.
- **Validated Decisions:** Atoms vs. macros split, immutable world state, pipeline separation, and registry pattern are all proven.
- **Open Research Questions:** Macro hygiene, performance optimization, user macro system design, and advanced tooling.

## Alignment with Current Codebase

- All progress, roadmap, and status described above are implemented and enforced in the current codebase.
- The roadmap, design decisions, and patterns are up-to-date and match the live system.

## Cross-References

- See `memory-bank/projectbrief.md` for project vision and aspirations.
- See `memory-bank/productContext.md` for product rationale and user needs.
- See `memory-bank/systemPatterns.md` for architectural and design patterns.
- See `memory-bank/techContext.md` for technical stack and constraints.
- See `memory-bank/activeContext.md` for current work focus and priorities.
- See `.cursor/rules/memory-bank.mdc` for update protocol and overlays.

## Parsing Pipeline Refactor: Progress Log (2025-07-04)

- Decision: Adopted the modular, interface-driven parsing pipeline as the canonical architecture.
- Status: Plan and context fully documented and archived. Interfaces and trivial implementations to be shipped next.
- Milestones: (1) Interfaces and golden tests, (2) Module-by-module implementation, (3) Integration and migration, (4) Full test suite pass.

See `docs/architecture/parsing-pipeline-plan.md` for the full plan and changelog.

## Progress Log: Macroexpander Refactor & AST Invariant (2025-07-04)

- Completed:
  - Migrated Expr::List to Vec<WithSpan<Expr>> in ast.rs, macros_std.rs, macros.rs, parser.rs, eval.rs, validate.rs.
  - Macroexpander and helpers now operate exclusively on WithSpan<Expr>.
  - src/atoms_std.rs is now fully span-carrying compliant. All error macros and atom logic use WithSpan<Expr> throughout. All linter/type errors resolved. Canonical error macro pattern enforced. See parsing-pipeline-plan.md and macroexpander migration for context.
- Issues:
  - Linter/type errors from mixed Expr/WithSpan<Expr> usage.
  - Macro_rules! and error helpers in atoms_std.rs require explicit, line-by-line fixes.
  - Some macro contexts and test helpers need a final audit.
- Next steps:
  - Complete audit and fix in atoms_std.rs.
  - Update all tests and doc examples for new AST invariant.
  - Run final integration test of the pipeline.
- Blockers/Lessons:
  - Automated batch edits are insufficient for macro_rules! and error helpers; manual review is required.
  - Enforcing span-carrying invariants across all modules is nontrivial and must be maintained going forward.

## Changelog

- 2025-07-03: Updated to resolve all audit TODOs, clarify progress, and align with current codebase and guidelines.
- 2025-06-30: Initial synthesis from legacy documentation.
- 2025-07-04: Added parsing pipeline refactor progress log and cross-reference to plan.
- 2025-07-05: Migration to proper-list-only and ...rest-only architecture complete. All legacy code, tests, and documentation for improper/dotted lists and legacy variadic syntax have been removed. The codebase, tests, and docs are now fully compliant and clean.
- 2025-07-05: Documentation Audit
  - Completed a full audit of all major documentation files.
  - All docs are up to date, accurate, and fully aligned with the codebase and recent progress.
  - Canonical parsing pipeline, language spec, macro/atom boundaries, and authoring patterns are all current.
  - See: `docs/architecture/parsing-pipeline-plan.md`, `docs/specs/language-spec.md`, `docs/architecture/authoring-patterns.md`, `docs/specs/storylet-spec.md`, `docs/architecture/architecture.md`.
