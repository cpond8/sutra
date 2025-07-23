# Sutra World State & Environment Refactor

## Intent

This refactor aims to eliminate the complexity and fragility of Sutra's current world state and environment management. The goal is to make global and lexical state handling robust, predictable, and simple, supporting both user and implementer needs.

## Rationale

- **Current issues:**
  - Global state is lost if not threaded manually between evaluations.
  - Lexical environment and closure capture are wasteful and error-prone.
  - Symbol resolution is multi-path and ambiguous, leading to shadowing and lookup confusion.
  - Atoms are managed in a separate registry, complicating the model and making shadowing impossible.
- **Desired outcome:**
  - Global state is always up-to-date and shared across all evaluation contexts.
  - Lexical scope is simple, ephemeral, and closures only capture what they reference.
  - Symbol resolution is unified, linear, and predictable.
  - Atoms and user functions are both first-class values in the global state.

## Refactor Plan (Phased, with Specifics)

### Phase 1: Canonical World State
- **Objective:** Replace the threaded, immutable world state with a canonical, shared mutable reference.
- **Rationale:**
  - Manual world threading is the root cause of global state loss and subtle bugs.
  - Functional purity (immutable world, explicit threading) is not needed in a single-threaded, imperative interpreter.
  - A single, canonical world reference is simpler, more robust, and matches user expectations.
- **Affected Subsystems:**
  - Evaluation engine (all code that currently threads or returns world state)
  - Test runner (must use the canonical world reference)
  - Atoms and special forms that mutate global state
- **Stepwise Implementation Plan:**
  1. **Introduce Canonical World Reference**
     - Add a single `Rc<RefCell<World>>` (or similar) to the root context (main interpreter struct or test runner).
     - All evaluation contexts, atoms, and special forms receive a reference to this canonical world.
     - *Self-critique:* This is the minimal change to centralize world state without disrupting other architecture yet.
  2. **Refactor Evaluation Engine**
     - Remove all world parameters and return values from evaluation functions.
     - All reads and writes to world state go through the canonical reference.
     - Update the evaluation context struct to hold a reference to the canonical world.
     - *Self-critique:* Eliminates unnecessary parameter passing and reduces boilerplate.
  3. **Refactor Atoms and Special Forms**
     - All atoms and special forms that mutate or read global state use the canonical world reference.
     - Remove world from atom function signatures and return values.
     - Ensure all mutations are performed via `borrow_mut()` and all reads via `borrow()`.
     - *Self-critique:* More concise, and the borrow checker enforces correct usage.
  4. **Refactor Test Runner**
     - Ensure all top-level forms and tests use the same canonical world.
     - Remove any code that manually threads world state between forms.
     - *Self-critique:* Reduces risk of state loss and makes the model clear to maintainers.
  5. **Remove World Cloning**
     - Audit all code for unnecessary world cloning.
     - Only clone the world for explicit snapshotting (e.g., for test isolation or undo functionality).
     - *Self-critique:* Improves performance and reduces memory usage.
  6. **Update Documentation and Comments**
     - Document the new model: “All global state is managed by a single, canonical world reference.”
     - Remove references to world threading and immutable world from docs and comments.
- **Concrete, Actionable Checklist:**
  - [x] Introduce a canonical `Rc<RefCell<World>>` in the root context.
  - [x] Update all evaluation contexts to hold a reference to this world.
  - [x] Remove world parameters and return values from all evaluation and atom functions.
  - [x] Refactor all world reads/writes to use the canonical reference.
  - [x] Update test runner to use the canonical world.
  - [x] Remove all unnecessary world cloning.
  - [x] Update documentation and code comments to reflect the new model.
  - [x] Test thoroughly to ensure global state is always up-to-date and visible.
- **Implementation Summary:**
  Phase 1 has been successfully completed. The core of this phase was the introduction of `pub type CanonicalWorld = Rc<RefCell<World>>;`. The `World` and `EvaluationContext` structs were refactored to use this new type, and the `Clone` trait was removed from `World` to enforce a single-owner pattern. All function signatures that previously passed `World` immutably (e.g., `Result<(Value, World), SutraError>`) were updated to return only `Result<Value, SutraError>`. Atom implementations and the evaluation engine now access the world state via `context.world.borrow()` and `context.world.borrow_mut()`. The `ExecutionPipeline` and `TestRunner` were updated to create and manage the single `CanonicalWorld` instance for each execution. Compiler-driven development was used to systematically address all borrow-checker and type errors. The entire codebase now compiles without errors or warnings, and all tests pass, confirming the refactor's success and that no regressions were introduced.

### Phase 2: Lexical Environment & Closures
- **Objective:** Simplify lexical environment management and closure capture.
- **Rationale:**
  - Capturing the entire lexical stack for closures is wasteful and can lead to memory bloat and subtle bugs.
  - The stack model is sound for block/function scope, but closure capture should be minimal and explicit.
  - Helper functions for stack manipulation should be direct and self-documenting, not over-abstracted.
- **Affected Subsystems:**
  - Lexical environment stack in the evaluation context
  - Closure (lambda) creation and invocation logic
  - Special forms: `let`, `lambda`, and any others that introduce scope
- **Stepwise Implementation Plan:**
  1. **Refactor Lexical Environment Stack**
     - Ensure the stack is only pushed for new blocks/lambdas and popped when leaving.
     - Remove any code that pushes/pops unnecessarily or redundantly.
     - *Self-critique:* Minimal, direct approach; audit for nonstandard block types.
  2. **Implement Minimal Closure Capture**
     - When creating a closure, analyze the closure body to determine which bindings are referenced.
     - Conservative approach: capture all bindings in the current frame; ideal: static analysis for only referenced bindings.
     - Store only these bindings in the closure’s environment.
     - *Self-critique:* Conservative approach is acceptable for now; optimize later if needed.
  3. **Refactor Closure Invocation**
     - When invoking a closure, reconstruct the lexical environment from the captured bindings plus the invocation’s own frame.
     - Ensure no unnecessary frames are pushed or leaked.
     - *Self-critique:* Matches recipe model; ensures no binding leaks.
  4. **Audit and Consolidate Helper Functions**
     - Remove or inline helpers that are only used once or add unnecessary abstraction.
     - Ensure all stack manipulation is explicit and self-documenting.
     - *Self-critique:* Reduces cognitive load; prefer clarity over unnecessary abstraction.
  5. **Update Documentation and Comments**
     - Document the new closure capture model: “Closures only capture the bindings they reference.”
     - Update all comments to reflect the new, minimal stack model.
- **Concrete, Actionable Checklist:**
  - [x] Refactor the `Lambda` struct to store a flat `HashMap` for captured variables.
  - [x] Implement a `find_and_capture_free_variables` function to analyze the lambda body and capture only referenced variables.
  - [x] Refactor `ATOM_LAMBDA` and `ATOM_DEFINE` to use the new free variable analysis.
  - [x] Overhaul `call_lambda` to construct a minimal two-frame environment from captures and arguments.
  - [x] Audit lexical environment helper functions to ensure correctness for `let` bindings.
  - [x] Create a comprehensive test suite (`tests/runtime/closures.sutra`) to validate the new model.
  - [x] Update documentation and comments to reflect the new minimal capture model.

- **Implementation Summary:**
  Phase 2 has been successfully completed. The core of this phase was the shift to a minimal closure capture model. Instead of wastefully cloning the entire lexical stack, `lambda` and `define` now perform a one-time analysis of the function body to find "free variables" (symbols that are not bound as parameters). Only the values of these free variables are stored in the `Lambda`'s `captured_env`, which was refactored from a `Vec<HashMap<...>>` to a simple `HashMap`. The `call_lambda` function was overhauled to be more efficient; it now constructs a clean, two-layer lexical environment consisting of the captured variables and the incoming arguments. This prevents memory bloat and makes closure invocation significantly more performant and predictable. A comprehensive test suite was added to validate correctness, including complex shadowing and nested capture cases.

### Phase 3: Unified Symbol Resolution
- **Objective:** Make symbol resolution linear and predictable.
- **Rationale:**
  - Multi-path symbol resolution (lexical → atom registry → world) is a source of subtle bugs and user confusion.
  - Atoms as a separate registry complicate shadowing and make built-ins less transparent.
  - Centralizing and unifying lookup logic makes the system more robust, predictable, and easier to maintain.
- **Affected Subsystems:**
  - Symbol lookup logic in the evaluation engine
  - Atom registry and registration logic
  - All code that currently distinguishes between atoms and user-defined globals
- **Stepwise Implementation Plan:**
  1. **Refactor Symbol Lookup Logic**
     - Centralize all symbol resolution in a single function or module.
     - Lookup order: lexical environment (innermost to outermost), then global world.
     - Remove all code that checks the atom registry as a separate path.
     - *Self-critique:* Minimal, direct approach; audit for legacy code paths.
  2. **Register Atoms as Global Functions**
     - At startup, register all built-in functions (atoms) as values in the global world.
     - Use a consistent naming convention to avoid conflicts.
     - *Self-critique:* More predictable; document and test for accidental shadowing.
  3. **Remove Atom Registry**
     - Eliminate the atom registry and all code that manages it.
     - Update all code that previously registered or looked up atoms to use the global world.
     - *Self-critique:* Reduces cognitive load; test thoroughly for completeness.
  4. **Update Error Reporting**
     - Ensure all symbol resolution errors report the full lookup path (lexical, then global).
     - Make error messages explicit about where the symbol was expected and not found.
     - *Self-critique:* More actionable for users; path is now simple and linear.
  5. **Update Documentation and Comments**
     - Document the new symbol resolution order and the fact that all functions (built-in or user) are first-class values.
     - Remove references to the atom registry and multi-path lookup.
- **Concrete, Actionable Checklist:**
  - [ ] Centralize symbol resolution logic to a single function/module with linear lookup order.
  - [ ] Register all built-in functions (atoms) as global values at startup.
  - [ ] Remove the atom registry and all related code.
  - [ ] Update all code to use the new symbol resolution path.
  - [ ] Update error reporting to reflect the new lookup order and provide clear diagnostics.
  - [ ] Update documentation and comments to describe the new model.
  - [ ] Test thoroughly to ensure shadowing, lookup, and error reporting work as expected.

### Phase 4: Tests, Documentation, Error Handling
- **Objective:** Ensure the new model is robust, well-documented, and easy to debug.
- **Affected Subsystems:**
  - All test suites (unit, integration, regression)
  - Documentation (language reference, architecture docs, code comments)
  - Error reporting and diagnostics
- **Key Actions:**
  - Update all tests to verify global state persistence, closure correctness, and symbol resolution.
  - Revise documentation to describe the new world state and environment model, including migration notes for users.
  - Ensure all error messages include the current lexical and global context for easier debugging.

## New Architecture Overview

- **Global State:**
  - Managed by a single, canonical reference shared across all evaluation contexts.
  - All global mutations are immediately visible to all forms and tests.
- **Lexical Environment:**
  - Managed as a stack, with frames pushed for each new block or function scope.
  - Closures only capture the bindings they reference, reducing memory and confusion.
- **Symbol Resolution:**
  - Always: lexical environment (innermost to outermost), then global world.
  - Atoms and user functions are both first-class, shadowable values in the global state.
- **Error Handling:**
  - All errors include full context, making debugging easier and more reliable.

## Expected Outcomes

- Global definitions and mutations always persist and are visible.
- Closures are minimal, predictable, and memory-efficient.
- Symbol resolution is clear, consistent, and shadowing works as expected.
- Atoms are managed like any other global function, simplifying the model.
- The codebase is simpler, more robust, and easier to maintain and extend.
- Tests and documentation are aligned with the new architecture, reducing onboarding and maintenance costs.