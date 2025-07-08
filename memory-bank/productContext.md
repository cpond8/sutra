# Sutra Engine - Product Context

## Why This Project Exists

Sutra addresses fundamental limitations in current game and narrative engines by providing a compositional, transparent, and extensible substrate for interactive systems.

## Problems Being Solved

### 1. Rigid, Inflexible Narrative Systems
- Most engines bake in specific narrative patterns (linear, branching, etc.), making experimentation with emergent or system-driven narratives difficult
- Authors are constrained by engine assumptions and cannot easily extend or combine systems

### 2. Non-Compositional Game Logic
- Features often implemented as monolithic, hard-coded systems, leading to feature bloat and brittle architectures
- Sutra enables novel combinations and extensions through compositional atoms and macros

### 3. Poor Authoring Transparency
- Authors struggle to inspect, debug, or understand how their content is processed
- Sutra prioritizes transparency with inspectable state, macro expansion, and error reporting

### 4. Limited Extensibility
- Adding new features in traditional engines often requires modifying the core or forking the codebase
- Sutra's registry and macro system allow new features without core changes

## Product Goals

- **Empower authors** to build any narrative or simulation system, from interactive fiction to complex emergent worlds
- **Lower the barrier to experimentation** by making all systems compositional and extensible
- **Provide full transparency** into all authoring, debugging, and state changes
- **Enable robust, testable, and maintainable content** through pure functions and immutable data

## User Experience Principles

- **Compositionality**: Authors build from small, orthogonal primitives
- **Transparency**: All computation and state are inspectable and debuggable
- **Extensibility**: New atoms/macros can be added without modifying the core
- **Minimalism**: The engine exposes only what is necessary, avoiding feature bloat
- **Portability**: Works across platforms and frontends

## Test Suite Protocol

**All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted.**

This protocol ensures that all testing validates the actual user experience and maintains transparency principles.

## Architecture Alignment

The current Rust codebase implements these principles through:
- **Modular Architecture**: Clear separation between parsing, macro expansion, evaluation, and output
- **Pure Functions**: All core logic with no global state or hidden side effects
- **Registry-Driven System**: Extensible atom/macro registration without core modifications
- **Transparent Pipeline**: Macro expansion and evaluation fully inspectable and testable

### File Organization (2025-07-07)
- `src/syntax/` - parsing, error handling, validation
- `src/ast/` - AST builder, value types  
- `src/atoms/` - core atom implementations
- `src/macros/` - macro system and standard library
- `src/runtime/` - evaluation, registry, world state
- `src/cli/` - CLI logic, args, output

This structure supports the product goals of transparency, maintainability, and extensibility.

## Cross-References

- See `memory-bank/projectbrief.md` for project vision and target use cases
- See `memory-bank/systemPatterns.md` for architectural patterns supporting these goals
- See `memory-bank/activeContext.md` for current work focus
- See `memory-bank/progress.md` for implementation status
- See `docs/architecture/parsing-pipeline-plan.md` for pipeline architecture

## Changelog

- **2025-07-07**: File hierarchy aligned with product goals, test organization updated
- **2025-07-06**: Integration test runner bootstrapped for protocol-compliant testing
- **2025-07-04**: Parsing pipeline connection to product goals documented
- **2025-07-03**: Product context aligned with current codebase and implementation
