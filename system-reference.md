# System Reference

> **Purpose:**
> This document is the single, always-up-to-date reference for the architecture, patterns, rationale, and ongoing evolution of this codebase. It is for your (the developer's) use: onboarding, planning, debugging, and as a changelog.
>
> **Maintenance:**
> After every meaningful codebase or design change, review and update this file first. Treat it as your primary source of context and system understanding.

---

## Changelog

- **2025-07-04** — Macro loader/parser modernization in progress: Batch-based, test-driven updates to resolve test failures related to parameter list struct migration, macro definition parsing, and error handling. Focus on strict test/production parity, robust error messages, and full alignment between parser, grammar, loader, and tests. See activeContext.md for current priorities and guidance for future contributors.
- **2025-07-02** — Parser refactor complete: Decomposed into per-rule helpers, robust error handling, and strict, canonical dotted list validation. All unreachable!()s replaced with structured errors. Dotted list parsing now asserts and errors on malformed shapes. Parser and macro system are now fully decoupled and testable. Current focus: debugging macro system test failures, especially for variadic macros and cond expansion.
- **2024-07-02** — Document created. Initial synthesis of architecture, patterns, and system structure from available documentation and directory structure.
- **2025-07-02** — Temporary Rust macro expander for `(cond ...)` added. This is a stopgap until variadic macro support is implemented and `cond` can be defined in the native language. See Macro System section for details.
- **2025-07-02** — Implemented a two-tiered, variadic macro system. Simple, declarative macros can be defined via `MacroTemplate`, while complex procedural macros (`cond`) are native `MacroFn`s. `cond` is now the primary conditional macro, and `if` is a simple macro that expands to it, using `Expr::If` as the underlying primitive.
- **2025-07-02** — Registry/Expander Reliability Audit: Comprehensive review of advanced strategies for macro registry and expander reliability (phantom types, registry hashing, sealing, logging, integration tests, smoke mode, provenance, mutation linting, opt-out API, fuzzing, singleton, metrics). Immediate implementation will focus on integration tests and registry hashing, with others staged for future adoption. See memory-bank/activeContext.md and memory-bank/systemPatterns.md for full details and rationale.
- **2025-07-04** — Added parsing pipeline refactor summary, rationale, and migration strategy.
- **2025-07-05** — Migration to proper-list-only and ...rest-only architecture complete. All legacy code, tests, and documentation for improper/dotted lists and legacy variadic syntax have been removed. The codebase, tests, and docs are now fully compliant and clean.
- [Add future entries here: date, summary]

---

## Quick Orientation

- **Project:** Sutra Engine
- **Goal:** A compositional, macro-driven, simulation/story engine with a focus on authoring flexibility and traceable, pure data flows.
- **Entry Points:** See `src/main.rs` (CLI), `src/lib.rs` (library interface).
- **How to Start:**
  - Review this document for system context.
  - See `docs/architecture/architecture.md` for evolving architectural notes.
  - Explore `src/` for implementation details.

---

## System Overview

### High-Level Architecture

```
parse → macro-expand → validate → evaluate → output/presentation
```
- **Parse:** Converts author code (brace-block or s-expr) to AST.
- **Macro-expand:** Applies macro definitions to AST, producing flattened code.
- **Validate:** Checks for errors, illegal atoms, or bad structure.
- **Evaluate:** Applies atoms to world state; produces new world state or computed values.
- **Output/presentation:** Renders results (text, choices, UI), decoupled from core logic.

### Data Flow
- World state is a single, serializable, deeply immutable data structure, accessible everywhere by path.
- Only atoms can produce side-effects; macros are pure (except explicit randomness, which is tracked).
- Debugging: Macroexpansion and world state diffs are available for inspection at every step.

### Module Boundaries
- Each layer exposes a pure API—no hidden state or side effects.
- Authors interact with macro libraries and author-facing syntax; core devs work with atoms and engine APIs.

---

## AST and Parsing (Detailed)

### AST (Abstract Syntax Tree)
- **File:** `src/ast.rs`
- **Purpose:** Canonical representation of all Sutra code, supporting both s-expression and brace-block syntaxes.
- **Key Types:**
  - `Span`: Source location tracking for all AST nodes.
    ```rust
    #[derive(Debug, Clone, PartialEq, Default)]
    pub struct Span {
        pub start: usize,
        pub end: usize,
        // Optionally: line/col for richer error UX.
    }
    ```
  - `Expr`: Core AST node for Sutra expressions.
    ```rust
    #[derive(Debug, Clone, PartialEq)]
    pub enum Expr {
        List(Vec<Expr>, Span),
        Symbol(String, Span),
        Path(Path, Span),
        String(String, Span),
        Number(f64, Span),
        Bool(bool, Span),
        If {
            condition: Box<Expr>,
            then_branch: Box<Expr>,
            else_branch: Box<Expr>,
            span: Span,
        },
    }
    ```
- **Key Methods:**
  - `Expr::span(&self) -> Span`: Returns the source span for any expression.
  - `Expr::into_list(self) -> Option<Vec<Expr>>`: Converts to a list if possible.
  - `Expr::pretty(&self) -> String`: Pretty-prints the expression as a string.
- **Usage Example:**
  ```rust
  let expr = Expr::Number(42.0, Span { start: 0, end: 2 });
  assert_eq!(expr.span().start, 0);
  assert_eq!(expr.pretty(), "42");
  ```
- **Design Notes:**
  - All nodes carry a `Span` for error reporting and explainability.
  - The AST is designed for lossless round-tripping between syntaxes.
  - The `If` variant is a special form, handled distinctly in macro expansion and evaluation.
- **Extension Points:**
  - To add new expression types, extend the `Expr` enum and update all pattern matches in parser, macro, and eval modules.
- **Invariants:**
  - All AST nodes must have valid, non-overlapping spans.
  - Only `List` nodes may contain other expressions as children, except for `If` which has explicit branches.

### Parsing
- **File:** `src/parser.rs`
- **Purpose:** Converts Sutra source code (brace-block or s-expr) into canonical AST.
- **Grammar:** Defined in `src/sutra.pest` (PEG grammar, single source of truth for syntax).
- **Key Responsibilities:**
  1. Use the formal grammar for both syntaxes.
  2. Produce identical AST for both s-expr and brace-block forms.
  3. Convert CST (Concrete Syntax Tree) from `pest` into `Expr` AST.
  4. Translate parsing errors into `SutraError` with span info.
- **Key Types:**
  - `pub fn parse(source: &str) -> Result<Vec<Expr>, SutraError>`: Main entry point. Parses a source string into a vector of top-level AST nodes.
- **Error Handling:**
  - All parse errors are mapped to `SutraError` with precise span/location.
  - TODO: Improve error formatting for more user-friendly messages.
- **Usage Example:**
  ```rust
  let code = "(print \"Hello\")";
  let ast = parse(code)?;
  assert!(matches!(ast[0], Expr::List(_, _)));
  ```
- **Extension Points:**
  - To add new syntax, update `sutra.pest` and the CST-to-AST conversion logic.
- **Invariants:**
  - The parser must always produce a valid AST or a precise error.
  - Both syntaxes must be fully round-trippable.

---

## Core Concepts & Patterns

- **AST:** Canonical s-expression (Lisp-style) or brace-block syntax; both map 1:1 to a nested list AST.
- **Atoms:** Primitive operations that can mutate world state.
- **Macros:** Pure code transformers; expand to atoms or other macros.
- **World State:** Deeply immutable, path-addressable, serializable.
- **Extensibility:** Macro libraries, author-facing syntax, and plugin points.
- **Debuggability:** Every transformation and state change is traceable.

---

## Component Reference

- **src/ast.rs:** AST structures and manipulation.
- **src/atom.rs, src/atoms_std.rs:** Atom definitions and standard library.
- **src/macros.rs, src/macros_std.rs:** Macro system and standard macros.
- **src/parser.rs:** Parsing logic for both syntaxes.
- **src/eval.rs:** Evaluation engine.
- **src/world.rs:** World state representation and manipulation.
- **src/cli/**: Command-line interface and output.
- **src/registry.rs:** Registration and lookup for atoms/macros.
- **src/path.rs:** Path addressing utilities.
- **src/error.rs:** Error types and handling.
- **tests/**: Core, macro, and parser tests.

---

## Configuration & Extensibility (Detailed)

- **Configuration Mechanisms:**
  - **File-based:** [TODO: Document if/when configuration files are supported.]
  - **Environment Variables:** [TODO: List any env vars used for runtime configuration.]
  - **CLI Flags:** See `src/cli/args.rs` for command-line argument parsing and available flags.
  - **Code-based:**
    - Atoms and macros can be registered programmatically via their registries.
    - World state can be seeded for deterministic runs.

- **Extension Points:**
  - **Atoms:**
    - Register new atoms in `src/atoms_std.rs` or via `AtomRegistry::register`.
    - Atoms must follow the `AtomFn` signature and return new world state.
  - **Macros:**
    - Register new macros in `src/macros_std.rs` or via `MacroRegistry::register`.
    - Macros must be pure AST transformers.
  - **Plugins:** [TODO: Document plugin system if/when implemented.]
  - **Output Sinks:**
    - Implement the `OutputSink` trait to customize output handling (e.g., for testing, logging, or UI integration).
  - **Testing:**
    - Add new test cases in `tests/` for new atoms, macros, or parser features.

- **How to Add a New Atom (Step-by-step):**
  1. Implement a function matching the `AtomFn` signature.
  2. Register it in `src/atoms_std.rs` or via `AtomRegistry` in your setup code.
  3. Add tests in `tests/core_eval_tests.rs`.

- **How to Add a New Macro (Step-by-step):**
  1. Implement a function matching the `MacroFn` signature.
  2. Register it in `src/macros_std.rs` or via `MacroRegistry` in your setup code.
  3. Add tests in `tests/macro_expansion_tests.rs`.

- **How to Extend the Parser:**
  1. Update the PEG grammar in `src/sutra.pest`.
  2. Update CST-to-AST conversion logic in `src/parser.rs`.
  3. Add tests in `tests/parser_tests.rs`.

---

## Development Practices (Detailed)

- **Testing Strategy:**
  - **Unit Tests:** Located in `tests/` (e.g., `core_eval_tests.rs`, `macro_expansion_tests.rs`, `parser_tests.rs`).
  - **Test Coverage:**
    - Atoms: `tests/core_eval_tests.rs`
    - Macros: `tests/macro_expansion_tests.rs`
    - Parser: `tests/parser_tests.rs`
  - **Property-based Testing:** [TODO: Consider for future, especially for AST round-tripping.]
  - **Testable I/O:** All output is routed through `OutputSink` for easy mocking.

- **Debugging & Traceability:**
  - Macro expansion can be traced step-by-step via `macroexpand_trace`.
  - All AST nodes carry spans for precise error reporting.
  - World state diffs can be inspected at each evaluation step.

- **Design Decisions:**
  - See `docs/architecture/architecture.md` for rationale, alternatives, and open questions.
  - All major changes should be logged in the changelog section above.

---

## Roadmap, TODOs, and Open Questions (Detailed)

- **Macro System Boundaries:**
  - Compile-time vs. runtime expansion is still under investigation.
  - TODO: Decide on final boundaries and document in this file.
- **Scheduling/Simulation Model:**
  - How to represent tick-based or real-time systems (scheduler as macro/module vs. engine core)?
  - TODO: Prototype and document chosen approach.
- **Type System/Validation:**
  - What level of static analysis to require or support?
  - TODO: Explore and document options for type checking and validation.
- **Plugin/Configuration System:**
  - [TODO: Specify and document if/when implemented.]
- **Serialization & Snapshots:**
  - TODO: Document serialization format for world state and support for save/load.
- **Open Questions:**
  - What are the best practices for authoring large projects in Sutra?
  - How to support advanced debugging and live coding?
  - [Add new TODOs and open questions here as they arise.]

---

## References (Annotated)

- `docs/architecture/architecture.md` — Core architecture, atom set, and system overview. Living draft; update with all major changes.
- `docs/philosophy/philosophy.md` — Project philosophy and guiding principles. Explains the "why" behind design choices.
- `docs/specs/language-spec.md` — Language syntax and semantics.
- `docs/specs/storylet-spec.md` — Storylet system specification.
- `docs/specs/thread-spec.md` — Threading and concurrency model (if any).
- `docs/references/narrative-design-reference.md` — Narrative design patterns and best practices.
- `docs/references/rust-lisp-reference.md` — Reference for Rust-Lisp interop and idioms.
- `memory-bank/activeContext.md` — Current active development context.
- `memory-bank/productContext.md` — Product-level context and goals.
- `memory-bank/progress.md` — Progress tracking and milestones.
- `memory-bank/projectbrief.md` — Project brief and high-level summary.
- `memory-bank/systemPatterns.md` — System-level patterns and idioms.
- `memory-bank/techContext.md` — Technical context and constraints.
- [Add new references as needed; annotate with purpose and relevance.]

---

## Atoms (Detailed)

- **Files:** `src/atom.rs`, `src/atoms_std.rs`
- **Purpose:** Atoms are the primitive operations of the Sutra engine. Only atoms can mutate world state or produce side effects. All core logic and author-facing commands ultimately reduce to atoms.

### Atom Function Type
- **Type Signature:**
  ```rust
  pub type AtomFn = fn(
      args: &[Expr],
      context: &mut EvalContext,
      parent_span: &Span,
  ) -> Result<(Value, World), SutraError>;
  ```
  - `args`: Arguments as AST nodes (already macro-expanded).
  - `context`: Mutable evaluation context (tracks call stack, output, etc.).
  - `parent_span`: For error reporting.
  - **Returns:** Result of evaluation and the new world state (ensures explicit, pure state transitions).

### Output Sink
- **Trait:**
  ```rust
  pub trait OutputSink {
      fn emit(&mut self, text: &str, span: Option<&Span>);
  }
  ```
  - Used for all output (e.g., `print`), making I/O testable and injectable.
  - `NullSink` is provided for silent/test runs.

### Atom Registry
- **Type:**
  ```rust
  #[derive(Default)]
  pub struct AtomRegistry {
      pub atoms: HashMap<String, AtomFn>,
  }
  ```
- **Key Methods:**
  - `register(&mut self, name: &str, func: AtomFn)`: Add a new atom.
  - `get(&self, name: &str) -> Option<&AtomFn>`: Lookup by name.
  - `list(&self) -> Vec<String>`: List all registered atoms.
- **Usage Example:**
  ```rust
  let mut reg = AtomRegistry::new();
  reg.register("print", print_atom);
  let atom = reg.get("print").unwrap();
  ```
- **Extension Points:**
  - Add new atoms by registering them in `src/atoms_std.rs` or via plugins (future).
- **Invariants:**
  - All atom names must be unique in the registry.
  - Atoms must not mutate world state except via their return value.

### Design Notes
- Atoms are the only way to perform I/O or mutate world state.
- All state changes are explicit and pure (no hidden side effects).
- The registry is inspectable at runtime for debugging and introspection.
- Output is always routed through an `OutputSink` for testability.

---

## Macro System (Detailed)

- **File:** `src/macros.rs`, `src/macros_std.rs`
- **Purpose:** Macros are pure, syntactic transformations of the AST. They enable high-level abstractions and code reuse, expanding to atoms or simpler macros before evaluation.

### Core Principles
- **Syntactic Only:** Macros operate solely on the AST (`Expr`). No access to world state or evaluation context.
- **Pure Transformation:** Macro expansion is a pure function: `(AST) -> Result<AST, Error>`.
- **Inspectable:** Expansion can be traced step-by-step for debugging.
- **Layered:** Macro expansion is a distinct pipeline stage after parsing, before validation/evaluation.

### Macro Function Type
- **Type Signature:**
  ```rust
  pub type MacroFn = fn(&Expr) -> Result<Expr, SutraError>;
  ```

### Macro Registry
- **Type:**
  ```rust
  #[derive(Default)]
  pub struct MacroRegistry {
      pub macros: HashMap<String, MacroFn>,
  }
  ```
- **Key Methods:**
  - `register(&mut self, name: &str, func: MacroFn)`: Add a new macro.
  - `expand_recursive(&self, expr: &Expr, depth: usize) -> Result<Expr, SutraError>`: Recursively expand macros in an expression (with depth limit).
  - `macroexpand_trace(&self, expr: &Expr) -> Result<Vec<TraceStep>, SutraError>`: Get a step-by-step trace of macro expansion.
- **Usage Example:**
  ```rust
  let mut reg = MacroRegistry::new();
  reg.register("when", when_macro);
  let expanded = reg.expand_recursive(&expr, 0)?;
  ```
- **Extension Points:**
  - Add new macros in `src/macros_std.rs` or via plugins (future).
- **Invariants:**
  - Macro expansion must terminate (depth-limited to prevent infinite recursion).
  - Macros must not perform evaluation or side effects.

### Design Notes
- Macro expansion is always pure and deterministic.
- The registry is rebuilt for each expansion pass (ensures fresh state).
- The trace facility is a powerful debugging tool for authors.
- Special forms (e.g., `if`) are handled explicitly in expansion.

### Two-Tiered Macro System (The "Sutra Way")

- **Status:** Implemented as of 2025-07-02.
- **Purpose:** To provide a macro system that is both powerful for the engine and safe/ergonomic for authors.
- **Architecture:**
  - **`Expr::If` (Primitive):** The core conditional construct is the `Expr::If` node in the AST, which is handled directly by the evaluator. It is not a macro.
  - **`cond` (Native Macro):** The primary, author-facing conditional is `(cond ...)`. It is a variadic `MacroFn` implemented in Rust that recursively expands into nested `(if ...)` macro calls.
  - **`if` (Simple Macro):** The `(if ...)` form is a simple, fixed-arity `MacroFn` that validates its three arguments and creates the primitive `Expr::If` node.
  - **`MacroTemplate` (Declarative Macros):** A system for authors to define their own simple, substitution-based macros with variadic capabilities.
- **Example Flow:**
  ```lisp
  ;; Author writes:
  (cond ((> x 10) "big") (else "small"))

  ;; `cond` expands to an `if` macro call:
  (if (> x 10) "big" "small")

  ;; `if` expands to the AST primitive:
  Expr::If { ... }
  ```
- **Rationale:** This layered approach is highly robust and compositional. It keeps the core primitive minimal while providing a powerful and ergonomic authoring experience. It also provides a safe path for user-defined macros via `MacroTemplate` without exposing the full complexity of the evaluator.

### Registry Hashing and Fingerprinting (2025-07-02)

- The macro registry now implements a `hash()` method that computes a SHA256 fingerprint of all macro names and their definitions, sorted deterministically.
- This hash is printed in the test suite (`macro_registry_hash_is_stable`) for CI traceability and can be asserted against a canonical value to prevent registry drift.
- For template/user macros, the hash includes the parameter list, variadic parameter, and macro body (pretty-printed). For native Rust macros, a stable placeholder is used.
- This approach ensures that any change to macro definitions (in code or user files) is immediately detectable in CI and review.
- See memory-bank/activeContext.md and systemPatterns.md for rationale and policy.

---

## World State (Detailed)

- **File:** `src/world.rs`
- **Purpose:** The world state is a single, deeply immutable, serializable data structure representing all simulation/game state. All state changes are explicit and tracked.

### World Structure
- **Type:**
  ```rust
  #[derive(Clone)]
  pub struct World {
      data: Value,
      prng: SmallRng, // Deterministic, seedable PRNG for randomness
  }
  ```
- **Key Methods:**
  - `World::new() -> Self`: Create a new, empty world.
  - `World::from_seed(seed: [u8; 32]) -> Self`: Create a world with a specific PRNG seed.
  - `get(&self, path: &Path) -> Option<&Value>`: Retrieve a value by path.
  - `set(&self, path: &Path, val: Value) -> Self`: Set a value at a path (returns new world).
  - `del(&self, path: &Path) -> Self`: Delete a value at a path (returns new world).
  - `next_u32(&mut self) -> u32`: Get a random u32 (for explicit, tracked randomness).
- **Usage Example:**
  ```rust
  let mut world = World::new();
  let path = Path::from(vec!["player", "score"]);
  world = world.set(&path, Value::Number(10.0));
  let score = world.get(&path);
  ```
- **Extension Points:**
  - To add new world operations, extend the `World` struct and update all relevant atoms.
- **Invariants:**
  - All world state changes must be explicit and return a new `World`.
  - The world is always serializable and path-addressable.
  - Randomness is always deterministic and seedable for reproducibility.

### Design Notes
- Uses `im::HashMap` for efficient, persistent data structures.
- All state is deeply immutable; no in-place mutation.
- PRNG is encapsulated for deterministic simulation and replay.
- Recursive helpers (`set_recursive`, `del_recursive`) ensure safe, immutable updates.
- TODO: Document serialization format and snapshotting.

---

## AST/Parser Audit (2025-07-02)

### Summary

- **Span-Carrying AST:** All AST nodes (`Expr` variants) carry a `Span` for source tracking, enabling precise error reporting and explainability.
- **Minimal, Uniform Structure:** The `Expr` enum is minimal and supports all required forms. Utility methods (`span`, `into_list`, `pretty`) are provided and documented.
- **Unified PEG Grammar:** The parser is driven by a single, formal PEG grammar (`sutra.pest`), which is the single source of truth for syntax. Both s-expression and brace-block syntaxes are supported and produce identical ASTs.
- **Uniform CST-to-AST Conversion:** The parser's public API is pure and only produces ASTs. CST→AST conversion is handled recursively, with spans preserved at every node. All atomics are parsed and converted in a uniform way.
- **Error Handling and Span Propagation:** All parse errors are mapped to `SutraError` with precise span/location. Error handling is consistent with the rest of the pipeline.
- **Documentation and Test/Production Parity:** All types and methods are documented with examples. The parser and AST are designed for round-trippability and property-based testing.
- **Single Source of Truth:** The PEG grammar is the only place where syntax is defined. The parser does not duplicate or reinterpret grammar rules.

**Conclusion:** AST and parser are fully uniform, minimal, and span-carrying. The PEG grammar is the single source of truth for syntax. Error handling and span propagation are consistent. Documentation and utility methods are present and clear.

---

## Macro System Audit (2025-07-02)

### Summary

- **Registry Pattern and Single Source of Truth:** All macros are registered via a single function (`register_std_macros` in `macros_std.rs`), called by both production and test code. The macro registry is a simple, extensible `HashMap<String, MacroFn>`, with a uniform API for registration and lookup.
- **Canonicalization and Uniform Expansion:** Path canonicalization is handled exclusively in `macros_std.rs` (`expr_to_path`), making it the single source of truth for path syntax. All author-facing macros expand to canonical AST forms using this function. Macro expansion is pure, stateless, and operates only on the AST.
- **Error Handling:** All macro errors use `SutraError` with `SutraErrorKind::Macro`, and always carry a span for precise error reporting. Helper macros ensure uniform arity and type checking, and consistent error construction.
- **Expansion Traceability and Debugging:** The macro system provides a stepwise expansion trace (`macroexpand_trace`), supporting full transparency and debuggability for authors. The CLI exposes this trace for user inspection.
- **Documentation and Test/Production Parity:** All macro expansion functions are documented with examples. The registry pattern ensures test and production parity.
- **Layered, Pure, and Extensible:** Macro expansion is a distinct, pure pipeline stage, with no access to world state or evaluation context. The macro system is fully extensible via the registry.

**Conclusion:** Macro system is fully uniform, pure, and registry-driven. Path canonicalization is centralized and consistent. Error handling, expansion, and documentation are uniform. Test and production environments are in sync.

---

## Atoms/Eval Audit (2025-07-02)

### Summary

- **Atom Contracts and Naming:** All atoms follow a uniform function signature (`AtomFn`), take canonical AST arguments, evaluation context, and parent span, and return a value and new world state. Naming is consistent and canonical. Atoms are registered via a single function, ensuring no duplication.
- **State Propagation and Immutability:** All atoms that mutate state return a new `World` instance; no in-place mutation. State is threaded through evaluation using helpers like `eval_args`, ensuring correct propagation and immutability. The evaluation engine enforces this contract.
- **Error Handling and Span Usage:** All atom errors use the `eval_err!` macro, ensuring uniform construction, span-carrying, and rich error messages. The evaluation engine propagates and enriches errors, always including spans and context. Errors for arity, type, and general issues are handled consistently.
- **Registry Usage and Extensibility:** Atoms are registered in a single, canonical registry (`AtomRegistry`), with a uniform API for registration, lookup, and listing. The registry is used identically in both test and production code.
- **Documentation and Test/Production Parity:** Atom contracts, usage, and extension points are clearly documented. Direct doctests for atoms are not feasible due to context requirements, but integration and unit tests are referenced. Test and production environments use the same registry and atom implementations.
- **Evaluation Engine Uniformity:** The evaluator is pure, stateless, and enforces canonical AST and atom contracts. All output is routed through the injectable `OutputSink` trait for testability and decoupling.

**Conclusion:** Atoms and evaluation are fully uniform, pure, and registry-driven. State propagation, error handling, and span usage are consistent. Naming, documentation, and extensibility are uniform. Test and production environments are in sync.

---

## CLI/Output and Tests Audit (2025-07-02)

### Summary

- **CLI/Output Uniformity:** The CLI is a pure orchestrator, delegating all logic to the core library. All user-facing output is centralized in a dedicated module, ensuring consistent formatting, colorization, and error display. Macro expansion traces are displayed with clear, color-coded diffs. Errors are routed through a single reporting path, using the same error types and enrichment as the rest of the system. Registry usage is canonical and test/production parity is maintained.
- **Test Suite Uniformity:** Core evaluation, macro expansion, and parser tests all use the canonical pipeline and registry builders. Tests cover math, predicates, state mutation, macro expansion, special forms, assignment macros, state propagation, path canonicalization, error cases, and round-trippability. No test-specific logic or duplication; all code paths are shared.
- **Documentation and Coverage:** Tests are well-documented, with clear comments and structure. Coverage is broad, including edge cases, error handling, and invariants.

**Conclusion:** CLI/output and tests are fully uniform, DRY, and use the same code paths as production. Output, error handling, and registry usage are consistent. Test coverage is comprehensive and mirrors production logic.

## Canonical Conditional Macro: `cond`

- `cond` is implemented as a macro that expands to nested `if` expressions. Only `if` is a primitive in the AST.
- All error and edge cases (no clauses, non-list, wrong arity, misplaced/multiple else, malformed else, empty clause, recursion) are now robustly tested.
- Error messages are clear, spanned, and author-facing.
- Migration plan: When macro system supports variadic/recursive user macros, port `cond` to user macro and remove Rust implementation.
- This approach is fully aligned with project principles: strict layering, compositionality, single source of truth, and transparency.

---

## Parser Refactor & Macro System Reliability (2025-07-02)

### Summary
- The parser is now decomposed into per-rule helpers, with robust error handling and strict, canonical dotted list validation. All unreachable!()s are replaced with structured errors that include rule, input, and span. Dotted list parsing asserts and errors on malformed shapes.
- Parser and macro system are now fully decoupled and testable. Round-trippability and robust error handling are enforced at every stage.
- Current focus: Debugging macro system test failures, especially for variadic macro parameter parsing and cond macro expansion. Ensuring parser and macro system are in sync for all edge cases.

### Impact
- The new parser structure makes subtle bugs easier to spot and fix, and supports future extensibility and onboarding.
- Error messages are now more precise and contextual, aiding debugging and test coverage.
- The strict, canonical handling of dotted lists ensures macro loader and expander reliability.

---

## Parsing Pipeline Refactor (2025-07-04)

A major, multi-phase refactor of the parsing pipeline has been adopted. The new architecture is modular, interface-driven, and maximally explicit, with each stage (CST parser, AST builder, macroexpander, validator, etc.) as a pure, swappable module. This plan is the result of extensive review and synthesis, and is now the canonical direction for all parser and macro system work.

- **Rationale:** Ensures maintainability, testability, and future extensibility. Supports robust diagnostics, editor integration, and authoring ergonomics. Aligns with Sutra's core values.
- **Migration Strategy:**
  1. Ship interfaces and trivial implementations with golden tests.
  2. Document and review contracts as a first-class citizen.
  3. Implement and test each module in isolation before integration.
  4. Incrementally migrate and integrate with the existing codebase.

See `docs/architecture/parsing-pipeline-plan.md` for the full plan, context, and changelog. All memory bank files have been updated to reference this plan.
