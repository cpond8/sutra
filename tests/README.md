# Sutra Test Suite & Harness

A comprehensive, author-focused testing framework for the Sutra engine, designed for maximal transparency, compositionality, and reliability. All tests are written in the Verse language, except for Rust-based CLI/integration regression tests.

---

## Test Suite Structure

- **atoms/**: Primitive operations (math, logic, comparison, string, list, etc.)
- **macros/**: Macro expansion, assignment, and world state macros
- **runtime/**: Control flow, consistency, and world state
- **syntax/**: Parsing, grammar, and security
- **io/**: Output and display atoms/macros
- **cli_regression.rs**: Rust integration test for CLI error diagnostics

Each `.sutra` file is a suite of Verse-language tests for a specific feature or domain. `cli_regression.rs` is a Rust test that ensures CLI errors are always rendered as miette diagnostics.

---

## Test Types and Coverage

### Atoms
- **Purpose:** Validate all primitive operations (math, logic, comparison, string, list, etc.)
- **Examples:**
  - `math.sutra`: Tests for `+`, `-`, `*`, `/`, `mod`, including edge cases and error handling
  - `logic.sutra`: Tests for `not`, truthiness, and arity errors
  - `comparison.sutra`: Tests for `eq?`, `gt?`, `lt?`, and their aliases, including type and arity errors
  - `define.sutra`: Tests for function/variable definition, closures, variadics, and error cases

### Macros
- **Purpose:** Validate macro expansion, assignment, and world state manipulation
- **Examples:**
  - `assignment.sutra`: Tests for `set!`, `get`, `del!`, `add`, `sub`, `inc!`, `dec!`, including error handling

### Runtime
- **Purpose:** Validate control flow, consistency, and world state
- **Examples:**
  - `control.sutra`: Tests for `if`, `do`, `cond`, including arity and type errors
  - `consistency.sutra`: Ensures test and production execution paths are identical

### Syntax
- **Purpose:** Validate parsing, grammar, and security
- **Examples:**
  - `parsing.sutra`: Tests for numbers, booleans, strings, lists, blocks, and error cases (unclosed, invalid, etc.)
  - `security.sutra`: Tests for path traversal and invalid path handling

### IO
- **Purpose:** Validate output atoms/macros
- **Examples:**
  - `output.sutra`: Tests for `print` and `display`, including arity errors and output matching

### CLI Regression
- **Purpose:** Ensure all CLI errors are rendered as miette diagnostics
- **Implementation:**
  - `cli_regression.rs`: Runs the CLI with invalid input and asserts that the output contains miette-formatted diagnostics (error codes, labels, help, and source context)

---

## Test Philosophy and Features

- **Homoiconic:** All tests are valid Sutra code, not comments or metadata
- **Composable:** Tests use macros like `(test ...)` and `(expect ...)` for clarity and extensibility
- **Transparent:** All errors are surfaced as miette diagnostics, with full context, code, and help
- **Automated Quality Gate:** The Rust regression test ensures no regressions in CLI error reporting

---

## How to Run the Test Suite

- **All Verse tests:**
  ```sh
  cargo test
  ```
  This runs all `.sutra` tests via the test harness and the Rust CLI regression test.

- **CLI regression only:**
  ```sh
  cargo test --test cli_regression
  ```

---

## Canonical Error Codes

All errors use canonical codes (e.g., `ParseError`, `ValidationError`, `TypeError`, `DivisionByZero`, etc.) for stable, meaningful diagnostics. See the table below for all codes and their meanings.

| Error Code               | Description                                  | Typical Use Case                    |
| ------------------------ | -------------------------------------------- | ----------------------------------- |
| `ParseError`             | Syntax or parsing failures                   | Malformed input, unexpected tokens  |
| `ValidationError`        | Post-expansion validation failures           | Semantic errors, invalid structures |
| `RecursionLimitExceeded` | Macro expansion or evaluation depth exceeded | Infinite recursion detection        |
| `ArityError`             | Function argument count mismatch             | Wrong number of arguments           |
| `TypeError`              | Type mismatch errors                         | Wrong value types for operations    |
| `DivisionByZero`         | Division by zero operations                  | Mathematical errors                 |
| `EvalError`              | General evaluation failures                  | Runtime errors not covered above    |
| `IoError`                | File or system I/O failures                  | File read/write errors              |
| `MalformedAstError`      | Internal AST structure errors                | Parser or AST construction bugs     |
| `InternalParseError`     | Internal parser state errors                 | Parser implementation bugs          |

---

## Example Test File Structure

```lisp
(test "feature description"
      (expect (value 42) (tags "math"))
      (+ 40 2))
```

---

## Extending the Test Suite

- Add new `.sutra` files in the appropriate subdirectory for new features or bug fixes
- Use the tagged `(expect ...)` syntax for new tests
- For CLI or integration-level checks, add Rust tests in the top-level `tests/` directory

---

## Cross-References

- For CLI philosophy and user experience, see the main [`README.md`](../README.md)
- For language reference and canonical patterns, see `docs/`

---

## Automated CLI Regression Test

Sutra includes an automated Rust regression test (`cli_regression.rs`) that ensures all CLI errors are rendered as miette diagnostics. This test runs the CLI with deliberately invalid input and asserts that the output contains miette-formatted diagnostics (error codes, labels, help, and source context). This serves as a quality gate, preventing regressions in user-facing error reporting and guaranteeing actionable, context-rich feedback for all CLI errors.

---
