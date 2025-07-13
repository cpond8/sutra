# Sutra Language Specification

## 1. Overview

Sutra is a minimal, expression-based language designed for composable, data-driven computation. This document is the canonical reference for Sutra’s syntax, constructs, semantics, and core operations.

---

## 2. Syntax

- **Canonical Syntax:** List style (Lisp/Scheme s-expressions) is the ground truth. Block style (`{}`) is authoring sugar, compiled to list style.
- **Whitespace** and **comments** (`; ...`) are ignored except within strings.
- **Top-level:** A program is a sequence of expressions.

**Template Example:**
```sutra
(define (function-name param1 param2) (body ...))
(function-name arg1 arg2)
```

---

## 3. Expressions

### 3.1 Atoms (Primitives)

| Type    | Example         | Description                                    |
|:--------|:----------------|:-----------------------------------------------|
| Number  | `42`, `-3.14`   | 64-bit float                                   |
| Boolean | `true`, `false` | Boolean                                        |
| String  | `"foo\nbar"`    | Double-quoted, escapes: `\n`, `\t`, `\"`, `\\` |
| Symbol  | `foo`, `+`      | Variable/function names                        |
| Nil     | `nil`           | Absence of value                               |

### 3.2 Collections

| Type | Example            | Description            |
|:-----|:-------------------|:-----------------------|
| List | `(1 2 "a" true)`   | Ordered, heterogeneous |
| Map  | `{foo: 1, bar: 2}` | Key-value, string keys |  *Planned*

### 3.3 Paths

- Used for world state access: `(path player hp)` or dotted symbol `player.hp`
- Canonicalized by the macro system.

### 3.4 Quote

- `'expr` — returns the literal expression, not evaluated.

### 3.5 Spread Argument

- `...symbol` — splices a list of arguments in call position.

---

## 4. Core Constructs

### 4.1 Lists & Blocks

- **List:** `(expr1 expr2 ...)` — function calls, data, etc.
- **Block:** `{expr1 expr2 ...}` — groups expressions, compiles to list.

### 4.2 Function & Macro Definition

- `(define (name param1 param2 ... ...rest) body)`
- Variadic via `...rest`

### 4.3 Control Flow

| Construct  | Arity | Template Example       | Description              | Impl.     |
|:-----------|:------|:-----------------------|:-------------------------|:----------|
| `if`       | 3     | `(if cond then else)`  | Conditional              | Impl.     |
| `do`       | 0..   | `(do expr1 expr2 ...)` | Sequence, returns last   | Impl.     |
| `when`     | 2..   | `(when cond ...)`      | Conditional `if` w/ `do` | *Planned* |
| `let`      | 2     | `(let (...) ...)`      | Lexical bindings         | *Planned* |
| `for-each` | 3     | `(for-each ...)`       | Looping construct        | *Planned* |

---

## 5. Atoms & Macros

All atoms and macros are described as if fully implemented.
*Planned* items are marked with an asterisk and a note.

### 5.1 Math Atoms

| Atom   | Arity | Purpose              | Impl.     |
|:-------|:------|:---------------------|:----------|
| `+`    | 0..   | Addition             | Impl.     |
| `-`    | 1..   | Subtraction/Negation | Impl.     |
| `*`    | 0..   | Multiplication       | Impl.     |
| `/`    | 1..   | Division             | Impl.     |
| `mod`  | 2     | Modulo (int)         | Impl.     |
| `min`  | 1..   | Minimum              | *Planned* |
| `max`  | 1..   | Maximum              | *Planned* |
| `abs`  | 1     | Absolute value       | *Planned* |

### 5.2 Logic Atoms

| Atom     | Arity | Purpose                  | Impl.     |
|:---------|:------|:-------------------------|:----------|
| `eq?`    | 2..   | Equality                 | Impl.     |
| `gt?`    | 2..   | Greater than             | Impl.     |
| `lt?`    | 2..   | Less than                | Impl.     |
| `gte?`   | 2..   | Greater or equal         | Impl.     |
| `lte?`   | 2..   | Less or equal            | Impl.     |
| `not`    | 1     | Logical negation         | Impl.     |
| `has?`   | 2..   | Membership in collection | *Planned* |
| `empty?` | 1     | Collection is empty      | *Planned* |

### 5.3 Collection Atoms

| Atom         | Arity | Purpose                       | Impl.     |
|:-------------|:------|:------------------------------|:----------|
| `list`       | 0..   | List creation                 | Impl.     |
| `len`        | 1     | Length of list or string      | Impl.     |
| `core/push!` | 1..   | Push value(s) to list at path | *Planned* |
| `core/pull!` | 1..   | Remove value(s) from list     | *Planned* |
| `str+`       | 0..   | String concatenation          | Impl.     |

### 5.4 World/State Atoms

| Atom           | Arity | Purpose                       | Impl.     |
|:---------------|:------|:------------------------------|:----------|
| `core/set!`    | 2     | Set value at path             | Impl.     |
| `core/get`     | 1     | Get value at path             | Impl.     |
| `core/del!`    | 1     | Delete value at path          | Impl.     |
| `core/exists?` | 1     | Path existence                | Impl.     |
| `core/push!`   | 1..   | Push value(s) to list at path | *Planned* |
| `core/pull!`   | 1..   | Remove value(s) from list     | *Planned* |

### 5.5 Execution Atoms

| Atom     | Arity | Purpose              | Impl. |
|:---------|:------|:---------------------|:------|
| `do`     | 0..   | Sequential execution | Impl. |
| `error`  | 1     | Raise an error       | Impl. |
| `apply`  | 2     | Function application | Impl. |

### 5.6 External/IO Atoms

| Atom         | Arity | Purpose             | Impl. |
|:-------------|:------|:--------------------|:------|
| `print`      | 1     | Output single value | Impl. |
| `core/print` | 1     | Output single value | Impl. |
| `rand`       | 0     | Random float        | Impl. |

### 5.7 String Atoms

| Atom   | Arity | Purpose              | Impl.     |
|:-------|:------|:---------------------|:----------|
| `str`  | 1     | Typecast to string   | *Planned* |
| `str+` | 0..   | String concatenation | Impl.     |

### 5.8 Macros

| Macro      | Arity | Purpose                       | Impl.     |
|:-----------|:------|:------------------------------|:----------|
| `set!`     | 2     | Set value at path             | Impl.     |
| `del!`     | 1     | Delete value at path          | Impl.     |
| `add!`     | 2     | Add to value at path          | Impl.     |
| `sub!`     | 2     | Subtract from value at path   | Impl.     |
| `inc!`     | 1     | Increment value at path       | Impl.     |
| `dec!`     | 1     | Decrement value at path       | Impl.     |
| `mul!`     | 2     | Multiply value at path        | *Planned* |
| `div!`     | 2     | Divide value at path          | *Planned* |
| `push!`    | 1..   | Push value(s) to list at path | *Planned* |
| `pull!`    | 1..   | Remove value(s) from list     | *Planned* |
| `is?`      | 2     | Equality/truthy               | Impl.     |
| `over?`    | 2     | Greater than                  | Impl.     |
| `under?`   | 2     | Less than                     | Impl.     |
| `at-least?`| 2     | Greater or equal              | *Planned* |
| `at-most?` | 2     | Less or equal                 | *Planned* |
| `exists?`  | 1     | Path/value existence          | *Planned* |
| `and`      | 0..   | Logical AND                   | *Planned* |
| `or`       | 0..   | Logical OR                    | *Planned* |
| `empty?`   | 1     | Collection is empty           | *Planned* |
| `display`  | 0..   | Print multiple values         | Impl.     |
| `join-str+`| 2..   | Join strings with separator   | *Planned* |
| `chance?`  | 1     | True with X% chance           | *Planned* |

*Note: Items marked with *Planned* are specified and part of the language, but not yet impl. as of July 2025.*

---

## 6. Error Handling

- **Parse-time:** Invalid tokens, unmatched delimiters, bad escapes.
- **Validation:** Unknown macros/atoms, arity/type errors, invalid paths.
- **Runtime:** Type errors, division by zero, invalid world access.

| Code        | Description               |
|-------------|---------------------------|
| PARSE_ERROR | Syntax parsing failures   |
| ARITY_ERROR | Wrong number of arguments |
| TYPE_ERROR  | Type mismatches           |
| EVAL_ERROR  | General evaluation errors |

---

## 7. Comments

- Start with `;` and continue to end of line.

---

## 8. References

- Canonical spec: `docs/specs/language-spec.md`
- Macro library: `src/macros/macros.sutra`
- Grammar: `src/syntax/grammar.pest`
- Atoms: `src/atoms/`
- AST: `src/ast/`
- Runtime: `src/runtime/`

---

*Planned* atoms/macros are part of the language specification and may appear in documentation, but are not yet impl. as of July 2025.
