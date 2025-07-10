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
- **AST Builder Decomposition**: Large pattern-matching functions should be decomposed into focused helpers with shared utilities for DRY compliance

## Key Lessons Learned

**Development Patterns**: Lockstep updates for macro system and parser, separation of concerns, batch-based iteration, span-aware error handling.

**Code Quality Patterns**: Function enumeration for audits, helper-driven decomposition, span-carrying invariants across modules, SRP enforcement through function extraction, guard clause refactoring for reduced nesting, DRY elimination via shared helper functions, logical function organization with dependency flow ordering and clear sectional boundaries, pipeline pattern extraction (file→AST→processing), environment setup consolidation, safe wrapper patterns for error-prone operations.
Guard clause (let-else) idioms and early returns are now standard for all atom/test atom logic and error handling.
Decompose any function >20 lines or with >2 logical phases into helpers.
CLI/output modules should always be organized by function group and dependency flow.

**AST Building Patterns**: Large pattern-matching functions (>20 lines, >2 phases) should be decomposed into focused helpers. Standard helper patterns include: `with_span()` for DRY WithSpan creation, `invalid_shape_error()` for consistent error construction, and individual `build_X_expr()` functions for each AST node type. Each helper should use guard clauses for validation and early returns for error cases. This pattern applies universally to AST construction (builders), macro template substitution (substitute_list, substitute_if), and any large pattern-matching function over Expr types.

**AST Display Patterns**: Large pretty-printing and formatting functions should follow the same decomposition approach: extract complex cases (`pretty_list`, `pretty_if`, `pretty_param_list`, `fmt_list`, `fmt_map`) into focused helpers while keeping simple cases inline. Main function becomes a clean dispatch table with helper calls. Pattern applies universally to Display implementations, pretty-printing, and any large pattern-matching function with complex cases.

**File Organization Patterns**: Large modules should be organized into clear logical sections with section dividers: 1) Module documentation and imports, 2) Core data structures, 3) Public API implementation (with helpers grouped under parent impl), 4) Conversions, 5) Infrastructure/traits, 6) Internal helpers (grouped by function), 7) Module exports. Use section dividers (`// ============================================================================`) and subsection comments for navigation. For very large files (>1000 lines), atoms should be grouped by functional area within the public API section (core atoms, arithmetic atoms, comparison atoms, etc.) for better navigation. All helper functions move to internal helpers section regardless of original location.

**Helper Function Patterns**: Extract common evaluation patterns to eliminate DRY violations: `eval_single_arg`, `eval_binary_args` for argument evaluation; `extract_number`, `extract_bool`, `extract_path` for type extraction with error handling. Group helpers by function (evaluation patterns, type extraction utilities). This pattern applies to any module with repetitive argument processing or type checking logic.

**Advanced DRY Patterns**: When helper functions themselves show repetitive structure, use traits and generics for further abstraction. Implement `ExtractValue<T>` trait for type extraction to eliminate match-based repetition across extract_* functions. Use generic `eval_n_args<const N: usize>` with compile-time arity checking to unify argument evaluation patterns. This reduces extract functions to one-liners and provides type-safe argument handling. Pattern applies when multiple helper functions follow identical structural templates.

**Recurring Challenges**: Maintaining parser/grammar/loader synchronization, span-carrying invariants, canonicalization contracts, test/production registry parity.

## Architectural Patterns

**Registry Reliability**: Integration tests for end-to-end pipeline validation, registry hashing (SHA256) for macro definitions, feature-gated test atoms.

**Macro System**: Layered registry (core, stdlib, user, scenario), provenance metadata, expansion traces, recursion depth enforcement (limit: 128).

**AST Construction**: Pattern-matching builders decomposed into type-specific helpers with shared utilities (`with_span`, error constructors). Each node type gets its own focused builder function following guard clause patterns.
