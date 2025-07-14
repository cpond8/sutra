# Sutra Test Harness

A sophisticated, author-focused testing framework designed for narrative game engines, built on Sutra's core principles of minimalism, compositionality, and transparency.

## Philosophy

The Sutra test harness embodies the engine's core design philosophy:

- **Minimal but Complete**: Essential testing capabilities with zero redundancy
- **Transparent**: Full visibility into compilation pipeline and diagnostic output
- **Composable**: Extensible annotation system that mirrors Sutra's "atoms + macros" approach
- **Author-Ergonomic**: Simple syntax for common cases, powerful features for complex scenarios

## Current Architecture (Implemented)

### Source-Embedded Annotations

Tests are defined directly in `.sutra` source files using special comment annotations, making them self-documenting and keeping test logic close to the code being tested.

```lisp
; Basic success test
//! @test "addition works"
//! @expect success
(+ 2 3)

; Error expectation test
//! @test "division by zero fails"
//! @expect eval_error messages=["division by zero"]
(/ 10 0)

; Skip problematic test
//! @test "complex feature"
//! @skip "waiting for macro system improvements"
(complex-operation)
```

### Ariadne-Centered Diagnostics

All test assertions are based on **unified diagnostic output** rendered through [Ariadne](https://github.com/zesterer/ariadne), ensuring consistency with the compiler's error reporting and eliminating custom assertion logic.

### Full Pipeline Execution

Each test executes the complete Sutra compilation pipeline:

1. **Parse** → AST generation with error detection
2. **Validate** → Semantic analysis and warning generation
3. **Expand** → Macro expansion with recursion limits
4. **Evaluate** → Runtime execution with state isolation

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

## Annotation Reference

### Core Annotations (Implemented)

| Annotation | Syntax                  | Purpose                               | Example                   |
| ---------- | ----------------------- | ------------------------------------- | ------------------------- |
| `@test`    | `@test "name"`          | Define a test case with optional name | `//! @test "basic math"`  |
| `@expect`  | `@expect TYPE [params]` | Set test expectation                  | `//! @expect success`     |
| `@skip`    | `@skip ["reason"]`      | Skip test execution                   | `//! @skip "known issue"` |
| `@only`    | `@only`                 | Run only this test (exclusive)        | `//! @only`               |

### Expectation Types (Implemented)

| Type               | Syntax                     | Purpose                                 | Example                   |
| ------------------ | -------------------------- | --------------------------------------- | ------------------------- |
| `success`          | `@expect success`          | Test should complete without errors     | Basic functionality tests |
| `parse_error`      | `@expect parse_error`      | Should fail during parsing              | Invalid syntax tests      |
| `validation_error` | `@expect validation_error` | Should fail during validation           | Semantic error tests      |
| `eval_error`       | `@expect eval_error`       | Should fail during evaluation           | Runtime error tests       |
| `snapshot`         | `@expect snapshot "path"`  | Compare against saved diagnostic output | Regression tests          |

**Note:** All error expectations support optional `codes=` and `messages=` parameters for precise matching against canonical error codes and message content.

### Error Matching (Implemented)

Current string-based error matching (to be improved):

```lisp
//! @test "current approach - string-based"
//! @expect eval_error codes=["ArityError", "TypeError"]
(+ 1 "not a number")
```

**Planned Ergonomic Improvements\***

Replace string-based codes with Sutra's native symbolic syntax:

```lisp
//! @test "division by zero"
//! @expect division-by-zero
(/ 10 0)

//! @test "type error"
//! @expect type-error
(+ 1 "string")

//! @test "arity error"
//! @expect arity-error
(/)   ; division requires at least 1 argument, zero args triggers arity error

//! @test "complex error matching"
//! @expect (or arity-error type-error)
(risky-operation)

//! @test "error with message check"
//! @expect (and type-error (message-contains "expected Number"))
(+ 1 "string")
```

Benefits of symbolic error matching:

- **Native Sutra syntax**: Uses symbols instead of error-prone strings
- **Composable**: Can use `and`, `or`, `not` for complex conditions
- **IDE-friendly**: Symbol validation and autocomplete support
- **Consistent**: Aligns with Sutra's homoiconic philosophy

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

#### Error Code Usage Examples

```lisp
//! @test "arity error with specific code"
//! @expect eval_error codes=["ArityError"]
(+)  ; Addition requires at least 0 args, but this is malformed

//! @test "type error with message"
//! @expect eval_error codes=["TypeError"] messages=["expected Number"]
(+ 1 "string")

//! @test "multiple error possibilities"
//! @expect eval_error codes=["ArityError", "TypeError"]
(complex-operation)

//! @test "parse error with location"
//! @expect parse_error codes=["ParseError"] messages=["unexpected token"]
(unclosed (expression
```

**Note:** Error codes provide stable identifiers for test matching, independent of message text changes during development. Use `codes=` for structural error matching and `messages=` for content-specific validation.

## Planned Extensions (Not Yet Implemented)

### Direct Value Assertions\*

_Direct value assertions using Sutra's native syntax for maximum ergonomics._

```lisp
//! @test "simple value check"
//! @expect 10
(+ 3 7)

//! @test "boolean result"
//! @expect true
(gt? 5 3)

//! @test "string equality"
//! @expect "hello world"
(str+ "hello" " " "world")

//! @test "list result"
//! @expect (1 2 3)
(list 1 2 3)

//! @test "nil result"
//! @expect nil
(get nonexistent.key)
```

**Advanced Value Assertions\***

For complex value matching, use Sutra's s-expression syntax:

```lisp
//! @test "range check"
//! @expect (and (gt? 7) (lt? 10))
(+ 1 7)

//! @test "type and value check"
//! @expect (and (number) (eq? 8))
(+ 1 7)

//! @test "list structure check"
//! @expect (and (list) (eq? (len) 3))
(list 1 2 3)
```

### Parameterized Testing\*

_Run the same test logic with multiple input sets using Sutra's list syntax._

```lisp
//! @test "addition cases"
//! @params ((1 2 3) (5 5 10) (-1 1 0))
//! @expect (nth @params 2)
(+ (nth @params 0) (nth @params 1))

//! @test "error cases"
//! @params ("string" true nil)
//! @expect type-error
(+ 1 @param)

//! @test "comparison cases"
//! @params ((5 3 true) (2 8 false) (10 10 false))
//! @expect (nth @params 2)
(gt? (nth @params 0) (nth @params 1))
```

### Fixtures and Setup\*

_Reusable world state initialization using Sutra's native map syntax._

```lisp
//! @fixture "player_setup"
//! @world {player: {health: 100, mana: 50}}

//! @test "health check"
//! @use_fixture "player_setup"
//! @expect 100
(get player.health)

//! @test "damage calculation"
//! @use_fixture "player_setup"
//! @expect 75
(do
  (set! player.health (- (get player.health) 25))
  (get player.health))
```

### Output Capture\*

_Assert on printed output and side effects._

```lisp
//! @test "print output"
//! @expect output "Hello, World!\n"
(print "Hello, World!")

//! @test "multiple prints"
//! @expect output ["Starting...", "Complete."]
(do
  (print "Starting...")
  (print "Complete."))
```

### Test Organization\*

_Hierarchical test organization and tagging._

```lisp
//! @test "complex math operation"
//! @group "math/advanced"
//! @tags ["slow", "integration"]
//! @timeout 5000
(complex-calculation)
```

### Property-Based Testing\*

_Randomized testing with property verification._

```lisp
//! @property "addition is commutative"
//! @forall x:int y:int
//! @expect (eq? (+ x y) (+ y x))

//! @property "string length invariant"
//! @forall s:string
//! @expect (gte? (len s) 0)
```

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
```

## Advanced Features

### Snapshot Testing

For complex diagnostic output verification:

```lisp
//! @test "complex error reporting"
//! @expect snapshot "expected_output.txt"
(deeply-nested
  (problematic
    (expression "with" multiple issues)))
```

Snapshots are automatically generated and can be updated with `--update-snapshots`.

### Test Filtering

Multiple filtering approaches:

```bash
# By test name pattern
cargo run --bin harness -- --filter "addition"

# By file pattern (planned*)
cargo run --bin harness -- --files "math*"

# By tags (planned*)
cargo run --bin harness -- --tags "integration,slow"
```

### Exclusive Testing

Focus on specific tests during development:

```lisp
//! @test "debug this specific case"
//! @only
(problematic-operation)
```

When `@only` tests exist, all other tests are ignored.

## Configuration

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

## Best Practices

### Test Organization

1. **Keep tests close to functionality** - Use source-embedded annotations
2. **Use descriptive names** - Test names should explain the expected behavior
3. **Group related tests** - Use consistent naming patterns for discoverability
4. **Minimize test dependencies** - Each test should be independently runnable

### Error Testing

1. **Test specific error conditions** - Use canonical error codes (`codes=`) for structural matching, message patterns (`messages=`) for content validation
2. **Prefer stable error codes** - Use `codes=["ArityError"]` over `messages=["expected 2 arguments"]` for maintenance-friendly tests
3. **Document expected failures** - Always include reasons in `@skip` annotations
4. **Use snapshots for complex errors** - When diagnostic output is the primary concern
5. **Reference canonical error codes** - See the Error Codes table above for all available codes

### Narrative Testing

1. **Use fixtures for world state** - Avoid repetitive setup in narrative tests\*
2. **Test state transitions** - Verify that actions produce expected world changes\*
3. **Mock external systems** - Use stubbing for file I/O, network calls, etc.\*

### Performance Considerations

1. **Mark slow tests** - Use `@tags ["slow"]` for time-intensive tests\*
2. **Set appropriate timeouts** - Prevent infinite loops from hanging test runs\*
3. **Use parallel execution judiciously** - Some narrative tests may require serialization\*

## Integration with Development Workflow

### Pre-commit Hooks

```bash
# Install test hooks
./scripts/install_hooks.sh

# Tests run automatically on:
# - Pre-commit (fast subset)
# - Pre-push (full suite)
```

### Continuous Integration

The test harness integrates seamlessly with CI/CD:

```yaml
# Example GitHub Actions integration
- name: Run Sutra Tests
  run: |
    cargo test
    cargo run --bin harness -- --verbose
```

### IDE Integration

Test annotations are recognized by language servers for:

- Syntax highlighting
- Test discovery in IDE test explorers\*
- Inline test execution\*
- Real-time error highlighting\*

---

## Legend

**✓** = Fully implemented and tested
**\*** = Planned feature, not yet implemented
**⚠** = Experimental or unstable feature

This documentation reflects the current state as of July 2025. Features marked with asterisks (\*) represent planned capabilities that align with Sutra's design philosophy but require implementation.
