# Sutra Engine - Project Brief

## Project Vision

Sutra aspires to be a **universal substrate for compositional, emergent, and narrative-rich game systems**. The core vision is an engine that enables designers to build everything from interactive fiction to deep simulations from a minimal, maximally compositional core.

## Core Aspirations

- **Model any gameplay or narrative system** via composition of simple parts ("atoms and macros")
- **Enable robust, transparent, and infinitely extensible authoring**
- **Ensure the core is simple enough to be fully understood, yet powerful enough to encode anything**
- Match the spirit of lambda calculus, Lisp, and digital logic minimalism

## Project Status

- **Core implementation in progress** - foundational stages complete.
- Key architectural decisions validated through implementation.
- Built in Rust for performance and type safety.

## Key Design Philosophy

**Minimalism as Power**: Following Scheme/Lisp tradition where a tiny set of core forms serves as the basis for an expressive, extensible, and Turing-complete language.

**Atoms and Macros Model**:

- **Atoms**: Truly irreducible "micro-operations" (queries, mutations, control flow, output, randomness)
- **Macros**: All higher-level language constructs built as macros that expand to atoms and other macros

## Target Use Cases

- Interactive fiction and narrative games
- Quality-based narrative (QBN) systems
- Storylet-driven content
- Agent-based simulations
- Emergent gameplay systems
- Educational/experimental game development

## Success Criteria

- **Expressiveness**: Can encode all major gameplay/narrative patterns (QBN, storylets, threads, etc.)
- **Compositionality**: All features are built from a small set of orthogonal primitives
- **Transparency**: All authoring, debugging, and state are fully inspectable
- **Extensibility**: New atoms/macros can be added without modifying the core
- **Performance**: Sub-millisecond evaluation for typical storylet/world updates
- **Portability**: Runs on all major platforms (macOS, Linux, Windows, WASM)

## Alignment with Current Codebase

- The engine is implemented in Rust for safety, performance, and cross-platform support.
- The architecture is strictly modular, with a clear separation between parsing, macro expansion, evaluation, and output.
- All core logic is implemented as pure functions, with no global state or hidden side effects.
- The macro system is the primary mechanism for extensibility and author ergonomics.
- The registry pattern is used for atoms and macros, ensuring a single source of truth and test/production parity.

## Cross-References

- See `memory-bank/productContext.md` for product rationale and user needs.
- See `memory-bank/systemPatterns.md` for architectural and design patterns.
- See `memory-bank/techContext.md` for technical stack and constraints.
- See `memory-bank/activeContext.md` for current work focus and priorities.
- See `memory-bank/progress.md` for completed work and next steps.
- See `.cursor/rules/memory-bank.mdc` for update protocol and overlays.

## Changelog

- 2025-07-03: Updated to resolve all audit TODOs, clarify vision, and align with current codebase and guidelines.
- 2025-06-30: Initial synthesis from legacy documentation.

## Current Development Phase

**Architectural Refactoring (Completed)**: A major overhaul was completed to introduce a canonical `Path`