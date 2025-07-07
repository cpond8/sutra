# Sutra Engine - Product Context

## Why This Project Exists

Sutra addresses fundamental limitations in current game and narrative engines by providing a compositional, transparent, and extensible substrate for interactive systems.

### Problems Being Solved

1. **Rigid, Inflexible Narrative Systems**
   - Most engines bake in specific narrative patterns (linear, branching, etc.), making it difficult to experiment with emergent or system-driven narratives.
   - Authors are constrained by engine assumptions and cannot easily extend or combine systems.

2. **Non-Compositional Game Logic**
   - Features are often implemented as monolithic, hard-coded systems, leading to feature bloat and brittle architectures.
   - Sutra enables novel combinations and extensions through compositional atoms and macros.

3. **Poor Authoring Transparency**
   - Authors struggle to inspect, debug, or understand how their content is processed.
   - Sutra prioritizes transparency, with inspectable state, macro expansion, and error reporting.

4. **Limited Extensibility**
   - Adding new features in traditional engines often requires modifying the core or forking the codebase.
   - Sutra's registry and macro system allow new features to be added without core changes.

## Product Goals

- **Empower authors** to build any narrative or simulation system, from interactive fiction to complex emergent worlds.
- **Lower the barrier to experimentation** by making all systems compositional and extensible.
- **Provide full transparency** into all authoring, debugging, and state changes.
- **Enable robust, testable, and maintainable content** through pure functions and immutable data.
- **All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted. A full test suite rewrite is required. See `memory-bank/README.md` and `memory-bank/activeContext.md`.**
- **Integration Test Runner Bootstrapped (2025-07-06):**
  - `tests/scripts/` directory created for protocol-compliant integration tests.
  - First `.sutra` test script (`hello_world.sutra`) and expected output (`hello_world.expected`) added. See `activeContext.md` and `progress.md`.

## User Experience Principles

- **Compositionality**: Authors build from small, orthogonal primitives.
- **Transparency**: All computation and state are inspectable and debuggable.
- **Extensibility**: New atoms/macros can be added without modifying the core.
- **Minimalism**: The engine exposes only what is necessary, avoiding feature bloat.
- **Portability**: Works across platforms and frontends.

## Alignment with Current Codebase

- The codebase implements these principles through a modular Rust architecture, pure functions, and a registry-driven macro/atom system.
- Macro expansion and evaluation are fully transparent and testable.
- The CLI and documentation are designed for author ergonomics and onboarding.

## Parsing Pipeline and Product Goals (2025-07-04)

The new modular parsing pipeline directly supports Sutra's product goals:
- **Transparency:** Every stage is explicit, auditable, and debuggable, making authoring and debugging easier for users.
- **Extensibility:** The architecture is designed for easy extension and evolution, supporting new syntax, macros, and validation features.
- **Authoring Ergonomics:** Clear error reporting, diagnostics, and round-trippability ensure a smooth authoring experience.

See `docs/architecture/parsing-pipeline-plan.md` for the full plan and rationale.

## Cross-References

- See `memory-bank/projectbrief.md` for project vision and aspirations.
- See `memory-bank/systemPatterns.md` for architectural and design patterns.
- See `memory-bank/techContext.md` for technical stack and constraints.
- See `memory-bank/activeContext.md` for current work focus and priorities.
- See `memory-bank/progress.md` for completed work and next steps.
- See `.cursor/rules/memory-bank.mdc` for update protocol and overlays.

## Changelog

- 2025-07-03: Updated to resolve all audit TODOs, clarify product context, and align with current codebase and guidelines.
- 2025-06-30: Initial synthesis from legacy documentation.
- 2025-07-04: Added section on parsing pipeline and product goals.
- 2025-07-06: Batch refactor for Rust idiom compliance (implicit/explicit return style), match exhaustiveness, and error handling. Explicit returns for early exits restored. All match arms for Expr variants in eval_expr restored. Protocol-driven, batch-based, test-first approach enforced. All tests pass. Lesson: Always enumerate all functions for audit, not just those surfaced by search.
- 2025-07-07: Macro/atom registry and test system are now fully Rust-idiomatic, with anti-nesting audits and iterator combinator refactors complete. Feature-gated (test-atom) and debug-assertion-based test atom registration is in place; integration tests that require test-only atoms are now feature-gated and optional. Protocol for feature-gated/optional integration tests is documented in systemPatterns.md. All code, tests, and documentation are up to date and compliant as of this session.
