# Sutra Engine - Active Context

## Current Work Focus

**Phase**: Stage 4 Implementation (Macro System)
**Priority**: Implementing the core macro expansion logic.

### Recent Changes (2025-07-01)
- **Began Stage 4: Macro System**:
  - **Created `src/macro.rs`**: Established the foundational structures for the macro system, including `MacroFn` and `MacroRegistry`.
  - **Created `src/macros_std.rs`**: Added the placeholder module for the standard macro library.
  - **Integrated Modules**: Declared the new modules in `src/lib.rs`.
- **Completed Stage 3: Atom Engine**:
  - **Full Tier 1 Atom Set**: Implemented the complete standard library of atoms in `src/atoms_std.rs`.
  - **Auto-Get Implemented**: Modified `src/eval.rs` to automatically resolve symbols.
  - **Path Bug Fixed**: Corrected path handling in `set!`, `get!`, and `del!`.
  - **Comprehensive Test Suite**: Created `tests/core_eval_tests.rs` with full coverage for Stages 0-3.

## Next Steps (Immediate Priority)

1.  **Implement Macro Expansion Logic**:
    - Flesh out the `expand_macros` function in `src/macro.rs`. This involves recursively traversing the AST and applying transformations for any registered macros.
    - Implement recursion depth limiting to prevent infinite loops.
2.  **Implement Standard Macros**:
    - Begin implementing the first standard macros (e.g., `storylet`, `choice`) in `src/macros_std.rs` and register them.
    - Create a new test file, `tests/macro_expansion_tests.rs`, to validate the expansion logic.

## Active Decisions and Considerations

### Confirmed Design Decisions
- **Dual syntax support**: Both brace-block and s-expression with lossless conversion
- **Pure immutable world state**: Using `im` crate for persistent data structures
- **Strict pipeline separation**: parse → macro-expand → validate → evaluate → output
- **Registry pattern**: For atoms and macros to enable introspection and extension
- **Span-based error reporting**: Throughout entire pipeline for best UX
- **Context Object for Evaluation**: The evaluator passes a single `EvalContext` struct to atoms, containing all necessary state (`world`, `output`, `opts`, `depth`). This keeps the `AtomFn` signature stable and the data flow explicit.

### Current Design Questions
- **Path representation**: Whether to use `&[&str]` or custom `Path` type for world navigation
- **Macro hygiene**: How sophisticated to make the hygiene system for user-defined macros
- **Performance optimization**: When to implement lazy evaluation or other optimizations

## Important Patterns and Preferences

### Cline's Implementation Protocol
1.  **Evaluate Before Writing**: For every file I create or modify, I must first explicitly write a "Code Evaluation" section. This section will analyze the proposed code against the implementation plan and the project's core design principles (Purity, Modularity, Separation of Concerns, etc.). I will not proceed with a `write_to_file` or `replace_in_file` operation until this evaluation is complete and confirms alignment.
2.  **Document All Incomplete Work**: For any feature or function that I intentionally leave unimplemented or in a placeholder state, I must leave a clear `// TODO:` comment block. This comment must explain what is missing and what the next steps are for completing the feature. This ensures no work is accidentally forgotten.

### Author Experience Priorities
1. **No explicit `get` operations** - automatic value resolution in all contexts
2. **Clear mutation marking** - all state changes use `!` suffix (`set!`, `add!`, etc.)
3. **Consistent predicate naming** - all boolean checks use `?` suffix (`is?`, `has?`, etc.)
4. **Readable aliases** - comparison operators have both canonical (`gt?`) and readable (`over?`) forms

### Technical Architecture Principles
1. **Library-first design** - core as pure library, CLI as thin wrapper
2. **No global state** - everything flows through explicit parameters
3. **Pure functions everywhere** - except for explicit atom mutations on world state
4. **Testability at every level** - each module independently testable
5. **Transparent debugging** - macro expansion and world state changes always inspectable

## Learnings and Project Insights

### Key Architectural Insights
- **Minimalism enables power**: Small atom set + macro composition provides unlimited expressiveness
- **Syntax flexibility matters**: Dual syntax removes adoption barriers while preserving power
- **Transparency is crucial**: Authors must be able to understand and debug their content
- **Immutability simplifies**: Pure functional approach eliminates many bug classes

### Implementation Strategy Insights
- **Staged approach is critical**: Each stage validates previous decisions before proceeding
- **Documentation-driven development**: Comprehensive design docs prevent architectural drift
- **Test-driven from start**: TDD approach ensures reliability and debuggability
- **Registry pattern scales**: Enables extension without core modifications

### Narrative Design Insights
- **QBN patterns are achievable**: All Emily Short patterns can be expressed as macros
- **Emergence from composition**: Complex narrative behaviors arise from simple building blocks
- **Author ergonomics matter**: Syntax and debugging tools are as important as functionality
- **Modularity enables reuse**: Storylets, pools, and threads compose cleanly

## Integration with Broader Goals

### Near-term (Next Month)
- Complete Stage 1-3 implementation (AST, parsing, atoms)
- Validate core architecture with realistic examples
- Build robust testing and debugging infrastructure

### Medium-term (Next Quarter)
- Implement full macro system and standard library
- Add brace-block syntax translator
- Create comprehensive example library
- Performance testing and optimization

### Long-term (Next Year)
- User-defined macros and modules
- Advanced tooling (editor support, visual debugging)
- Community and ecosystem development
- Research applications (academic, educational)

*Last Updated: 2025-06-30*
