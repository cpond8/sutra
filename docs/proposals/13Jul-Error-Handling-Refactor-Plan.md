# Error Handling Refactor Plan: High-Impact Atomic Overhaul

This document outlines the approved three-stage plan to refactor the Sutra error handling system. The goal is to move the codebase to a stable, modern, and maintainable architecture in a minimal number of large, atomic steps, counteracting previous architectural drift.

**Important:** The objective is to complete the entire refactor in the fewest possible steps, minimizing the time the system spends in a transitional state and reducing the opportunity for model quality degradation or human error to introduce cascading failures.

---

## Stage 1: Atomic Isolation of Error Data & `thiserror` Integration

**Objective:** Surgically convert the error structs into pure data containers, breaking all legacy presentation logic at once to prepare for the new system.

- [x] **Add `thiserror` Dependency:** Add `thiserror = "1.0"` to the `[dependencies]` section of `Cargo.toml`.
- [x] **Refactor `src/syntax/error.rs`:**
  - [x] Add `use thiserror::Error;` to the top of the file.
  - [x] Add `#[derive(Error)]` to the `SutraError`, `SutraErrorKind`, and `EvalError` structs.
  - [x] Add `#[error("...")]` message attributes to all error variants to define their canonical, unformatted `Display` representation.
  - [x] Delete the entire `impl Display for SutraError` block.
  - [x] Delete the entire `impl Display for EvalError` block.
  - [x] Delete the `with_source` method from `SutraError`.
- [x] **Triage Unit Tests:**
  - [x] Run the test suite. Expect tests that assert on formatted strings to fail.
  - [x] Mark failing string-based tests as `#[ignore]` for later updates.
  - [x] Ensure tests that validate error _kinds_ or _codes_ still pass.

---

## Stage 2: Unified Diagnostic Presentation Layer

**Objective:** Build and deploy a new, centralized presentation engine, making the system fully functional and architecturally sound in a single step.

- [x] **Create `src/cli/diagnostics.rs`:**
  - [x] Create the new file.
  - [x] Define the `SutraDiagnostic<'a>` struct, which holds a reference to the `SutraError` and the source code string.
  - [x] Implement `impl<'a> SutraDiagnostic<'a> { pub fn new(...) }`.
- [x] **Implement `impl Display for SutraDiagnostic`:**
  - [x] This implementation will contain all presentation logic.
  - [x] Print the error's primary message (from its `thiserror`-derived `Display` trait).
  - [x] Print the source location (`[line:col]`) using the error's `span`.
  - [x] Implement a robust, multi-line code snippet generator that uses the `span` and source code to create a pointer (`^-- Here`).
  - [x] Add colorization using the `termcolor` crate.
- [x] **Atomically Swap Implementations:**
  - [x] In `src/main.rs`, replace the old error printing logic with `eprintln!("{}", SutraDiagnostic::new(&error, &source_code));`.
  - [x] In `src/cli/output.rs`, delete the now-unnecessary `print_error_to` function.
- [x] **Create Golden Master Tests:**
  - [x] Create a new test file for snapshot testing of diagnostic output.
  - [x] For each major error type, create a test that captures the full, colored output of `SutraDiagnostic` to a snapshot file.

---

## Stage 3: Purge and Seal

**Objective:** Eliminate all remnants of the legacy system and verify the new architecture is sealed and complete.

[x] **Perform Codebase Purge:**
[x] Globally search for and delete any remaining dead code related to the old error system (e.g., `extract_code_pointer` or other private helpers).
[x] **Update Ignored Tests:**
[x] Re-enable the tests that were marked as `#[ignore]` in Stage 1.
[x] Update their assertions to match the new, correct output from `SutraDiagnostic`.
[x] **Final Verification:**
[x] Run the full test suite, including the golden master snapshot tests, and ensure all tests pass.
[x] Run `cargo clippy -- -D dead_code` to programmatically confirm no dead error-handling code remains.

---

**Stage 3 complete. The error handling refactor is finished and the codebase is sealed.**
