# Sutra Language Reference

## Overview

Sutra is a minimal, homoiconic, expression-based language designed for composable, data-driven computation. This document provides a concise reference for the syntax, constructs, and semantics of the Sutra language as implemented in the current engine.

---

## 1. Syntax

- **Canonical Syntax:** List style (Lisp/Scheme s-expressions) is the ground truth. Block style (`{}`) is authoring sugar, compiled to list style.
- **Whitespace** and **comments** (`; ...`) are ignored except within strings.
- **Top-level:** A program is a sequence of expressions.

### Example

```sutra
(define (add x y) (+ x y))
(add 2 3) ; => 5
```

---

## 2. Expressions

### 2.1 Atoms (Primitives)

| Type    | Example         | Description                                         | Implementation  |
| ------- | --------------- | --------------------------------------------------- | --------------- |
| Number  | `42`, `-3.14`   | 64-bit float                                        | `Value::Number` |
| Boolean | `true`, `false` | Boolean                                             | `Value::Bool`   |
| String  | `"foo\nbar"`    | Double-quoted, escapes: `\\n`, `\\t`, `\\"`, `\\\\` | `Value::String` |
| Symbol  | `foo`, `+`      | Variable/function names                             | `Value::Symbol` |
| Nil     | `nil`           | Absence of value                                    | `Value::Nil`    |

### 2.2 Collections

| Type | Example            | Description            | Implementation |
| ---- | ------------------ | ---------------------- | -------------- |
| List | `(1 2 "a" true)`   | Ordered, heterogeneous | `Value::List`  |
| Map  | `{foo: 1, bar: 2}` | Key-value, string keys | `Value::Map`   |

### 2.3 Paths

- Used for world state access: `(path player hp)` or dotted symbol `player.hp`
- Canonicalized by macro system (`src/macros/std.rs`)

### 2.4 Quote

- `'expr` — returns the literal expression, not evaluated.
- AST: `Expr::Quote`

### 2.5 Spread Argument

- `...symbol` — splices a list of arguments in call position.
- Grammar: `spread_arg`

---

## 3. Core Constructs

### 3.1 Lists & Blocks

- **List:** `(expr1 expr2 ...)` — function calls, data, etc.
- **Block:** `{expr1 expr2 ...}` — groups expressions, compiles to list.

### 3.2 Function & Macro Definition

- `(define (name param1 param2 ... ...rest) body)`
- Variadic via `...rest`
- AST: `Expr::Define`

### 3.3 Control Flow

| Construct | Example                | Description              | Implementation |
| --------- | ---------------------- | ------------------------ | -------------- |
| `if`      | `(if cond then else)`  | Conditional, not a macro | `Expr::If`     |
| `do`      | `(do expr1 expr2 ...)` | Sequence, returns last   | Atom: `do`     |

---

## 4. Assignment & State

| Macro  | Expansion Pattern                                        | Status      | Implementation |
| ------ | -------------------------------------------------------- | ----------- | -------------- |
| `set!` | `(core/set! (path ...) value)`                           | Implemented | Macro, Atom    |
| `del!` | `(core/del! (path ...))`                                 | Implemented | Macro, Atom    |
| `add!` | `(core/set! (path ...) (+ (core/get (path ...)) value))` | Implemented | Macro, Atom    |
| `sub!` | `(core/set! (path ...) (- (core/get (path ...)) value))` | Implemented | Macro, Atom    |
| `inc!` | `(core/set! (path ...) (+ (core/get (path ...)) 1))`     | Implemented | Macro, Atom    |
| `dec!` | `(core/set! (path ...) (- (core/get (path ...)) 1))`     | Implemented | Macro, Atom    |

---

## 5. Predicates & Logic

| Macro    | Expands to | Purpose         | Status      | Implementation |
| -------- | ---------- | --------------- | ----------- | -------------- |
| `is?`    | `eq?`      | Equality/truthy | Implemented | Macro, Atom    |
| `over?`  | `gt?`      | Greater than    | Implemented | Macro, Atom    |
| `under?` | `lt?`      | Less than       | Implemented | Macro, Atom    |
| `not`    | `not`      | Negation        | Implemented | Macro, Atom    |
| `eq?`    | —          | Equality        | Implemented | Atom           |
| `gt?`    | —          | Greater than    | Implemented | Atom           |
| `lt?`    | —          | Less than       | Implemented | Atom           |
| `gte?`   | —          | Greater/equal   | Implemented | Atom           |
| `lte?`   | —          | Less/equal      | Implemented | Atom           |

---

## 6. Math & Value Operations

| Atom  | Purpose              | Status      | Implementation |
| ----- | -------------------- | ----------- | -------------- |
| `+`   | Addition             | Implemented | Atom           |
| `-`   | Subtraction/Negation | Implemented | Atom           |
| `*`   | Multiplication       | Implemented | Atom           |
| `/`   | Division             | Implemented | Atom           |
| `mod` | Modulo (int)         | Implemented | Atom           |
| `len` | Length (list/string) | Implemented | Atom           |

---

## 7. String Utilities

| Macro/Atom  | Signature                 | Purpose             | Status      | Implementation |
| ----------- | ------------------------- | ------------------- | ----------- | -------------- |
| `str+`      | `(str+ a b ...)`          | Concatenate strings | Implemented | Macro, Atom    |
| `str`       | `(str x)`                 | Typecast to string  | Planned     | —              |
| `join-str+` | `(join-str+ sep a b ...)` | Join with separator | Planned     | —              |

---

## 8. World Interaction

| Atom           | Purpose              | Status      | Implementation |
| -------------- | -------------------- | ----------- | -------------- |
| `core/set!`    | Set value at path    | Implemented | Atom           |
| `core/get`     | Get value at path    | Implemented | Atom           |
| `core/del!`    | Delete value at path | Implemented | Atom           |
| `core/exists?` | Path existence       | Implemented | Atom           |

---

## 9. I/O & Random

| Atom    | Purpose      | Status      | Implementation |
| ------- | ------------ | ----------- | -------------- |
| `print` | Output value | Implemented | Atom           |
| `rand`  | Random float | Implemented | Atom           |

---

## 10. Error Handling

- **Parse-time:** Invalid tokens, unmatched delimiters, bad escapes.
- **Validation:** Unknown macros/atoms, arity/type errors, invalid paths.
- **Runtime:** Type errors, division by zero, invalid world access.

---

## 11. Comments

- Start with `;` and continue to end of line.

---

## 12. Example Program

```sutra
; Factorial function
(define (fact n)
  (if (<= n 1)
      1
      (* n (fact (- n 1)))))
(fact 5) ; => 120
```

---

## 13. Grammar Summary

- See `src/syntax/grammar.pest` for the full PEG grammar.
- All syntax is formally specified and enforced by the parser.

---

## 14. Macro System

- All author-facing macros are defined in `src/macros/macros.sutra` and registered at startup.
- Macro expansion is canonical and deterministic.
- Macro environment is built by `build_canonical_macro_env()`.

---

## 15. Validation & Evaluation

- All code is parsed, macroexpanded, validated, and then evaluated.
- Validation checks for unknown macros/atoms, arity, and type errors.
- Evaluation is recursive, with world state and output managed by the runtime.

---

## 16. Value Types

| Type   | Example         | Description            |
| ------ | --------------- | ---------------------- |
| Nil    | `nil`           | Absence of value       |
| Number | `42`, `3.14`    | 64-bit float           |
| String | `"hello"`       | UTF-8 string           |
| Bool   | `true`, `false` | Boolean                |
| List   | `(1 2 "a")`     | Ordered, heterogeneous |
| Map    | `{foo: 1}`      | Key-value, string keys |
| Path   | `(path a b)`    | World state access     |

---

## 17. CLI & Tooling

- CLI supports: run, macroexpand, macrotrace, validate, format, test, list-macros, list-atoms, ast, gen-expected.
- Output is pretty-printed and colorized.

---

## 18. Extensibility

- New atoms/macros can be added via Rust or Sutra macro files.
- All macro and atom registration is centralized and canonical.

---

## 19. Not Yet Implemented (Planned)

- Tier 2+ macros (e.g., `when`, `let`, `for-each`, `debug`, `fail`, `error`, `assert`, `min`, `max`, `abs`, `str`, `join-str+`).
- Map literals and advanced collection utilities.
- Advanced error handling and debugging macros.

---

## 20. References

- Canonical spec: `docs/specs/language-spec.md`
- Macro library: `src/macros/macros.sutra`
- Grammar: `src/syntax/grammar.pest`
- Atoms: `src/atoms/`
- AST: `src/ast/`
- Runtime: `src/runtime/`

---

This document is fully synchronized with the codebase and spec as of July 2025. For any ambiguity or missing feature, consult the canonical spec and the relevant module in the codebase.
