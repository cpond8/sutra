# Sutra Test Harness

A sophisticated, author-focused testing framework designed for narrative game engines, built on Sutra's core principles of minimalism, compositionality, and transparency. All tests are written in the Verse language.

## Philosophy

The Sutra test harness embodies the engine's core design philosophy:

- **Minimal but Complete**: Essential testing capabilities with zero redundancy.
- **Transparent**: Full visibility into the compilation pipeline and diagnostic output, now powered by miette for rich, actionable diagnostics at every stage.
- **Composable**: An extensible, homoiconic system where tests are first-class code.
- **Author-Ergonomic**: Simple syntax for common cases, powerful features for complex scenarios.

## Homoiconic Test Architecture

Sutra tests are defined directly in `.sutra` source files as first-class code, not as special comments. This homoiconic approach makes tests discoverable, extensible, and easy to manipulate with the same tools used for application code. The system is built on two primary macros and a single underlying atom.

1.  **`(test ...)` Macro**: The main entry point for defining a test case. It provides a clean, declarative syntax for naming a test, specifying its expectation, and providing the body to execute.
2.  **`(expect ...)` Macro**: A flexible, multivariadic macro used within `test` to define the expected outcome(s) and annotations. Each argument is a tagged form, and the order does not matter.
3.  **`(register-test! ...)` Atom**: The low-level primitive that the `test` macro expands into. It registers the fully-formed test case with the test harness's central registry for execution.

This design ensures that all test logic and metadata are valid, parsable Sutra code, aligning the testing framework perfectly with the language's philosophy.

### Defining Tests

Tests are written using a clear, Lisp-style syntax that keeps test logic and expectations tightly coupled.

```lisp
; Basic success test
(test "addition works"
      (expect (value 10))
      (+ 7 3))

; Error expectation test
(test "division by zero fails"
      (expect (error division-by-zero))
      (/ 10 0))

; A skipped test
(test "complex feature"
      (expect (skip "waiting for macro system improvements"))
      (complex-operation))
```

## Test Syntax Reference

### Core Macros

| Macro                          | Syntax                                  | Purpose                                                                 |
| ------------------------------ | --------------------------------------- | ----------------------------------------------------------------------- |
| `(test name expect-form body)` | `(test "my test" (expect ...) (+ 1 2))` | Defines a test case with a name, expectation, and body.                 |
| `(expect tagged [tagged...])`  | `(expect (value 42) (tags "math"))`     | Defines the expected outcome(s) and optional configuration annotations. |

### Expectation Types

The `(expect ...)` macro now supports a tagged, multivariadic, order-insensitive syntax. Each argument is a tagged form, and the order does not matter. You may specify as many as you want; the harness will interpret them all.

| Tag      | Purpose                         | Example                      |
| -------- | ------------------------------- | ---------------------------- |
| value    | Expected value                  | `(value 42)`                 |
| error    | Expected error (code, msg, ...) | `(error type-error "msg")`   |
| output   | Expected output                 | `(output "foo\n")`           |
| params   | Parameterization                | `(params ((1 2 3) ...))`     |
| skip     | Skip with reason                | `(skip "wip")`               |
| tags     | Tagging                         | `(tags "math" "regression")` |
| timeout  | Timeout in ms                   | `(timeout 5000)`             |
| fixture  | Fixture setup                   | `(fixture "player_setup")`   |
| group    | Grouping                        | `(group "math/advanced")`    |
| snapshot | Snapshot assertion              | `(snapshot "file.txt")`      |

Legacy positional forms are still supported for compatibility, but new tests should use the tagged syntax for clarity and extensibility.

---

## Assertion Examples

### Value with Params and Tags

```lisp
(test "addition cases"
      (expect
        (value (nth @params 2))
        (params ((1 2 3) (5 5 10) (-1 1 0)))
        (tags "math" "parametric"))
      (+ (nth @params 0) (nth @params 1)))
```

### Error with Message and Skip

```lisp
(test "error with message"
      (expect
        (error type-error "expected Number")
        (skip "waiting for macro system improvements"))
      (+ 1 @param))
```

### Output and Timeout

```lisp
(test "print output with timeout"
      (expect
        (output "hello\n")
        (timeout 1000))
      (print "hello"))
```

### Snapshot Assertion

```lisp
(test "diagnostic snapshot"
      (expect
        (snapshot "snapshots/math_addition.txt"))
      (+ 1 2 3 4))
```

## Parameterized Testing

Run the same test logic with multiple input sets using the `(params ...)` annotation. The special variables `@param` (for single-item lists) or `(nth @params N)` give access to the data.

```lisp
(test "addition cases"
      (expect
        (value (nth @params 2))
        (params ((1 2 3) (5 5 10) (-1 1 0))))
      (+ (nth @params 0) (nth @params 1)))

(test "error cases"
      (expect
        (error type-error)
        (params ("string" true nil)))
      (+ 1 @param))
```

## Canonical Error Codes

The test harness uses Sutra's canonical error codes for stable test matching. All errors are surfaced as miette diagnostics, with code, message, span, and help text where available.

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
