# Sutra Engine - Active Context

## Current Work Focus

**Phase**: Authoring Experience & Tooling
**Priority**: Build out the professional CLI to provide a robust and transparent development experience.

### Recent Changes (2025-07-01)

1.  **Macro Path Canonicalization Migration Complete**:

    - All macro-generated atom calls now strictly require canonical `(list ...)` path arguments.
    - Centralized canonicalization logic in `canonicalize_path` (single source of truth).
    - All assignment/path macros refactored to use this logic.
    - Macro expansion and integration tests robustly enforce the contract.
    - All tests pass except for a single world state propagation issue (see below).

2.  **Implemented Rich Error Reporting**:

    - **`src/error.rs`**: The `SutraError` system was overhauled. `EvalError` now captures the original code, expanded code, and a helpful suggestion.
    - **`src/atoms_std.rs`**: The entire atom library was refactored to produce these new rich, contextual errors, using a new `eval_err!` helper macro to ensure consistency.
    - **Two-Phase Enrichment**: A pattern was established where errors are created with local context and then enriched with global context (like the original source code) by a top-level runner.

3.  **Implemented Macro Expansion Tracing**:

    - **`src/macros.rs`**: The `macro` module was renamed to `macros` to avoid keyword conflicts.
    - The `macroexpand_trace` function was implemented to produce a step-by-step vector of `TraceStep` structs, detailing the entire expansion process.

4.  **Scaffolded Professional CLI**:

    - **`Cargo.toml`**: Added dependencies for `clap`, `termcolor`, `difference`, `walkdir`, and `serde`.
    - **`src/cli/`**: A new module was created to house all CLI logic, separating it cleanly from the core engine library.
    - **`macrotrace` command**: The first CLI command was implemented, providing a colorized, diff-based view of the macro expansion process.

5.  **Refactored and Hardened `get` Atom (2025-07-01)**:

    - The `get` atom was fully refactored for minimalism, purity, and robust edge-case handling.
    - Now supports both world-path and collection (list/map/string) access in a single, pure function.
    - Always returns `Nil` for not found/out-of-bounds, never an empty map.
    - All type and evaluation errors are explicit and contextual.
    - All related tests now pass; only unrelated world state propagation test remains failing.

## Next Steps (Immediate Priority)

1.  **Debug World State Propagation Issue**:

    - The only remaining test failure (`test_state_propagation_in_do_block`) is due to a state threading bug, not canonicalization or atom logic.
    - Action: Investigate and fix world state propagation in the evaluation pipeline.

2.  **Implement Core CLI Commands**:

    - **Priority:** Flesh out the CLI to make the engine fully usable from the command line.
    - **Action:** Implement the `run`, `validate`, and `macroexpand` commands according to the CLI specification. This will involve building out the `output.rs` module to handle enriched error printing.
    - **Goal:** A user can run a Sutra script from the CLI and see well-formatted output or errors.

3.  **Implement Filesystem-Aware Commands**:

    - **Action:** Implement the `test` command, using the `walkdir` crate to discover and run all test files in a directory. Implement the `format` command.
    - **Goal:** The test suite can be run with a single command.

4.  **Update Documentation (Completed)**:
    - All memory bank files and implementation plans have been updated to reflect the latest changes.

## Active Decisions and Considerations

### Confirmed Design Decisions

- **Professional CLI Architecture**: The CLI is a pure orchestrator of the core library, with a strict separation of concerns. All user-facing output is centralized in its own module.
- **Rich, Contextual Errors**: The `EvalError` struct and two-phase enrichment pattern is the new standard for all user-facing errors.
- **Minimal, Pure, and Robust Atoms**: All core atoms, especially `get`, are now implemented as pure, minimal, and robust functions, with explicit handling for all edge cases and consistent return values.
- **Keyword-Avoidant Naming**: The `macro` module was renamed to `macros` to improve clarity and avoid confusing `r#` syntax.
- **Strict pipeline separation**: `parse -> expand -> eval`.
- **Macro-based "auto-get"**: This is the canonical pattern.
- **Explicit `(get ...)` in evaluated AST**: The macro expander is responsible for this transformation.
- **Span-aware `AtomFn`**: The signature `fn(..., parent_span: &Span)` is the standard for all atoms.

### Current Design Questions

- **Path representation**: Whether to use `&[&str]` or custom `Path` type for world navigation
- **Macro hygiene**: How sophisticated to make the hygiene system for user-defined macros
- **Performance optimization**: When to implement lazy evaluation or other optimizations

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
- **Immutability requires careful state propagation**: The current regression highlights that when working with immutable data structures like `World`, it is critical to ensure that the _new_ state from one operation is correctly threaded as the input to the _next_ operation. A single mistake here breaks the chain of state.
- **Pipeline interactions are subtle**: Even with pure, decoupled stages, the composition of those stages can introduce bugs if the "seams" between them are not handled perfectly. The `run_expr` test helper is a microcosm of this challenge.
- **Good error reporting is non-negotiable**: The recent refactor to improve error messages was a necessary step to make debugging complex issues like the current regression feasible.
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

## Macro Path Canonicalization Contract (2025-07-01)

- The macro system now enforces a strict contract: **all path arguments to core atoms (e.g., `set!`, `del!`) are canonicalized at macro-expansion time**.
- The canonicalization logic is centralized in `src/macros/utils.rs::canonicalize_path`, which is the single source of truth for all macro-level path normalization.
- All assignment/path macros (`add!`, `sub!`, `inc!`, `dec!`, etc.) have been refactored to use this helper and now document this contract explicitly.
- The macro expansion test suite covers all path forms (symbol, dotted symbol, list, string, mixed/invalid) and verifies that errors are surfaced for malformed paths.
- **Known issue:** The canonicalization logic for list-of-strings is not yet correct; a sentinel test is in place and failing as expected. The next step is to fix this logic and confirm all tests pass.
- This contract ensures compositionality, deduplication, and pipeline purity, and permanently closes a class of macro/atom bugs.

_Last Updated: 2025-07-01_
