# Sutra Engine - Project Brief

## Project Vision

Sutra aspires to be a **universal substrate for compositional, emergent, and narrative-rich game systems**. The core vision is an engine that enables designers to build everything from interactive fiction to deep simulations from a minimal, maximally compositional core.

## Core Aspirations

- **Model any gameplay or narrative system** via composition of simple parts ("atoms and macros")
- **Enable robust, transparent, and infinitely extensible authoring**
- **Ensure the core is simple enough to be fully understood, yet powerful enough to encode anything**
- Match the spirit of lambda calculus, Lisp, and digital logic minimalism

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
- **Compositionality**: All features built from a small set of orthogonal primitives
- **Transparency**: All authoring, debugging, and state fully inspectable
- **Extensibility**: New atoms/macros can be added without modifying the core
- **Performance**: Sub-millisecond evaluation for typical storylet/world updates
- **Portability**: Runs on all major platforms (macOS, Linux, Windows, WASM)

## Test Suite Protocol

**All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted.**

**A full test suite rewrite is required to comply with this protocol.**

## Current Architecture

### Core Implementation
- Built in Rust for safety, performance, and cross-platform support
- Strictly modular architecture with clear separation: parsing → macro expansion → evaluation → output
- All core logic implemented as pure functions with no global state
- Registry pattern for atoms and macros ensuring single source of truth

### File Organization (2025-07-07)
Modular directory structure:
- `src/syntax/` - parsing, error handling, validation
- `src/ast/` - AST builder, value types
- `src/atoms/` - core atom implementations
- `src/macros/` - macro system and standard library
- `src/runtime/` - evaluation, registry, world state
- `src/cli/` - CLI logic, args, output

### Test Organization
- Rust integration/unit tests: `tests/rust/`
- Protocol-compliant integration tests: `tests/scripts/` (`.sutra` scripts + expected output)
- Debug infrastructure: `debug/macro-testing/` for systematic investigation

## Strategic Architecture

### Parsing Pipeline (2025-07-07)
A modular, interface-driven parsing pipeline designed for:
- Long-term maintainability and contributor onboarding
- Robust, testable, and auditable parsing and macroexpansion
- Alignment with core values: compositionality, transparency, extensibility
- Foundation for future features (editor integration, diagnostics, incremental parsing)

### Current Status
- **Core Infrastructure**: Complete and functional
- **Native .sutra File Loading**: ~85% complete with one critical blocker
- **Macro System**: Robust implementation, user-defined macro integration in progress
- **Test Suite**: Transitioning to protocol-compliant `.sutra` scripts

## Cross-References

- See `memory-bank/activeContext.md` for current work focus and critical blockers
- See `memory-bank/progress.md` for completed work and roadmap
- See `memory-bank/systemPatterns.md` for architectural patterns
- See `memory-bank/techContext.md` for technical implementation details
- See `docs/architecture/parsing-pipeline-plan.md` for pipeline architecture

## Changelog

- **2025-07-07**: File hierarchy reorganized for modularity, test organization updated
- **2025-07-06**: Integration test runner bootstrapped, protocol-compliant tests established
- **2025-07-04**: Parsing pipeline plan established as strategic architecture direction
- **2025-07-03**: Project brief aligned with current codebase and guidelines
