# Sutra Engine – Project Patterns (Living Intelligence Log)

## Purpose

This file is the canonical, living intelligence log for the Sutra project. It records project-specific heuristics, workflow preferences, patterns, lessons learned, recurring challenges, and meta-decisions. It is updated frequently as new insights or patterns emerge and is the authoritative replacement for the deprecated `.cursorrules` file.

## Heuristics and Preferences
- Use the registry pattern for all extensible components (atoms, macros, etc.).
- Enforce pure function architecture; avoid global state and hidden side effects.
- Prefer batch-based, test-driven modernization for all core system changes.
- Maintain strict test/production parity: all registry, loader, and macro expansion logic must be identical in both environments.
- Document all new patterns, preferences, and workflow intelligence immediately after discovery.

## Patterns and Lessons Learned
- Macro system and parser must be updated in lockstep for all syntax or AST changes.
- All error handling must be robust, contextual, and span-aware, with error messages matching the canonical spec and test suite.
- Registry, macro system, and error handling must be fully aligned with systemPatterns.md.
- Use batch-based, test-driven development to isolate and resolve failures incrementally.
- Maintain strict separation of concerns: parser only parses, macro loader only loads, macro expander only expands.
- All error messages must be clear, actionable, and span-carrying.
- Update documentation and memory bank files after every significant change.
- Lesson (2025-07-05): Automated or semantic search-based code audits are insufficient for never-nester and complexity checks. Always enumerate and check every function in a file for excessive nesting or complexity, not just those surfaced by search. Supplement search with pattern-based grep for function definitions and manual review for large or critical files.
- **Test Suite Protocol:** All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted. A full test suite rewrite is required. See `memory-bank/README.md` and `memory-bank/activeContext.md`.

## Recurring Challenges
- Ensuring parser, grammar, loader, and tests are updated together when making changes to core syntax or AST structure.
- Maintaining span-carrying invariants across all modules.
- Enforcing canonicalization contracts and registry patterns in all environments.
- Preventing drift between test and production registries.

## Meta-Decisions
- projectPatterns.md is the single source of truth for project heuristics, preferences, and workflow intelligence.
- All overlays and additional context files must be strictly additive and never conflict with core files.
- All contributors must review and update this file after any significant change, decision, or insight.

## Cross-References
- See `memory-bank/projectbrief.md` for project vision and goals.
- See `memory-bank/systemPatterns.md` for architectural and design patterns.
- See `memory-bank/activeContext.md` for current work focus and priorities.
- See `.cursor/rules/memory-bank.mdc` for update protocol and overlays.

## Changelog
- 2025-07-05: Initialized projectPatterns.md as the canonical living intelligence log, replacing .cursorrules.

## 2025-07-05: Documentation Audit

- Confirmed that all major documentation is up to date, accurate, and fully aligned with the codebase and recent progress.
- Canonical parsing pipeline, language spec, macro/atom boundaries, and authoring patterns are all current.
- See: `docs/architecture/parsing-pipeline-plan.md`, `docs/specs/language-spec.md`, `docs/architecture/authoring-patterns.md`, `docs/specs/storylet-spec.md`, `docs/architecture/architecture.md`.

## Macro System Patterns

- Adopt helper-driven, guard-first decomposition for macro expansion logic (e.g., `expand_template`).
    - Each logical step (arity check, parameter binding, substitution) should be a named helper.
    - All error cases must be explicit and robust.
- For major architectural changes (layered/provenance macro system), always prototype in a separate branch after incremental improvements are validated.

## Pattern: Layered, Provenance-Aware Macro System

- Use a layered macro registry (core, stdlib, user, scenario) to support modularity and shadowing.
- Attach provenance metadata (origin, author, file, line) to all macro definitions and expansions.
- Expansion context should include provenance, hygiene, and layer for advanced features.
- Record and expose expansion traces for debugging and auditing.
- Prototype radical changes in a separate branch after incremental improvements are validated.

## 2025-07-05: Macro System, CLI, and Test Harness Refactor (Session Patterns)

- Batch-based, test-driven modernization is the standard for all core system changes.
- Recursion depth enforcement is now explicit and robust in macro expansion.
- All legacy code and tests must be removed or updated immediately after refactor.
- Protocol requires immediate documentation and memory bank updates after significant changes.

## Meta-Decision
- This session's work sets a precedent for future refactors: all architectural, CLI, and test changes must be reflected in both code and documentation, with protocol-driven audits.

## Changelog
- 2025-07-05: Macro system, CLI, and test harness refactor patterns and meta-decision added.
- 2025-07-06: Batch refactor for Rust idiom compliance (implicit/explicit return style), match exhaustiveness, and error handling. Explicit returns for early exits restored. All match arms for Expr variants in eval_expr restored. Protocol-driven, batch-based, test-first approach enforced. All tests pass. Lesson: Always enumerate all functions for audit, not just those surfaced by search.
- 2025-07-06: Macro system helpers (arity, binding, expansion, duplicate check) refactored for protocol compliance: pure, linear, early-return, and fully documented. Protocol-driven audit and batch-based, test-driven modernization enforced.

## Native-Language Test Suite Blockers (2025-07-06)

See activeContext.md and progress.md for the canonical, timestamped list of explicit gaps blocking a fully native-language (s-expr and brace/block) test suite. All blockers must be resolved before the test suite rewrite can proceed.

## Prioritized Action Plan: Native-Language Test Suite Blockers (2025-07-06)

See activeContext.md and progress.md for the canonical, dependency-ordered plan for resolving all blockers to a fully native-language (s-expr and brace/block) test suite. Each step must be completed before the next can proceed. See system-reference.md for architectural rationale.