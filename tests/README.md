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

## Full Pipeline Execution

Each test executes the complete Sutra compilation pipeline, with miette-powered diagnostics at every stage:

1.  **Parse** → AST generation with error detection (miette diagnostics on error)
2.  **Validate** → Semantic analysis and warning generation (miette diagnostics on error)
3.  **Expand** → Macro expansion with recursion limits (miette diagnostics on error)
4.  **Evaluate** → Runtime execution with state isolation (miette diagnostics on error)
5.  **Capture Output** → All output is buffered and compared if needed
6.  **Collect Diagnostics** → All errors, warnings, and suggestions are captured and surfaced via miette

### Test Isolation

Every test runs in a completely fresh execution environment:

- Clean world state
- Fresh macro environment
- Isolated output buffers
- Independent diagnostic collection

## Test Syntax Reference

### Core Macros

| Macro                           | Syntax                                  | Purpose                                                              |
| ------------------------------- | --------------------------------------- | -------------------------------------------------------------------- |
| `(test name expect-form body)`  | `(test "my test" (expect ...) (+ 1 2))` | Defines a test case with a name, expectation, and body.              |
| `(expect tagged [tagged...])`   | `(expect (value 42) (tags "math"))`     | Defines the expected outcome(s) and optional configuration annotations. |

### Expectation Types

The `(expect ...)` macro now supports a tagged, multivariadic, order-insensitive syntax. Each argument is a tagged form, and the order does not matter. You may specify as many as you want; the harness will interpret them all.

| Tag      | Purpose                        | Example                                 |
|----------|--------------------------------|-----------------------------------------|
| value    | Expected value                 | `(value 42)`                            |
| error    | Expected error (code, msg, ...) | `(error type-error "msg")`              |
| output   | Expected output                | `(output "foo\n")`                      |
| params   | Parameterization               | `(params ((1 2 3) ...))`                |
| skip     | Skip with reason               | `(skip "wip")`                          |
| tags     | Tagging                        | `(tags "math" "regression")`            |
| timeout  | Timeout in ms                  | `(timeout 5000)`                        |
| fixture  | Fixture setup                  | `(fixture "player_setup")`               |
| group    | Grouping                       | `(group "math/advanced")`               |
| snapshot | Snapshot assertion             | `(snapshot "file.txt")`                 |

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

## Fixtures and Setup

Define reusable world state with fixtures and apply them to tests with the `(fixture ...)` annotation.

```lisp
(fixture "player_setup" {
  player: {health: 100, mana: 50}
})

test "health check"
      (expect
        (value 100)
        (fixture "player_setup"))
      (get player.health))

test "damage calculation"
      (expect
        (value 75)
        (fixture "player_setup"))
      (do
        (set! player.health (- (get player.health) 25))
        (get player.health)))
```

## Output Capture

Assert on printed output and other side effects using the `(output ...)` expectation.

```lisp
(test "print output"
      (expect (output "Hello, World!\n"))
      (print "Hello, World!"))

test "multiple prints"
      (expect (output ["Starting..." "Complete."]))
      (do
        (print "Starting...")
        (print "Complete.")))
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

## Harness Module Structure and Philosophy Alignment

The test harness is implemented as a set of small, focused modules in `tests/harness/`, each with a single responsibility. This structure maximizes minimalism, compositionality, transparency, and extensibility, as described in the project philosophy.

| Module         | Responsibility                                                      |
|--------------- |---------------------------------------------------------------------|
| `mod.rs`       | Root orchestrator, re-exports submodules, documents overall design   |
| `parse.rs`     | Pure, stateless parsing of test and expectation forms               |
| `expectation.rs`| Expectation enum, tag logic, and matching                          |
| `runner.rs`    | Full test execution pipeline: parse → validate → expand → eval      |
| `reporting.rs` | Miette-powered reporting and diagnostics rendering                  |
| `isolation.rs` | World/macro/output isolation for test purity                        |
| `legacy.rs`    | Legacy compatibility for old positional forms                       |
| `util.rs`      | Small, pure helpers (parameter expansion, tag extraction, etc.)     |
| `snapshot.rs`  | Snapshot testing logic (feature-gated, extensible)                  |

- **Minimalism:** Each module does one thing, with no cross-layer leakage.
- **Compositionality:** All logic is pure and reusable; pipeline is built from composable stages.
- **Transparency:** All diagnostics and results are surfaced via miette, with full context.
- **Extensibility:** New tags, pipeline stages, and features are easy to add by extending the relevant module.
- **Encapsulation:** Legacy support is isolated and can be removed cleanly in the future.

This modular structure ensures that the harness is easy to understand, extend, and maintain, and that it remains a first-class example of Sutra's design philosophy in practice.

## File Organization

### Canonical File Organization

Test files mirror the structure of the `src/` directory. Each module’s tests live under a same-named folder in `tests/`, with a shared `common/` directory for cross-cutting scenarios:

```text
tests/
├── atoms/         ← mirrors `src/atoms`
├── cli/           ← mirrors `src/cli`
├── common/        ← cross-cutting & integration tests
├── harness/       ← test harness implementation (discovery, runner, snapshots, reporting)
├── macros/        ← mirrors `src/macros`
├── runtime/       ← mirrors `src/runtime`
└── syntax/        ← mirrors `src/syntax`
```

## Test Runner Configuration

The behavior of the test harness can be configured via environment variables and CLI options.

### Environment Variables

| Variable           | Purpose                           | Example                         |
| ------------------ | --------------------------------- | ------------------------------- |
| `TEST_FILTER`      | Filter tests by name/pattern      | `TEST_FILTER="math" cargo test` |
| `UPDATE_SNAPSHOTS` | Update snapshot files on mismatch | `UPDATE_SNAPSHOTS=1 cargo test` |
| `TEST_TIMEOUT`     | Default timeout in milliseconds*  | `TEST_TIMEOUT=10000 cargo test` |
| `TEST_PARALLEL`    | Enable parallel execution*        | `TEST_PARALLEL=1 cargo test`    |

### CLI Options

| Option               | Short | Purpose              | Example               |
| -------------------- | ----- | -------------------- | --------------------- |
| `--filter`           | `-f`  | Filter by pattern    | `--filter "addition"` |
| `--update-snapshots` | `-u`  | Update snapshots     | `--update-snapshots`  |
| `--timeout`          | `-t`  | Set timeout*         | `--timeout 5000`      |
| `--verbose`          | `-v`  | Detailed output*     | `--verbose`           |
| `--parallel`         | `-p`  | Parallel execution*  | `--parallel 4`        |

## Planned Extensions

### Property-Based Testing*

Randomized testing with property verification.

```lisp
(property "addition is commutative"
          (forall x:int y:int)
          (expect (value (eq? (+ x y) (+ y x)))))
```

## Legend

**✓** = Fully implemented and tested
**\*** = Planned feature, not yet implemented
**⚠** = Experimental or unstable feature

This documentation reflects the current state as of 16 July 2025. Features marked with asterisks (*) represent planned capabilities that align with Sutra's design philosophy but require implementation.
