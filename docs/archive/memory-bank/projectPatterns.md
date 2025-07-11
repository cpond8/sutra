# Sutra Engine - Project Patterns

## Core Heuristics

- **Systematic Refactoring Methodology**: 5-phase workflow: (1) Analysis - functions >20 lines, DRY violations, nested control flow; (2) Decomposition - helpers <15 lines, DRY utilities; (3) Guard Clauses - let-else patterns, early returns; (4) Organization - 7-section hierarchy; (5) Verification - cargo check/test, document changes
- **Registry Pattern**: Use for all extensible components (atoms, macros, etc.)
- **Pure Function Architecture**: Avoid global state and hidden side effects
- **Guard Clause Idiom**: let-else and early return for edge cases, eliminate nesting
- **Function Decomposition**: Break functions >20 lines into focused helpers
- **7-Section Organization**: Module docs → data structures → public API → conversions → infrastructure → internal helpers → exports

## Key Lessons Learned

**Code Quality**: Guard clause (let-else) standard for all logic. Decompose >20 line functions. Organize by function group with dependency flow. Extract DRY utilities for repetitive patterns.

**File Organization**: Large modules use 7-section structure with visual dividers. Group helpers by function. Use subsection comments for navigation.

**Helper Patterns**: Extract common evaluation patterns (eval_single_arg, extract_number). Use traits/generics when helpers show repetitive structure.

**AST Patterns**: Decompose large pattern-matching functions into focused helpers. Standard utilities: with_span(), error constructors. Guard clauses for validation.

**Systematic Success**: Rules 1,2,3 applied to eval.rs (321→381 lines), macros/mod.rs (702→763 lines), registry.rs (45→8 lines). Decomposition exposes DRY patterns, guard clauses work best on decomposed functions.

## Architectural Patterns

**Registry Reliability**: Integration tests, registry hashing (SHA256), feature-gated test atoms.

**Macro System**: Layered registry (core, stdlib, user), recursion depth enforcement (limit: 128).

**Development**: Lockstep parser/macro updates, span-aware errors, test/production parity.
