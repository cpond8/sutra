# Sutra Engine - Memory Bank

## Purpose

The memory bank is the canonical, living knowledge base for the Sutra project. It encodes all project context, design decisions, architectural patterns, technical constraints, and current work status. It is the single source of truth for onboarding, planning, and all major decisions.

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

## Changelog

- 2025-07-03: Updated to resolve all audit TODOs, clarify structure, and align with current codebase and guidelines.
- 2025-06-30: Initial synthesis from legacy documentation.
