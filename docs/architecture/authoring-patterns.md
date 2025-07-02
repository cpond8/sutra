---
status: living
last-reviewed: 2024-07-03
summary: Authoring patterns, best practices, and pragmatic guidelines for Sutra.
---

# 03_SUTRA_AUTHORING_PATTERNS_AND_PRAGMATIC_GUIDELINES.md

---

LAST EDITED: 2025-06-30

⚠️ LIVING DRAFT—NOT FINAL SPECIFICATION

This document captures the evolving toolkit, best practices, and real-world patterns for authoring with Sutra.
Patterns and macros here are not "baked in," but are subject to regular review, refactoring, and replacement as experience and author needs dictate.

Use this file as both a cookbook and a notebook: adapt, annotate, and extend as the engine and your design ideas evolve.

**Sutra supports two fully equivalent authoring syntaxes:**

- **Brace-block, newline-per-form** (recommended for most authors)
- **Classic s-expression (parentheses)** (for power-users and toolchains)
  Both map _one-to-one_ to the canonical AST. You may use either, or mix as suits your workflow; all macroexpansion and tooling works identically on both.

Cross-reference with File 1 (Philosophy/Principles) and File 2 (Core Architecture/Atoms) for context and rationale.

---

## 1. Preamble: Patterns as Evolving Toolkit

- This file is not a library reference for end-users, but a designer's evolving cookbook and "lab notebook."
- Every recipe or macro is open to revision. Regularly revisit and test these against real-world projects or author feedback.
- Each pattern below includes:
  - Pattern goal and "why"
  - Current best implementation in Sutra (macro or atoms)
  - Comments on clarity, authoring pain, and ideas for improvement

> **A Note on Naming:** To make the language as accessible as possible, all examples in this guide use plain-English aliases for comparison operators (e.g., `over?`, `at-least?`). These have shorter, canonical equivalents (`gt?`, `gte?`) that are fully interchangeable and may be preferred by technical authors.

---

## 2. Current Mapping of Narrative/Game Patterns

**All examples use brace-block syntax. S-expr syntax is always available and equivalent.**

### 2.1 Storylet (Quality-Based Narrative Node)

Goal:
Present narrative content when certain qualities/conditions are met, with effects on the world state and/or further choices.

**Canonical macro:**

```
storylet "find-key" {
  and {
    is? player.location "cellar"
    has? player.items "rusty-key"
  }
  do {
    print "You unlock the door with the rusty key."
    set! world.door.unlocked true
  }
}
```

- Why: Mirrors QBN/storylet design: conditionally eligible, effectful, modular.
- Authoring comments: Readable; macro pattern covers 99% of "gated event" needs.
- Improvements: Consider macros for multi-block content, richer text, or salience/weight (see below).

---

### 2.2 Choices, Branches, and Delayed Branching

Goal:
Let players make narrative choices that set qualities/stats, affecting later content ("delayed branching").

**Current pattern:**

```
choice {
  "Fight honorably" {
    set! player.tactic "honor"
    inc! player.brutality
  }
  "Set a trap" {
    set! player.tactic "trick"
    inc! player.finesse
  }
}
```

Later:

```
cond {
  is? player.tactic "honor"
    print "You win the duel with honor."
  is? player.tactic "trick"
    print "Your trap catches the foe."
}
```

- Why: Cleanly separates decision from downstream consequence.
- Improvements: For common "choice sets that update a stat," macro-ize as choice-set-with-update.

---

### 2.3 Resource/Cost Gates and Menace Loops

Goal:
Require a resource (currency, stat, etc.) to access content, or accumulate "menace" for failure/soft lockout.

**Pattern:**

```
cond {
  over? player.gold 10
    do {
      sub! player.gold 10
      print "You buy the artifact."
    }
}
```

Or, for menace:

```
cond {
  over? player.poverty 5
    do {
      print "Hardship strikes! You must find work."
      sub! player.poverty 3
    }
}
```

- Why: Captures the gating/spending pattern, and the "soft fail" recovery loop.
- Improvements: Macro for cost-action (spend, then do) and menace-trigger (on threshold, trigger effect).

---

### 2.4 Hub/Spoke, Open Map, and Cyclic Structures

Goal:
Multiple content nodes can be available in parallel, often in any order; looping or "cycle" play structures.

**Pattern:**

```
storylet "find-clue-A" {
  and {
    not clues.A_found
    is? player.location "library"
  }
  do {
    set! clues.A_found true
    print "You find the secret letter."
  }
}

storylet "find-clue-B" {
  and {
    not clues.B_found
    is? player.location "garden"
  }
  do {
    set! clues.B_found true
    print "You find a torn page."
  }
}

storylet "finale" {
  and {
    clues.A_found
    clues.B_found
  }
  print "You solve the case!"
}
```

- Why: Handles free ordering, parallel gating, bottleneck/merge.
- Improvements: Macros for "storylet pool," "cycle," or "advance progress" (auto-unlock next).

---

### 2.5 Loops, Repetition, and Simulation

Goal:
Model repeated actions/events, "turns," or simulation ticks.

**Current macro pattern:**

```
define repeat n body {
  if (is? n 0)
    {}
    do {
      body
      repeat (- n 1) body
    }
}
```

Or, dynamic:

```
repeat-until (under? player.hp 100) {
  inc! player.hp
}
```

Simulation tick:

```
for-each agents agent {
  agent-tick agent
}
```

- Why: Enables batch updates, agent loops, bounded/unbounded repetition.
- Improvements: Macro library should include robust repeat, for-each, repeat-until, and batch agent patterns.

---

### 2.6 Salience and System-Selected Content

Goal:
System picks the "most relevant" content (rather than user menu), based on context.

**Pattern:**

```
salience-storylet "danger-warning" {
  weight (over? player.menace 7)
  when (not player.warned)
  do {
    print "You feel a sense of dread."
    set! player.warned true
  }
}
```

Engine evaluates all storylets' weight/when conditions, fires the highest.

- Why: Mirrors AI/director/salience models in games.
- Improvements: Standardize a salience macro interface for the engine.

---

### 2.7 Waypoints, Milestones, and Progression

Goal:
Content or story nodes become available after a milestone or "waypoint" is reached, often independent of prior path.

**Pattern:**

```
storylet "reveal-secret" {
  world.next_waypoint_secret
  do {
    print "A revelation!"
    set! world.next_waypoint_secret false
    set! world.next_waypoint_finale true
  }
}
```

- Why: Ensures progression, event gating, "main quest" advancement.
- Improvements: Macro for waypoint or "on-milestone" event blocks.

---

### 2.8 Hybrid Narrative/Gameplay Patterns

Goal:
Link gameplay results to narrative logic, e.g. "spend resource to unlock storylet," "outcome of minigame branches story."

**Pattern:**

```
storylet "star-unlock" {
  over? player.stars 3
  do {
    sub! player.stars 3
    print "You unlock the next story scene!"
  }
}
```

- Why: Generalizes to all resource/quality triggers.
- Improvements: Macro for "spend resource for narrative unlock."

---

### 2.9 Reflective Choices and Personality Traits

Goal:
Choices record not just outcomes but "how" or "why," supporting later flavor or dialogue.

**Pattern:**

```
choice {
  "Be kind" {
    set! player.personality "kind"
  }
  "Be cruel" {
    set! player.personality "cruel"
  }
}
# Later
cond {
  is? player.personality "kind"
    print "You are remembered for your kindness."
}
```

- Why: Enables subtle roleplay/branching.
- Improvements: Macro for choice-set that both sets stat and tags choice.

---

## 3. Macro Library and Author-Facing Abstractions

### 3.1 Standard Macro Definitions

- storylet: eligibility, effects, text
- choice/choice-set: multi-option, stat/flag updates
- repeat, repeat-until, for-each: loops/iteration
- cost-action: resource-spending pattern
- menace-trigger: on stat threshold
- salience-storylet: system-selected content
- waypoint: milestone/progression events

Example Macro Definition (Brace Syntax):

```
define storylet id prereq effects text {
  cond {
    prereq
      do {
        effects
        print text
      }
  }
}
```

### 3.2 Modules and Project Structure

- Macro libraries are grouped by module (e.g. "adventure_macros", "salience_macros").
- Projects import/compose modules as needed.
- Each module documents macro signatures, example use, and "why".

### 3.3 When/How to Define New Macros

- When a pattern is repeated 3+ times, or shows up in Emily Short's pattern catalog, factor as a macro.
- Every macro should be documented with:
  - Example usage
  - Expanded atom form
  - Rationale and notes on edge cases

---

## 4. Author Experience, Debugging, and Tooling

### 4.1 Syntax Reference

- Brace-block (recommended):

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

- Canonical s-expr (for power-users):

```lisp
(storylet "meet-bishop"
  (and
    (over? player.reputation 10)
    (is? player.location "cathedral"))
  (do
    (add! player.reputation 2)
    (print "The Bishop welcomes you.")))
```

- Parentheses and braces can be freely converted; all code is parsed to the canonical AST.

### 4.2 Macroexpansion and Debugging

- Authors can expand any macro to see underlying atoms, in either syntax.
- Macroexpansion traces show all intermediate expansion steps.
- Error reporting links author code, macro-expanded code, and atom execution for debugging.

### 4.3 Author Feedback and "Explain" Features

- On execution, the engine can "explain":
  - Which storylets were eligible/ineligible (and why)
  - Which conditions/choices fired
  - How and why a stat changed

### 4.4 Evolving the Author Surface

- Regularly test patterns for verbosity, ambiguity, or author "pain points."
- When repeated boilerplate emerges, factor a new macro or improve existing ones.

---

## 5. Pragmatic Compromises and Future Decisions

### 5.1 When to Promote Patterns to Atoms

- Only after a pattern is:
  - Extremely common
  - Cannot be robustly or clearly handled via macros
  - Would improve clarity, performance, or maintainability for most authors

### 5.2 How to Iterate and Document

- When a macro or atom is added/changed:
  - Update pattern example, signature, rationale, and implementation
  - Cross-reference change in appendices and changelog

### 5.3 Dealing with Boilerplate

- Identify sources of repetitive code (e.g. manual gating, stat updates).
- Macro-ize where practical; note in docs any cases where macro expansion is unsatisfying, to revisit atom set if needed.

### 5.4 Open Questions for Future Iteration

- How to handle truly complex simulation patterns (agents, events, ticks)?
- When to extend author-facing syntax vs. rely on canonical s-expr?
- Are there patterns that don't fit the atoms+macros model?

---

## 6. Workflow for Ongoing Iteration

### 6.1 Testing and Feedback

- For every new game/narrative mechanic, script as macro + atom composition.
- Gather feedback from actual usage; note awkwardness, confusion, or "missing power."

### 6.2 Versioning and Updating

- Macro libraries and project modules should be versioned (major/minor), with changes documented.
- Old macros should be deprecated, not removed, until all dependent content is updated.

### 6.3 Documentation Process

- Every new pattern or macro:
  - Provide example usage, macroexpansion, rationale, and known limitations.
  - Mark as "experimental" until validated in real use.

### 6.4 Living Pattern and FAQ Log

- Regularly log all questions, problems, and "aha" moments in the appendices for later reflection.

---

## Appendices

---

### A. Emily Short Design Patterns: Concrete Sutra Implementations

- For each major QBN/storylet pattern, a macro/atom code example with comments.
- Storylet, branch-and-bottleneck, sorting hat, hub/spoke, loop/cycle, menace loop, salience/director, hybrid gameplay.

---

### B. Troubleshooting and FAQ for Authors

- Common mistakes (block structure, macro errors, argument grouping)
- "Why didn't my storylet fire?"—Checklist
- How to debug macro expansion
- When to ask for new macros/atoms

---

### C. Glossary

- Atom, macro, storylet, module, world state, salience, etc.
- Terms marked as \[STABLE] or \[EXPERIMENTAL] for ongoing review

---

### D. Reference Scripts

- Example: Small branching narrative, open map, resource gate, simulation tick, etc.
- Macroexpansion views for each.

---

### E. Further Reading and Inspiration

- Emily Short, QBN, salience, modular narrative systems
- SICP, Write Yourself a Scheme, lispy-in-rust
- Blog posts and research cited throughout Files 1 and 2

---

### F. How to Annotate and Update This File

- Log new patterns, macro additions, authoring wins/pain points.
- Mark all "experimental" or "pending" changes clearly.
- Cross-reference with related changes in Files 1 and 2.

---

## Authoring Patterns

## Canonical Conditional: `cond`

- `cond` is the author-facing, variadic conditional macro. It expands to nested `if` expressions, with only `if` as a primitive in the AST.
- All control flow except `if` is macro sugar, defined in the macro library.
- Error cases: no clauses, non-list clause, wrong arity, misplaced/multiple else, malformed else, empty clause, deeply nested cond (recursion limit).
- Migration plan: When the macro system supports variadic/recursive user macros, `cond` will be ported to a user macro and the Rust implementation removed.
- Authors can use `macroexpand` or `macrotrace` to see how their cond is rewritten.

### Example

Input:
```lisp
(cond
  ((over? player.gold 10) (do (sub! player.gold 10) (print "You buy the artifact.")))
  ((is? player.quest "started") (print "You have already begun your quest."))
  (else (print "Not enough gold."))
)
```

Expansion:
```lisp
(if (over? player.gold 10)
    (do (sub! player.gold 10) (print "You buy the artifact."))
    (if (is? player.quest "started")
        (print "You have already begun your quest.")
        (print "Not enough gold.")))
```

See CLI help for more details and macroexpansion tracing.

---

**END OF FILE 3**
(Review, extend, and iterate—this is your evolving author's lab notebook!)
