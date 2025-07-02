---
status: authoritative
last-reviewed: 2024-07-03
summary: Project vision, philosophy, and guiding principles for the Sutra Engine.
---

# 01_SUTRA_ENGINE_PHILOSOPHY_AND_DESIGN_PRINCIPLES.md

---

LAST EDITED: 2025-06-30

> **⚠️ LIVING DRAFT—NOT FINAL SPECIFICATION**
>
> This document is an evolving, *in-progress* record of Sutra's design philosophy and guiding principles. It is **not a finalized or normative specification**. Everything described here (except the foundational philosophy and aims) is subject to ongoing exploration, questioning, and change.
>
> The purpose of this file is to capture design intent, document our reasoning, support future iteration, and provide a north star for anyone (especially future-you) working on this project. Update liberally as the design evolves.

---

## 1. Preamble: Nature of This Document

*   **Audience:** This file is intended as a private, internal design reference—not public documentation or marketing.
*   **Purpose:** To capture the *why*, *what*, and *how* of Sutra's evolving design philosophy and principles, to keep future work grounded and context-rich.
*   **Update policy:** Revise, annotate, and cross-reference as new insights arise. Note all major design changes in the appendices.

---

## 2. Introduction

### 2.1 Project Vision (“Why Sutra?”)

Sutra aspires to be a universal substrate for **compositional, emergent, and narrative-rich game systems**. The guiding vision is an engine that allows designers to build everything from interactive fiction to deep simulations—**from a minimal, maximally compositional core**.

**Key aspirations:**

*   Model any gameplay or narrative system via *composition* of simple parts (“atoms and macros”).
*   Enable robust, transparent, and infinitely extensible authoring.
*   Ensure the core is simple enough to be fully understood, yet powerful enough to encode anything—matching the spirit of the lambda calculus, Lisp, and digital logic.

### 2.2 Scope and Status

*   Sutra is **early-stage and highly experimental**. Most decisions are provisional.
*   This document, and all design artifacts, exist to **guide and record iterative evolution**—not to enforce a "final" architecture.

### 2.3 How to Use and Update This Document

*   Whenever a design question arises, consult this document for context and rationale.
*   When a major change or new insight is made, annotate here (and in the Appendix/Changelog).
*   Each section should reflect both *current thinking* and *open questions*.

---

## 3. Foundational Inspirations

### 3.1 Scheme, Lisp, Lambda Calculus: Minimalism as Power

*   **Scheme/Lisp** demonstrated that a *tiny set of core forms* can serve as the basis for an expressive, extensible, and Turing-complete language.
*   The "everything is an s-expression" and "everything reducible to lambda" principles inspire Sutra's "atoms and macros" approach.
*   **Sutra takes this further:** all engine logic and narrative is canonically represented as nested lists (s-expressions), with both Lisp-style parentheses **and** block-brace syntax supported one-to-one, interchangeably.
*   We are **not copying Scheme**, but embracing its spirit of simplicity, composability, and macro extensibility—while providing a surface syntax comfortable for authors beyond the Lisp community.

### 3.2 Logic Gates, Turing Machines, Physics

*   The *logic gate* metaphor: all of digital computation comes from just a few simple gates—composition yields power.
*   **Turing completeness** as a minimal standard: If our atoms + macros can simulate a Turing machine, we have "enough."
*   Like the universe: from a handful of particles (or rules), emergent complexity arises.

### 3.3 Real-World Narrative Design (Emily Short, QBN, Storylets)

*   The design patterns of **quality-based narrative (QBN)**, **storylets**, and **emergent systems** (see Emily Short's writings) are used as a *testbed* for the engine's expressiveness.
*   We aim for Sutra to be the *substrate* for all these narrative patterns—not to bake in a single narrative structure.

### 3.4 Why Minimalism?

*   **Transparency**: The smaller and more explicit the core, the easier it is to debug, reason about, and extend.
*   **Robustness and flexibility**: Compositional engines resist "feature bloat" and privileged code.
*   **Extensibility**: All "advanced" features can be built as macros or libraries atop the same core.
*   **User empowerment**: Designers are not boxed in by rigid, "hard-coded" systems.

---

## 4. Philosophy of Minimalism and Composition

### 4.1 Atoms and Macros: Our Core Model

*   **Atoms**: The truly irreducible "micro-operations" (queries, mutations, control flow, output, randomness).
*   **Macros**: Any higher-level language construct or feature—**including everything from "storylet" to "loop" to "choice"**—is built as a macro that expands to atoms and other macros.
*   **Why?** This model allows for *arbitrary abstraction* and extension, with a minimal, stable foundation.

### 4.2 Uniform Syntax: S-Expressions and Brace-Block Equivalence

*   **Canonical representation**: All code is, at its core, an s-expression AST—just like Lisp or Scheme.
*   **Surface syntax**: Sutra offers two fully equivalent, losslessly interchangeable authoring styles:

    *   **Parenthesis-based s-expressions** (for maximum explicitness and power-user workflows)
    *   **Block-brace, newline-driven syntax** (familiar and accessible for authors from other backgrounds)
*   **Block-brace syntax**: Every call or operation is a line; blocks are in `{}` braces; each statement/child is a new line.
*   **One-to-one mapping:** There is *no* loss of structure, semantics, or arity between brace-block and s-expr. The canonical AST is always a tree of lists, regardless of surface style.
*   **Why?** This approach gives authors a clear, accessible syntax, while preserving compositional power and full introspection. Power users and tools can use s-expr directly; others may prefer braces.

**Example (brace-block):**

```
storylet "find-key" {
  and {
    is? player.location "cellar"
    has? player.items "rusty-key"
  }
  do {
    print "You unlock the door."
    set! world.door.unlocked true
  }
}
```

**Equivalent (s-expr):**

```lisp
(storylet "find-key"
  (and
    (is? player.location "cellar")
    (has? player.items "rusty-key"))
  (do
    (print "You unlock the door.")
    (set! world.door.unlocked true)))
```

*Tooling can convert between them freely; all macroexpansion and evaluation is done on the AST.*

### 4.3 The Goal of Turing Completeness

*   The engine should support *any* computable gameplay or narrative system, including:

    *   Unbounded looping/recursion
    *   Simulation, scheduling, agent-based systems
    *   Emergent, system-driven narratives
*   **Why?** Ensures the engine is never a "dead end" for any idea.

---

## 5. Guiding Design Principles (Living List)

This list is not a "constitution," but a *current* set of touchstones—always open to refinement as the project evolves.

1.  **Minimal Irreducible Atom Set**
    *   Only include primitives that cannot be composed from others.
    *   Avoid redundancy or overlap.

2.  **Maximal Compositionality**
    *   All "features" beyond atoms are composed as macros.
    *   No "privileged" constructs; even core narrative/game patterns are just macro libraries.

3.  **Implicit Value Resolution ("Auto-Get")**
    *   Authors **never** write `get` to fetch a value.
    *   In any value context (math, comparison, etc.), a path is automatically resolved to its value.
    *   This creates a clean, spreadsheet-like authoring experience where names are used directly.

4.  **Explicit and Intentional Mutation**
    *   State-changing operations are **always** explicit and visually distinct, marked with a `!` suffix (e.g., `set!`, `add!`, `sub!`).
    *   Pure, non-mutating expressions (`+`, `-`, `*`, `/`) **never** change state; they only compute and return a value.
    *   There is no operator overloading; the difference between a calculation and a mutation is unambiguous.

5.  **Consistent and Accessible Predicate Naming**
    *   All boolean-returning functions, atoms, and macros consistently end in `?` (e.g., `is?`, `has?`, `gt?`).
    *   This makes conditional logic immediately identifiable.
    *   To enhance readability, comparison operators are provided as macros with both concise canonical names (`gt?`, `lte?`) and human-readable aliases (`over?`, `at-most?`), allowing authors to choose the style that best suits their needs.

6.  **Pure Functions and Immutability by Default**
    *   Operations are pure expressions unless explicitly marked as mutating.
    *   All world state is deeply immutable; mutations yield new state, preserving the original.

7.  **Single Source of Truth**
    *   The world state is a single, serializable, inspectable data structure.
    *   No hidden or duplicated state.

8.  **Separation of Concerns**
    *   Parsing, macro-expansion, validation, evaluation, and presentation are strictly layered.
    *   Authoring, logic, and presentation are fully decoupled.

9.  **Transparency and Traceability**
    *   All computation is inspectable and debuggable, down to the atom level.
    *   Macro expansion and world diffs are always available for tracing.
    *   **All code is convertible between brace-block and s-expr for debugging, author preference, and tooling.**

10. **Determinism and Reproducibility**
    *   All randomness is explicit and tracked as part of world state.
    *   Engine runs are reproducible given the same world state and code.

11. **Extensibility via Macros/Modules**
    *   Authors can define new constructs, features, and syntactic sugar—no engine changes required.
    *   Macro libraries and modules are first-class.

12. **Pragmatism Over Dogma**
    *   Every principle is subject to challenge and revision in the face of real authoring needs.
    *   "Minimalism with a human face"—clarity and usability come first.

---

## 6. Pragmatism and Evolution

### 6.1 "Pragmatic Minimalism": A Policy for Extension

*   **Minimal core**: Start with as few atoms as possible.
*   **Macros/libraries**: Build everything else as macros—test in real authoring scenarios.
*   **Compromise**: If authoring pain or repeated boilerplate emerges, *consider* promoting common macro idioms to "standard macros" or, rarely, new atoms.
*   **All changes must be justified**: Each addition or extension should be documented here, with reasons and examples.

### 6.2 Iterative and Empirical Process

*   **Everything in these docs besides the core philosophy is provisional.**
*   Each pattern, atom, or macro is open to refactoring or replacement.
*   Use authoring exercises, test scripts, and feedback to guide improvements.
*   Keep an "Open Questions and Next Steps" log in the appendices.

### 6.3 On Documentation and Reflection

*   These files are *not* fixed specifications—they are working notebooks.
*   Regularly update with:

    *   Why a new idea was adopted (or rejected)
    *   What was hard or surprising in practice
    *   Author/tester feedback
    *   Reflections on usability, extensibility, and transparency

---

## 7. The Role of Author Experience

### 7.1 Syntax and Surface

*   Sutra supports both brace-block and classic s-expression syntax, with a one-to-one, lossless mapping.
*   *Why this matters*: Authors can choose the style that fits their background, team, or tooling.
*   The canonical syntax is always a nested list AST; the author-facing surface is user-chosen.
*   Tooling allows macroexpansion, debugging, and author feedback in either style.

### 7.2 Macro Libraries and Patterns

*   All user-facing "features" (storylets, choices, cost gates, cycles, etc.) are macro libraries built atop the atom set.
*   Authors are encouraged to use, extend, or rewrite these macros to fit their workflow.
*   This empowers experimentation and rapid iteration.

### 7.3 Tooling and Feedback

*   Tooling (macroexpansion, world diffing, debugging) is as important as the language itself.
*   Authors (like your future self!) should always be able to "see what's really happening" under the hood—in either syntax.
*   Syntax, macro libraries, and error messages should *evolve* in response to real usage and test projects.

---

# Appendices

---

## A. Mini-History of Minimalist Languages

*   **Scheme and R5RS**: Proved a tiny, uniform core (lambda, quote, if, define) can support an entire programming paradigm; most other constructs (let, cond, and, or, etc.) are macros or "derived forms."
*   **Other Minimal Cores**: Janet, PicoLisp, Forth, and others all exemplify how powerful abstraction emerges from composition.
*   **Key lesson**: The smaller the core, the easier the language is to reason about, extend, and reimplement.

---

## B. Emily Short's QBN/Storylet Design Summarized

*   **Storylet model**: Narrative content is modular, eligibility-driven, and effectful; game state ("qualities") is the central driver.
*   **Why relevant?**: Sutra's goal is to provide a foundation flexible enough to *express* (but not constrain) all these narrative and gameplay patterns.
*   **See File 3 for implementation patterns mapped to Sutra macros/atoms.**

---

## C. Pitfalls and Anti-Patterns in Minimalism

*   **"Too clever" minimalism**: Rejecting all sugar or abstraction for purity's sake often results in inaccessible, impractical systems.
*   **Complexity inversion**: If macros are so convoluted they hide logic, the gain in minimalism is lost.
*   **Privilege in the macro layer**: Beware accidental "engine code" sneaking in through macros that become de facto required or too opaque.
*   **Best practice**: Keep macro expansion transparent and approachable; avoid "macro hell."

---

## D. Design Notes and Reflections

*   **Major decisions to date:**

    *   Adopted atoms/macros as core model for compositional power.
    *   Decided on fully explicit, interchangeable brace-block and s-expression syntax for authoring and canonical representation.
    *   Committed to macroexpansion transparency for all features.
    *   Chose Turing-completeness as minimal bar for expressiveness.
    *   **Refined the authoring language based on key principles (2025-06-30):**
        *   **No raw `get`:** All value fetching is implicit ("auto-get").
        *   **Explicit mutation:** All state changes use `!` operators (`add!`, `set!`). Pure math operators (`+`, `-`) do not mutate.
        *   **Consistent predicates:** All boolean checks use `?` operators (`is?`, `>?`).
*   **Ongoing debates:**

    *   Which atoms are truly irreducible vs. which are "convenient."
    *   How best to balance minimalism and author comfort.
    *   How much surface syntax to provide before breaking regularity.
*   **Unresolved/open:**

    *   See "Open Questions and Loose Ends" in the appendices.

---

## E. References and Further Reading

*   [Scheme (Wikipedia)](https://en.wikipedia.org/wiki/Scheme_%28programming_language%29)
*   SICP: *Structure and Interpretation of Computer Programs*
*   [Emily Short's blog](https://emshort.blog/)
*   [Failbetter Games: Quality-Based Narrative](https://www.failbettergames.com/)
*   [Write Yourself a Scheme in 48 Hours (Haskell)](http://en.wikibooks.org/wiki/Write_Yourself_a_Scheme_in_48_Hours)
*   [Build Your Own Lisp in Rust](https://fasterthanli.me/series/build-your-own-lisp-in-rust)
*   [lispy-in-rust](https://github.com/DoctorWkt/lispy-in-rust)

---

## F. How to Use and Update This File

*   **When to update:** After any significant insight, design change, or user/author test.
*   **How to annotate:** Add rationale and date to "Design Notes and Reflections." Mark unresolved questions with "TODO".
*   **How to cross-reference:** Link to related sections/files for decisions affecting syntax, macro system, architecture, or authoring patterns.

---

## G. Open Questions and "Loose Ends"

*   What atoms, if any, might be promoted or demoted as the macro library matures?
*   How will large-scale simulation and agent-based systems stress-test the current macro system?
*   Are there authoring "pain points" not yet fully addressed by the planned syntax and macro approach?
*   What meta-features or tooling would most support authors and maintainers in long-term evolution?

---

**END OF FILE 1**
(Review, extend, and iterate—this is your evolving author's lab notebook!)
