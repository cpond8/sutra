# RFP-001: Two-Tiered Atom Architecture

**Date:** 2025-07-12
**Author:** Roo
**Status:** Proposed

## 1. Abstract

This document proposes a significant architectural refactoring of the Sutra engine's atom system. The current single `AtomFn` signature forces a tight coupling between all atoms and the evaluation machinery, leading to brittle, hard-to-maintain code. We will replace this with a two-tiered system that distinguishes between **Pure Atoms** and **Stateful Atoms**. This change will dramatically simplify atom implementations, improve testability, and align the engine with the principle of pragmatic minimalism by making the system easier to read, debug, and maintain. This proposal includes a strategy for incremental migration to de-risk the transition.

## 2. Motivation & Problem Statement

The current `AtomFn` signature is defined as:
`fn(args: &[AstNode], context: &mut EvalContext, parent_span: &Span) -> Result<(Value, World), SutraError>`

This design has several critical flaws:

*   **High Coupling:** Every atom is coupled to the `AstNode` (syntax), `EvalContext` (evaluation machinery), and `Span` (source location).
*   **Leaky Abstraction:** Atoms are responsible for evaluating their own arguments, a responsibility that should belong to the core evaluator. This has led to a cascade of subtle bugs related to type mismatches and incorrect evaluation order.
*   **Poor Readability:** The core logic of an atom is obscured by the boilerplate of argument evaluation and context management.
*   **Difficult Maintenance:** A change in evaluation strategy requires modifying every single atom in the codebase.

The goal of this refactoring is to address these issues by introducing a cleaner, more decoupled interface between the evaluator and the atoms.

## 3. Architectural Evaluation: Pragmatic Minimalism

Before detailing the solution, it's crucial to evaluate it against the engine's design philosophy: **pragmatic minimalism**. Is this change a necessary simplification, or is it over-engineering?

*   **Current State:** The existing architecture forces complexity onto every atom. Atom authors must constantly be aware of the `AstNode` vs. `Value` distinction and manually handle argument evaluation. This has already proven to be a source of subtle, hard-to-trace bugs, which is the opposite of minimalism.
*   **Proposed State:** The proposed change centralizes the complexity of argument evaluation into a single location (the evaluator). Atoms become dramatically simpler, focusing only on their core logic.
*   **Verdict:** This is **not over-engineering; it is a simplification**. It removes a redundant, error-prone responsibility from dozens of functions and consolidates it. The one-time cost of this refactoring will be paid back through significantly reduced time spent on debugging and maintenance, and increased clarity for future development. This change is a direct embodiment of pragmatic minimalism.

## 4. Proposed Solution: Pure and Stateful Atoms

We will replace the monolithic `AtomFn` with an `Atom` enum that represents two distinct types of primitive functions.

### 4.1. New Atom Definitions

The following types will be defined in `src/atoms/mod.rs`:

```rust
/// Represents a pure, stateless atom that operates only on its arguments.
/// It has no access to the world state. Its contract is simple:
/// given values, return a new value or an error.
pub type PureAtomFn = fn(args: &[Value]) -> Result<Value, SutraError>;

/// Represents a stateful atom that requires access to the evaluation context,
/// primarily for reading from or writing to the World state.
pub type StatefulAtomFn = fn(args: &[Value], context: &mut EvalContext) -> Result<(Value, World), SutraError>;

/// The legacy function signature for incremental migration.
pub type LegacyAtomFn = fn(args: &[AstNode], context: &mut EvalContext, parent_span: &Span) -> Result<(Value, World), SutraError>;

/// An enum that holds either a pure or a stateful atom function.
/// This allows the registry to store both types and the evaluator to
/// dispatch to them correctly.
#[derive(Clone)]
pub enum Atom {
    Pure(PureAtomFn),
    Stateful(StatefulAtomFn),
    Legacy(LegacyAtomFn),
}
```

### 4.2. Evaluator Responsibilities & Boundaries

To avoid making the evaluator a "god object," its responsibilities will be strictly defined:

1.  **Core Role: Dispatch:** The primary role of `eval_list` is to orchestrate the call to an atom.
2.  **Modular Helper:** The logic for evaluating arguments (`&[AstNode]` -> `Vec<Value>`) will be encapsulated in a private helper function within `eval.rs`, called by `eval_list`. This keeps the dispatch logic clean.
3.  **Dispatch Logic:** `eval_list` will retrieve the `Atom` enum and dispatch to the correct function type, handling the `Legacy` variant as a pass-through to maintain backward compatibility during the transition.

### 4.3. Atom Responsibilities

*   **Pure Atoms** (e.g., `+`, `-`, `eq?`): Implement pure, stateless logic via the `PureAtomFn` signature.
*   **Stateful Atoms** (e.g., `core/set!`, `core/get`): Interact with the world via the `StatefulAtomFn` signature.

## 5. Implementation Plan: Incremental Migration

This plan is designed to be executed incrementally, avoiding a "big bang" refactor.

**Phase 1: Setup and Shimming**
1.  Define the `Atom` enum with `Pure`, `Stateful`, and `Legacy` variants in `src/atoms/mod.rs`.
2.  Update the `AtomRegistry` to store `Atom` enums.
3.  Modify `eval_list` in `src/runtime/eval.rs` to implement the three-way dispatch logic.
4.  **Crucially, update all existing atom registrations to use the `Atom::Legacy` variant.** At the end of this phase, the codebase should compile and pass all tests, functioning identically to before.

**Phase 2: Incremental Atom Refactoring**
1.  Choose a single module to refactor (e.g., `math.rs`).
2.  For each atom in the module, update its function signature to either `PureAtomFn` or `StatefulAtomFn`.
3.  Remove all internal evaluation logic from the refactored atoms.
4.  Update the atom's registration to use `Atom::Pure` or `Atom::Stateful`.
5.  Run `cargo test` to ensure the module is still correct.
6.  Repeat this process for all atom modules until no `Legacy` atoms remain.

**Phase 3: Final Cleanup**
1.  Once all atoms are migrated, remove the `LegacyAtomFn` type and the `Atom::Legacy` variant.
2.  Remove the legacy dispatch path from `eval_list`.
3.  Remove the now-redundant `eval_args` helper function from `src/atoms/helpers.rs`.

## 6. Benefits

*   **Pragmatic Minimalism:** The final design is simpler and easier to understand.
*   **Loose Coupling:** Atoms are decoupled from the AST and evaluation machinery.
*   **Improved Testability:** Pure atoms become trivial to unit test.
*   **Enhanced Maintainability:** Changes to evaluation logic only need to be made in one place.
*   **Safe Transition:** The incremental migration plan eliminates the risk of a disruptive, all-at-once refactor.

## 7. Architectural Review of `logic.rs` Refactor

**Date:** 2025-07-12

**1. Analysis**

A previous attempt to refactor `src/atoms/logic.rs` to the `PureAtomFn` model resulted in the addition of two new helper functions to `src/atoms/helpers.rs`:
- `eval_binary_numeric_op_pure`
- `eval_unary_bool_op_pure`

These functions were created to abstract the common pattern of:
1. Checking arity.
2. Extracting typed values from the `&[Value]` slice.
3. Applying a given operation.

While the intent to reduce boilerplate is sound, the execution introduces an unnecessary layer of abstraction that runs counter to the principle of **pragmatic minimalism** outlined in this RFP.

**2. Verdict: Flawed Abstraction**

The new `_pure` helper functions are a **flawed abstraction**.

*   **Redundant Complexity:** The core logic of a `PureAtomFn` is already simple. The helpers save only 2-3 lines of code per atom, but at the cost of introducing new functions that must be learned and maintained. The cognitive overhead of navigating to the helper function's definition to understand an atom's behavior is greater than the benefit of the lines saved.
*   **Poor Cohesion:** The helpers separate the error handling (arity checks, type extraction) from the core logic of the atom. The ideal `PureAtomFn` implementation should be self-contained and immediately readable. For example, the `ATOM_EQ` function in `logic.rs` is a perfect example of this principle: its logic is simple, direct, and requires no helper functions.
*   **Scalability Concerns:** This pattern is not scalable. As we refactor more atoms with different type signatures (e.g., string operations, list operations), `helpers.rs` would become bloated with a combinatorial explosion of specialized `_pure` helpers (`eval_binary_string_op_pure`, `eval_unary_list_op_pure`, etc.). This is precisely the kind of complexity this RFP aims to eliminate.

The `_pure` helpers are a solution in search of a problem. The `PureAtomFn` signature is already minimal enough that such helpers are not required.

**3. Revised Implementation Plan**

The initial implementation plan is sound, but requires a revised approach for Phase 2 that explicitly rejects the flawed helper pattern.

**Phase 1: Setup and Shimming (Completed)**

This phase is complete and was successful.

**Phase 2 (Revised): Incremental Atom Refactoring**

This phase will proceed on a per-module basis, starting with `logic.rs`. The guiding principle is to implement the logic **directly** within the `PureAtomFn` closure, without adding new helpers to `helpers.rs`.

**Action Plan for `code` agent:**

1.  **Remove Flawed Helpers:** Delete the `eval_binary_numeric_op_pure` and `eval_unary_bool_op_pure` functions from `src/atoms/helpers.rs`.
2.  **Refactor `logic.rs` Atoms:**
    *   Go to `src/atoms/logic.rs`.
    *   Refactor the following atoms to match the `PureAtomFn` signature, implementing all logic directly within the function body. Use the existing `ATOM_EQ` as a template for clarity and directness.
        *   `ATOM_GT`
        *   `ATOM_LT`
        *   `ATOM_GTE`
        *   `ATOM_LTE`
        *   `ATOM_NOT`
    *   The refactored atoms should handle their own arity checks and type extraction using the existing, non-`_pure` helpers like `arity_error` and `extract_number`.

**Example of a correctly refactored `ATOM_GT`:**

```rust
pub const ATOM_GT: PureAtomFn = |args| {
    if args.len() != 2 {
        return Err(arity_error(None, args.len(), "gt?", 2));
    }
    let n1 = extract_number(&args[0], 0, None, "gt?")?;
    let n2 = extract_number(&args[1], 1, None, "gt?")?;
    Ok(Value::Bool(n1 > n2))
};
```

3.  **Verify:** Run `cargo test` to ensure that all tests pass after the refactor.
4.  **Continue Migration:** Proceed with refactoring other modules (`collections.rs`, etc.) following this same principle of direct implementation.

**Phase 3: Final Cleanup (No Change)**

This phase remains as originally specified. Once all atoms are migrated, the `LegacyAtomFn` type and related code will be removed.

This revised plan ensures that the refactoring adheres to the core principle of pragmatic minimalism, resulting in a codebase that is simpler, more readable, and easier to maintain.