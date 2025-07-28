# Writing Tests for Sutra

This guide shows you how to write effective tests using Sutra's testing framework and how to run the test suite.

## What Makes a Good Sutra Test?

Sutra tests are written in the Sutra language itself—they're just regular Sutra code that uses the `(test ...)` form. When you write a test, you're describing what should happen when Sutra runs your code.

Here's what a simple test looks like:

```lisp
(test "addition works correctly"
      (expect (value 10)
              (tags "math"))
      (+ 4 6))
```

This test runs `(+ 4 6)` and expects the result to be `10`. If the result matches, the test passes. If not, you'll see a detailed error message showing what went wrong.

## How to Structure Your Tests

### The Basic Test Form

Every test follows this pattern:

```lisp
(test "descriptive test name"
      (expect expectation-type
              (tags "category"))
      your-code-here)
```

Let's break this down:

- **Test name**: A clear description of what you're testing. Make it specific enough that someone else can understand what should happen.
- **Expectation**: What you expect to happen when your code runs.
- **Tags**: Categories that help organize tests (like "math", "string", "error-handling").
- **Test body**: The actual Sutra code you want to test.

### Types of Expectations

You can expect different kinds of results:

**Value expectations** test that your code produces a specific result:

```lisp
(test "string concatenation"
      (expect (value "hello world")
              (tags "string"))
      (str+ "hello" " " "world"))
```

**Error expectations** test that your code fails in the expected way:

```lisp
(test "division by zero fails"
      (expect (error Runtime)
              (tags "math" "error"))
      (/ 10 0))
```

This is especially useful for testing error conditions—you want to make sure Sutra gives helpful error messages when things go wrong.

## Running Your Tests

### Run All Tests

To run the entire test suite:

```bash
cargo run test
```

This discovers all `.sutra` files in the `tests/` directory and runs every test it finds. You'll see output like this:

```
✓ math: addition works correctly
✓ string: concatenation works
✗ math: division by zero fails
```

Green checkmarks (✓) mean tests passed. Red X marks (✗) mean tests failed, and you'll see detailed error information.

### Run Tests from Rust

You can also run the test suite through Cargo:

```bash
cargo test
```

This runs both the Sutra tests and additional Rust-based integration tests.

### Understanding Test Output

When tests pass, you see a simple success message. When they fail, Sutra shows you exactly what went wrong:

- What you expected to happen
- What actually happened
- Where in your code the problem occurred
- Suggestions for fixing the issue

The error messages use the same system as Sutra's regular error reporting, so they include source context and helpful guidance.

## Where to Put Your Tests

Tests are organized by what they're testing:

- **`core/`**: Core language features like literals, special forms, and scoping
- **`builtins/`**: Built-in functions like arithmetic, string operations, and list manipulation
- **`control/`**: Control flow like conditionals and loops
- **`world/`**: World state operations like variable assignment
- **`io/`**: Input and output operations
- **`syntax/`**: Parsing and syntax validation

When you add a new feature, put its tests in the most appropriate directory. If you're not sure, look at existing tests for similar features.

## Writing Effective Tests

### Test One Thing at a Time

Good tests focus on a single behavior:

```lisp
;; Good - tests one specific case
(test "addition with positive numbers"
      (expect (value 5)
              (tags "math"))
      (+ 2 3))

;; Good - tests error handling separately
(test "addition requires numbers"
      (expect (error Runtime)
              (tags "math" "error"))
      (+ 2 "not a number"))
```

### Use Descriptive Names

Your test names should explain what's being tested:

```lisp
;; Good - specific and clear
(test "car returns first element of non-empty list"
      (expect (value 1)
              (tags "list"))
      (car '(1 2 3)))

;; Less helpful - too vague
(test "car works"
      (expect (value 1)
              (tags "list"))
      (car '(1 2 3)))
```

### Test Both Success and Failure Cases

Don't just test that things work—test that they fail appropriately:

```lisp
;; Test the happy path
(test "list access with valid index"
      (expect (value 2)
              (tags "list"))
      (car (cdr '(1 2 3))))

;; Test error conditions
(test "car fails on empty list"
      (expect (error Runtime)
              (tags "list" "error"))
      (car '()))
```

### Use Tags Consistently

Tags help organize and filter tests. Use consistent tag names:

- `"math"` for arithmetic operations
- `"string"` for string manipulation
- `"list"` for list operations
- `"error"` for error-handling tests
- `"parse"` for syntax and parsing tests

## Advanced Testing Patterns

### Testing Complex Expressions

You can test more complex code by building up expressions:

```lisp
(test "nested function calls"
      (expect (value 8)
              (tags "math"))
      (+ (* 2 3) (/ 4 2)))
```

### Testing Functions You Define

You can test functions you define within the test:

```lisp
(test "user-defined function works"
      (expect (value 25)
              (tags "function"))
      (do
        (define square (lambda (x) (* x x)))
        (square 5)))
```

The `do` form lets you run multiple expressions in sequence, so you can set up what you need and then test it.

## What Gets Tested Automatically

The test suite includes additional automated checks:

### CLI Error Reporting

A Rust-based test (`cli_regression.rs`) ensures that when you run Sutra from the command line with invalid input, you get properly formatted error messages. This prevents regressions in error reporting quality.

### Integration Testing

The test framework itself runs integration tests to make sure the testing infrastructure works correctly.

## When Tests Fail

If your tests fail, here's how to debug them:

1. **Read the error message carefully**—Sutra's error messages are designed to be helpful and specific.

2. **Check your expectations**—Make sure you're expecting the right type of result (value vs. error).

3. **Run individual tests**—You can focus on specific test files by running `sutra test tests/your-file.sutra`.

4. **Use the REPL**—Try running your test code in the Sutra REPL to see what it actually produces.

## Best Practices Summary

- Write one test per behavior you want to validate
- Use clear, descriptive test names
- Test both success cases and error conditions
- Organize tests in appropriate directories
- Use consistent tagging
- Keep test code simple and focused

The goal is to build confidence that Sutra works as expected. Good tests catch bugs early and make it safe to change code, knowing that you'll be warned if something breaks.

---

## Reference: Test Suite Structure

The test suite is organized into these directories:

| Directory   | Purpose                  | Example Files                                            |
| ----------- | ------------------------ | -------------------------------------------------------- |
| `core/`     | Core language constructs | `literals.sutra`, `special_forms.sutra`, `scoping.sutra` |
| `builtins/` | Built-in functions       | `arithmetic.sutra`, `comparison.sutra`, `string.sutra`   |
| `control/`  | Control flow             | `conditionals.sutra`, `execution.sutra`                  |
| `world/`    | World state operations   | `assignment.sutra`, `persistence.sutra`                  |
| `io/`       | Input/output             | `output.sutra`                                           |
| `syntax/`   | Parsing and syntax       | `parsing.sutra`                                          |

Plus `cli_regression.rs` for CLI integration testing.
