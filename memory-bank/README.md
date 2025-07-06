# Sutra Engine - Memory Bank

> **MAXIMALLY IMPORTANT: Throughout this project, “memory bank” ALWAYS refers to the files in the `memory-bank/` directory (as described in `memory-bank.mdc`). It NEVER refers to any built-in, agent, or AI memory. All project context, decisions, and updates MUST be recorded here, not in any external or ephemeral memory.**

## Purpose

The memory bank is the canonical, living knowledge base for the Sutra project. It encodes all project context, design decisions, architectural patterns, technical constraints, and current work status. It is the single source of truth for onboarding, planning, and all major decisions.

## TEST SUITE PROTOCOL & REQUIRED REWRITE

> **All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted.**
>
> **A full test suite rewrite is required to comply with this protocol.**

## Structure

The memory bank consists of the following files:

- `projectbrief.md`: Project vision, goals, and high-level context
- `productContext.md`: Product rationale, user needs, and market context
- `systemPatterns.md`: Architectural and design patterns, system-wide decisions
- `techContext.md`: Technical stack, constraints, and rationale
- `activeContext.md`: Current work focus, priorities, and open questions
- `progress.md`: Completed work, current status, and next steps

## Update Protocol

- All files must be updated after any significant change, decision, or insight.
- The update process is strictly defined in `.cursor/rules/memory-bank.mdc`.
- All changes must be versioned and logged in the changelog sections.
- No undocumented or ambiguous changes are permitted.

## Cross-References

- See `docs/architecture/architecture.md` for high-level system architecture.
- See `docs/specs/language-spec.md` for canonical language specification.
- See `system-reference.md` for detailed system reference and rationale.
- See `.cursor/rules/memory-bank.mdc` for update protocol and overlays.
- See `.cursorrules` (to be initialized) for project patterns, preferences, and workflow intelligence.

## Usage

- All contributors must read and understand the memory bank before making changes.
- All planning, design, and implementation must reference the memory bank as the source of truth.
- Any new patterns, preferences, or workflow intelligence must be added to `.cursorrules`.

## Parsing Pipeline Plan (2025-07-04)

- The canonical plan and context for the modular parsing pipeline refactor are archived in `docs/architecture/parsing-pipeline-plan.md`.
- All contributors must review this document before working on parser or macro system code.

## 2025-07-05: Milestone Update
- Macro system, CLI, and test harness refactor completed. Memory bank and documentation are current and protocol-compliant as of this session.

## Changelog

- 2025-07-03: Updated to resolve all audit TODOs, clarify structure, and align with current codebase and guidelines.
- 2025-06-30: Initial synthesis from legacy documentation.
- 2025-07-04: Added reference to parsing pipeline plan and archival location.
- 2025-07-05: Milestone and protocol compliance update for macro system, CLI, and test harness refactor.
