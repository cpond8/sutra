# Sutra Test Suite & Harness

A comprehensive, author-focused testing framework for the Sutra engine, designed for maximal transparency, compositionality, and reliability. All tests are written in the Verse language, except for Rust-based CLI/integration regression tests.

---

# Sutra Test Suite & Harness

A comprehensive, author-focused testing framework for the Sutra engine, designed for maximal transparency, compositionality, and reliability. All tests are written in the Verse language, except for Rust-based CLI/integration regression tests.

---

## Test Suite Structure

- **core/**: Core language constructs (literals, collections, special forms, scoping)
- **builtins/**: Built-in functions that operate on values (arithmetic, comparison, logic, etc.)
- **world/**: World state manipulation macros (assignment, persistence)
- **control/**: Control flow and execution semantics (conditionals, execution, consistency)
- **io/**: Input/output operations (output, display)
- **syntax/**: Parsing and syntax validation (grammar errors)
- **cli_regression.rs**: Rust integration test for CLI error diagnostics

Each `.sutra` file is a suite of Verse-language tests for a specific feature or domain. `cli_regression.rs` is a Rust test that ensures CLI errors are always rendered as miette diagnostics.

---

## Test Types and Coverage

### Core Language Constructs

- **Purpose:** Validate fundamental language features and special forms
- **Examples:**
  - `literals.sutra`: Tests for numbers, booleans, strings, nil parsing and evaluation
  - `collections.sutra`: Tests for lists, quoting, and basic collection operations
  - `special_forms.sutra`: Tests for `define`, `lambda`, `let`, including closures and scoping
  - `scoping.sutra`: Tests for lexical scoping, closures, variable capture, and shadowing

### Built-in Functions

- **Purpose:** Validate all built-in functions that operate on values
- **Examples:**
  - `arithmetic.sutra`: Tests for `+`, `-`, `*`, `/`, `mod`, including edge cases and error handling
  - `comparison.sutra`: Tests for `eq?`, `gt?`, `lt?`, and their aliases, including type and arity errors
  - `logic.sutra`: Tests for `not`, truthiness, and arity errors
  - `string.sutra`: Tests for string manipulation functions like `str+`
  - `list.sutra`: Tests for `car`, `cdr`, `cons`, and other list operations
  - `random.sutra`: Tests for `rand` and other random functions

### World State Operations

- **Purpose:** Validate world state manipulation and persistence
- **Examples:**
  - `assignment.sutra`: Tests for `set!`, `get`, `del!`, `add!`, `sub!`, `inc!`, `dec!`, including error handling
  - `persistence.sutra`: Tests for world state consistency and persistence across operations

### Control Flow

- **Purpose:** Validate control flow constructs and execution semantics
- **Examples:**
  - `conditionals.sutra`: Tests for `if`, `cond`, including arity and type errors
  - `execution.sutra`: Tests for `do`, sequencing, and execution order
  - `consistency.sutra`: Ensures test and production execution paths are identical

### Input/Output

- **Purpose:** Validate input/output operations
- **Examples:**
  - `output.sutra`: Tests for `print`, `println`, `display`, including arity errors and output matching

### Syntax and Parsing

- **Purpose:** Validate parsing and syntax error handling
- **Examples:**
  - `parsing.sutra`: Tests for grammar error detection (unclosed lists, invalid escapes, etc.)

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

- **All Sutra tests:**
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

## Advanced Diagnostics: Multi-Label and Error Chaining

Sutra's error system now supports:
- **Multi-label diagnostics:** Errors can highlight multiple locations (spans) across one or more source files, with custom labels for each.
- **Error chaining:** Errors can wrap a cause (another error), and the full chain is rendered in diagnostics output.
- **Actionable help:** Errors can include detailed help messages, and help is aggregated from all levels of the error chain.

These features are tested in:
- **Rust unit tests:** See `src/diagnostics.rs` for direct tests of multi-label, chaining, and help rendering using `miette::Report`.
- **CLI regression test:** `tests/cli_regression.rs` ensures that all CLI errors are rendered as miette diagnostics, including error codes, labels, help, and source context.

**Edge cases** (errors with/without help, with/without cause, with/without labels) are also covered in the diagnostics unit tests.

---
