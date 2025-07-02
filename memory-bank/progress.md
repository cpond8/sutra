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

## What's Left to Build

### Stage 5: Advanced Macro System (Immediate Priority)

**Major Components:**

- **(Next) Re-implement `cond` as a Macro**: With the core evaluator now stable, the `cond` construct should be re-introduced as a macro that expands into a series of nested `if` expressions.
- **(Next) Complete `get` Atom**: The `get` atom must be extended to support collection access (lists, maps, strings) to fulfill its design contract.
- Pattern-matching macro expansion
- Hygiene system for variable scoping
- Standard macro library (storylet, choice, pool, etc.)
- Recursion depth limiting

**Estimated Effort:** 3-4 weeks
**Dependencies:** Stage 4
**Risk:** Medium-High - macro hygiene and expansion can be complex

### Stage 5: Advanced Macro System (Not Started)

**Major Components:**

- Pattern-matching macro expansion
- Hygiene system for variable scoping
- Standard macro library (storylet, choice, pool, etc.)
- Recursion depth limiting

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

_Last Updated: 2025-07-01_
