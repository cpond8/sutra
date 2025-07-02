---
status: living
last-reviewed: 2024-07-03
summary: Core architecture, atom set, and system overview for the Sutra Engine.
---

# 02_SUTRA_CORE_ARCHITECTURE_AND_ATOM_SET.md

---

LAST EDITED: 2025-06-30

⚠️ LIVING DRAFT—NOT FINAL SPECIFICATION

This document is a snapshot-in-progress of Sutra's core architecture and atom set. All structures, systems, and APIs described here are subject to revision as the design evolves.

The aim is to document not only the current architecture but also the rationale, alternatives, and open questions, to support ongoing, disciplined iteration.

See File 1 for the project's foundational philosophy and guiding principles.
Update this file whenever architecture, atom set, or macro system changes.

---

## 1. Preamble: Draft Status and Iterative Intent

- This document is not a contract or finalized spec—it is a "living notebook" for design, planning, and critical reflection.
- As new macro patterns emerge, as author needs evolve, or as simulation/gameplay requirements change, update this file to reflect both the what and the why.

---

## 2. Current High-Level Architecture

### 2.1 Engine Pipeline

The Sutra engine is structured as a sequence of pure, compositional, and strictly layered modules:

parse → macro-expand → validate → evaluate → output/presentation

Each layer is decoupled, testable, and extensible:

- Parse: Converts user code (brace-block or s-expr) to AST (canonical s-expr form).
- Macro-expand: Applies macro definitions to AST, producing "flattened" code (atoms + expanded macros).
- Validate: Checks for errors, illegal atoms, or bad structure before runtime.
- Evaluate: Applies atoms to world state; produces new world state or computed values.
- Output/presentation: Renders final results (text, choices, UI, etc.), completely decoupled from core logic.

### 2.2 Data Flow and Single Source of Truth

- World state is a single, serializable, deeply immutable data structure, accessible everywhere by path.
- Macros never mutate the world; only atoms can produce side-effects (except for explicit randomness, which is tracked).
- Debugging and traceability: At every step, macroexpansion and world state diffs are available for inspection.

### 2.3 Module Boundaries and API Surfaces

- Each layer exposes a pure API—no hidden state or side effects.
- Authors and tools interact primarily with macro libraries and author-facing syntax; only core developers need to touch atoms and engine APIs.

### 2.4 Areas Under Investigation

- Macro system: Final boundaries (compile-time vs runtime expansion) still being explored.
- Scheduling/Simulation: How best to represent real-time or tick-based systems (scheduler as macro/module, not engine core).
- Type system/validation: What level of static analysis to require or support.

---

## 3. Syntax: Canonical and Author-Facing Forms (Updated)

### 3.1 Canonical Representation: Uniform Block-List AST

All Sutra code is canonically represented as a **nested collection of lists**—an s-expression AST, directly mapping to Lisp-style syntax. Every command, macro, or primitive operation is a list, where the first element is the function or macro name and the following elements are its arguments (which may be literals, further calls, or nested lists).

This canonical structure can be written in **two fully equivalent, lossless syntaxes**:

- **Classic s-expression (parentheses) style**, familiar to Lisp users
- **Block-brace syntax**, familiar to users of C-family, JSON, or modern scripting languages

Both syntaxes are first-class, interchangeable, and map one-to-one. The engine and all tooling operate on the AST, not the surface syntax, so authors (and tools) may freely use or convert between both.

---

### 3.2 Block-Brace Authoring Syntax

The recommended default syntax for most authors is a **brace-block, newline-driven style**:

- **Every top-level command/call appears on its own line.**
- **Blocks (arguments, children, or sequences) are always enclosed in `{ ... }` braces.**
- **Each statement or operation in a block is placed on its own line.**
- **No indentation or inline separators are required for the parser, though indentation is encouraged for readability.**
- **Arguments may be literals, identifiers, grouped calls, or blocks.**
- **Strings, numbers, booleans, and identifiers are all valid atoms.**
- **Comments may begin with `#` or `//` and extend to the end of the line.**

**Example:**

```
storylet "find-key" {
  and {
    is? player.location "cellar"
    has? player.items "rusty-key"
  }
  do {
    print "You unlock the door."
    set! world.door.unlocked true
    choice {
      "Go up the stairs" {
        set! player.location "attic"
        print "You climb to the attic."
      }
      "Search the cellar" {
        print "You search the room."
        add! player.inventory "strange-coins"
      }
    }
  }
}
```

This syntax is **unambiguous**: the only structural markers required are braces (`{` and `}`), and every new statement or argument starts on a new line. There is no reliance on indentation, colons, or other punctuation for structure. The parser need only tokenize by line and match braces to construct the AST.

---

### 3.3 Canonical S-Expression Form (Lisp-Style)

Every brace-block script directly maps to a classic s-expression with parentheses. The conversion is one-to-one and lossless; all structure, order, and arity is preserved.

**Example:**

```lisp
(storylet "find-key"
  (and
    (is? player.location "cellar")
    (has? player.items "rusty-key"))
  (do
    (print "You unlock the door.")
    (set! world.door.unlocked true)
    (choice
      ("Go up the stairs"
        (set! player.location "attic")
        (print "You climb to the attic."))
      ("Search the cellar"
        (print "You search the room.")
        (add! player.inventory "strange-coins")))))
```

Authors may use either syntax interchangeably, or convert between them as desired. Tooling (macroexpansion, debugging, etc.) operates on the canonical AST, and can present code in either form.

---

### 3.4 Rationale and Benefits

- **Explicitness:** All structure is explicitly marked by braces or parentheses; there is never ambiguity about argument grouping or nesting.
- **Parser Simplicity:** No indentation, offside rules, or ambiguous block starts/ends; only braces or parentheses define structure.
- **Author Accessibility:** The brace-block style will feel familiar to most non-Lisp programmers, while Lisp enthusiasts may use classic s-expr syntax.
- **Macro System Power:** Macros operate on the unified AST; code written in either syntax is treated identically.
- **Tooling and Round-Tripping:** Scripts can be automatically converted between brace-block and paren-based forms with no loss or transformation. Macroexpansion and code transformations can be presented in the author's preferred syntax.
- **Robustness:** No structural information is lost or inferred; structure and semantics are preserved across all transformations and expansions.

---

### 3.5 Macro System Implications

- **Macros are defined and expanded on the canonical AST, agnostic to source syntax.**
- **Pattern-matching and rewriting macros is equally simple and transparent in both styles.**
- **Macro libraries, modules, and all code transformations are surface-syntax-agnostic.**
- **Authors may define, use, and debug macros in their preferred style.**

---

### 3.6 Example: Macro Definition and Expansion (Brace Syntax)

**Macro definition (pseudocode):**

```
define storylet id cond_block do_block {
  cond {
    cond_block
    do do_block
  }
}
```

**Usage:**

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

**Expands to:**

```
cond {
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

(The same holds for s-expression syntax.)

---

## 4. The Atom Set

The atom set is the **irreducible core** of the engine. These are the fundamental operations from which all other author-facing functionality is composed via macros. Authors **do not** use most of these atoms directly.

### 4.1 Atom Table: Names, Signatures, Semantics

| Atom        | Args        | Type         | Semantics (What it does)                        | Notes                                   |
| ----------- | ----------- | ------------ | ----------------------------------------------- | --------------------------------------- |
| get         | path        | Query        | Fetch value at path in world                    | **Internal use only.** Not for authors. |
| set!        | path, value | Mutation     | Set value at path in world                      | Fundamental state change                |
| del!        | path        | Mutation     | Delete value at path                            | Only way to remove keys/slots           |
| push!       | path, value | Mutation     | Append value to array at path                   | Needed for dynamic arrays/lists         |
| pull!       | path, value | Mutation     | Remove value from array at path                 | Needed for array element removal        |
| +, -, \*, / | a, b, ...   | Expression   | Pure math operators; compute and return a value | **Does not mutate state.**              |
| eq?         | a, b        | Predicate    | Returns true if a == b                          | Foundational equality check             |
| gt?         | a, b        | Predicate    | True if a > b                                   | Needed for numeric logic                |
| lt?         | a, b        | Predicate    | True if a < b                                   | Needed for numeric logic                |
| gte?        | a, b        | Predicate    | True if a >= b                                  | Needed for numeric logic                |
| lte?        | a, b        | Predicate    | True if a <= b                                  | Needed for numeric logic                |
| has?        | path, value | Predicate    | True if array at path contains value            | Array set membership                    |
| and         | …           | Predicate    | Returns true if all args are true               | Needed for composition                  |
| or          | …           | Predicate    | Returns true if any arg is true                 | Needed for composition                  |
| not         | a           | Predicate    | Logical negation                                | Needed for logic                        |
| rand        | min, max    | Query/Effect | Random int in [min, max]; updates PRNG in world | Only source of randomness               |
| cond        | cases       | Control      | Conditional branching; if-then-else             | Minimal universal control               |
| do          | …           | Control      | Sequentially evaluates each arg                 | Needed for block/sequence               |
| print       | text, …     | Output       | Renders/display text; may offer choices         | Fundamental for IO/output               |

- **Mutation atoms** (`!`) are the only atoms that change world state.
- **Predicate atoms** (`?`) are the only atoms that perform boolean comparisons.
- **Expression atoms** (`+`, `-`, `*`, `/`) are pure and only return computed values.
- **`get` is internal:** The engine's "auto-get" feature makes it invisible to authors.

### 4.2 Naming Conventions and Rationale

- Follows Scheme/Lisp conventions: `!` for mutation, `?` for predicates.
- This creates a clear, unambiguous distinction between operations that read state, compute values, and change state.

### 4.3 Candidate Atoms Under Review

- "let"/""define" for local bindings: only if macro patterns become too convoluted.
- All proposals for new atoms must be documented here, with reasons and alternatives considered.

---

## 5. Standard Macro Library: The Author's Toolkit

While the atom set is minimal, the **authoring language** is rich and user-friendly. This is achieved through a standard library of macros that provide the primary interface for writing Sutra logic. Authors should always prefer these macros over raw atoms.

### 5.1 Predicate and Comparison Macros

- **`is? path [value]`**: The universal check.
  - `is? path`: Tests if the value at `path` is `true`.
  - `is? path value`: Tests if the value at `path` is equal to `value`.
  - _Expands to `(eq? (get path) true)` or `(eq? (get path) value)`._
- **Comparison Macros**: All comparison operators are macros that auto-get their arguments. They have both a concise **canonical name** and a more readable **alias**. Both forms are interchangeable.

| Operation                | Canonical | Alias       | Example                             |
| ------------------------ | --------- | ----------- | ----------------------------------- |
| Greater than             | `gt?`     | `over?`     | `over? player.gold 100`             |
| Less than                | `lt?`     | `under?`    | `under? player.hp 5`                |
| Greater than or equal to | `gte?`    | `at-least?` | `at-least? player.level 5`          |
| Less than or equal to    | `lte?`    | `at-most?`  | `at-most? player.inventory.size 10` |

### 5.2 Mutation Macros

- **`add! path amount`**: Adds `amount` to the value at `path`.
  - _Expands to `(set! path (+ (get path) amount))`._
- **`sub! path amount`**: Subtracts `amount` from the value at `path`.
  - _Expands to `(set! path (- (get path) amount))`._
- **`mul! path amount`**: Multiplies the value at `path` by `amount`.
- **`div! path amount`**: Divides the value at `path` by `amount`.
- **`inc! path`**: Increments the value at `path` by 1. _Macro for `add! path 1`._
- **`dec! path`**: Decrements the value at `path` by 1. _Macro for `sub! path 1`._

### 5.3 Benefits of the Macro Layer

- **Safety:** Prevents authors from accidentally using pure operators (`+`, `-`) for mutation.
- **Clarity:** `add! player.hp 10` is more readable than `set! player.hp (+ (get player.hp) 10)`.
- **Consistency:** Provides a uniform, intention-revealing language for all common operations.

---

## 6. World State Model

### 5.1 Path-Based Addressing

- All world state is a deeply immutable, persistent data tree: maps, arrays, primitives.
- Paths can be strings, numbers, or mixed (e.g., player.stats.hp or world.agents[0].hunger).
- All atoms and macros operate by referencing these paths.

### 5.2 Representing Singleton, Global, and Entity Data

- "Singletons" (globals, player, config, etc.) are just well-known paths in world.
- "Entities" can be modeled as arrays/objects at a path (agents[23]), with properties as keys.
- Hierarchical data (inventory within agent, quests within location) are just nested maps/arrays.
- No privileged "object"/ECS model—all data is paths and structures.

### 5.3 Immutability, Serialization, Snapshotting

- All state transitions produce new world objects. No mutation in place.
- The entire world state is serializable to JSON or equivalent.
- Snapshots for debugging, "time travel," save/load, etc.

### 5.4 Conventions for Tree vs Flat Data

- Use parent-child relationships (IDs, paths) for containment hierarchies.
- Macro libraries can define conventions for "parent," "children," "inventory," etc., as needed.

### 5.5 What's Under Investigation

- Whether to formalize "entity"/"component" (ECS) conventions or keep purely path-based.
- How to optimize or index state for performance as data grows.
- Whether to allow user-defined types for validation.

---

## 6. Macro System (Living Spec)

### 6.1 Macro Semantics

- Macros are user-defined forms that expand to atoms or other macros.
- Macro expansion is hygienic, pure, and always visible (authors can inspect the expanded s-expr tree).
- Parameter passing: Macros can take any values, code blocks, or symbols as arguments.

### 6.2 Layered Macro System

- Layer 1: Utility/idiom macros (condition checks, dice rolls, effects, "if"/"when" sugar)
- Layer 2: Narrative macros (award-xp, lock-location, etc.)
- Layer 3: Structural/narrative patterns (storylet, choice, loop, agent, etc.)
- Layer 4: Modules/groupings (import/export, macro libraries, project structure)

### 6.3 Namespaces and Modules

- Macros are grouped in modules (namespaces).
- Modules can import/export macros for reuse.
- No global namespace pollution; all cross-file dependencies explicit.

### 6.4 Macro Expansion Transparency and Debugging

- Authors can "macroexpand" any block to see the resulting s-expr.
- Tooling provides expansion traces for debugging and error reporting.
- Expansion always terminates at atom-only code.

### 6.5 Open Questions and Experimental Areas

- Whether to support runtime (vs. compile-time) macro expansion for dynamic constructs.
- How best to represent recursive macros for looping and simulation.
- If/when to allow macro overloading or pattern-matching.

---

## 7. Repetition and Simulation

### 7.1 Loops, Iteration, and Recursion

- No primitive "loop" or "while" atom.
- All repetition handled via macros that expand to recursion or multiple atoms.
- Example:
- repeat n body as a macro that expands to n copies of body.
- for-each list var body as a macro expanding to recursion over list.

### 7.2 Achieving Turing Completeness

- The macro + atom system supports conditional logic, state mutation, and recursion.
- "repeat-until" or simulation "tick" loops are implemented via tail-recursive macros.
- The engine should support tail-call optimization (TCO) in the evaluator, to enable unbounded runtime recursion for simulation and agent logic.

### 7.3 Tail-Call Optimization (TCO) in Practice

- TCO is achievable and straightforward in Rust (see Appendix A).
- Evaluator is structured as a loop, not a recursive function, to reuse stack frames for tail calls.
- Macro expansion handles bounded, compile-time loops; TCO enables runtime/unbounded recursion for simulation and agent logic.

### 7.4 Scheduling and Real-Time Simulation

- Batch/agent logic, world ticks, and event queues are composed from standard macros—not hard-coded in the engine.
- "Scheduler" is a macro/module, not an engine privilege.
- See examples in appendices.

---

## 8. Standard Library and Macro Patterns

### 8.1 Common Macros

- repeat, for-each, repeat-until
- storylet, choice, cost-action
- agent-loop, salience-storylet
- All macros are author-accessible and editable.

### 8.2 Macro Expansion Trace and Debugging

- Engine tooling provides:
- View: user code → expanded macro → atoms
- Debug: what atoms executed, what world changes occurred, why a condition failed, etc.

### 8.3 Updating Macro Libraries

- When a new narrative or simulation pattern emerges:

1. Prototype as a macro.
2. Document macro and rationale here.
3. If it proves widely useful, promote to standard library.

---

## 9. Worked Example: From Atoms to Game Systems

(Expanded in full in File 3, but summary here for architectural clarity)

- Author writes:

```
storylet "meet-bishop" {
  and {
    over? player.reputation 10
    is? player.location "cathedral"
  }
  do {
    add! player.reputation 2
    print "The Bishop welcomes you."
  }
}
```

- Macro expansion:

```
cond {
  and {
    (gt? (get player.reputation) 10)
    (is? (get player.location) "cathedral")
  }
  do {
    (set! player.reputation (+ (get player.reputation) 2))
    (print "The Bishop welcomes you.")
  }
}
```

- Atom layer:
- All control flow and effects reduced to atoms.
- World state after execution:
- player.reputation incremented
- Narrative text output
- Simulation example:
- for-each agents agent (agent-tick agent) macro expands to recursive application of agent-tick macro over all entities in agents.

---

## Appendices

---

### A. Reference Implementation Sketches

- TCO in Rust:

```rust
fn eval(expr: Expr, env: &mut Env) -> Result<Value, Error> {
    let mut current_expr = expr;
    let mut current_env = env.clone();
    loop {
        match current_expr {
            // ...cases...
            Expr::Call(func, args) if in_tail_position => {
                current_expr = func.body;
                current_env = new_env_with_args;
                continue;
            }
            // ...more cases...
            _ => return result,
        }
    }
}
```

- Macro expansion pseudocode:

```lisp
(define repeat n body
  (if (eq? n 0)
      '()
      (do body (repeat (- n 1) body))))
```

---

### B. Alternatives Considered & Discarded

- Primitive loop atoms (e.g., "while", "for"): Rejected in favor of macro-powered recursion for maximal compositionality.
- Built-in ECS/object models: All state is path-based; ECS can be built as a macro/data convention.
- Non-uniform authoring syntax: Committed to a regular, s-expr-based canonical form, with optional indented sugar.

---

### C. Author-Facing Syntax Reference

- Brace-block:

```
storylet "open-door" {
  and {
    is? player.location "hall"
    has? player.items "key"
  }
  do {
    set! world.door.open true
    print "You unlock the door!"
  }
}
```

- Equivalent s-expr:

```lisp
(storylet "open-door"
  (and
    (is? player.location "hall")
    (has? player.items "key"))
  (do
    (set! world.door.open true)
    (print "You unlock the door!")))
```

---

### D. Macro System Deep Dive

- Macro expansion rules:
- Parameters are pattern-matched and spliced into body at expansion.
- Macroexpansion occurs recursively until only atoms remain.
- Hygiene (variable scoping) and argument evaluation order are documented here.
- Pitfalls:
- Macro recursion depth, infinite expansion
- Naming collisions; best avoided via module scoping

---

### E. Cross-references to Philosophy and Authoring Patterns

- For narrative/gameplay patterns and full authoring workflows, see File 3.
- For philosophical underpinnings and rationale, see File 1.

---

### F. How to Update/Revise This File

- Major architecture, atom, or macro system changes must be recorded here, with date, rationale, and "what was improved."
- Experimental/unstable features should be clearly marked as such, with ongoing evaluation logs.
- Review and update after major authoring projects or engine refactors.

---

**END OF FILE 2**
(Review, extend, and iterate—this is your evolving author's lab notebook!)
