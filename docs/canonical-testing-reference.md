# Sutra Engine Testing System Reference

---

## Overview

The Sutra Engine employs a robust, protocol-compliant testing system to ensure correctness, maintainability, and author-facing reliability. This document provides a comprehensive reference for the test architecture, file formats, error types, execution flow, and best practices.

---

## 1. Test Suite Structure

- **Location:** All protocol-compliant tests reside in the `tests/scripts/` directory, organized by feature (e.g., `atoms`, `macros`, `parser`, `integration`, `examples`, `engine`).
- **Test Pairing:** Each test consists of a `.sutra` script and a corresponding `.expected` output file. Both must exist for a test to be valid.
- **Rust-based tests:** Additional unit and integration tests are in `tests/unit_tests.rs`, `tests/eval_tests.rs`, and `tests/harness.rs`.

---

## 2. Test Protocol & Execution

- **Runner:** The canonical runner is implemented in `src/test_utils.rs` and invoked by Rust test modules and the CLI.
- **Discovery:** All `.sutra` files in a directory (and subdirectories) are paired with `.expected` files. Missing pairs are reported as errors.
- **Execution:**
  1. The test runner executes the Sutra binary (`sutra run <file.sutra>`).
  2. Captures stdout (or stderr if stdout is empty).
  3. Compares normalized output to the `.expected` file.
  4. Reports pass/fail, with a diff if enabled.
- **Normalization:** Whitespace is normalized by default for robust comparison.
- **Diffing:** On failure, a line-by-line diff is shown (if enabled in config).

---

## 3. Test Configuration

- **Config struct:** `TestConfig` (see `src/test_utils.rs`)
  - `binary_path`: Path to the Sutra binary (default: `./target/debug/sutra`).
  - `normalize_whitespace`: Normalize whitespace before comparison (default: true).
  - `show_diff`: Show diff output on failure (default: true).

---

## 4. Error Types & Diagnostics

- **Error Types:**
  - `EvalError`: Arity, type, division by zero, or general evaluation errors.
  - `ValidationErrorKind`: Recursion limit, general validation errors.
  - `SutraErrorKind`: Parse, validation, eval, IO, malformed AST, macro expansion, internal parse errors.
- **Diagnostics:**
  - All errors and diagnostics carry a `span` (source location).
  - `SutraDiagnostic`: Contains severity (`Error`, `Warning`, `Info`), message, and span.
  - All errors are user-friendly and protocol-compliant.
- **Error Reporting:**
  - Error messages must start with the rule name and describe expected vs. found.
  - Never use `.unwrap()`, `.expect()`, or `panic!` in production code.

---

## 5. Authoring & Syntax

- **Test Scripts:** Use either brace-block or s-expression syntax. Both are canonical and interchangeable.
- **Macros:** Prefer macro-based authoring for clarity and maintainability. See `docs/architecture/authoring-patterns.md` for idioms.
- **Example Test Script:**
  ```
  storylet "find-key" {
    and {
      is? player.location "cellar"
      has? player.items "rusty-key"
    }
    do {
      print "You unlock the door."
      set! world.door.unlocked true
    }
  }
  ```
- **Expected Output:** Should match the observable output of the script (text, world state, etc.).

---

## 6. CLI & Automation

- **Run all tests:**
  ```sh
  cargo test
  # or
  sutra test [path]
  ```
- **Generate expected output:**
  ```sh
  sutra gen-expected path/to/file.sutra
  sutra gen-expected path/to/directory/
  ```
  _Warning: Overwrites existing `.expected` files._

---

## 7. Extending & Maintaining the Test System

- **Add new tests:** Place `.sutra` and `.expected` files in the appropriate `tests/scripts/` subdirectory.
- **Add new error types:** Extend `src/syntax/error.rs` and update helpers.
- **Update macro patterns:** Document and test new macros in `docs/architecture/authoring-patterns.md`.
- **Refactor test logic:** Update `src/test_utils.rs` and ensure all protocol invariants are preserved.

---

## 8. Best Practices & Invariants

- All test cases must be self-contained and reproducible.
- Never mutate global state or rely on side effects outside the world model.
- All errors must be span-carrying and user-friendly.
- Prefer macro-based authoring for clarity and maintainability.
- Keep `.expected` files up to date with `sutra gen-expected` after intentional changes.

---

## 9. References

- `src/test_utils.rs` — Test runner, config, normalization, diffing
- `tests/eval_tests.rs`, `tests/unit_tests.rs`, `tests/harness.rs` — Test entry points
- `src/syntax/error.rs`, `src/syntax/validate.rs`, `src/syntax/validator.rs` — Error types and diagnostics
- `docs/architecture/architecture.md`, `docs/architecture/authoring-patterns.md` — System and authoring patterns

---

This document is canonical. Update it with any changes to the test protocol, error types, or authoring patterns.
