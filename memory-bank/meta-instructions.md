# Copilot Meta-Instructions

## Memory Bank Maintenance

- At the start of every session or complex task, **read ALL memory bank files** to refresh on project context, goals, and recent decisions. Treat the memory bank as the single source of truth for the project's state and history.
- **Update the memory bank** diligently whenever significant changes are made, new insights are gained, or when the user requests an update. Focus especially on `activeContext.md` and `progress.md` for tracking current work and status.
- When updating, review every memory bank file for accuracy and consistency. Do not duplicate information; always centralize and deduplicate state or logic.
- If the memory bank is missing or incomplete, create or repair the necessary files immediately.
- Document all new patterns, architectural decisions, and learnings in the appropriate file. Use clear, concise language and maintain the established structure.

## Design Philosophy Adherence

- **Always abide by the Sutra Engine's design philosophy** as documented in the memory bank and design docs:
  - Favor pure functions and immutability
  - Maintain a single source of truth
  - Strive for minimalism and simplicity
  - Decompose systems into modular, loosely coupled components
  - Prefer composition over inheritance
  - Enforce separation of concerns
- Before writing or refactoring code, clarify the high-level design and how the component fits into the overall system. Ensure consistency with the project's architecture and long-term vision.
- Avoid quick hacks or solutions that introduce duplication or misalignment with the broader design. Every change or feature should improve or maintain the codebase's organization.
- Leverage all available tools and context (memory bank, sequential thinking, up-to-date documentation, knowledge graph) to maximize effectiveness and maintain architectural integrity.

## Documentation Process

- When triggered by **update memory bank**, review every memory bank file, even if some do not require updates. Focus on `activeContext.md` and `progress.md` for current state.
- When context needs clarification, document the current state, clarify next steps, and record insights and patterns.
- Treat the memory bank as a living, evolving record. Keep it precise, clear, and up to date at all times.

*Last updated: 2025-06-30*
