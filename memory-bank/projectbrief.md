# Sutra Engine - Project Brief

## Project Vision

Sutra aspires to be a **universal substrate for compositional, emergent, and narrative-rich game systems**. The core vision is an engine that enables designers to build everything from interactive fiction to deep simulations from a minimal, maximally compositional core.

## Core Aspirations

- **Model any gameplay or narrative system** via composition of simple parts ("atoms and macros")
- **Enable robust, transparent, and infinitely extensible authoring**
- **Ensure the core is simple enough to be fully understood, yet powerful enough to encode anything**
- Match the spirit of lambda calculus, Lisp, and digital logic

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

1. **Turing Completeness**: Support any computable gameplay or narrative system
2. **Compositionality**: All features beyond atoms can be composed as macros
3. **Transparency**: All computation is inspectable and debuggable down to atom level
4. **Extensibility**: Authors can define new constructs without engine changes
5. **Determinism**: All runs are reproducible given same world state and code

## Current Development Phase

**Stages 0-3**: Core Engine and Primitives (completed)
**Next**: Stage 4 - Macro System implementation

*Last Updated: 2025-06-30*
