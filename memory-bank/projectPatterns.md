# Sutra Engine – Project Patterns

## Purpose

Canonical living intelligence log for project-specific heuristics, workflow preferences, patterns, lessons learned, and meta-decisions. This replaces deprecated `.cursorrules` and is updated frequently as new insights emerge.

## Core Heuristics

- **Registry Pattern**: Use for all extensible components (atoms, macros, etc.)
- **Pure Function Architecture**: Avoid global state and hidden side effects
- **Batch-Based Development**: Test-driven modernization for all core system changes
- **Test/Production Parity**: Identical registry, loader, and macro expansion logic
- **Immediate Documentation**: Update patterns and memory bank after every significant change

## Key Lessons Learned

### Development Patterns
- **Lockstep Updates**: Macro system and parser must be updated together for syntax/AST changes
- **Separation of Concerns**: Parser only parses, loader only loads, expander only expands
- **Batch-Based Iteration**: Isolate and resolve failures incrementally
- **Error Handling**: All errors must be span-aware, contextual, and match canonical spec

### Code Quality Patterns
- **Function Enumeration**: Always enumerate and check every function for complexity audits, not just those surfaced by search
- **Helper-Driven Decomposition**: Each logical step should be a named helper with explicit error cases
- **Span-Carrying Invariants**: Maintain across all modules for proper error reporting

## Test Suite Protocol

**All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors. No direct Rust API manipulation permitted.**

**Test Organization:**
- Protocol-compliant integration tests: `tests/scripts/` (`.sutra` scripts + expected output)
- Rust integration/unit tests: `tests/rust/`

## Recurring Challenges

- Ensuring parser, grammar, loader, and tests updated together for syntax changes
- Maintaining span-carrying invariants across all modules
- Enforcing canonicalization contracts and registry patterns in all environments
- Preventing drift between test and production registries

## Architectural Patterns

### Registry Reliability
- Integration tests for end-to-end pipeline validation
- Registry hashing (SHA256) for fingerprinting macro definitions
- Feature-gated test atoms for development builds

### Macro System
- Layered registry (core, stdlib, user, scenario) for modularity
- Provenance metadata for all definitions and expansions
- Expansion traces for debugging and auditing
- Recursion depth enforcement (limit: 128)

## Meta-Decisions

- **Single Source of Truth**: This file is canonical for project heuristics and workflow intelligence
- **Additive Overlays**: Additional context files must never conflict with core files
- **Mandatory Updates**: All contributors must review and update after significant changes

## Cross-References

- See `memory-bank/activeContext.md` for current work focus and critical blockers
- See `memory-bank/systemPatterns.md` for architectural patterns
- See `memory-bank/progress.md` for native file loading status and roadmap
- See `debug/macro-testing/README.md` for debug infrastructure patterns

## Changelog

- **2025-07-07**: Distilled to essential patterns and heuristics for token efficiency
- **2025-07-06**: Batch refactor patterns, function enumeration lesson, protocol-driven audits
- **2025-07-05**: Initialized as canonical intelligence log, macro system patterns documented
