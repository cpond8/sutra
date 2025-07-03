# Sutra Engine - Progress

## What Works (Current State)

### Completed: Foundational Scaffolding (Stages 0-3)

- **Project Structure**: `Cargo.toml` configured and `tests/` directory created.
- **Stage 1: Core Data Types**:
  - `src/ast.rs`: `Expr` and `Span` for the AST.
  - `src/value.rs`: `Value` enum for all runtime data.
  - `src/world.rs`: `World` struct for immutable state.
  - `src/error.rs`: Unified `SutraError` and `SutraErrorKind`.
- **Stage 2: Unified PEG Parser (Fully Implemented & Tested)**:
  - The original hand-rolled parser has been replaced with a robust, unified PEG-based system.
  - `src/sutra.pest`: A formal grammar now defines both s-expression and brace-block syntaxes.
  - `src/parser.rs`: Has been successfully refactored to use the `pest` library, driven by the new grammar.
- **Stage 3: Atom Engine (Completed & Refactored)**:
  - `src/eval.rs`: The evaluation loop is stable. The original "auto-get" feature was **removed** and refactored into the macro system in Stage 4. The evaluator now correctly rejects bare symbols.
  - `src/atoms_std.rs`: The complete Tier 1 atom set is implemented and has been refactored to produce rich, contextual errors.
  - `src/world.rs`: `set` and `del` methods are fully implemented and simplified.
- **Module Integration**: All modules correctly declared in `src/lib.rs`.

### Completed: Stage 4 - Core Engine Stabilization

- **`src/macros.rs`**: The recursive macro expansion engine was fixed to correctly handle `Expr::If` nodes.
- **`src/macros_std.rs`**: The standard macro library was refactored to remove redundant code.
- **`src/atoms_std.rs`**: The critical state-propagation bug in `eval_args` was fixed.
- **`tests/`**: The entire test suite was corrected and now passes, confirming the stability of the core engine.
- **Status**: The core engine is stable and architecturally sound.

### Completed: Language Specification Synchronization

- **`docs/A_LANGUAGE_SPEC.md`**: The language specification has been thoroughly reviewed and updated to be in complete synchronization with the canonical codebase.
- **Status**: The specification is now a reliable, living document for developers.

### Completed: Comprehensive Design Phase

- **Architectural Foundation**: Core philosophy, atom/macro specs, pipeline design are all documented and stable.
- **Implementation Planning**: The 10-stage plan and per-file breakdowns are complete.

### Macro Path Canonicalization (2025-07-01)

- Macro system contract: all atom path arguments are canonicalized at macro-expansion time.
- Centralized in `canonicalize_path` (single source of truth).
- All assignment/path macros refactored to use this logic.
- Macro expansion and integration tests now robustly enforce the contract.
- **Status:** Canonicalization migration is complete and fully tested. All related tests pass.

### Atom Engine Refactor (2025-07-01)

- The `get` atom was fully refactored for minimalism, purity, and robust edge-case handling.
- Now supports both world-path and collection (list/map/string) access in a single, pure function.
- Always returns `Nil` for not found/out-of-bounds, never an empty map.
- All type and evaluation errors are explicit and contextual.
- All related tests now pass.

### Completed: Registry Pattern Audit (2025-07-02)

- The registry pattern for atoms and macros was audited. Both use canonical builder functions in `src/registry.rs`, with all registration logic centralized in the standard library modules. There is no duplication between production and test code. Naming, data flow, and extensibility are uniform. Test and production environments are guaranteed to be in sync.

### Completed: Error Handling Audit (2025-07-02)

- The error handling system was audited. All errors use structured, span-carrying types (`SutraError`, `EvalError`). Two-phase enrichment is implemented and used. Error construction and reporting are uniform across all pipeline stages. Test and production environments are in sync.

### Completed: AST/Parser Audit (2025-07-02)

- The AST and parser were audited. All AST nodes carry spans for source tracking. The parser uses a unified PEG grammar as the single source of truth for syntax. CST-to-AST conversion is uniform and preserves spans. Error handling and documentation are consistent and clear.

### Completed: Atoms/Eval Audit (2025-07-02)

- The atoms and evaluation engine were audited. All atoms follow uniform contracts and naming. State propagation is immutable and consistent. Error handling and span usage are uniform. Atoms are registered via a single registry. Documentation and test/production parity are maintained.

### Completed: CLI/Output and Tests Audit (2025-07-02)

- The CLI, output, and test suite were audited. The CLI is a pure orchestrator with output centralized in a dedicated module. Error handling and registry usage are consistent. All tests use the canonical pipeline and registry builders. Test/production parity and comprehensive coverage are maintained.

### Stage 6: Macro System Bootstrapping (2025-07-02)

- Transitioned from uniformity audit to macro bootstrapping. The goal is for all higher-level constructs (control flow, state mutation, narrative patterns) to be implemented as macros in the native engine language, not Rust. Focus is on self-hosting, canonicalization as macro, and data-driven macro registration/expansion.

## Macro System Bootstrapping Roadmap (2025-07-02)

The following stepwise plan is now the canonical reference for all macro system bootstrapping and self-hosting work. All previous 'What's Next' or 'Next Steps' sections are superseded by this roadmap.

### 1. Implement Full Variadic/Recursive Macro Support
- [x] Extend the macro system so user-defined macros can be variadic and recursive.
- [x] Ensure robust error handling and recursion limits.
- [x] Comprehensive tests for all edge cases (arity, recursion, parameter validation).

### 2. Migrate All Standard Macros to the Native Macro Language
- [ ] Rewrite all higher-level macros (especially `cond`) as native macros using the new support.
- [ ] Remove Rust-native macro implementations.
- [ ] Update and expand tests for new macro definitions.

### 3. Design and Implement Macro Hygiene
- Design a hygiene system (e.g., gensym for local bindings) to prevent accidental variable capture.
- Integrate into the macro expander and document its behavior.

### 4. Expand the Standard Macro Library (Tier 2+)
- Implement all higher-level narrative/gameplay macros (`storylet`, `choice`, `pool`, etc.) as native macros.
- Develop comprehensive example scripts and usage patterns.
- Performance test with realistic content.

### 5. Validation and Author Feedback
- Implement structural and semantic validation before and after macro expansion.
- Integrate validation and error reporting with CLI and author tools.

### 6. Documentation, CLI, and Tooling
- Audit and update documentation to reflect the new macro system.
- Ensure CLI exposes macroexpansion, registry, and validation features.

_This roadmap is the single source of truth for macro system bootstrapping and supersedes all previous plans._

_Last Updated: 2025-07-02_

## What's Left to Build

### Stage 5: Advanced Macro System (In Progress)

**Major Components:**

- **(Done) Implement Two-Tiered Macro System**: A new declarative `MacroTemplate` system has been implemented to support simple, author-defined variadic macros. This coexists with the native `MacroFn` system used for complex procedural macros.
- **(Next) Refactor `cond` and `if` Macros**: Refactor the conditional macros to establish `cond` as the primary, author-facing variadic macro. `cond` will be a native `MacroFn` that recursively expands into nested `if` calls. `if` will be a simple, 3-arity `MacroFn` that is the sole gateway to the `Expr::If` primitive. This change is isolated to `macros_std.rs` and tests, with no changes to the core AST or evaluator.
- **(Future) Hygiene System**: Design and implement a hygiene system (e.g., `gensym`) to prevent accidental variable capture in macros.
- **(Future) Expansion-Time Evaluation**: Explore adding a small, safe set of `template/...` functions that can be evaluated during macro expansion to allow for more powerful declarative macros.

**Estimated Effort:** 1 week
**Dependencies:** Stage 4
**Risk:** Low - The refactor is isolated to the macro layer.

### Stage 6: Standard Library Expansion (Not Started)

**Major Components:**

- Implement Tier 2+ macros: `storylet`, `choice`, `pool`, `select`, etc.
- Develop comprehensive example scripts and usage patterns.
- Performance testing with realistic content.
- Authoring documentation and tutorials.

**Estimated Effort:** 3-4 weeks
**Dependencies:** Stage 4
**Risk:** Medium-High - macro hygiene and expansion can be complex

### Stage 6: Validation and Author Feedback (Partially Complete)

**Two-Pass Validation:**

- **(✓) Author-friendly error messages**: The `EvalError` system provides rich, contextual errors for the evaluation phase. This pattern will be extended to other phases.
- Structural validation (pre-expansion)
- Semantic validation (post-expansion)
- Error aggregation and reporting

**Estimated Effort:** 1-2 weeks
**Dependencies:** Stage 5
**Risk:** Low - validation rules are well-specified

### Stage 7: CLI and Testing Infrastructure (In Progress)

**Command-Line Interface:**

- **(✓) Professional CLI Scaffolding**: A new `src/cli` module has been created using `clap` for robust argument parsing.
- **(✓) Macro expansion tracing and debugging**: The `macrotrace` subcommand is fully implemented.
- **(Next)** Script execution (`run` command).
- World state inspection and snapshotting.
- Integration with all pipeline stages.

**Estimated Effort:** 1-2 weeks
**Dependencies:** Stage 6
**Risk:** Low - thin wrapper over library API

### Stage 7: Standard Macro Library (Not Started)

**Narrative/Gameplay Macros:**

- All Tier 2-3 macros: pools, history, selection, grammar
- Comprehensive example scripts and usage patterns
- Performance testing with realistic content
- Documentation and tutorials

**Estimated Effort:** 2-3 weeks
**Dependencies:** Stages 1-6
**Risk:** Medium - requires balancing power and simplicity

### Stage 8: Brace-Block Translator (MERGED INTO STAGE 2)

**Note:** This stage has been merged into the work for Stage 2. The new unified PEG parser will handle both syntaxes, removing the need for a separate translator module.

## Current Status Assessment

### Strengths

- **Exceptionally thorough design phase** - all major architectural decisions resolved
- **Clear implementation path** - each stage builds cleanly on previous work
- **Comprehensive documentation** - principles, specifications, and examples all complete
- **Risk mitigation** - potential issues identified and addressed in planning

### Potential Challenges

- **Macro system complexity** - hygiene and expansion can be tricky to get right
- **Performance optimization** - may need tuning for large worlds
- **User experience** - need real-world testing with content creators
- **Documentation maintenance** - keeping specs aligned with implementation

### Timeline Estimates

**Total Implementation Time:** 15-20 weeks for complete system

- **MVP (Stages 1-5):** 8-12 weeks
- **Complete Core (Stages 1-7):** 12-16 weeks
- **Full System (Stages 1-8):** 15-20 weeks

## Known Issues and Technical Debt

### Current Issues

- Performance characteristics not yet validated with large-scale content.

### Future Technical Debt Risks

- **Macro system feature creep** - need to resist adding too many convenience macros
- **Performance optimization pressure** - may conflict with purity/simplicity goals
- **User macro system** - will require careful namespace and security design
- **Editor integration** - may need API extensions for rich editing features

## Evolution of Project Decisions

### Original Concept (Early Design)

- Started with minimalist Lisp-like language
- Focus on narrative scripting and interactive fiction

### Refined Architecture (Current)

- Expanded to support any game/simulation system
- Dual syntax for broader accessibility
- Emphasis on macro composition and extensibility
- Strong focus on debugging and author experience

### Validated Decisions

- **Atoms vs. macros split** - proven through exhaustive pattern mapping
- **Immutable world state** - simplifies debugging and testing significantly
- **Pipeline separation** - enables modular testing and tool development
- **Registry pattern** - provides clean extension points

### Open Research Questions

- **Optimal macro hygiene approach** - balance simplicity vs. power
- **Performance optimization strategies** - when and how to optimize
- **User macro system design** - security and namespace management
- **Advanced tooling requirements** - editor support, visual debugging

## 2025-07-04 Progress Update
- Implemented a temporary Rust macro expander for `(cond ...)`, with full tests and documentation. This allows Scheme-style multi-branch conditionals in author code, expanding to nested `if` expressions.
- Confirmed that the 'auto-get' feature is obsolete; all macro expansion is now explicit and canonical.
- All macro expansion tests, including for `cond`, now pass, confirming correct behavior.
- Next step: implement proper variadic macro support in the macro system. This is required before beginning the macro bootstrapping phase, where all macros will be defined in the native authoring language.

_Last Updated: 2025-07-01_

## What Works
- `cond` macro is robust, fully documented, and all error/edge cases are tested.

## What's Next
- Begin migration of all standard macros to the native macro language, starting with `cond`.
- Update documentation and CLI to reflect macro system migration and new capabilities.

## Current Status
- Canonical implementation and coverage for `cond` macro are complete.

- Canonical macro definition syntax now requires parentheses around the macro name and parameters, e.g. `define (my-list first . rest) { ... }`.
- Language spec updated to reflect this; all unrelated content restored after accidental removal.
- Only macro definition section changed; all other sections remain as originally specified.
- Future spec/documentation edits must be surgical and avoid regressions in unrelated content.

## Registry/Expander Reliability Audit (2025-07-02)

A major audit of macro registry and expander reliability was completed. Advanced strategies (phantom types, registry hashing, sealing, logging, integration tests, smoke mode, provenance, mutation linting, opt-out API, fuzzing, singleton, metrics) were reviewed and rated. Immediate implementation will focus on integration tests and registry hashing, with others staged for future adoption as needed. This marks a major milestone in macro system reliability and future-proofing. See activeContext.md and systemPatterns.md for full details and rationale.

### Completed: Stage 6 - Parser Refactor & Macro System Reliability (2025-07-02)

- **Parser Refactor:** The parser has been decomposed into per-rule helpers, with robust error handling and explicit dotted list validation. All unreachable!()s replaced with structured errors. Dotted list parsing now asserts and errors on malformed shapes.
- **Test Suite Run:** The parser compiles and passes borrow checker, but several macro loader and macro expansion tests (especially for variadic macros and cond) are failing. Parser and macro system are now fully decoupled and testable.

### In Progress: Debugging Macro System Test Failures

- Focus is on analyzing and resolving failing macro system tests, especially for variadic macro parameter parsing and cond macro expansion.
- Ensuring parser and macro system are in sync for all edge cases.

## Next Steps

1. Analyze and debug failing macro system tests. Focus on variadic macro parameter parsing and cond macro expansion.
2. Ensure parser and macro system are in sync. Confirm AST output matches macro loader expectations for all edge cases.
3. Update documentation and CLI to reflect parser refactor and macro system changes after all tests pass.
4. Expand/adjust tests as needed to cover new edge cases and regression scenarios.

_Last Updated: 2025-07-02_
