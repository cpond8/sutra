# Sutra Engine - Project Patterns

## Core Heuristics

- **Registry Pattern**: Use for all extensible components (atoms, macros, etc.)
- **Pure Function Architecture**: Avoid global state and hidden side effects
- **Batch-Based Development**: Test-driven modernization for all core system changes
- **Test/Production Parity**: Identical registry, loader, and macro expansion logic
- **Immediate Documentation**: Update patterns and memory bank after every significant change
- **Guard Clause Idiom**: Use let-else and early return for edge/invalid cases to reduce nesting and clarify main logic
- **Decomposition of Complex Atoms**: Break up large atom implementations (e.g., ATOM_APPLY) into focused helpers for testability and clarity
- **Modular CLI/Output**: Organize CLI/output code by logical function groups, extract helpers for DRY and clarity

## Key Lessons Learned

**Development Patterns**: Lockstep updates for macro system and parser, separation of concerns, batch-based iteration, span-aware error handling.

**Code Quality Patterns**: Function enumeration for audits, helper-driven decomposition, span-carrying invariants across modules, SRP enforcement through function extraction, guard clause refactoring for reduced nesting, DRY elimination via shared helper functions, logical function organization with dependency flow ordering and clear sectional boundaries, pipeline pattern extraction (file→AST→processing), environment setup consolidation, safe wrapper patterns for error-prone operations.
Guard clause (let-else) idioms and early returns are now standard for all atom/test atom logic and error handling.
Decompose any function >20 lines or with >2 logical phases into helpers.
CLI/output modules should always be organized by function group and dependency flow.

**Recurring Challenges**: Maintaining parser/grammar/loader synchronization, span-carrying invariants, canonicalization contracts, test/production registry parity.

## Architectural Patterns

**Registry Reliability**: Integration tests for end-to-end pipeline validation, registry hashing (SHA256) for macro definitions, feature-gated test atoms.

**Macro System**: Layered registry (core, stdlib, user, scenario), provenance metadata, expansion traces, recursion depth enforcement (limit: 128).
