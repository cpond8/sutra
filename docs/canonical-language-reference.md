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

| Type    | Example         | Description                                         | Impl.           |
| :------ | :-------------- | :-------------------------------------------------- | :-------------- |
| Number  | `42`, `-3.14`   | 64-bit float                                        | `Value::Number` |
| Boolean | `true`, `false` | Boolean                                             | `Value::Bool`   |
| String  | `"foo\nbar"`    | Double-quoted, escapes: `\\n`, `\\t`, `\\"`, `\\\\` | `Value::String` |
| Symbol  | `foo`, `+`      | Variable/function names                             | `Value::Symbol` |
| Nil     | `nil`           | Absence of value                                    | `Value::Nil`    |

### 2.2 Collections

| Type | Example            | Description            | Impl.         |
| :--- | :----------------- | :--------------------- | :------------ |
| List | `(1 2 "a" true)`   | Ordered, heterogeneous | `Value::List` |
| Map  | `{foo: 1, bar: 2}` | Key-value, string keys | `Value::Map`  |

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

| Construct  | Arity | Example                     | Description              | Impl.      | Status  |
| :--------- | :---- | :-------------------------- | :----------------------- | :--------- | :------ |
| `if`       | 3     | `(if condition then else)`  | Conditional              | `Expr::If` | impl.   |
| `cond`     | 3..   | `(cond ((cond1) then) ...)` | Branching Conditional    | Macro      | impl.   |
| `do`       | 0..   | `(do expr1 expr2 ...)`      | Sequence, returns last   | Atom: `do` | impl.   |
| `when`     | 2..   | `(when condition ...)`      | Conditional `if` w/ `do` | Macro      | planned |
| `let`      | 2     | `(let (...) ...)`           | Lexical bindings         | Macro      | planned |
| `for-each` | 3     | `(for-each ...)`            | Looping construct        | Macro      | planned |

### 3.4 Arity (Function & Macro Arguments)

Arity refers to the number of arguments a function or macro accepts. Sutra supports several kinds of arity:

- **Fixed Arity:** The function takes an exact number of arguments (e.g., `(lt? a b)` has an arity of 2).
- **Variadic Arity:** The function can take a variable number of arguments. This is indicated by `..` in the arity column.
  - `N..` means at least N arguments are required (e.g., `+` requires at least two arguments).
  - `..N` means at most N arguments are allowed.
- **Optional Arguments:** While not a formal feature, optionality can be implemented within a macro or function's logic.

In function definitions, a variadic parameter is specified using `...` before the parameter name (e.g., `(define (my-func ...args) ...)`) which collects all subsequent arguments into a single list named `args`.

---

## 4. Design Notes & Edge Cases

This section highlights specific behaviors and design choices in Sutra that might be non-obvious but are intentional. Understanding these can help in writing more robust and idiomatic Sutra code.

- **Identity Values for Arithmetic Atoms:** When called with no arguments, `+` returns its identity value `0`, and `*` returns its identity value `1`. This is a common convention in Lisp-family languages.

  - `(+)` => `0`
  - `(*)` => `1`

- **Unary Negation and Reciprocal:** The `-` and `/` atoms can be called with a single argument.

  - `(- x)` returns the negation of `x`.
  - `(/ x)` returns the reciprocal `1/x`.

- **Trivial Truth for Comparison Atoms:** Comparison atoms (`eq?`, `gt?`, `lt?`, `gte?`, `lte?`) return `true` when given zero or one argument. The logic is that any sequence with one or zero elements is trivially ordered or equal.

  - `(gt? 5)` => `true`
  - `(lt?)` => `true`

- **`print` Arity:** The `print` atom strictly requires one argument. Providing a different number of arguments will raise an arity mismatch error. For multi-argument printing, use the `display` macro.

---

## 5. Assignment & State

| Macro   | Arity | Expansion Pattern                            | Impl.       | Status  |
| :------ | :---- | :------------------------------------------- | :---------- | :------ |
| `set!`  | 2     | `(core/set! (path ...) value)`               | Macro, Atom | impl.   |
| `del!`  | 1     | `(core/del! (path ...))`                     | Macro, Atom | impl.   |
| `add!`  | 2     | `(core/set! (path ...) (+ (get ...) value))` | Macro, Atom | impl.   |
| `sub!`  | 2     | `(core/set! (path ...) (- (get ...) value))` | Macro, Atom | impl.   |
| `inc!`  | 1     | `(core/set! (path ...) (+ (get ...) 1))`     | Macro, Atom | impl.   |
| `dec!`  | 1     | `(core/set! (path ...) (- (get ...) 1))`     | Macro, Atom | impl.   |
| `mul!`  | 2     | `(core/set! (path ...) (* (get ...) value))` | Macro, Atom | planned |
| `div!`  | 2     | `(core/set! (path ...) (/ (get ...) value))` | Macro, Atom | planned |
| `push!` | 1..   | `(core/push! (path ...) value ...)`          | Macro, Atom | planned |
| `pull!` | 1..   | `(core/pull! (path ...) value ...)`          | Macro, Atom | planned |

---

## 6. Predicates & Logic

| Macro     | Arity | Expands to     | Purpose                  | Impl. | Status  | Macro Aliases     |
| :-------- | :---- | :------------- | :----------------------- | :---- | :------ | :---------------- |
| `eq?`     | 2..   | —             | Equality                 | Atom  | impl.   | `=`, `is?`        |
| `gt?`     | 2..   | —             | Greater than             | Atom  | impl.   | `>`, `over?`      |
| `lt?`     | 2..   | —             | Less than                | Atom  | impl.   | `<`, `under?`     |
| `gte?`    | 2..   | —             | Greater/equal            | Atom  | impl.   | `>=`, `at-least?` |
| `lte?`    | 2..   | —             | Less/equal               | Atom  | impl.   | `<=`, `at-most?`  |
| `not`     | 1     | —             | Negation                 | Atom  | impl.   | —                |
| `has?`    | 2..   | —             | Membership in collection | Atom  | planned | —                |
| `exists?` | 1     | `core/exists?` | Path/value existence     | Macro | planned | —                |
| `and`     | 0..   | `(if ...)`     | Logical AND              | Macro | planned | —                |
| `or`      | 0..   | `(if ...)`     | Logical OR               | Macro | planned | —                |
| `empty?`  | 1     | `eq?` + `len`  | Collection is empty      | Macro | planned | —                |

---

## 7. Math & Value Operations

| Atom  | Arity | Purpose              | Impl. | Status  |
| :---- | :---- | :------------------- | :---- | :------ |
| `+`   | 0..   | Addition             | Atom  | impl.   |
| `-`   | 1..   | Subtraction/Negation | Atom  | impl.   |
| `*`   | 0..   | Multiplication       | Atom  | impl.   |
| `/`   | 1..   | Division             | Atom  | impl.   |
| `mod` | 2     | Modulo (int)         | Atom  | impl.   |
| `len` | 1     | Length (list/string) | Atom  | impl.   |
| `min` | 1..   | Minimum              | Atom  | planned |
| `max` | 1..   | Maximum              | Atom  | planned |
| `abs` | 1     | Absolute value       | Atom  | planned |

---

## 8. String Utilities

| Macro/Atom  | Arity | Signature                 | Purpose                       | Impl.       | Status  |
| :---------- | :---- | :------------------------ | :---------------------------- | :---------- | :------ |
| `display`   | 0..   | `(display a b ...)`       | Print multiple values         | Macro       | impl.   |
| `str+`      | 0..   | `(str+ a b ...)`          | Concatenate strings           | Macro, Atom | impl.   |
| `str`       | 1     | `(str x)`                 | Typecast to string            | Atom        | planned |
| `join-str+` | 2..   | `(join-str+ sep a b ...)` | Join strings with a separator | Macro       | planned |

---

## 9. World Interaction

| Atom           | Arity | Purpose              | Impl. |
| :------------- | :---- | :------------------- | :---- |
| `core/set!`    | 2     | Set value at path    | Atom  |
| `core/get`     | 1     | Get value at path    | Atom  |
| `core/del!`    | 1     | Delete value at path | Atom  |
| `core/exists?` | 1     | Path existence       | Atom  |

---

## 10. I/O & Random

| Atom/Macro | Arity | Purpose                    | Impl. | Status  |
| :--------- | :---- | :------------------------- | :---- | :------ |
| `print`    | 1     | Output single value        | Atom  | impl.   |
| `display`  | 0..   | Output multiple values     | Macro | impl.   |
| `rand`     | 0     | Random float               | Atom  | impl.   |
| `chance?`  | 1     | Macro: true with X% chance | Macro | planned |

---

## 11. Error Handling

- **Parse-time:** Invalid tokens, unmatched delimiters, bad escapes.
- **Validation:** Unknown macros/atoms, arity/type errors, invalid paths.
- **Runtime:** Type errors, division by zero, invalid world access.

---

## 12. Comments

- Start with `;` and continue to end of line.

---

## 13. Example Program

```sutra
; Factorial function
(define (fact n)
  (if (<= n 1)
      1
      (* n (fact (- n 1)))))
(fact 5) ; => 120
```

---

## 14. Grammar Summary

- See `src/syntax/grammar.pest` for the full PEG grammar.
- All syntax is formally specified and enforced by the parser.

---

## 15. Macro System

- All author-facing macros are defined in `src/macros/macros.sutra` and registered at startup.
- Macro expansion is canonical and deterministic.
- Macro environment is built by `build_canonical_macro_env()`.

---

## 16. Validation & Evaluation

- All code is parsed, macroexpanded, validated, and then evaluated.
- Validation checks for unknown macros/atoms, arity, and type errors.
- Evaluation is recursive, with world state and output managed by the runtime.

---

## 17. Value Types

| Type   | Example         | Description            |
| :----- | :-------------- | :--------------------- |
| Nil    | `nil`           | Absence of value       |
| Number | `42`, `3.14`    | 64-bit float           |
| String | `"hello"`       | UTF-8 string           |
| Bool   | `true`, `false` | Boolean                |
| List   | `(1 2 "a")`     | Ordered, heterogeneous |
| Map    | `{foo: 1}`      | Key-value, string keys |
| Path   | `(path a b)`    | World state access     |

---

## 18. CLI & Tooling

- CLI supports: run, macroexpand, macrotrace, validate, format, test, list-macros, list-atoms, ast, gen-expected.
- Output is pretty-printed and colorized.

---

## 19. Extensibility

- New atoms/macros can be added via Rust or Sutra macro files.
- All macro and atom registration is centralized and canonical.

---

## 20. Not Yet impl. (Planned)

- Tier 2+ high-level macros (e.g., `requires`, `threshold`, `hub`, `select`).
- Map literals and advanced collection utilities.
- Advanced error handling and debugging macros.

---

## 21. References

- Canonical spec: `docs/specs/language-spec.md`
- Macro library: `src/macros/macros.sutra`
- Grammar: `src/syntax/grammar.pest`
- Atoms: `src/atoms/`
- AST: `src/ast/`
- Runtime: `src/runtime/`

---

This document is fully synchronized with the codebase and spec as of July 2025. For any ambiguity or missing feature, consult the canonical spec and the relevant module in the codebase.
