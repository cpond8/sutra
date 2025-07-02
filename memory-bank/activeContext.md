# Sutra Engine - Active Context

## Current Work Focus

**Phase**: Advanced Macro System
**Priority**: Re-implement `cond` as a macro.

### Recent Changes (2025-07-02)

1.  **Language Specification Synchronized**: The official language specification (`docs/A_LANGUAGE_SPEC.md`) has been updated to be in full alignment with the canonical codebase, ensuring it is a reliable "living document".
2.  **Core Engine Stabilized**: A full audit and hardening pass was completed, fixing critical bugs in state propagation and macro expansion, and bringing the entire test suite to a passing state.
3.  **Unified Registry Pattern Implemented**: Atom and macro registry setup is now fully DRY. Both production and test code use canonical builder functions, eliminating all duplication.
4.  **Doctest Audit and Documentation**: All public-facing modules have been reviewed and documented with doctests where appropriate.

## Next Steps (Immediate Priority)

1.  **Implement `cond` as a Macro**:
    - With the core evaluator now stable, the `cond` construct will be implemented as a macro that expands into a series of nested `if` expressions. This will restore full multi-branch conditional functionality.
2.  **Complete `get` Atom**:
    - The `get` atom must be extended to support collection access (lists, maps, strings) to fulfill its design contract.
3.  **Finalize and Document**: Once the above tasks are complete, the memory bank will be given a final review to ensure all documentation is consistent with the final, stable architecture.

## Active Decisions and Considerations

### Confirmed Design Decisions

- **Professional CLI Architecture**: The CLI is a pure orchestrator of the core library, with a strict separation of concerns. All user-facing output is centralized in its own module.
- **Rich, Contextual Errors**: The `EvalError` struct and two-phase enrichment pattern is the new standard for all user-facing errors.
- **Minimal, Pure, and Robust Atoms**: All core atoms, especially `get`, are now implemented as pure, minimal, and robust functions, with explicit handling for all edge cases and consistent return values.
- **Keyword-Avoidant Naming**: The `macro` module was renamed to `macros` to improve clarity and avoid confusing `r#` syntax.
- **Strict pipeline separation**: `parse -> expand -> eval`.
- **Path Canonicalization**: The macro system is the sole authority for converting user-facing path syntax into canonical `Expr::Path` nodes. This is the primary architectural pattern.
- **Strict Pipeline Separation**: The `parse -> expand -> eval` pipeline is strictly enforced. The evaluator will reject any non-canonical AST forms (like bare symbols), ensuring the macro expander has done its job.
- **Span-aware `AtomFn`**: The signature `fn(..., parent_span: &Span)` is the standard for all atoms.
- **Unified Registry Pattern**: All atom and macro registration is now centralized in canonical builder functions. No duplication exists between test and production setup; all code paths use the same logic for registry construction.
- **Doctest Documentation Pattern**: All public API functions that can be meaningfully demonstrated in isolation (e.g., registry builders) have doc examples that compile and run. Internal or context-dependent functions (e.g., atoms) are documented with comments explaining why doctests are not appropriate.

### Current Design Questions

- **Macro hygiene**: How sophisticated to make the hygiene system for user-defined macros.
- **Performance optimization**: When to implement lazy evaluation or other optimizations.
- **`get` atom completion**: The `get` atom needs to be extended to support collection access (lists, maps) in addition to world paths.

## Important Patterns and Preferences

### Cline's Implementation Protocol

1.  **Evaluate Before Writing**: For every file I create or modify, I must first explicitly write a "Code Evaluation" section. This section will analyze the proposed code against the implementation plan and the project's core design principles (Purity, Modularity, Separation of Concerns, etc.) and assign each point a rating from 1-10. I will not proceed with a `write_to_file` or `replace_in_file` operation until this evaluation is complete and confirms alignment.
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

- **Canonicalization Contract Fully Enforced**: The macro system now guarantees that all atom path arguments are in the canonical flat `(list ...)` form, with a single source of truth and comprehensive test coverage.
- **Immutability requires careful state propagation**: The `ATOM_DO` bug is a perfect example of this. When working with immutable data structures like `World`, it is critical to ensure that the _new_ state from one operation is correctly threaded as the input to the _next_ operation. A single mistake here breaks the chain of state.
- **Registries are a critical architectural component**: The discovery of duplicated and incomplete atom/macro registries highlights that a single source of truth for standard libraries is non-negotiable for a robust, maintainable system.
- **Test environments must mirror production**: The fact that the test suite's macro registry was incomplete but tests were still passing (incidentally) shows the danger of divergent environments. A test setup must be identical to the production pipeline to be reliable.
- **Minimalism enables power**: Small atom set + macro composition provides unlimited expressiveness
- **Syntax flexibility matters**: Dual syntax removes adoption barriers while preserving power
- **Transparency is crucial**: Authors must be able to understand and debug their content
- **Immutability simplifies**: Pure functional approach eliminates many bug classes
- **Doctest Documentation Pattern**: The project now follows a clear pattern: add doctests to all public APIs where feasible, and document with comments where not. This ensures clarity for future contributors and robust, example-driven documentation.

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

_Last Updated: 2025-07-01_
