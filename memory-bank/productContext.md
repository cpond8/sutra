# Sutra Engine - Product Context

## Why This Project Exists

Sutra addresses fundamental limitations in current game and narrative engines:

### Problems Being Solved

**1. Rigid, Inflexible Narrative Systems**
- Most engines bake in specific narrative patterns (linear, branching, etc.)
- Difficult to experiment with emergent or system-driven narratives
- Authors constrained by engine assumptions

**2. Non-Compositional Game Logic**
- Features implemented as monolithic, hard-coded systems
- Cannot combine or extend systems in novel ways
- "Feature bloat" makes engines complex and brittle

**3. Poor Authoring Transparency**
- Authors can't inspect or debug underlying engine logic
- Macro systems are opaque or non-existent
- Difficult to understand why content behaves in certain ways

**4. Limited Extensibility**
- Adding new features requires engine modifications
- No clear path for user-defined constructs or patterns
- Coupling between content and engine implementation

## How Sutra Should Work

### For Game Designers and Authors

**Clean, Accessible Syntax**
- Choice between brace-block (familiar) and s-expression (powerful) syntax
- One-to-one mapping ensures no loss of structure or semantics
- Auto-resolution of values eliminates boilerplate

**Compositional Building Blocks**
- Start with simple atoms for basic operations
- Compose macros for complex narrative patterns
- Build custom constructs without touching engine code

**Full Transparency**
- Inspect macro expansions at any level
- Debug world state changes step by step
- Understand exactly why content fired or didn't fire

### For Engine Developers

**Minimal, Stable Core**
- Small set of irreducible operations (atoms)
- All features built as composable macros
- No privileged or hard-coded game logic

**Pure, Testable Architecture**
- Strict separation between parsing, expansion, validation, and evaluation
- Immutable world state with explicit change tracking
- Every component testable in isolation

## User Experience Goals

### Immediate Experience (First Hour)
- Author can write simple storylets and choices
- Clear error messages guide towards correct syntax
- Macro expansion helps understand what's happening

### Intermediate Experience (First Week)
- Compose complex narrative patterns from simple building blocks
- Debug failing conditions with macro expansion traces
- Create custom helper macros for common patterns

### Advanced Experience (First Month)
- Build sophisticated emergent systems
- Extend engine with domain-specific constructs
- Understand and modify the entire system

## Target Audiences

**Primary**: Independent game developers and interactive fiction authors who want maximum creative flexibility

**Secondary**: Educational users learning about programming language design and compositional systems

**Tertiary**: Researchers exploring emergent narrative and agent-based storytelling

## Success Metrics

- Authors can implement any Emily Short QBN pattern as macros
- New narrative techniques can be prototyped without engine changes
- System is learnable and teachable to non-programmers
- Performance scales to substantial games (thousands of entities/storylets)

*Last Updated: 2025-06-30*
