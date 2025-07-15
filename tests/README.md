# Sutra Test Harness

A sophisticated, author-focused testing framework designed for narrative game engines, built on Sutra's core principles of minimalism, compositionality, and transparency. All tests are written in the Verse language.

## Philosophy

The Sutra test harness embodies the engine's core design philosophy:

- **Minimal but Complete**: Essential testing capabilities with zero redundancy.
- **Transparent**: Full visibility into the compilation pipeline and diagnostic output.
- **Composable**: An extensible, homoiconic system where tests are first-class code.
- **Author-Ergonomic**: Simple syntax for common cases, powerful features for complex scenarios.

## Homoiconic Test Architecture

Sutra tests are defined directly in `.sutra` source files as first-class code, not as special comments. This homoiconic approach makes tests discoverable, extensible, and easy to manipulate with the same tools used for application code. The system is built on two primary macros and a single underlying atom.

1.  **`(test ...)` Macro**: The main entry point for defining a test case. It provides a clean, declarative syntax for naming a test, specifying its expectation, and providing the body to execute.
2.  **`(expect ...)` Macro**: A flexible, variadic macro used within `test` to define the expected outcome. Its first argument is the core expectation (a value, an error type, etc.), and all subsequent arguments are optional annotations that configure the test's behavior (e.g., skipping, tagging, or parameterizing).
3.  **`(register-test! ...)` Atom**: The low-level primitive that the `test` macro expands into. It registers the fully-formed test case with the test harness's central registry for execution.

This design ensures that all test logic and metadata are valid, parsable Sutra code, aligning the testing framework perfectly with the language's philosophy.

### Defining Tests

Tests are written using a clear, Lisp-style syntax that keeps test logic and expectations tightly coupled.

```lisp
; Basic success test
(test "addition works"
      (expect 10)
      (+ 7 3))

; Error expectation test
(test "division by zero fails"
      (expect division-by-zero)
      (/ 10 0))

; A skipped test
(test "complex feature"
      (expect success (skip "waiting for macro system improvements"))
      (complex-operation))
```

### Ariadne-Centered Diagnostics

All test assertions are based on **unified diagnostic output** rendered through [Ariadne](https://github.com/zesterer/ariadne), ensuring consistency with the compiler's error reporting and eliminating custom assertion logic.

### Full Pipeline Execution

Each test executes the complete Sutra compilation pipeline:

1.  **Parse** → AST generation with error detection
2.  **Validate** → Semantic analysis and warning generation
3.  **Expand** → Macro expansion with recursion limits
4.  **Evaluate** → Runtime execution with state isolation

### Test Isolation

Every test runs in a completely fresh execution environment:

- Clean world state
- Fresh macro environment
- Isolated output buffers
- Independent diagnostic collection

## Running Tests

### Integration with Cargo

```bash
# Run all tests through cargo
cargo test

# Run with filter
TEST_FILTER="addition" cargo test

# Update snapshots
UPDATE_SNAPSHOTS=1 cargo test
```

### Standalone Test Harness

```bash
# Run standalone harness
cargo run --bin harness

# With filtering and snapshot updates
cargo run --bin harness -- --filter "math" --update-snapshots
```

## Test Syntax Reference

### Core Macros

| Macro                           | Syntax                                  | Purpose                                                              |
| ------------------------------- | --------------------------------------- | -------------------------------------------------------------------- |
| `(test name expect-form body)`  | `(test "my test" (expect ...) (+ 1 2))` | Defines a test case with a name, expectation, and body.              |
| `(expect expectation [ann...])` | `(expect 42 (tags "math"))`             | Defines the expected outcome and optional configuration annotations. |

### Expectation Types

The first argument to the `(expect ...)` macro defines what the test's outcome should be.

| Type                  | Syntax                           | Purpose                                                    |
| --------------------- | -------------------------------- | ---------------------------------------------------------- |
| **Value Assertion**   | `(expect <value>)`               | Asserts that the test body evaluates to an exact value.    |
| **Error Assertion**   | `(expect <error-symbol>)`        | Asserts that the test fails with a specific error type.    |
| **Complex Assertion** | `(expect (or <sym1> <sym2>))`    | Asserts a more complex condition using `and`, `or`, `not`. |
| **Output Assertion**  | `(expect (output "text"))`       | Asserts that the test body prints specific text to stdout. |
| **Success**           | `(expect success)`               | Asserts that the test completes without any errors.        |
| **Snapshot**          | `(expect (snapshot "path.txt"))` | Compares diagnostic output against a saved snapshot.       |

### Annotations

Optional forms passed to `(expect ...)` after the main expectation to configure test behavior.

| Annotation         | Syntax                       | Purpose                                                |
| ------------------ | ---------------------------- | ------------------------------------------------------ |
| `(skip "reason")`  | `(skip "wip")`               | Skips the test, providing an optional reason.          |
| `(only)`           | `(only)`                     | Exclusively runs tests with this annotation.           |
| `(tags "t1" "t2")` | `(tags "slow" "db")`         | Assigns tags for filtering.                            |
| `(group "path")`   | `(group "math/advanced")`    | Organizes tests into hierarchical groups.              |
| `(timeout <ms>)`   | `(timeout 5000)`             | Sets a custom timeout for the test in milliseconds.    |
| `(params ((...)))` | `(params ((1 2 3) (4 5 9)))` | Runs the test multiple times with different data sets. |
| `(fixture "name")` | `(fixture "player_setup")`   | Initializes the world state from a named fixture.      |

## Assertion Examples

### Direct Value Assertions

Use Sutra's native syntax for clear and ergonomic value checks.

```lisp
(test "string equality"
      (expect "hello world")
      (str+ "hello" " " "world"))

(test "list result"
      (expect (1 2 3))
      (list 1 2 3))
```

For more complex matching, use assertion helpers within the `expect` form:

```lisp
(test "range check"
      (expect (and (gt? 7) (lt? 10)))
      (+ 1 7))
```

### Error Assertions

Use canonical error symbols for stable and readable error tests.

```lisp
(test "division by zero"
      (expect division-by-zero)
      (/ 10 0))

(test "type error"
      (expect type-error)
      (+ 1 "string"))
```

Combine assertions for more complex error matching:

```lisp
(test "complex error matching"
      (expect (or arity-error type-error))
      (risky-operation))

(test "error with message check"
      (expect (and type-error (message-contains "expected Number")))
      (+ 1 "string"))
```

#### Canonical Error Codes

The test harness uses Sutra's canonical error codes for stable test matching:

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

## Parameterized Testing

Run the same test logic with multiple input sets using the `(params ...)` annotation. The special variables `@param` (for single-item lists) or `(nth @params N)` give access to the data.

```lisp
(test "addition cases"
      (expect (nth @params 2)
              (params ((1 2 3) (5 5 10) (-1 1 0))))
      (+ (nth @params 0) (nth @params 1)))

(test "error cases"
      (expect type-error
              (params ("string" true nil)))
      (+ 1 @param))
```

## Fixtures and Setup

Define reusable world state with fixtures and apply them to tests with the `(fixture ...)` annotation.

```lisp
(fixture "player_setup" {
  player: {health: 100, mana: 50}
})

(test "health check"
      (expect 100 (fixture "player_setup"))
      (get player.health))

(test "damage calculation"
      (expect 75 (fixture "player_setup"))
      (do
        (set! player.health (- (get player.health) 25))
        (get player.health)))
```

## Output Capture

Assert on printed output and other side effects using the `(output ...)` expectation.

````lisp
(test "print output"
      (expect (output "Hello, World!\n"))
      (print "Hello, World!"))

(test "multiple prints"
      (expect (output ["Starting..." "Complete."]))
## File Organization

### Canonical File Organization

Test files now mirror the structure of the `src/` directory. Each module’s tests live under a same-named folder in `tests/`, with a shared `common/` directory for cross-cutting scenarios:

```text
tests/
├── atoms/         ← mirrors `src/atoms`
├── cli/           ← mirrors `src/cli`
├── common/        ← cross-cutting & integration tests
├── harness/       ← test harness implementation (discovery, runner, snapshots, reporting)
├── macros/        ← mirrors `src/macros`
├── runtime/       ← mirrors `src/runtime`
└── syntax/        ← mirrors `src/syntax`
````

## Test Runner Configuration

The behavior of the test harness can be configured via environment variables and CLI options.

### Environment Variables

| Variable           | Purpose                           | Example                         |
| ------------------ | --------------------------------- | ------------------------------- |
| `TEST_FILTER`      | Filter tests by name/pattern      | `TEST_FILTER="math" cargo test` |
| `UPDATE_SNAPSHOTS` | Update snapshot files on mismatch | `UPDATE_SNAPSHOTS=1 cargo test` |
| `TEST_TIMEOUT`     | Default timeout in milliseconds\* | `TEST_TIMEOUT=10000 cargo test` |
| `TEST_PARALLEL`    | Enable parallel execution\*       | `TEST_PARALLEL=1 cargo test`    |

### CLI Options

| Option               | Short | Purpose              | Example               |
| -------------------- | ----- | -------------------- | --------------------- |
| `--filter`           | `-f`  | Filter by pattern    | `--filter "addition"` |
| `--update-snapshots` | `-u`  | Update snapshots     | `--update-snapshots`  |
| `--timeout`          | `-t`  | Set timeout\*        | `--timeout 5000`      |
| `--verbose`          | `-v`  | Detailed output\*    | `--verbose`           |
| `--parallel`         | `-p`  | Parallel execution\* | `--parallel 4`        |

      (do
        (print "Starting...")
        (print "Complete.")))

````

## Test Organization

Organize tests with hierarchical groups and tags for better management and filtering.

```lisp
(test "complex math operation"
      (expect ...
              (group "math/advanced")
              (tags "slow" "integration")
              (timeout 5000))
      (complex-calculation))
````

## Planned Extensions

### Property-Based Testing\*

Randomized testing with property verification.

```lisp
(property "addition is commutative"
          (forall x:int y:int)
          (expect (eq? (+ x y) (+ y x))))
```

## Legend

**✓** = Fully implemented and tested
**\*** = Planned feature, not yet implemented
**⚠** = Experimental or unstable feature

This documentation reflects the current state as of July 2025. Features marked with asterisks (\*) represent planned capabilities that align with Sutra's design philosophy but require implementation.
