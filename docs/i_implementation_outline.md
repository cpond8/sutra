---

# **Sutra Engine Staged Implementation Plan (Refined)**

---
## **Stage 0: Project & Philosophy Bootstrapping (COMPLETED)**

- **0.1:** **(✓)** Guiding principles reaffirmed and documented.
- **0.2:** **(✓)** Project initialized with Cargo.
- **0.3:** **(✓)** Core `sutra` library crate scaffolded.


---

## **Stage 1: Canonical AST and Data Types (COMPLETED)**

- **1.1:** **(✓)** `Expr` enum defined in `src/ast.rs`.
- **1.2:** **(✓)** `Value` enum defined in `src/value.rs`.
- **1.3:** **(✓)** `World` struct defined in `src/world.rs` using `im::HashMap`.
- **1.4:** **(✓)** `Debug` and `PartialEq` derived for core types, enabling test assertions.


---

## **Stage 2: Canonical S-Expr Parser (COMPLETED)**

- **2.1:** **(✓)** Recursive-descent parser implemented in `src/parser.rs`.
- **2.2:** **(✓)** `SutraError` and `Span` system provides robust error reporting.
- **2.3:** **(✓)** Parser is covered by integration tests in `tests/core_eval_tests.rs`.


---

## **Stage 3: Atom Engine / Evaluator (COMPLETED)**

- **3.1:** **(✓)** Implemented atom evaluation as pure functions: `(AST, World) -> (Result<Value>, World)`.
- **3.2:** **(✓)** Implemented full Tier 1 atom set: `set!`, `del!`, `get`, `+`, `-`, `*`, `/`, `eq?`, `gt?`, `lt?`, `not`, `cond`, `list`, `len`.
- **3.3:** **(✓)** World updates are immutable, returning a new world state on each mutation.
- **3.4:** **(✓)** `OutputSink` trait established for injectable I/O.
- **3.5:** **(✓)** Comprehensive integration test suite created in `tests/core_eval_tests.rs`, validating the parser, all atoms, and the full evaluation pipeline.
- **Bonus:** **(✓)** "Auto-get" feature implemented in the evaluator, allowing direct use of symbols as world paths.


**Optimization:**

- **(✓)** Evaluator is structured as a pure function, ready for future TCO.


---

## **Stage 4: Macro System (Expansion Only)**

- **4.1:** Implement pattern-matching macro expansion engine (Tier 1–2 macros).

- **4.2:** Macro expansion is **purely syntactic:** AST-in, AST-out—never touch World.

- **4.3:** Macro hygiene: name hygiene for locals, recursion limits to avoid runaway expansions.

- **4.4:** Provide debug tracing for macroexpansion (author-inspectable at any step).

- **4.5:** Write and test all standard macros (storylet, choice, etc.) as macros, not as atoms or engine logic.

    - **Flaw to avoid:** Never let macro code “sneak” into atom engine—keep macro system and atoms fully layered.

- **4.6:** Macroexpand “explain” CLI/test tool for authors.


**Improvement:**

- Macro system should be generic: treat author, system, or future user macros identically (no “privileged” macros).


---

## **Stage 5: Validation and Author Feedback**

- **5.1:** Validation functions:

    - Structural validation: malformed AST, missing macro fields, etc.

    - Semantic validation: type mismatches, duplicate definitions, etc.

- **5.2:** Integrate validation **before** macro expansion (parse-time) and **after** (expanded form).

- **5.3:** Author-centric error reporting—print errors in the original surface syntax where possible.

    - **Optimization:** Validation should be functional and stateless; emit all errors found, don’t stop at first.


**Principle:**

- Keep validator in its own crate/module; don’t couple to macro or atom implementations.


---

## **Stage 6: Test Harness and CLI (Optional at First, Then Iterative)**

- **6.1:** Build a minimal test CLI to run s-expr scripts, macroexpand, step World, and debug.

- **6.2:** Provide snapshot output and macroexpansion traces for every test.

    - **Flaw to avoid:** Don’t let CLI code pollute engine—engine should be library-first.


**Improvement:**

- Add a `ScriptTest` trait or similar to easily compose test scripts, macroexpansion checks, and golden output.


---

## **Stage 7: History, Pools, and Selection (Macro Layer)**

- **7.1:** Implement “history” tracking (seen events), pool selection, weighted/random selection as macros or as atom extensions (if truly irreducible).

- **7.2:** Test against QBN, storylet, and pool selection patterns from design docs.

- **7.3:** Document these as canonical usage patterns; provide macroexpansion for all.


**Optimization:**

- Make sure no part of “storylet selection” is privileged—macro-driven all the way.


---

## **Stage 8: Documentation and Example Library**

- **8.1:** Inline docstrings, macro signatures, and usage examples in code.

- **8.2:** Minimal Markdown/README explaining system pipeline, principles, extension, and debugging.

- **8.3:** Canonical example scripts and golden test suite.


---

## **Stage 9: (After MVP) Brace-Block DSL Translator**

- **9.1:** Build brace-block-to-s-expr translator as a pure function/module. **Test this independently; don’t entangle with AST, macro, or world logic.**

- **9.2:** Add CLI/test harness option for brace-block input; output canonical s-expr for debug.

- **9.3:** Expand and refactor tests/examples to use both input modes.


---

## **Stage 10: Continuous Iteration, Macro Growth, and Future-proofing**

- **10.1:** Harden macro expansion (user macros, modules, hygiene, error explain).

- **10.2:** Refactor atom set if any new irreducible ops arise from real macro/authoring pain.

- **10.3:** Add advanced macro features (grammar/templating, agent simulation) only if needed, and only as macros unless proven otherwise.

- **10.4:** Plan for future UI, editor, or online integration.


---

## **Improvements and Checks vs. Principles**

- **Pipeline is strictly layered and functional:** All stages are pure, pass data, and never cross layers.

- **All code is testable, inspectable, and debuggable at every stage.**

- **No privileged “magic” in atoms or macro system; everything is author-explorable.**

- **Validation and error reporting prioritized early, not deferred to “cleanup.”**

- **World state is a single, persistent tree—no leaking state, no accidental mutation.**

- **No dependencies between surface syntax and the core—engine can run on canonical s-expr only.**

- **Macro and atom sets evolve only through test-driven, documented need—never preemptively.**

- **All author-facing language constructs (storylet, pool, etc.) are implemented as macros.**


---

## **Pipeline Visualization (Updated)**

```text
[ S-Expr Script ]
      |
[ S-Expr Parser ]
      |
[ Validator ] (structural)
      |
[ Macro System ]
      |
[ Validator ] (expanded)
      |
[ Atom Engine ]
      |
[ World State / Output ]
```

**(Brace-block translator inserts only after MVP, as an optional front-end. All authoring, debugging, and testing is pipeline-aware and inspectable at each stage.)**

---

## **Potential Flaws/Oversights to Avoid**

- Do **not** entangle CLI/IO with any engine layer (library-first, always).

- Do **not** let macro or atom sets become “fat” with rarely used or duplicate functionality.

- Avoid “quick hacks” for authoring pain points—always macroize first, then promote to atom only if necessary.

- Do **not** build in assumptions about pool structures, thread patterns, or “standard” game data; keep the engine truly generic.

- Do **not** “leak” validation, macro, or parsing logic into the World state or evaluation loop.

- **Document everything** as you go; treat the engine as a model for other projects.


---
