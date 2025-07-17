---
last-reviewed: 2024-07-09
summary: Deuterocanonical language specification for Verse, always in sync with the implementation.
---

# Verse Language Specification

> **Canonical Reference Notice (2024-07-09):**
>
> The Verse language has two canonical syntaxes:
>
> - **List style**: The canonical, Scheme/Lisp-inspired s-expression syntax. All engine, macro, and test logic is specified and validated exclusively in this style. This is the ground truth for the language.
> - **Block style**: The brace-based, authoring-friendly syntax for game authors. This style is compiled to list style and is not canonical for engine or test purposes.
>
> All references to 's-expression', 'brace-based', or 'authoring dialect' in this document are now replaced with 'list style' and 'block style' respectively.

> **Note:** This document is a living specification, synchronized with the canonical implementation in the codebase. It reflects the actual, implemented state of the language, not a planned or aspirational one.

---

## **Changelog**

### **2025-07-09: Canonical Reference Declaration**

- Declared **list style** (Scheme/Lisp s-expression) as the canonical reference for all Verse language semantics, macro expansion, and test expectations.
- Clarified that the canonical syntax is the **list style**; the **block style** is for authoring convenience and is not canonical for engine or test purposes.

### **2025-07-02: Major Synchronization with Codebase**

This update brings the language specification in line with the canonical implementation, while preserving forward-looking designs. The previous version was based on a pre-implementation design and was significantly out of date.

- **Architecture:** Clarified the distinction between author-facing **macros** and internal, namespaced **core atoms** (e.g., `set!` is a macro that expands to `core/set!`).
- **Control Flow:** Corrected the most critical error in the previous spec. **`if` is the core conditional construct**, not a macro. `cond` is now marked **obsolete**.
- **Status Tracking:** Added a `Status` column to all tables to explicitly track what is `Implemented`, `Planned`, or `Obsolete`.
- **Atoms & Macros:** Correctly categorized all items based on the codebase. Marked `gte?`, `lte?`, and `mod` as `Implemented`.
- **Aspirational Tiers (2+):** Added a prominent note clarifying that **Tier 2 and 3 features are design concepts and are NOT YET IMPLEMENTED**.

### **2025-07-05: Macro System, CLI, and Test Harness Refactor (Changelog)**

- Removed references to legacy macroexpander types and updated macro system documentation.
- Documented recursion depth enforcement (limit: 128) in macro expansion.
- Updated CLI and test harness documentation and examples.
- All code examples and explanations now match the current implementation.

---

# Verse Macro Library — Tier 1: Canonical Specification

This document provides a formal, precise, and minimal-but-pragmatic specification of Verse's Tier 1 macro and atom set. This is the foundational author-facing and engine-level vocabulary on which all higher patterns (Tier 2+) will be built.
All design choices are explained in context of Verse's guiding principles: minimal core, maximal compositionality, author ergonomics, and zero redundancy.

---

## **Overview: Core Atoms vs. Macros**

Verse's design distinguishes between two fundamental concepts: **Core Atoms** and **Macros**.

- **Core Atoms:** These are the irreducible, primitive operations executed by the engine. They are often namespaced (e.g., `core/set!`) and are not intended for direct use by authors. They operate on a fully expanded, canonical AST and expect their arguments to be in a precise format (e.g., `core/get` requires an `Expr::Path`).

- **Macros:** These form the ergonomic, author-facing language surface. They are syntactic abstractions that expand into expressions containing core atoms or other macros. The macro expansion phase of the compiler is responsible for this transformation. For example, the author-facing `set!` macro expands into a call to the `core/set!` atom.

> **Guiding Principle:** Authors should always use the clean, ergonomic macros for logic, state manipulation, and control flow. The engine relies on the macro-expansion step to translate this author-friendly code into the strict, canonical forms that the core atoms expect.

---

## **Tier 1 Table: Canonical Atoms and Macros**

> **Note:** This table represents the ground truth of the language's core vocabulary. It distinguishes between engine-level `Core Atoms` and author-facing `Macros`.

| Category        | Status        | Core Atoms (Engine Primitives)                             | Macros (Author-Facing)                                            |
| :-------------- | :------------ | :--------------------------------------------------------- | :---------------------------------------------------------------- |
| **Control**     |               |                                                            |                                                                   |
|                 | `Implemented` | `do`                                                       | `if` (core construct)                                             |
|                 | `Obsolete`    | `cond`                                                     |                                                                   |
|                 | `Planned`     |                                                            | `when`, `else`, `let`, `for-each`                                 |
| **Assignment**  |               |                                                            |                                                                   |
|                 | `Implemented` | `core/set!`, `core/del!`                                   | `set!`, `del!`, `add!`, `sub!`, `inc!`, `dec!`                    |
|                 | `Planned`     | `core/push!`, `core/pull!`                                 | `push!`, `pull!`, `mul!`, `div!`                                  |
| **Data Access** |               |                                                            |                                                                   |
|                 | `Implemented` | `core/get`, `list`, `len`                                  | `get`                                                             |
| **Predicates**  |               |                                                            |                                                                   |
|                 | `Implemented` | `eq?`, `gt?`, `lt?`, `gte?`, `lte?`, `not`, `core/exists?` | `is?`, `over?`, `under?`                                          |
|                 | `Planned`     | `has?`                                                     | `at-least?`, `at-most?`, `has?`, `exists?`, `and`, `or`, `empty?` |
| **Math/Value**  |               |                                                            |                                                                   |
|                 | `Implemented` | `+`, `-`, `*`, `/`, `mod`                                  |                                                                   |
|                 | `Planned`     | `min`, `max`, `abs`                                        |                                                                   |
| **Output**      |               |                                                            |                                                                   |
|                 | `Implemented` | `print`                                                    | `display`                                                         |
| **Random**      |               |                                                            |                                                                   |
|                 | `Implemented` | `rand`                                                     | `chance?`                                                         |
| **Utility**     |               |                                                            |                                                                   |
|                 | `Implemented` | (Auto-get is a macro feature)                              |                                                                   |
|                 | `Planned`     |                                                            | `path`, `first`, `last`, `nth`                                    |
| **Debug**       |               |                                                            |                                                                   |
|                 | `Planned`     |                                                            | `debug`, `fail`, `error`, `assert`                                |

---

## Predicate Macros

**Pattern:** Predicate macros are thin wrappers around the boolean atoms (`eq?`, `gt?`, etc.). Their primary function is to provide **auto-get** functionality, automatically wrapping path-like arguments in `(core/get ...)` calls.

| Macro       | Expands to (Atom) | Purpose                   | Status      |
| ----------- | ----------------- | ------------------------- | ----------- |
| `is?`       | `eq?`             | Equality or truthy test.  | Implemented |
| `over?`     | `gt?`             | Greater than comparison.  | Implemented |
| `under?`    | `lt?`             | Less than comparison.     | Implemented |
| `at-least?` | `gte?`            | Greater or equal.         | Planned     |
| `at-most?`  | `lte?`            | Less or equal.            | Planned     |
| `has?`      | `has?`            | Membership in collection. | Planned     |
| `exists?`   | `exists?`         | Path/value existence.     | Planned     |
| `not`       | `not`             | Logical negation.         | Implemented |
| `and`       | `(if ...)`        | Logical AND.              | Planned     |
| `or`        | `(if ...)`        | Logical OR.               | Planned     |
| `empty?`    | `eq?` + `len`     | Collection is empty.      | Planned     |

**Example (Implemented Macros):**

```lisp
;; Author-written code:
(is? player.hp 10)

;; After macro expansion:
(eq? (core/get (path player hp)) 10)
```

---

## Assignment Macros

**Pattern:** Assignment macros provide an ergonomic way to modify world state. They all expand into expressions using the `core/set!` atom.

| Macro   | Expansion Pattern                                                              | Status      |
| :------ | :----------------------------------------------------------------------------- | :---------- |
| `set!`  | `(set! path value)` → `(core/set! (path ...) value)`                           | Implemented |
| `del!`  | `(del! path)` → `(core/del! (path ...))`                                       | Implemented |
| `add!`  | `(add! path value)` → `(core/set! (path ...) (+ (core/get (path ...)) value))` | Implemented |
| `sub!`  | `(sub! path value)` → `(core/set! (path ...) (- (core/get (path ...)) value))` | Implemented |
| `inc!`  | `(inc! path)` → `(core/set! (path ...) (+ (core/get (path ...)) 1))`           | Implemented |
| `dec!`  | `(dec! path)` → `(core/set! (path ...) (- (core/get (path ...)) 1))`           | Implemented |
| `mul!`  | `(mul! path value)` → `(core/set! (path ...) (* (core/get (path ...)) value))` | Planned     |
| `div!`  | `(div! path value)` → `(core/set! (path ...) (/ (core/get (path ...)) value))` | Planned     |
| `push!` | `(push! path value)` → `(core/push! (path ...) value)`                         | Planned     |
| `pull!` | `(pull! path value)` → `(core/pull! (path ...) value)`                         | Planned     |

**Example (Implemented Macro):**

```lisp
;; Author-written code:
(add! player.gold 5)

;; After macro expansion:
(core/set! (path player gold) (+ (core/get (path player gold)) 5))
```

---

## Math/Value Atoms

All basic math/value operations are atoms and always author-facing; no macro wrapper is required.

| Atom  | Purpose              | Status      |
| ----- | -------------------- | ----------- |
| `+`   | Addition             | Implemented |
| `-`   | Subtraction/Negation | Implemented |
| `*`   | Multiplication       | Implemented |
| `/`   | Division             | Implemented |
| `mod` | Modulo (integers)    | Implemented |
| `len` | Collection length    | Implemented |
| `min` | Minimum              | Planned     |
| `max` | Maximum              | Planned     |
| `abs` | Absolute value       | Planned     |

> Authors may pass values or `(get path)`.
> Optional macro for auto-get is allowed, but not required.

---

## Core Atom Semantics and Edge Cases

This section highlights specific behaviors and design choices in Verse that might be non-obvious but are intentional. Understanding these can help in writing more robust and idiomatic Verse code.

- **Identity Values for Arithmetic Atoms:** When called with no arguments, `+` returns its identity value `0`, and `*` returns its identity value `1`. This is a common convention in Lisp-family languages.

  - `(+)` => `0`
  - `(*)` => `1`

- **Unary Negation and Reciprocal:** The `-` and `/` atoms can be called with a single argument.

  - `(- x)` returns the negation of `x`.
  - `(/ x)` returns the reciprocal `1/x`.

- **Trivial Truth for Comparison Atoms:** Comparison atoms (`eq?`, `gt?`, `lt?`, `gte?`, `lte?`) return `true` when given zero or one argument. The logic is that any sequence with one or zero elements is trivially ordered or equal.
  - `(gt? 5)` => `true`
  - `(lt?)` => `true`

# Verse Value Types — Canonical Reference

The Verse engine supports the following value types at the language level. These types are the only first-class values recognized by the engine and macro system.

| Verse Type | Example Literal    | Description                            |
| ---------- | ------------------ | -------------------------------------- |
| Nil        | `nil`              | Default/null value; absence of a value |
| Number     | `42`, `3.14`       | 64-bit floating point number           |
| String     | `"hello"`          | UTF-8 string, double-quoted            |
| Bool       | `true`, `false`    | Boolean values                         |
| List       | `(1 2 "a" true)`   | Ordered, heterogeneous list of values  |
| Map        | `{foo: 1, bar: 2}` | Key-value map (keys are strings)       |
| Path       | `(path player hp)` | Special type for world state access    |

---

# String Utilities — Canonical Specification

This section documents the canonical string utility macros and atoms available in Verse. All syntax is prefix, and all macros follow the canonical macro definition form.

## Typecasting to String: `str`

**Purpose:** Converts any value (number, boolean, symbol, etc.) to its string representation.

**Signature:**

```sutra
(str x)
```

**Behavior:**

- If `x` is already a string, returns it unchanged.
- If `x` is a number, returns its canonical string form (e.g., `42` → `"42"`).
- If `x` is a boolean, returns `"true"` or `"false"`.
- If `x` is a symbol, returns its name as a string.
- For other types, returns a canonical string representation or raises an error (TBD).

**Status:** Planned

---

## String Concatenation: `str+`

**Purpose:** Concatenates any number of string arguments into a single string.

**Signature:**

```sutra
(str+ arg1 arg2 ... argN)
```

**Behavior:**

- Accepts two or more arguments.
- Each argument must be a string (no type coercion for now).
- Returns a new string that is the concatenation of all arguments, in order.
- If any argument is not a string, raises a type error (future: will use `str` for coercion).

**Examples:**

```sutra
(str+ "foo" "bar")         ; => "foobar"
(str+ "hello, " "world!")  ; => "hello, world!"
(str+ "a" "b" "c" "d")     ; => "abcd"
```

**Status:** Implemented (Priority 1)

---

## String Join with Separator: `join-str+`

**Purpose:** Concatenates any number of string arguments, inserting a separator string between each.

**Signature:**

```sutra
(join-str+ sep arg1 arg2 ... argN)
```

**Behavior:**

- `sep` is the separator string.
- All other arguments must be strings (future: will use `str` for coercion).
- Returns a new string with `sep` inserted between each argument.
- If any argument is not a string, raises a type error.

**Examples:**

```sutra
(join-str+ ", " "a" "b" "c")   ; => "a, b, c"
(join-str+ "-" "foo" "bar")    ; => "foo-bar"
```

**Status:** Planned (deferred until after `str+` and `str`)

---

## String Utilities Summary Table

| Macro/Atom  | Signature                 | Purpose                      | Status      |
| ----------- | ------------------------- | ---------------------------- | ----------- |
| `str`       | `(str x)`                 | Typecast any value to string | Planned     |
| `str+`      | `(str+ arg1 ... argN)`    | Concatenate strings          | Implemented |
| `join-str+` | `(join-str+ sep a ... n)` | Join strings with separator  | Planned     |

---

# Macro Environment — Canonical Single Source of Truth (SSOT)

The Verse engine enforces a single source of truth for macro environment construction. All macro environments (for CLI, test harness, REPL, etc.) are built using a single, canonical function:

**Function:**

```rust
build_canonical_macro_env() -> MacroExpansionContext
```

**Location:**

- `src/runtime/registry.rs`

**Purpose:**

- Registers all core/built-in macros.
- Loads and registers all standard library macros from `src/macros/macros.sutra`.
- Returns a fully populated `MacroExpansionContext`.

**Usage:**

- All entrypoints (CLI, test harness, etc.) must use this function to construct the macro environment.
- No ad-hoc macro loading is permitted elsewhere in the codebase.

**Rationale:**

- Guarantees that all user-facing and core macros are always available in every code path.
- Prevents drift, duplication, and accidental omission of standard macros.
- Greatly simplifies auditing, testing, and onboarding.

**Example:**

```rust
let macro_env = build_canonical_macro_env();
// Use macro_env for macro expansion, validation, and evaluation
```

**Test:**

- The test suite includes a check that all expected macros are present in the canonical macro environment.

**Documentation:**

- This policy is documented here and in `CONTRIBUTING.md`.

---

## Control Flow

### **`if` (Core Construct)**

`if` is the canonical conditional construct in Verse. It is a special form in the AST (`Expr::If`) and is **not** a macro. It requires exactly three arguments: a condition, a "then" branch, and an "else" branch. The `else` is not optional.

**Status:** Implemented

**Example:**

```lisp
;; If player.hp is 0, the result is the string "You die!".
;; Otherwise, the result is the string "You live!".
(if (is? player.hp 0)
    "You die!"
    "You live!")
```

### **`do` (Atom)**

The `do` atom executes a sequence of expressions and returns the value of the final expression. It is the standard way to group multiple actions.

**Syntax:** `(do <expression1> <expression2> ...)`

**Status:** Implemented

### **Planned Control Macros**

The following control macros are planned but not yet implemented: `when`, `let`, `for-each`. The `cond` macro is considered **obsolete**.

---

## Output Atoms and Macros

- **`print`**: Atom; author-facing for narrative/UI/debug output. `print` is a unary atom and strictly requires one argument.
  **Example:**

  ```lisp
  print "You open the door."
  ```

- **`display`**: Macro; author-facing for printing multiple values to output, separated by spaces.
  **Example:**

  ```lisp
  (display "Your score is:" 100)
  ; Output: Your score is: 100
  ```

---

## Random Atoms and Macros

| Atom   | Macro     | Purpose                    | Example                             |
| ------ | --------- | -------------------------- | ----------------------------------- |
| `rand` | —         | Random integer in range    | rand 1 10                           |
| —      | `chance?` | Macro: true with X% chance | chance? 25 → (lte? (rand 1 100) 25) |

- Random atoms are always deterministic/reproducible by virtue of tracked seed.

---

## Utility & Debug

- **`auto-get`**: Not a macro or atom, but a macro-expansion feature: macro args that are paths are always auto-converted to `(get path)`.
- **`first`, `last`, `nth`, `path`**: Reserved for future author need, not part of Tier 1.
- **`debug`, `fail`, `error`, `assert`**: Reserved for future Tier 4 (debugging, tracing, error reporting).

---

## Principles Upheld

- **Minimal but pragmatic atom set:** Only include atoms for operations that are not robustly/clearly composable as macros, or are universal and efficient.
- **Macros for ergonomics:** Authors never write `get`, always use surface macros for predicates/assignment/control.
- **Author surface is clear, readable, and aligned with standard programming/narrative patterns.**

---

## End of Tier 1 Canonical Spec

---

> ## **Note on Tier 2+ Features**
>
> The following sections (**Tier 2, Tier 3, etc.**) describe powerful, high-level macros that are part of the **long-term design vision** for Verse.
>
> **These features are NOT YET IMPLEMENTED.**
>
> They are preserved here as a reference for future development and to guide the architectural direction of the engine. They should not be considered part of the current, usable language.

---

# **Verse Tier 2 Macros — Canonical Specification**

---

## 1. `requires` — Resource/Cost Gate

### **Purpose**

Gates the visibility/availability of a choice or action on a condition (usually a resource or stat check), and, optionally, handles resource spending or custom "can't afford" feedback.

### **Canonical Syntax**

```sutra
choice {
  requires (at-least? player.gold 10) {
    "Bribe the guard (10 gold)" {
      sub! player.gold 10
      print "The guard lets you pass."
      set! player.location "market"
    }
  }
  "Sneak past" {
    print "You slip by unseen."
    set! player.location "market"
  }
}
```

_Only shows/enables the "Bribe" choice if player.gold ≥ 10; author handles cost in block._

### **Macro Expansion**

Expands to a gating pattern—only renders (or enables) the inner block if the predicate is true:

```lisp
(if (at-least? player.gold 10)
  (choice
    ("Bribe the guard (10 gold)"
      (do
        (sub! player.gold 10)
        (print "The guard lets you pass.")
        (set! player.location "market")))))
(choice
  ("Sneak past"
    (do
      (print "You slip by unseen.")
      (set! player.location "market"))))
```

_(In a UI, the engine can also auto-show "locked" choices or feedback if desired.)_

### **Rationale**

- Self-documenting intent ("this is a cost/resource gate").

- Clean, repeatable authoring of pay-to-act, affordance, or ability-gated options.

- Enables system/UI to consistently show/hide/grey out options and track resource cost patterns.

### **Edge Cases**

- Author is responsible for handling the cost deduction or custom "can't afford" messaging.

- Can nest, or combine with multiple requires in a single choice block for multi-resource gates.

---

## 2. `threshold` — Menace/Fail/Trigger Pattern

### **Purpose**

Triggers a narrative or systemic consequence _automatically_ when a stat or resource crosses a specified value.

### **Canonical Syntax**

```sutra
threshold (at-least? player.suspicion 5) {
  print "You've been caught snooping! Game over."
  set! player.status "lost"
}
```

_Runs immediately when the condition first becomes true, regardless of location or thread context._

### **Macro Expansion**

Registers a global/system "watcher":

```lisp
(if (and (not (get player.status "lost"))
         (at-least? player.suspicion 5))
  (do
    (print "You've been caught snooping! Game over.")
    (set! player.status "lost")))
```

_(Engine runs these threshold blocks after every state mutation.)_

### **Rationale**

- Eliminates scattered menace/fail/loop checks; centralizes all critical triggers.

- Author can manage all event triggers from one place.

- Engine/system can display tension, meter, or drama more easily.

### **Edge Cases**

- Trigger can be one-shot (default) or repeatable if designed so.

- If multiple thresholds can fire at once, engine defines order (first-match, all, priority).

---

## 3. `hub` — Open Navigation/Repeatable Node Pattern

### **Purpose**

Defines a set of repeatable, open "spoke" options from a central hub—classic for open maps, safe zones, or menu navigation.

### **Canonical Syntax**

```sutra
hub {
  "Study"      -> study!
  "Library"    -> library!
  "Garden"     -> garden!
  "Main Hall"  -> main-hall!
}
```

_Each choice can be selected in any order, as often as author allows (unless restricted by gating logic)._

### **Macro Expansion**

Expands to a choice block with all options presented at once:

```lisp
(choice
  ("Study" (set! player.location "study"))
  ("Library" (set! player.location "library"))
  ("Garden" (set! player.location "garden"))
  ("Main Hall" (set! player.location "main-hall")))
```

_(Or, if steps are modular, triggers the corresponding thread/step.)_

### **Rationale**

- Encapsulates open-ended navigation without authoring extra logic per move.

- Makes "hub zones" safe, extensible, and visualizable.

- Reduces copy/paste and navigation spaghetti.

### **Edge Cases**

- Can be nested inside a thread, or global (e.g., world map).

- Spokes can have their own gating (using `requires`).

---

## 4. `select` — Salience/Weighted Storylet/Event

### **Purpose**

Lets the engine/system pick the most "salient" (highest-weight) event/storylet from a list—QBN/AI Director/priority-driven selection.

### **Canonical Syntax**

```sutra
select {
  (storylet "strange noise" weight: (player.suspicion * 2))
  (storylet "find clue"      weight: (player.clues.missing))
  (storylet "rest"           weight: 1)
}
```

_System automatically triggers the event with the highest current weight._

### **Macro Expansion**

- Evaluates all candidate weights, triggers one with the highest value:

```lisp
(let { candidates [ ... ] }
  (do
    (set! winner (max-by-weight candidates))
    (call winner)))
```

- `max-by-weight` is an engine/system utility.

### **Rationale**

- Enables dynamic, systemic content surfacing.

- Authors can express both emergent narrative and "director" logic in one block.

### **Edge Cases**

- Weight can be static or computed.

- Engine can randomize among ties, or use secondary keys.

---

## 5. `at` — Waypoint/Landmark Event

### **Purpose**

Triggers event logic whenever a specific location (or state) is reached.

### **Canonical Syntax**

```sutra
at "kitchen" {
  print "The aroma of baking bread greets you."
  unless player.found-bread {
    set! player.found-bread true
    push! player.items "fresh bread"
  }
}
```

_Automatically fires when the player's location becomes "kitchen"._

### **Macro Expansion**

```lisp
(if (and (eq? player.location "kitchen")
         (not (get player.found-bread)))
  (do
    (print "The aroma of baking bread greets you.")
    (set! player.found-bread true)
    (push! player.items "fresh bread")))
```

- Typically, the engine watches for `player.location` changes and checks all `at` blocks.

### **Rationale**

- Centralizes location-based events.

- Avoids scattering "arrival" logic in many different places.

### **Edge Cases**

- Can trigger once or be repeatable, per author logic.

- Can combine with `requires`/`threshold` for extra gating.

---

## 6. `tick` — Simulation/Agent Cycle

### **Purpose**

Runs logic for all agents/entities/world each simulation step, turn, or tick.

### **Canonical Syntax**

```sutra
tick {
  for-each world.npcs npc {
    if npc.hungry {
      sub! npc.energy 1
      print npc.name "searches for food."
    }
  }
}
```

_Engine automatically executes this block each world tick._

### **Macro Expansion**

```lisp
(for-each world.npcs npc
  (if (get npc.hungry)
    (do
      (sub! npc.energy 1)
      (print npc.name "searches for food."))))
```

- The engine triggers all `tick` macros at the start/end of each simulation step.

### **Rationale**

- Centralizes all simulation/agent logic.

- Easy to extend and maintain complex worlds.

### **Edge Cases**

- Can have multiple `tick` blocks (run in order of appearance or by tag).

- Can be combined with `threshold` for dynamic loops.

---

## 7. `intent` — Reflective/Roleplay Choice

### **Purpose**

Lets players choose their motivation/attitude/"why" for an action, not just the "what"—records emotional or style choices for callbacks or narrative flavor.

### **Canonical Syntax**

```sutra
intent {
  "Attack out of anger" {
    set! player.intent "anger"
    print "You lash out recklessly."
  }
  "Attack with honor" {
    set! player.intent "honor"
    print "You challenge your foe with dignity."
  }
  "Attack for survival" {
    set! player.intent "survival"
    print "You fight only to save yourself."
  }
}
```

_Records the player's chosen intent for later use by the story or mechanics._

### **Macro Expansion**

Expands to a `choice` block that records an intent variable and executes side-effects:

```lisp
(choice
  ("Attack out of anger"
    (do
      (set! player.intent "anger")
      (print "You lash out recklessly.")))
  ("Attack with honor"
    (do
      (set! player.intent "honor")
      (print "You challenge your foe with dignity.")))
  ("Attack for survival"
    (do
      (set! player.intent "survival")
      (print "You fight only to save yourself."))))
```

### **Rationale**

- Makes reflective/roleplay choices a first-class authoring pattern.

- Encourages more expressive, reactive narratives.

### **Edge Cases**

- Can store the last selected intent, or accumulate a history (with `push!`).

- Can be combined with gating/thresholds for advanced branching.

---

## **End of Canonical Tier 2 Macro Spec**

---

# Verse Tier 3: Emergent Gameplay, Pools, History, and Dynamic Text — Canonical Reference

---

## **I. Event Pools, Weighted Selection, Sampling, and History**

### **1. Pool Declaration (`pool`)**

- **Purpose:**
  Define a set of eligible storylets/events, by explicit list, tag, or query.

- **Syntax:**

  ```sutra
  pool {
    (storylet "find-clue"   (tag discovery))
    (storylet "ambush"      (tag danger)   (weight (* player.menace 2)))
    (storylet "find-food"   (tag resource) (weight 1))
  }
  ```

  Or, by tag:

  ```sutra
  pool (tag discovery)
  ```

  Filtered:

  ```sutra
  pool (tag danger) exclude seen
  ```

---

### **2. Selection Macros (`select`, `random`, `sample`, `shuffle`)**

- **Purpose:**
  Choose one or more events from a pool by weight, randomness, or sampling, possibly excluding seen events.

- **Syntax:**

  ```sutra
  select from pool {
    (storylet "find-clue"   (weight player.menace))
    (storylet "ambush"      (weight (* player.menace 2)))
    (storylet "rest"        (weight 1))
  }
  ```

  Random pick:

  ```sutra
  random from pool (tag resource)
  ```

  Sample N:

  ```sutra
  sample 2 from pool (tag discovery) exclude seen
  ```

  Shuffle:

  ```sutra
  shuffle pool (tag rumor)
  ```

---

### **3. History Tracking (`history`, `seen`, `exclude`)**

- **Purpose:**
  Track and gate seen events, enable "no repeats," achievements, recaps, and replayability.

- **Syntax:**
  Mark event as tracked:

  ```sutra
  storylet "find-clue" (tag discovery) history {
    print "You find a cryptic note."
  }
  ```

  Exclude already-seen:

  ```sutra
  sample 1 from pool (tag event) exclude seen
  ```

  Check if seen:

  ```sutra
  if seen "find-clue" {
    print "You already found the clue here."
  }
  ```

---

### **4. Metadata and Tag Syntax**

- **Use parenthesized, space-separated key-value pairs:**

  - `(tag danger)` `(weight 2)` `(area forest)`

  - Multiple tags/fields: `(tag danger) (weight player.menace) (region east)`

---

### **5. Canonical Example: Emergent Event Draw Loop**

```sutra
tick {
  for-each world.agents agent {
    pool (tag npc-event) for agent {
      sample 1 exclude seen {
        call selected-event
        history selected-event
      }
    }
  }
}
```

- Pools and sampling can be filtered/tagged and are extensible by agent, group, or context.

---

### **6. Implementation Notes**

- **Parser:**

  - Block/brace structure, s-expr–style metadata.

- **Engine:**

  - Pool construction, random/weighted selection, history tracking are all O(N) and scalable.

---

---

## **II. Dynamic Text, Interpolation, and Inline Fragment Insertion**

### **1. Variable and Expression Interpolation**

- **Syntax:**
  Use braces `{ ... }` inside quoted text for variables or expressions.

- **Examples:**

  ```sutra
  print "Welcome, {player.name}!"
  print "You have {player.gold} gold."
  print "Your HP is now {+ player.hp 5}."
  print "Inventory: {count player.items} items."
  ```

- **No string concatenation or macro clutter required.**

---

### **2. Inline Random Fragment Insertion (Symbolic Pattern)**

- **Syntax:**
  Use braces with `|` pipe symbol: `{option1|option2|option3}`

- **Examples:**

  ```sutra
  print "You find a {rusty|shiny|ancient} key."
  print "You spot a {blue|red|yellow} door."
  print "Your rival is {player.rival|a stranger|nobody}."
  ```

- **Notes:**

  - No keywords/macro words in text; parser/engine samples at runtime.

---

### **3. Template/Grammar Expansion**

- **Grammar Definition Syntax:**

  ```sutra
  define grammar {
    adjective ["sleepy" "fierce" "luminous"]
    animal    ["wolf" "owl" "fox"]
    action    ["howls" "prowls" "glows"]
    place     ["forest" "clearing" "ruins"]
  }
  ```

- **Referencing slots in text:**
  Use `[slot]`, e.g.:

  ```sutra
  print "The [adjective] [animal] [action] in the [place]."
  ```

- **Nesting/Recursion:**
  Grammar rules can reference other slots:

  ```sutra
  define grammar {
    origin ["A [adjective] [animal] appears."]
    adjective ["giant" "old"]
    animal ["wolf" "fox"]
  }
  print "[origin]"
  ```

- **With Metadata:**

  ```sutra
  adjective [
    ("sleepy" (weight 2))
    ("luminous" (player.location "ruins"))
  ]
  ```

---

### **4. Summary Table**

| Macro/Pattern      | Syntax Example                                      | Use Case                    |
| ------------------ | --------------------------------------------------- | --------------------------- |
| pool               | `(storylet "id" (tag X) (weight Y))`                | Event collection            |
| select/random      | `select from pool {...}` `random from pool ...`     | Weighted/random selection   |
| sample/shuffle     | `sample 2 from pool ...` `shuffle pool (tag rumor)` | Variety, no repeats         |
| history/seen       | `history`, `if seen "id" {...}`                     | Track, gate, recap          |
| Interpolation      | `"HP: {player.hp}"`                                 | Seamless variable insertion |
| Fragment insertion | `"{red                                              | blue                        |
| Grammar/template   | `[adjective]`, `[animal]` in text                   | Templates/procgen flavor    |

---

### **5. Implementation Feasibility**

- **Parser:**

  - All features extend naturally from Verse's block/brace + s-expr conventions.

  - Metadata/weights are handled as in all other blocks.

  - `{...}` for interpolation/fragments and `[slot]` for templates are both regular, simple patterns.

- **Engine:**

  - Random, weighted, and sampling logic is standard; history is just a set/flag; grammar expansion is recursive but simple to bound.

  - Can support deterministic runs if desired (by PRNG seeding).

---

## **Best Practices**

- **Always use parenthesized `(key value)` tags for metadata/fields in pools/events.**

- **Use `{...}` for all variable/fragment insertion and `[slot]` for grammar/template expansion.**

- **Prefer pools and grammar for variety, replayability, and procedural generation.**

- **Track history and "seen" status to maximize emergent gameplay.**

- **Keep templates/pools shallow and composable for easy scaling.**

---

## **What's Next (Suggested Order)**

- **Formalize agent/group event execution (per-agent pools and history).**

- **Expand context and weighting in pools/grammar for even richer simulation.**

- **Eventually add procedural event/pool generation and debugging/analytics macros.**

---

**This is your canonical Tier 3 reference for dynamic, emergent, and replayable Verse games.**
If you need a PDF, sample projects, or parser/AST specs, just ask!

---

Here is the **formalized, canonical Tier 3+ Verse macro and grammar/event/pool system**, now fully **prefix-uniform**, using block structure for readability, and reflecting the explicit `else` in all author-facing conditionals.

---

# Verse Tier 3+: Pools, Weighted Selection, Grammar, Dynamic Text, and Conditionals

---

## **I. Pools, Events, and History (Uniform Prefix, Block Style)**

### **1. Pool Declaration and Events**

**Purpose:**
Define a set of eligible storylets/events, each as a block, with modifiers/logic as prefix expressions.

**Syntax:**

```sutra
pool {
  storylet "fight-bandits" {
    tag combat
    weight (* agent.bravery 3)
    when (eq? agent.location "road")
  }
  storylet "harvest-crops" {
    tag work
    weight (if agent.hungry 0 5)
  }
  storylet "study-magic" {
    tag learning
    weight agent.intelligence
    when agent.has-book
  }
  storylet "pick-flowers" {
    tag leisure
    when (eq? agent.location "meadow")
  }
  storylet "visit-temple" {
    tag spiritual
    when agent.faithful
  }
}
```

- **All expressions are prefix:** e.g. `(eq? agent.location "meadow")`, `(if agent.hungry 0 5)`.

- **No infix, no ambiguous ordering.**

---

### **2. Selection, Sampling, and Shuffle**

**Weighted/Salient Selection:**

```sutra
select from pool {
  storylet "find-clue" { weight player.menace }
  storylet "ambush"    { weight (* player.menace 2) }
  storylet "rest"      { weight 1 }
}
```

**Random Pick:**

```sutra
random from pool (tag resource)
```

**Sample N (no repeats):**

```sutra
sample 2 from pool (tag discovery) exclude seen
```

**Shuffle Pool:**

```sutra
shuffle pool (tag rumor)
```

---

### **3. History Tracking**

**Mark as seen:**

```sutra
storylet "find-clue" { tag discovery history
  print "You find a cryptic note."
}
```

**Exclude already-seen:**

```sutra
sample 1 from pool (tag event) exclude seen
```

**Check if seen:**

```sutra
if (seen "find-clue") {
  print "You already found the clue here."
}
```

- `seen` is a prefix macro/atom as in Tier 1.

---

### **4. Metadata/Tags**

- Use key-value, prefix syntax inside blocks:

  - `tag combat`

  - `weight 2`

  - Multiple fields:

    ```
    tag danger
    weight (* agent.rage 3)
    region east
    ```

---

### **5. Best Practices**

- **Every field and expression is prefix notation.**

- **Block style for every event or grammar rule for clarity.**

- **Use explicit `if ... else ...` (prefix: `(if cond a b)`), not infix.**

---

## **II. Dynamic Text, Interpolation, and Fragment Insertion**

### **1. Interpolation:**

```sutra
print "Welcome, {player.name}!"
print "You have {player.gold} gold."
print "Your HP is now {+ player.hp 5}."
print "Inventory: {count player.items} items."
```

- Inside `{ ... }`, use prefix: `{+ player.hp 5}`.

---

### **2. Inline Random Fragment Insertion:**

```sutra
print "You find a {rusty|shiny|ancient} key."
print "Your rival is {player.rival|a stranger|nobody}."
```

- Fragment selection is _not_ an expression; just enumerate options with `|`.

---

### **3. Grammar/Template Expansion:**

**Define grammar (block, options, modifiers in prefix):**

```sutra
define grammar {
  adjective {
    sleepy       { weight 2 }
    fierce       { when (eq? time "night") }
    sly          { when agent.cunning }
    old          { when (gt? agent.age 40) }
    young
  }
  animal {
    wolf
    fox
    owl
    stag
  }
}
```

**Usage in text:**

```sutra
print "The [adjective] [animal] appears."
```

- Each `[slot]` is expanded using the grammar rule (optionally context/weighted).

---

### **4. Canonical If/Else, Always Prefix**

**Authoring:**

```sutra
weight (if agent.hungry 0 5)
when (if (at-least? agent.rivalry 2) true false)
```

or, inside code blocks:

```sutra
if (is? player.hp 0) {
  print "You die!"
}
else {
  print "You live!"
}
```

- All expressions, conditions, and branches are **prefix**.

- `else` keyword _must_ be used when there is a second branch.

---

### **5. Full Example: Agent Event Draw with Contextual Logic**

```sutra
tick {
  for-each world.npcs agent {
    pool {
      storylet "duel" {
        tag social
        weight (* agent.rivalry 2)
        when (at-least? agent.rivalry 2)
      }
      storylet "chat" {
        tag social
        weight agent.friendliness
      }
    }
    sample 1 exclude seen {
      call selected-event
      history selected-event
    }
  }
}
```

---

## **III. Parser and Engine Notes**

- **Parser:**

  - Everything after a field or macro is parsed as prefix (S-expression) until the next field, block, or end.

  - `{ ... }` opens a block; lines inside are `key <prefix-expr>`.

  - No ambiguity, no need for infix parsing or operator precedence.

- **Engine:**

  - Evaluates prefix expressions using the standard macro/atom set.

  - Grammar and event pools are resolved with prefix expressions for weights/conditions.

---

## **IV. Summary Table**

| Macro/Pattern      | Syntax Example                             | Usage/Note                               |
| ------------------ | ------------------------------------------ | ---------------------------------------- |
| Block/pool/grammar | `storylet "id" { ... }`                    | Each field is prefix expr                |
| Tag/metadata       | `tag combat`                               | Key-value, prefix                        |
| Conditionals       | `when (if agent.tired false true)`         | Always prefix                            |
| If/else            | `if (gt? agent.hp 5) { ... } else { ... }` | Block style, explicit else               |
| Interpolation      | `"HP: {+ player.hp 5}"`                    | Prefix in braces                         |
| Grammar slot       | `[adjective]`                              | Grammar rule expansion                   |
| Fragment insertion | `"{red\|blue\|yellow}"`                    | Ad-hoc random choice (not an expression) |

---

## **V. Authoring Principles**

- **Uniformity:**

  - Prefix notation everywhere, in pools, conditions, weights, and all logic.

- **Explicitness:**

  - `else` is always present for two-branch conditionals.

- **Block structure for all pools/grammar/events.**

- **No reliance on alignment, indentation, or special separators for modifiers.**

---

## **Canonical Macro Definition Syntax**

### **Macro Definition**

All macro definitions must use the following canonical form:

```sutra
define (macro-name param1 param2 ... [. variadic-param]) {
  ; macro body (prefix expressions, statements, etc.)
}
```

- The macro name and all parameters (including variadic, if any) are grouped in a single parenthesized list immediately after `define`.
- The body is always a brace block.
- The dot (`.`) before the last parameter indicates a variadic parameter, as in canonical Lisp/Scheme.
- No braces or parentheses are used around the parameter list except for the single, required parenthesized header.

#### **Examples**

| Macro Type    | Syntax Example                                      | Notes                              |
| ------------- | --------------------------------------------------- | ---------------------------------- |
| Fixed arity   | `define (add3 a b c) { + a b c }`                   | No dot, all params required        |
| Variadic      | `define (my-list first . rest) { list first rest }` | Dot before last param for variadic |
| Only variadic | `define (collect . args) { list args }`             | Dot at start, all args to `args`   |

#### **Macro Call**

- Macro calls are always prefix, with no parentheses or braces unless a block body is required:
  ```sutra
  my-list 1 2 3 4
  ; expands to: list 1 (2 3 4)
  ```
- If a block is required as an argument, braces are used for that argument only.

#### **Rationale**

- **Minimalism:** Only one set of parentheses for the header, one set of braces for the body.
- **Clarity:** The macro's "signature" is visually distinct and easy to scan.
- **Unambiguity:** The parser can unambiguously identify the name and all parameters, including variadic, regardless of whitespace or line breaks.
- **Lossless translation:** This form is trivially translatable to and from canonical s-expr (Lisp/Scheme).
- **Consistency:** Matches the block/brace usage in threads, storylets, pools, etc. Parens are only used for grouping where grouping is semantically meaningful.
- **Extensibility:** If you ever want to add metadata, docstrings, or type annotations, the parenthesized header is a natural place.

---

## Variadic Macro Forwarding and Argument Splicing

### Overview

Verse's macro system implements canonical Lisp/Scheme-style variadic macro forwarding and argument splicing. This allows macros to accept a variable number of arguments and forward them ergonomically, matching user expectations from other Lisp-family languages.

### Canonical Behavior

- **Variadic Macro Definition:**
  - A macro can be defined with a variadic parameter using the `...param` syntax, e.g. `(define (str+ ...args) (core/str+ ...args))`.
- **Call Position Splicing:**
  - When a variadic parameter is referenced in call position (i.e., as an argument in a list), the macro expander splices its bound arguments as individual arguments, not as a single list.
  - Example:
    ```sutra
    (define (str+ ...args)
      (core/str+ ...args))
    (str+ "a" "b" "c") ; expands to (core/str+ "a" "b" "c")
    ```
- **Explicit Spread:**
  - The spread operator (`...expr`) is supported in call position. If the spread expression evaluates to a list, its elements are spliced into the parent list.

### Edge Cases and Details

- **Non-call Position:**
  - If a variadic parameter is used outside of call position (e.g., as a value or in a non-list context), it is substituted as a list, not spliced.
    ```sutra
    (define (collect ...items) items)
    (collect 1 2 3) ; expands to (list 1 2 3)
    ```
- **Empty Variadic:**
  - If no arguments are provided for a variadic parameter, it is substituted as an empty list, and splicing results in no arguments being inserted.
- **Multiple/Nested Spreads:**
  - Multiple spreads or nested spreads are handled recursively; all are flattened in call position.
- **Non-variadic Parameters:**
  - Non-variadic parameters are substituted as single values. If (improperly) bound to a list, they are also spliced in call position (not recommended).

### User-Facing Summary

- Macros with variadic parameters behave as expected for both authors and users, supporting idiomatic macro patterns.
- The macro expander ensures that argument lists are flattened as needed, matching the semantics of Scheme and other Lisps.

### Examples

```sutra
;; Variadic macro forwarding
(define (join sep ...items) (core/join sep ...items))
(join "," "a" "b" "c") ; expands to (core/join "," "a" "b" "c")

;; Spread operator
(define (wrap x) (list ...x))
(wrap (list 1 2 3)) ; expands to (list 1 2 3)

;; Variadic parameter outside call position
(define (collect ...items) items)
(collect 1 2 3) ; expands to (list 1 2 3)
```

See the macro system implementation and tests for further details and edge cases.
