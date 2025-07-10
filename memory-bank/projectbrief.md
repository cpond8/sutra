# Sutra Engine - Project Brief

## Project Vision

Sutra is a universal substrate for compositional, emergent, and narrative-rich game systems. Build everything from interactive fiction to deep simulations from a minimal, maximally compositional core.

## Core Aspirations

- Model any gameplay or narrative system via composition of simple parts
- Enable robust, transparent, and infinitely extensible authoring
- Core simple enough to be fully understood, yet powerful enough to encode anything
- Match the spirit of lambda calculus, Lisp, and digital logic minimalism

## Key Design Philosophy

**Minimalism as Power**: Following Scheme/Lisp tradition where a tiny set of core forms serves as the basis for an expressive, extensible, and Turing-complete language.

**Atoms and Macros Model**: Atoms are irreducible micro-operations, macros are higher-level constructs built from atoms and other macros.

## Target Use Cases

- Interactive fiction and narrative games
- Quality-based narrative (QBN) systems
- Storylet-driven content
- Agent-based simulations
- Emergent gameplay systems
- Educational/experimental game development

## Success Criteria

- **Expressiveness**: Can encode all major gameplay/narrative patterns
- **Compositionality**: All features built from small set of orthogonal primitives
- **Transparency**: All authoring, debugging, and state fully inspectable
- **Extensibility**: New atoms/macros without core modifications
- **Performance**: Sub-millisecond evaluation for typical updates
- **Portability**: Runs on all major platforms (macOS, Linux, Windows, WASM)
