# Sutra Engine - Product Context

## Problems Being Solved

**Rigid Narrative Systems**: Current engines bake in specific narrative patterns, constraining authors to predetermined structures.

**Non-Compositional Game Logic**: Features implemented as monolithic systems, leading to feature bloat and brittle architectures.

**Poor Authoring Transparency**: Authors cannot inspect, debug, or understand content processing.

**Limited Extensibility**: Adding features requires core modifications or forking.

## Product Goals

- Empower authors to build any narrative or simulation system
- Lower barrier to experimentation through compositional systems
- Provide full transparency into authoring, debugging, and state changes
- Enable robust, testable, maintainable content through pure functions

## User Experience Principles

- **Compositionality**: Build from small, orthogonal primitives
- **Transparency**: All computation and state inspectable
- **Extensibility**: Add atoms/macros without core modifications
- **Minimalism**: Engine exposes only necessary features
- **Portability**: Works across platforms and frontends
