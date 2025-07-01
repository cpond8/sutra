LAST EDITED: 2025-06-30

# Sutra Macro Library — Tier 1: Canonical Specification
This document provides a formal, precise, and minimal-but-pragmatic specification of Sutra’s Tier 1 macro and atom set. This is the foundational author-facing and engine-level vocabulary on which all higher patterns (Tier 2+) will be built.
All design choices are explained in context of Sutra’s guiding principles: minimal core, maximal compositionality, author ergonomics, and zero redundancy.

---

## Overview: Atoms vs. Macros

* **Atoms:** The irreducible core engine operations; authors may use these directly (especially for math, print, and basic state mutation).
* **Macros:** The ergonomic, author-facing surface. Each expands to one or more atoms. Macros are used for logic, assignment sugar, auto-get, and higher-level idioms.

> **Authors are expected to use macros for all state queries, predicates, assignments, and control patterns, except for basic arithmetic and output.**

---

## Tier 1 Table: Canonical Atoms and Macros

> **Note:** Features marked with `(implemented)` are complete. Others are planned.

| Category   | Atoms (irreducible core)                                         | Macros (author-facing, expand to atoms)                                                               |
| ---------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| Predicate  | `eq?` (implemented), `gt?` (implemented), `lt?` (implemented), `not` (implemented), `gte?`, `lte?`, `has?`, `exists?` | `is?`, `over?`, `under?`, `at-least?`, `at-most?`, `has?` (macro alias), `and`, `or`, `empty?`      |
| Assignment | `set!` (implemented), `del!` (implemented), `add!`, `sub!`, `mul!`, `div!`, `push!`, `pull!` | `inc!`, `dec!`                                                                                        |
| Data       | `get` (implemented), `list` (implemented), `len` (implemented)   | (N/A)                                                                                                 |
| Math/Value | `+` (implemented), `-` (implemented), `*` (implemented), `/` (implemented), `min`, `max`, `abs`, `mod` | (N/A)                                                                                                 |
| Control    | `cond` (implemented), `do`                                       | `if`, `when`, `else`, `let`, `for-each`                                                               |
| Output     | `print`                                                          | —                                                                                                     |
| Random     | `rand`                                                           | `chance?`                                                                                             |
| Utility    | `auto-get` (implemented evaluator feature)                       | `path`, `first`, `last`, `nth` (deferred)                                                             |
| Debug      | (future/reserved)                                                | `debug`, `fail`, `error`, `assert` (not part of Tier 1)                                               |

---

## Predicate Macros

**Pattern:** Most are thin wrappers over atoms, auto-getting paths where needed. No manual get ever needed in author code.

| Macro       | Expands to (Atom) | Purpose                               |
| ----------- | ----------------- | ------------------------------------- |
| `is?`       | `eq?`             | Equality or truthy test.              |
| `over?`     | `gt?`             | Greater than comparison.              |
| `under?`    | `lt?`             | Less than comparison.                 |
| `at-least?` | `gte?`            | Greater or equal comparison.          |
| `at-most?`  | `lte?`            | Less or equal comparison.             |
| `has?`      | `has?`            | Membership in collection.             |
| `exists?`   | `exists?`         | Path/value existence check.           |
| `not`       | `not`             | Logical negation.                     |
| `and`       | `and`             | Logical AND (all predicates true).    |
| `or`        | `or`              | Logical OR (any predicate true).      |
| `empty?`    | `eq?` + `len`     | Collection at path has zero elements. |

**Example:**

```lisp
is? player.hp 10        ; (eq? (get player.hp) 10)
over? player.gold 5     ; (gt? (get player.gold) 5)
not has? items "key"    ; (not (has? (get items) "key"))
empty? inventory        ; (eq? (count (get inventory)) 0)
```

---

## Assignment Macros

**Pattern:** Macro wrappers expanding to atom pattern `(set! path (op (get path) value))` for safe, explicit mutation.

| Macro/Atom | Expansion or Atoms Used | Purpose                            |
| ---------- | ----------------------- | ---------------------------------- |
| `set!`     | (atom)                  | Set path to value                  |
| `add!`     | `set!` + `+` + `get`    | Add amount to value at path        |
| `sub!`     | `set!` + `-` + `get`    | Subtract amount from value at path |
| `mul!`     | `set!` + `*` + `get`    | Multiply value at path             |
| `div!`     | `set!` + `/` + `get`    | Divide value at path               |
| `inc!`     | `add!`                  | Increment by 1                     |
| `dec!`     | `sub!`                  | Decrement by 1                     |
| `push!`    | (atom)                  | Append value to array              |
| `pull!`    | (atom)                  | Remove value from array            |
| `del!`     | (atom)                  | Delete key from object/map         |

**Example:**

```lisp
add! player.gold 5      ; (set! player.gold (+ (get player.gold) 5))
inc! player.hp          ; (add! player.hp 1)
push! items "amulet"    ; atom: append to array
```

---

## Math/Value Atoms

All basic math/value operations are atoms and always author-facing; no macro wrapper is required except for optional auto-get.

| Atom    | Purpose              | Example                    |
| ------- | -------------------- | -------------------------- |
| `+`     | Addition             | + 2 3<br>+ (get hp) 1      |
| `-`     | Subtraction/Negation | - 5 2<br>- (get hp)        |
| `*`     | Multiplication       | \* 2 3<br>\* (get hp) 2    |
| `/`     | Division             | / 10 2<br>/ (get hp) 2     |
| `min`   | Minimum              | min 2 7<br>min (get hp) 3  |
| `max`   | Maximum              | max 1 6<br>max (get hp) 10 |
| `abs`   | Absolute value       | abs -7<br>abs (get hp)     |
| `mod`   | Modulo               | mod 10 4<br>mod (get hp) 3 |
| `len`   | Collection length    | len (get items)            |

> Authors may pass values or `(get path)`.
> Optional macro for auto-get is allowed, but not required.

---

## Control Macros

**Pattern:** Surface sugar expanding to atom control (`cond`, `do`), plus block-local binding and iteration as macros.

| Macro      | Expansion/Atoms Used              | Purpose                                   |
| ---------- | --------------------------------- | ----------------------------------------- |
| `cond`     | Atom                              | Multi-branch conditional                  |
| `do`       | Atom                              | Sequence block                            |
| `if`       | Macro (`cond`, `do`)              | Two-branch conditional with optional else |
| `when`     | Macro (`cond`, `do`)              | Single-branch conditional                 |
| `else`     | Macro (`cond`)                    | Used as fallback clause in if/cond/when   |
| `let`      | Macro (`do`, `set!` or subst)     | Local binding for a block                 |
| `for-each` | Macro (`do`, `cond`, `let`, etc.) | Iterate over each element in collection   |

**Example:**

```lisp
if is? player.hp 0 {
  print "You die!"
} else {
  print "You live!"
}

let { x 5 y (+ 1 2) } {
  add! player.hp x
  print y
}

for-each inventory item {
  print item
}
```

---

## Output Atom

* **`print`**: Atom; author-facing for narrative/UI/debug output.
  **Example:**

  ```lisp
  print "You open the door."
  ```

---

## Random Atoms and Macros

| Atom   | Macro     | Purpose                    | Example                             |
| ------ | --------- | -------------------------- | ----------------------------------- |
| `rand` | —         | Random integer in range    | rand 1 10                           |
| —      | `chance?` | Macro: true with X% chance | chance? 25 → (lte? (rand 1 100) 25) |

* Random atoms are always deterministic/reproducible by virtue of tracked seed.

---

## Utility & Debug

* **`auto-get`**: Not a macro or atom, but a macro-expansion feature: macro args that are paths are always auto-converted to `(get path)`.
* **`first`, `last`, `nth`, `path`**: Reserved for future author need, not part of Tier 1.
* **`debug`, `fail`, `error`, `assert`**: Reserved for future Tier 4 (debugging, tracing, error reporting).

---

## Principles Upheld

* **Minimal but pragmatic atom set:** Only include atoms for operations that are not robustly/clearly composable as macros, or are universal and efficient.
* **Macros for ergonomics:** Authors never write `get`, always use surface macros for predicates/assignment/control.
* **Author surface is clear, readable, and aligned with standard programming/narrative patterns.**

---

## End of Tier 1 Canonical Spec

---

# **Sutra Tier 2 Macros — Canonical Specification**

---

## 1. `requires` — Resource/Cost Gate

### **Purpose**

Gates the visibility/availability of a choice or action on a condition (usually a resource or stat check), and, optionally, handles resource spending or custom “can’t afford” feedback.

### **Canonical Syntax**

```sutra
choice {
  requires (player.gold >= 10) {
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

_Only shows/enables the “Bribe” choice if player.gold ≥ 10; author handles cost in block._

### **Macro Expansion**

Expands to a gating pattern—only renders (or enables) the inner block if the predicate is true:

```lisp
(if (gte? player.gold 10)
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

_(In a UI, the engine can also auto-show “locked” choices or feedback if desired.)_

### **Rationale**

- Self-documenting intent (“this is a cost/resource gate”).

- Clean, repeatable authoring of pay-to-act, affordance, or ability-gated options.

- Enables system/UI to consistently show/hide/grey out options and track resource cost patterns.


### **Edge Cases**

- Author is responsible for handling the cost deduction or custom “can’t afford” messaging.

- Can nest, or combine with multiple requires in a single choice block for multi-resource gates.


---

## 2. `threshold` — Menace/Fail/Trigger Pattern

### **Purpose**

Triggers a narrative or systemic consequence _automatically_ when a stat or resource crosses a specified value.

### **Canonical Syntax**

```sutra
threshold (player.suspicion >= 5) {
  print "You've been caught snooping! Game over."
  set! player.status "lost"
}
```

_Runs immediately when the condition first becomes true, regardless of location or thread context._

### **Macro Expansion**

Registers a global/system “watcher”:

```lisp
(if (and (not (get player.status "lost"))
         (gte? player.suspicion 5))
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

Defines a set of repeatable, open “spoke” options from a central hub—classic for open maps, safe zones, or menu navigation.

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

- Makes “hub zones” safe, extensible, and visualizable.

- Reduces copy/paste and navigation spaghetti.


### **Edge Cases**

- Can be nested inside a thread, or global (e.g., world map).

- Spokes can have their own gating (using `requires`).


---

## 4. `select` — Salience/Weighted Storylet/Event

### **Purpose**

Lets the engine/system pick the most “salient” (highest-weight) event/storylet from a list—QBN/AI Director/priority-driven selection.

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

- Authors can express both emergent narrative and “director” logic in one block.


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

_Automatically fires when the player’s location becomes "kitchen"._

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

- Avoids scattering “arrival” logic in many different places.


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

Lets players choose their motivation/attitude/“why” for an action, not just the “what”—records emotional or style choices for callbacks or narrative flavor.

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

_Records the player’s chosen intent for later use by the story or mechanics._

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

# Sutra Tier 3: Emergent Gameplay, Pools, History, and Dynamic Text — Canonical Reference

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
    Track and gate seen events, enable “no repeats,” achievements, recaps, and replayability.

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

|Macro/Pattern|Syntax Example|Use Case|
|---|---|---|
|pool|`(storylet "id" (tag X) (weight Y))`|Event collection|
|select/random|`select from pool {...}` `random from pool ...`|Weighted/random selection|
|sample/shuffle|`sample 2 from pool ...` `shuffle pool (tag rumor)`|Variety, no repeats|
|history/seen|`history`, `if seen "id" {...}`|Track, gate, recap|
|Interpolation|`"HP: {player.hp}"`|Seamless variable insertion|
|Fragment insertion|`"{red|blue|
|Grammar/template|`[adjective]`, `[animal]` in text|Templates/procgen flavor|

---

### **5. Implementation Feasibility**

- **Parser:**

    - All features extend naturally from Sutra’s block/brace + s-expr conventions.

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

- **Track history and “seen” status to maximize emergent gameplay.**

- **Keep templates/pools shallow and composable for easy scaling.**


---

## **What’s Next (Suggested Order)**

- **Formalize agent/group event execution (per-agent pools and history).**

- **Expand context and weighting in pools/grammar for even richer simulation.**

- **Eventually add procedural event/pool generation and debugging/analytics macros.**


---

**This is your canonical Tier 3 reference for dynamic, emergent, and replayable Sutra games.**
If you need a PDF, sample projects, or parser/AST specs, just ask!

---

Here is the **formalized, canonical Tier 3+ Sutra macro and grammar/event/pool system**, now fully **prefix-uniform**, using block structure for readability, and reflecting the explicit `else` in all author-facing conditionals.

---

# Sutra Tier 3+: Pools, Weighted Selection, Grammar, Dynamic Text, and Conditionals

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
    old          { when (> agent.age 40) }
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
when (if (> agent.rivalry 2) true false)
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
        when (> agent.rivalry 2)
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
