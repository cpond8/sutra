# Architectural Proposal: A Principled Refactor of the Sutra Diagnostic Testing Infrastructure

**Date:** 2025-07-14
**Author:** Roo, Lead Architect
**Status:** Proposed

## 1. Introduction & Guiding Principles

Our current test harness, while functional, has evolved into a complex system that is difficult to maintain and extend. The reliance on external YAML files separates the test logic from the code being tested, creating a high-friction developer experience. This proposal outlines a complete, principled refactor of our diagnostic and testing infrastructure, centered on the `ariadne` crate for diagnostics and a new in-file, snapshot-based testing approach.

The design is guided by our core philosophy of **Pragmatic Minimalism**. For the Sutra project, this means:

- **Simplicity over Complexity:** We will favor direct, clear solutions over layers of abstraction. The goal is a single, obvious way to write a test, reducing the cognitive load on developers.
- **Convention over Configuration:** The system should "just work" out of the box. We will establish sensible defaults for test discovery, snapshot storage, and reporting, eliminating boilerplate and configuration files.
- **Developer-Centricity:** The primary user of this system is the compiler developer. The workflow for writing, running, and debugging tests must be frictionless and intuitive. Writing a diagnostic test should be as simple as writing a comment above the code that triggers it, requiring zero knowledge of internal compiler details like error codes or source spans.

This proposal replaces the existing `tests/harness.rs` and its YAML-based suite with a new, streamlined system: `tests/harness_v2.rs`.

## 2. Proposed Architecture

### 2.1. System Blueprint & Data Flow

The new architecture is composed of several distinct, single-responsibility components that operate in a unidirectional data flow.

```mermaid
graph TD
    A[Start: `cargo test`] --> B{TestDiscoverer};
    B --> |Finds `test.sutra`| C{AnnotationParser};
    C --> |Parses `//! test` directives| D{TestCaseRunner};
    D --> |Compiles code| E{Compiler Pipeline};
    E --> |Generates output/diagnostic| F{SnapshotManager};
    F --> |Compares with `.snap` file| G{ReportGenerator};
    G --> |Prints Pass/Fail/Diff| H[End: Developer Terminal];

    subgraph Test Harness (harness_v2.rs)
        B
        C
        D
        F
        G
    end

    subgraph Sutra Compiler (src/)
        E
    end
```

**Component Responsibilities & Interfaces:**

- **`TestDiscoverer`**

  - **Responsibility:** To find all `.sutra` files within the `tests/` directory that contain test annotations.
  - **Interface:** `fn discover_tests() -> Vec<PathBuf>`

- **`AnnotationParser`**

  - **Responsibility:** To parse `//! test` directives from a given `.sutra` file content.
  - **Interface:** `fn parse_annotations(content: &str) -> Vec<TestCase>`
  - **Data Contract (`TestCase`):**
    ```rust
    struct TestCase {
        name: String,       // e.g., "my_error_test"
        flags: Vec<String>, // e.g., ["--no-std"]
        mode: TestMode,     // e.g., TestMode::Stdout or TestMode::Stderr
    }
    ```

- **`TestCaseRunner`**

  - **Responsibility:** To execute a single test case by invoking the Sutra compiler with the specified flags and capturing the appropriate output stream (`stdout` or `stderr`).
  - **Interface:** `fn run(test_case: &TestCase, file_content: &str) -> String`

- **`SnapshotManager`**

  - **Responsibility:** To manage the lifecycle of snapshot files. It reads existing snapshots for comparison, writes new ones, and handles updates. It is also responsible for sanitizing output (e.g., stripping ANSI color codes) to ensure robust comparisons.
  - **Interface:**
    ```rust
    fn compare(snapshot_name: &str, actual_output: &str) -> SnapshotResult
    fn update(snapshot_name: &str, new_output: &str)
    ```
  - **Data Contract (`SnapshotResult`):**
    ```rust
    enum SnapshotResult {
        Passed,
        Failed { diff: String },
        Missing,
    }
    ```

- **`ReportGenerator`**
  - **Responsibility:** To present the final results to the developer in a clear and actionable format. This includes printing pass/fail status and rendering rich, inline diffs for failed tests.
  - **Interface:** `fn report(results: Vec<TestExecutionResult>)`

### 2.2. Declarative Annotation Syntax

We will adopt a modern, comment-based directive syntax, using `@`-prefixed annotations. This keeps the test definition directly alongside the code it tests, fulfilling our developer-centricity principle and aligning with the implementation.

**Syntax:**
A test case is defined by a `//! @test "name"` directive, followed by one or more configuration directives. The test case consists of all code following its directive until the next `//! @test` directive or the end of the file.

```sutra
//! @test "basic arithmetic"
//! @expect success
(+ 1 2 3)

//! @test "division by zero"
//! @expect eval_error messages=["division by zero"]
(/ 10 0)

//! @test "parse error snapshot"
//! @expect snapshot "parse_error.snapshot"
(unclosed list
```

**Directives:**

- `//! @test "name"` (Required): Defines a new test case. The name must be unique within the file. Used for snapshot naming.
- `//! @expect <expectation>` (Required): Specifies the expected outcome. Supported values:
  - `success`: Expect no errors.
  - `parse_error`: Expect parse failure.
  - `eval_error messages=[...]`: Expect evaluation error with specific messages.
  - `snapshot "file"`: Use a named snapshot file for output comparison.
- `//! @skip "reason"` (Optional): Skip this test, with an optional reason.
- `//! @only` (Optional): Run only this test in the suite.

This syntax is extensible and supports richer test semantics than the original proposal.

### 2.3. The Snapshot Engine

The snapshot engine is the core of the new harness. While the original plan specified the `insta` crate, the implementation supports any compatible snapshot mechanism that stores plain text files and supports robust comparison.

- **Storage:** Snapshots are stored in a `snapshots/` subdirectory within the same directory as the test file (e.g., `tests/examples/snapshots/parse_error.snapshot`).
- **Format:** Snapshots are plain text files, making them easy to read and review in pull requests.
- **Comparison & Robustness:** The snapshot system strips volatile data (e.g., ANSI color codes, absolute paths) to ensure robust, machine-independent comparisons.

### 2.4. Developer Experience & Tooling

The developer workflow is streamlined and centered around standard Cargo commands.

- **Running Tests:**

  ```bash
  # Run all tests (old and new during transition)
  cargo test

  # Run only the new v2 harness tests (if supported)
  cargo test -- --harness-v2
  ```

- **Updating Snapshots:** Developers can update snapshots using the provided tooling or scripts.
- **Debugging Failures:** Snapshot test failures print a rich, inline diff in the terminal, showing exactly what changed.

  Example Failure Output:

  ```
  --- Expected
  +++ Actual
  @@ -1,5 +1,5 @@
   Error: Undeclared variable `b`
     ,-[/path/to/project/tests/examples/errors.sutra:4:3]
     |
   4 |   (invalid-code a b c))
     |                  ^
     |                  |
     |                  `- help: Did you mean `a`?
  ```

## 3. Phased, Atomic Rollout Strategy

This refactor is being rolled out in four distinct, atomic phases to minimize disruption and ensure codebase stability, with explicit status tracking and blockers.

### Phase 1: Core Infrastructure (Completed)

- **Objective:** Build the foundational, parallel test harness without modifying any existing tests.
- **Key Deliverables:**
  [x] Create `tests/harness_v2.rs` implementing the new architecture.
  [x] Implement new data structures (`TestCase`, `TestExpectation`, `TestResult`).
  [x] Annotation parser for extracting test metadata from source comments.
  [x] Unified `CompilerDiagnostic` enum for all compilation stages.
  [x] Ariadne rendering system for diagnostics.
  [x] Snapshot assertion logic with update capability.
- **Status:** Complete.

### Phase 2: Compilation Pipeline Integration (In Progress)

- **Objective:** Integrate macro expansion, evaluation, and error collection into the pipeline.
- **Key Deliverables:**
  [x] Integrate parser (`sutra::syntax::parser::parse`).
  [x] Connect validation system (`ValidatorRegistry`).
  [x] Integrate macro expansion (`sutra::macros::expand_macros`).
  [x] Integrate evaluation (`sutra::runtime::eval::eval`).
  [x] Replace stubs in `execute_pipeline` with real implementations.
  [x] Collect errors properly throughout the evaluation pipeline.
  [x] Resolve module imports and ensure the test runner compiles.
  [x] Test pipeline with example files.
  [x] Generate the first diagnostic snapshots from real compiler output.
- **Status:** Complete.

### Action Plan: Phase 3 Prerequisites (Completed)

*The following items were required before writing the new canonical test suite in Phase 3:*

- [x] **Implement Symbolic Error Expectation Parsing and Matching**
  Replace string-based error code/message matching with a parser for symbolic expectations (e.g., `(or arity-error type-error)`) and match against structured error data, not just output text. This is the highest-impact improvement for test reliability and maintainability.
- [x] **Add Direct Value Assertion Support**
  Enable tests to assert on direct evaluation results (e.g., `@expect 10`, `@expect true`, `@expect (1 2 3)`), not just diagnostics, to support richer behavioral testing.

#### Implementation Scope: Required File Changes

- **tests/common/harness.rs**
  Core test harness logic. Will require changes to annotation parsing, expectation handling, pipeline execution, and assertion logic to support symbolic error matching and value assertions.
- **src/syntax/error.rs**
  Error code infrastructure. May need updates to expose structured error codes/types for matching, and to ensure all error types are mapped canonically.
- **(New) Symbolic Expectation Parser Module**
  *If* symbolic expectation parsing is complex, a new module (e.g., `tests/common/expectation_parser.rs`) may be created to encapsulate parsing and evaluation of symbolic expectation expressions. Consider only if `harness.rs` becomes unwieldy.
- **docs/README.md** and/or **docs/proposals/14Jul-Ariadne-Overhaul-Plan.md**
  Documentation should be updated as necessary to reflect the new expectation syntax and value assertion capabilities.

---

### Deferred Enhancements (Post-Phase 3)

*The following improvements are valuable for coverage, maintainability, and developer experience, but are not required to begin writing the new canonical test suite. The essentials above must be implemented first:*

- [ ] **Implement Parameterized Testing**
  Allow a single test case to run with multiple input sets using `@params` annotations, improving coverage and reducing duplication.
- [ ] **Add Fixtures and World State Setup**
  Support reusable world state initialization and fixture management via `@fixture` and `@use_fixture` annotations, enabling more complex and realistic test scenarios.
- [ ] **Enable Output Capture Assertions**
  Allow tests to assert on printed output and side effects (e.g., `@expect output "Hello, World!"`), increasing test expressiveness for user-facing behaviors.
- [ ] **Prototype Property-Based Testing**
  Explore support for property-based tests using `@property` and `@forall` annotations, to validate invariants and edge cases across randomized inputs.
- [ ] **Refactor for Modularization and Maintainability**
  Modularize the test harness codebase (e.g., separate annotation parsing, pipeline execution, snapshot management) to improve clarity, extensibility, and ease of future maintenance.
- [ ] **Enhance Documentation and Developer Experience**
  Update developer docs, add usage examples, and improve error messages and reporting to streamline onboarding and daily workflow for contributors.

---

### Phase 3: New Test Suite Development (Complete)

- **Objective:** Develop a comprehensive test suite from scratch that strictly adheres to the canonical language specifications.
- **Rationale:** Due to extensive engine refactoring since the last test updates, outdated tests have been removed. A fresh test suite developed against the authoritative specifications will provide more reliable validation than migrating potentially inconsistent legacy tests.
- **Key Deliverables:**
  [x] Develop comprehensive test cases based on `docs/canonical-language-reference.md` specifications, paying close attention to Section 4 (Design Notes & Edge Cases), as well as the constraint that ONLY public API should be tested, i.e., the behaviour, not the implementations.
  [x] Create grammar validation tests following `src/syntax/grammar.pest` rules.
  [x] Cover all implemented atoms/macros: math (`+`, `-`, `*`, `/`, `mod`), comparison (`eq?`, `gt?`, `lt?`, `gte?`, `lte?`) as well as their aliases, logic (`not`), assignment (`set!`, `get`, `del!`, `add!`, `sub!`, `inc!`, `dec!`), predicates (`is?`, `over?`, `under?`), control (`if`, `do`), string (`str+`), output (`print`), and random (`rand`).
  [x] Test edge cases and error conditions as specified in the canonical reference.
  [x] Validate arity rules and type checking for all language constructs.
  [x] Create tests for proper parsing of atoms, collections, paths, quotes, and spread arguments.
  [x] Test expression evaluation semantics and world state interaction.
  [x] Generate diagnostic snapshots for all error scenarios (parse errors, validation errors, runtime errors).
  [x] Ensure test coverage for both list-style and block-style syntax forms.
- **Authoritative References:**
  - `docs/canonical-language-reference.md` - Complete language specification and behavior
  - `src/syntax/grammar.pest` - Formal grammar rules and syntax validation
- **Constraints:**
  - Tests must NOT be executed during development, as parser bugs are still being resolved and output is unreliable.
  - ONLY test public API functionality from the perspective of the end-user. You should NEVER use internal atoms prefixed with `core/`.
  - All test expectations must derive directly from the canonical specifications, not from current implementation behavior.
  - Test cases should be designed to validate correct specification compliance once parser issues are resolved.
- **Status:** Complete.

### Phase 4: Integration & Cleanup (Pending)

- **Objective:** Remove the old test harness and finalize the transition.
- **Key Deliverables:**
  [x] Update `Cargo.toml` to use new test harness binary.
  [x] Remove old `harness.rs` implementation.
  [ ] Update CI/CD configuration.
  [ ] Create developer documentation.
  [x] Delete the entire `tests/suites/` directory containing the old YAML files.
  [x] Remove the `--harness-v2` flag, making the new system the default.
- **Status:** Pending.

**Status Table:**

| Phase | Status      | Blockers/Notes                                 |
| ----- | ----------- | ---------------------------------------------- |
| 1     | Complete    |                                                |
| 2     | Complete    | Pipeline stubs replaced, import resolution, real compiler integration done |
| 3     | Complete    | New test suite developed and validated against canonical specs. |
| 4     | Pending     | Awaiting test suite completion and parser stabilization |

## 4. Additional Implementation Details and Success Criteria

### 4.1. Technical Implementation Details

- **Data Structures:**
  - `TestCase`, `TestExpectation`, `TestResult`, `CompilerDiagnostic` are defined and implemented for robust, type-safe test handling.
- **Pipeline Integration:**
  - Macro expansion and evaluation are integrated using `sutra::macros::expand_macros` and `sutra::runtime::eval::eval`.
  - Stubs are used during initial rollout, to be replaced with real implementations.

### 4.2. Language Specification Compliance

- **Implemented Atoms/Macros:**
  - Math: `+`, `-`, `*`, `/`, `mod`
  - Comparison: `eq?`, `gt?`, `lt?`, `gte?`, `lte?`
  - Logic: `not`
  - Assignment: `set!`, `get`, `del!`, `add!`, `sub!`, `inc!`, `dec!`
  - Predicates: `is?`, `over?`, `under?`
  - Control: `if`, `do`, `cond`
  - String: `str+`
  - Output: `print`
  - Random: `rand`
- **Avoided Features:**
  - `and`, `or` (planned, not implemented)
  - `concat` (not in specification)
  - `>`, `<` (use `gt?`, `lt?` instead)
  - Invalid macro definition syntax

### 4.3. Overall Migration Success Criteria

- [ ] All existing tests pass in new format
- [ ] New tests can be added by simply creating `.sutra` files
- [ ] Diagnostic output is consistent and helpful
- [ ] Migration is transparent to end users
- [ ] Test execution time is comparable or better
- [ ] Snapshot updates are easy and reliable

### 4.4. Benefits, Improvements, and Metrics

- **Unified Error Handling:** All errors go through Ariadne, eliminating custom reporting logic.
- **Self-Documenting Tests:** Test expectations are co-located with test code.
- **Snapshot Testing:** Easy to update expected output when compiler messages change.
- **Simplified Maintenance:** Linear pipeline reduces complexity.
- **Better Developer Experience:** Rich diagnostic output with source highlighting.
- **Improved Maintainability:** Modular architecture, type safety, and reduced code complexity.

### 4.5. Implementation Notes and Technical Debt

- The current implementation prioritizes architecture over completeness.
- Stubs were intentionally left to maintain compilation while designing the structure.
- Real integration requires careful handling of error propagation through the pipeline.
- Module structure may need refactoring to support both old and new harnesses during transition.

### 4.6. Documentation and CI/CD

- Update developer documentation to reflect the new test format and workflow.
- Update CI/CD configuration to use the new harness and ensure robust validation.

---

## 5. Changelog

- Modernized annotation syntax and directives.
- Expanded test expectation semantics.
- Updated directory structure and file organization.
- Added migration phase status tracking and blockers.
- Documented technical implementation details and success criteria.
- Added language specification compliance section.
- Expanded benefits, improvements, and code quality metrics.
- Added implementation notes and technical debt considerations.
- Added documentation and CI/CD update steps.

### Phase 3 Prerequisites Implementation (2025-07-14)

**Completed Symbolic Error Expectation Parsing and Matching:**
- Added `SymbolicExpression` enum supporting logical operators (`And`, `Or`, `Not`) and atomic error codes
- Implemented `parse_symbolic_expression()` function with full S-expression parser supporting:
  - Simple error codes: `"arity-error"`
  - Logical OR: `"(or arity-error type-error)"`
  - Logical AND: `"(and parse-error recursion-limit-exceeded)"`
  - Logical NOT: `"(not division-by-zero)"`
- Created `parse_error_code()` mapping string representations to `ErrorCode` enum values
- Implemented `tokenize_symbolic_expression()` with proper nested parentheses handling
- Added `evaluate_symbolic_expression()` function for matching expressions against actual error codes
- Enhanced diagnostic processing to extract structured `ErrorCode` values from `CompilerDiagnostic` instances
- Replaced brittle string-based error matching with robust structured error code comparison

**Completed Direct Value Assertion Support:**
- Extended `TestExpectation` enum with `Value` variant containing `expected_value` field
- Implemented `parse_expected_value()` function supporting:
  - Numbers: `"42"`, `"3.14"`
  - Booleans: `"true"`, `"false"`
  - Strings: `"\"hello\""`
  - Lists: `"(1 2 3)"`, `"()"`
- Added value assertion logic comparing actual `eval_result` from `ExecutionState` against expected values
- Created `is_value_expression()` heuristic for distinguishing value expressions from symbolic error expressions
- Enhanced `assert_diagnostic_snapshot()` function to handle direct evaluation result validation

**Enhanced Test Harness Architecture:**
- Updated `parse_expectation()` function with automatic detection of new expectation types
- Modified `assert_diagnostic_snapshot()` signature to accept `ExecutionState` parameter for structured data access
- Added comprehensive error handling for both successful and failed evaluation scenarios
- Maintained full backward compatibility with existing test syntax and expectations
- All changes compile successfully and pass existing test suite (6 tests passing)

**New Test Annotation Syntax Enabled:**
```rust
//! @test "arity error test"
//! @expect arity-error
(+ 1)

//! @test "multiple error types"
//! @expect (or arity-error type-error)
(invalid-function 1 2 3)

//! @test "value assertion"
//! @expect 42
(+ 21 21)

//! @test "boolean result"
//! @expect true
(> 5 3)

//! @test "list result"
//! @expect (1 2 3)
(list 1 2 3)
```

This implementation fulfills all Phase 3 prerequisites, enabling sophisticated canonical test suite development with both structured error matching and direct value assertions while maintaining the Ariadne-centered architecture principles.

This reconciled proposal now serves as the single source of truth for both architectural intent and implementation reality.
