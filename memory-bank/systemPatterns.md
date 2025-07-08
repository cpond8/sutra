# Sutra Engine - System Patterns

## Core Architectural Patterns

- **Pipeline Separation**: The engine enforces a strict `parse → macroexpand → validate → evaluate` pipeline:
  - Each stage is independently testable and documented
  - No hidden state or side effects between layers
  - All transformations are inspectable and reversible
  - Debugging available at every stage

- **Registry Pattern**: All atoms and macros registered via canonical builder functions:
  - Single source of truth for both production and test environments
  - Ensures extensibility, test/production parity, prevents duplication
  - Test atoms feature-gated (`cfg(debug_assertions)`, `cfg(test)`, `test-atom` feature)

- **Pure Function Architecture**: All core logic implemented as pure functions with no global state:
  - State propagated explicitly through immutable data structures
  - Only atoms can produce side effects on world state
  - All mutations return new world state, preserving original

- **Macro-Driven Extensibility**: All higher-level features implemented as macros, not core engine code:
  - Macro system supports variadic, recursive, and hygienic macros
  - Macro expansion is fully transparent and testable
  - Single source of truth: macro library defines all surface constructs

- **Error Handling and Transparency**: All errors structured, span-carrying, and contextual:
  - `EvalError` and two-phase enrichment pattern standard for user-facing errors
  - Clear, actionable error messages with debugging information

- **Minimalism and Compositionality**: Engine exposes minimal set of irreducible operations (atoms):
  - All complexity composed via macros and user-defined constructs
  - No privileged engine code in macro layer

## Design Principles

- Modular, testable, and compositional system
- No privileged code in macro layer
- Modern Rust idioms: direct function calls, type aliases, helper functions

## Reference

- See `techContext.md` for technologies and dependencies
- See `activeContext.md` for current work focus
- See `progress.md` for implementation status
