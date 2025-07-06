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