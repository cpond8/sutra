# Sutra Engine - Active Context

## CRITICAL STATUS: Native .sutra File Loading Assessment (2025-07-07)

### ✅ Current State: ~85% Complete with One Critical Blocker

Native `.sutra` file loading and interpretation is **substantially working** but has **one critical blocker** preventing full functionality.

**Working Components:**
- CLI can load and execute `.sutra` files: `./target/debug/sutra run file.sutra`
- Complete parsing pipeline: parse → macroexpand → validate → evaluate
- Basic scripts work perfectly: `(print "Hello, world!")` ✅
- Built-in macros work: `print`, arithmetic, control flow ✅
- Parser and grammar support s-expression and brace-block syntax ✅
- Fixed critical parser bug in `build_define_form` ✅

### ❌ CRITICAL BLOCKER: User-Defined Macro Pipeline

**The Issue:**
```lisp
;; This syntax parses correctly but fails at runtime:
(define (greet name) (print (+ "Hello, " name "!")))
(greet "Alice")  ;; Error: Unknown macro or atom: greet
```

**Root Cause Analysis:**
1. ✅ Macro definition parsing works (fixed parser bug)
2. ✅ AST structure is correct (`Expr::ParamList` etc.)
3. ❌ **BROKEN**: Macro definitions not properly partitioned from user code
4. ❌ **BROKEN**: User-defined macros not found during expansion
5. ❌ **BROKEN**: `define` forms appear in final expanded output (should be filtered)

**Evidence from `macrotrace`:**
- Output: `(do (define (greet name) (core/print ...)) (greet "Sutra"))`
- The `define` form should NOT appear in final output
- The `(greet "Sutra")` call should be expanded but isn't

### 🎯 IMMEDIATE ACTION PLAN

**BLOCKING ALL OTHER WORK until resolved:**

1. **Debug Macro Definition Pipeline**
   - Investigate `is_macro_definition` function - verify it correctly identifies define forms
   - Debug partitioning logic in `src/lib.rs` - ensure macro definitions are separated
   - Verify macro registry construction - ensure user macros are properly loaded

2. **Debug Macro Expansion**
   - Investigate macro environment construction - verify user macros are passed to expander
   - Debug macro lookup in expansion phase - ensure user registry is searched
   - Test parameter binding and substitution logic

3. **Integration Testing**
   - Verify end-to-end macro definition and usage works
   - Test parameter passing and template substitution
   - Ensure macro definitions don't appear in final expanded output

### 📋 Debug Files and Investigation Tools

**Location**: `debug/macro-testing/` directory contains systematic test files.

**Key Files**:
- `test_macro_native.sutra` - Primary test case for macro definition and usage
- `debug_minimal.sutra` - Minimal macro definition for isolated testing
- `debug_simple.sutra` - Basic script validation
- See `debug/macro-testing/README.md` for complete documentation

**Significance**: This debugging infrastructure provided evidence that isolated the parser layer (fixed) from the integration layer (still needs work).

## Parsing Pipeline Status

The parsing pipeline implementation is **substantially complete** per the canonical plan in `docs/architecture/parsing-pipeline-plan.md`.

### ✅ COMPLETED MODULES
- **CST Parser Module**: Contract and scaffolding with `SutraCstParser` trait
- **Parser/AST Builder**: Full implementation using pest-based PEG grammar
- **Macroexpander Module**: Robust implementation with template system and depth limiting
- **Validator Module**: Contract complete with `SutraValidator` trait and diagnostics
- **Pipeline Integration**: Complete modular pipeline with working integration tests

### 🔄 REMAINING WORK (Low Priority)
- **Interface Formalization**: Extract formal traits where logic is embedded
- **CST Traversal APIs**: Implement iterator and visitor patterns
- **Advanced Features**: Incremental parsing, auto-fix interface, advanced hygiene

**Critical Insight**: The parsing pipeline plan is essentially COMPLETE. The architecture is modular, interface-driven, and working. Priority should remain on native .sutra file loading, NOT pipeline refactoring.

## TEST SUITE PROTOCOL

**All tests must be written as user-facing Sutra scripts (s-expr or braced), asserting only on observable output, world queries, or errors as surfaced to the user. No direct Rust API or internal data structure manipulation is permitted.**

**A full test suite rewrite is required to comply with this protocol.**

## Current Work Focus

**Phase:** Native .sutra File Loading - Macro Pipeline Debug

**Priority:**
- Debug and resolve user-defined macro pipeline blocker
- Ensure macro definitions are properly partitioned and expanded
- Complete end-to-end native file loading functionality
- Maintain systematic debug infrastructure in `debug/macro-testing/`

## File Hierarchy Update (2025-07-07)

The Rust codebase has been reorganized for maximal modularity:
- `src/syntax/` (parser, validator, grammar, errors)
- `src/ast/` (builder, value)
- `src/atoms/` (std library atoms)
- `src/macros/` (macro system)
- `src/runtime/` (eval, path, registry, world)
- `src/cli/` (args, output)

**Test Organization:**
- Inline tests for small modules
- Rust integration/unit tests in `tests/rust/`
- Protocol-compliant integration tests in `tests/scripts/` (Sutra scripts + expected output)

## Key Design Decisions

- **Professional CLI Architecture**: CLI as pure orchestrator of core library
- **Rich, Contextual Errors**: `EvalError` struct with span-carrying diagnostics
- **Strict Pipeline Separation**: `parse -> expand -> eval` pipeline enforced
- **Path Canonicalization**: Macro system converts user syntax to canonical `Expr::Path`
- **Unified Registry Pattern**: Centralized atom and macro registration
- **Span-aware `AtomFn`**: Standard signature with parent span parameter

## Technical Architecture Principles

- **Library-first design**: Core as pure library, CLI as thin wrapper
- **No global state**: Everything flows through explicit parameters
- **Pure functions everywhere**: Except for explicit atom mutations on world state
- **Testability at every level**: Each module independently testable
- **Transparent debugging**: Macro expansion and world state always inspectable

## Cross-References

- See `docs/architecture/parsing-pipeline-plan.md` for canonical pipeline evaluation
- See `memory-bank/progress.md` for completed work and status
- See `memory-bank/systemPatterns.md` for architectural patterns
- See `debug/macro-testing/README.md` for debug infrastructure documentation
- See `system-reference.md` for detailed system reference and rationale

## Changelog

- **2025-07-07**: Native .sutra file loading assessment completed, critical blocker identified, debug infrastructure documented
- **2025-07-07**: File hierarchy reorganized for modularity, test organization updated
- **2025-07-06**: Integration test runner bootstrapped with `tests/scripts/`
- **2025-07-04**: Parsing pipeline plan assessment completed, architecture confirmed sound
