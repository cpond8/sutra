# **Sutra Engine: Module/File Plan**

*Last Updated: 2025-07-01*

---

## **Stage 0: Project Bootstrapping**

- **/README.md**

    - Principles, pipeline overview, usage examples

- **/Cargo.toml**

- **/src/lib.rs**

    - Expose core API only (no IO/CLI code).


---

## **Stage 1: AST and Data Types**

- **/src/ast.rs**

    - `pub enum Expr` — List, Symbol, String, Number, Bool

    - `pub type Span` (optional): for error reporting

    - Formatting and debug utilities

    - _No dependencies except std._

- **/src/value.rs**

    - `pub enum Value` — Number, String, Bool, List, Map

    - Conversions to/from Expr

    - Serialization (for snapshots)

    - _Used by evaluator, world, macros_

- **/src/world.rs**

    - `pub struct World` — deep persistent data tree (use `im::HashMap` or similar)

    - PRNG state

    - Serialization, deep cloning, diff/merge utilities

    - _No dependency on parser or macro logic._

- **/src/error.rs**

    - Error types for parsing, expansion, validation, evaluation

    - Pretty-print helpers

    - _Reused across all modules._

- **Testing:**

    - Unit tests for serialization, debug output, conversions


---

## **Stage 2: Unified PEG Parser**

- **/src/parser.rs**
    - `pub fn parse(source: &str) -> Result<Expr, SutraError>`
    - **Unified PEG-based parser for both s-expression and brace-block syntaxes.**
    - Uses the formal grammar from `src/sutra.pest`.
    - **No macro or world dependency.** Pure, stateless.

- **/src/sutra.pest**
    - The formal PEG grammar defining both the canonical s-expression and the author-friendly brace-block syntax.

- **Testing:**
    - Golden tests for valid/invalid input, edge cases, and syntax parity.


---

## **Stage 3: Atom Evaluator**

- **/src/atom.rs**

    - Atom dispatch table: atom name to Rust function

    - Atom function signature:
        `(args: &[Expr], context: &mut EvalContext) -> Result<(Value, World), SutraError>`

    - Implements all core atoms (`set!`, `add!`, etc.)

    - Output atoms take callback as argument (for testability)

    - **Only depends on ast, value, world, error**

    - **No macro or parser dependency**

- **/src/eval.rs**

    - `eval(expr: &Expr, world: &World, output: &mut OutputSink) -> Result<(Value, World), Error>`

    - TCO loop for recursion

    - Handles atom lookup and world updates

- **Testing:**

    - Unit tests for all atoms, property-based tests for state mutation


---

## **Stage 4: Macro System**

- **/src/macro.rs**

    - Macro pattern definitions (Tier 1–2, no user macros yet)

    - `expand_macros(expr: &Expr) -> Result<Expr, Error>`

    - Recursion and hygiene checks

    - Debug macroexpansion tree tracing

    - **Depends only on ast, value, error**

    - No dependency on atom, world, parser

- **Testing:**

    - Macroexpansion golden tests

    - Edge cases for recursion/hygiene


---

## **Stage 5: Validation**

- **/src/validate.rs**

    - Structural validation (pre-expansion): required fields, malformed AST

    - Semantic validation (post-expansion): types, known atoms, etc.

    - Error aggregation for author feedback

    - **Depends on ast, macro, error**

    - **No dependency on atom, world, parser**

- **Testing:**

    - Failure cases for author mistakes

    - Pass/fail golden validation files


---

## **Stage 6: CLI/Test Harness (Optional, Parallel to Stages 3–5)**

- **/src/cli.rs** or **/src/bin/sutra.rs**

    - Entry point for:

        - Running s-expr scripts

        - Macroexpansion tracing

        - World stepping

        - Output/capture options

    - **Depends only on public lib API**

    - _Never imports macro, parser, etc. directly; only via API_

- **/src/testdata/**

    - Golden test files, macroexpansion expectations, example scripts

- **Testing:**

    - CLI and integration tests

    - Regression tests on golden data


---

## **Stage 7: Macro Library (Pools, History, Storylets)**

- **/src/macros_std.rs**

    - Canonical macro patterns:

        - `storylet`, `choice`, `pool`, `history`, `select`, etc.

    - All implemented as macros, **not engine code**

    - **Re-exported in macro system**

    - _Documented with canonical usage and expansion tests_

- **Testing:**

    - Macroexpansion and evaluation tests for all major narrative/gameplay patterns


---

## **Stage 8: Docs & Examples**

- **/src/examples/**

    - Canonical s-expr scripts: storylet, pool, simulation, choice patterns

- **/docs/**

    - Markdown docs for:

        - System overview

        - Macro/atom reference

        - Pipeline diagrams

        - Example content


---

## **Stage 9: (After Core) Brace-Block Translator (MERGED INTO STAGE 2)**

- **Note:** This module is no longer needed as its functionality is now integrated into the unified parser in `src/parser.rs`.


---

## **Stage 10: (Continuous) Macro/Atom Evolution and Refactoring**

- As needed, new files/modules for:

    - Advanced macro features (user macros, modules)

    - Grammar/template macros

    - Simulation/agent loop patterns


---

# **File/Module Overview Table**

|File/Module|Purpose|Depends on|
|---|---|---|
|ast.rs|AST types, formatting|std|
|value.rs|Data values, conversions, serialization|std, im|
|world.rs|Persistent world state|value, im|
|error.rs|Error types, pretty-printing|std|
|**parser.rs**|**Unified PEG Parser (S-Expr/Brace) → AST**|**ast, error, pest**|
|**sutra.pest**|**Formal PEG Grammar**|**-**|
|atom.rs|Atom functions|ast, value, world, error|
|eval.rs|Eval engine|ast, atom, world, value, error|
|macro.rs|Macro system|ast, value, error|
|macros_std.rs|Narrative/gameplay macro patterns|macro, ast|
|validate.rs|Structural & semantic validation|ast, macro, error|
|cli.rs/bin/|CLI/test harness|lib API|
|examples/, docs/|Content, tests, docs|none|

---

**Every file can be unit/integration tested independently.**
**All modules, except CLI/test harness, are pure and stateless (except world state, which is persistent, never mutated in place).**
**Extension is possible by adding new macros, atoms, or validators—never by forking or patching core logic.**

---

# **Critical Review & Iteration: File/Module Plan**

---

## **General Principles Check**

- **Single Responsibility:** Each module should only do one job.

- **No Leaky Abstractions:** Data and control flow should be explicit between layers.

- **No Circular Dependencies:** Every module must be importable and testable in isolation.

- **Extensibility:** Adding new atoms, macros, or syntax should never require core refactoring.

- **Testability:** All logic must be unit-testable with minimal scaffolding.

- **Statelessness/Purity:** All modules except world state (which is persistently immutable) must be stateless and functional.


---

## **Module-by-Module Critique and Opportunities**

---

### **ast.rs**

- **Good:** Only for tree structure.

- **Opportunity:** Consider splitting into `expr.rs` (AST) and `span.rs` (source location tracking), for cleaner separation and less risk of bloat if spans get more complex (eg, for better error highlighting).


---

### **value.rs**

- **Good:** Houses only runtime data values.

- **Potential Flaw:** Avoid putting world-specific logic here. All “world as a value tree” logic should live in world.rs.


---

### **world.rs**

- **Check:**

    - Is all mutation (even helper functions) always pure, returning a new World?

    - Should we add a `WorldDiff` or “undo” feature here for future “time travel” debugging? (Not for MVP, but don’t design it out.)

- **Opportunity:** Make “path” navigation and updates idiomatic and safe.

    - Eg, encapsulate world traversal in a small API so that you never have raw stringly-typed paths scattered through the code.


---

### **parser.rs**

- **Potential Flaw:**

    - Parser should never perform semantic validation—leave this strictly to validator.

    - Output AST must preserve original source locations (for errors and macroexpansion explain).

- **Opportunity:**

    - If the AST is generic over span type, you can later drop spans for runtime, making memory and equality checks easier.


---

### **atom.rs & eval.rs**

- **Flaw to Watch:**

    - Don’t allow “macro-only” atoms or “engine magic” atoms—ensure atom table is transparent, documented, and as small as possible.

- **Opportunity:**

    - Consider formalizing the “atom registry” as data, so user code can inspect what atoms exist (e.g., for help/CLI).

    - Design atom function signature so that all outputs—including errors, world updates, print/output events—are explicit. No hidden side-effects.

- **Output:**

    - Output hooks should be injectable for testing, not global or static.


---

### **macro.rs & macros_std.rs**

- **Potential Flaw:**

    - Avoid leaking macro implementation details (eg, expansion strategies, hygiene tricks) into AST or atom layer.

    - Be wary of accidental “privilege escalation”: everything in macros_std.rs should be written _in the macro language_, not as Rust.

- **Opportunity:**

    - If you support user macros later, plan for namespace management and error reporting (even if deferred).

    - Document macro expansion and output for every standard macro.


---

### **validate.rs**

- **Check:**

    - Structural vs. semantic validation—ensure these are two passes, so you can catch bad authoring early and bad expansions separately.

- **Opportunity:**

    - Consider a “linting” subsystem (optional), so best practices can be flagged without hard errors.


---

### **cli.rs**

- **Flaw:**

    - Never let CLI functions call internal helpers directly; always go through the public API.

- **Opportunity:**

    - Expose macroexpansion, validation, evaluation, and world snapshot as separate CLI subcommands for granular scripting and CI integration.


---

*This section has been removed as `brace_translator.rs` is obsolete.*


---

### **examples/**

- **Opportunity:**

    - Golden tests for macroexpansion and world snapshots, not just CLI output.

    - Encourage “living documentation”—all tested, all up to date with main engine.


---

## **Cross-Module Concerns and Improvements**

- **Error Handling:**

    - Consider a shared, extensible error type with variants for parse, macro, validation, eval, IO, etc., all supporting source span info.

- **Public API:**

    - Expose a small, composable API (eg, `parse`, `expand`, `validate`, `eval`) so all downstream consumers are insulated from internals.

- **Testing:**

    - Add property-based tests (eg, eval should be deterministic and world-pure; macroexpansion is idempotent on atoms).

- **Documentation:**

    - Every module should have a short “purpose” doc comment at top, not just in README.


---

## **Risks and Mitigations**

- **Hidden Coupling:**

    - World “paths” as strings: fix by using a path struct or type-safe wrapper, to avoid typos.

    - Macro recursion/infinite expansion: fix with expansion depth counters and clear errors.

    - CLI and library drifting out of sync: enforce that all CLI uses only public API.

- **Future-Proofing:**

    - Don’t overfit to current patterns (eg, “storylet” as a special case). Macro and atom systems must be pattern-agnostic.

- **Maintainability:**

    - Every new macro, atom, or feature must have a test, a doc, and a golden example.

    - Never “fast-path” a quick hack into atom or macro_std—always test real-world usage and maintain transparency.


---

## **Final Notes on Alignment**

- The architecture remains holistically unified, each module is an island with bridges only where strictly needed.

- Engine is always library-first, never IO-first.

- All extensions—new syntax, new atoms, new macros—add new files or tests, never rewrite old code.

- All debugging and introspection features (macroexpansion trace, world diff, etc.) are built in from day one.


---
