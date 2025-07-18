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
(define (add x y)
  (+ x y))
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

### 2.3 Symbol Resolution

When evaluating a symbol (e.g., `x`), Sutra follows a specific precedence order:

1. **Lexical Environment**: Variables bound by `let` or `lambda`
2. **Atom Registry**: Built-in functions and special forms (must be called)
3. **World State**: Global state paths (e.g., `x` → `world.state.x`)
4. **Undefined**: Error if symbol is not found anywhere

#### Examples

```sutra
(let x 42)     ; x resolves to 42 (lexical binding)
(+ 1 2)        ; + resolves to atom (callable)
x              ; Error: 'x' is an atom and must be called with arguments
undefined-var  ; Error: undefined symbol: 'undefined-var'
```

#### Error Messages

- **Atom Reference**: `'symbol' is an atom and must be called with arguments (e.g., (symbol ...))`
- **Undefined Symbol**: `undefined symbol: 'symbol'`

### 2.4 Paths

- Used for world state access: `(path player hp)` or dotted symbol `player.hp`
- Canonicalized by macro system (`src/macros/std.rs`)

### 2.5 Quote

- `'expr` — returns the literal expression, not evaluated.
- AST: `Expr::Quote`

### 2.6 Spread Argument

- `...symbol` — splices a list of arguments in call position.
- Grammar: `spread_arg`

---

## 3. Core Constructs

### 3.1 Lists & Blocks

- **List:** `(function arg1 arg2 ...)` — function calls, data structures, etc.
- **Block:** Newline-separated statements, optionally grouped by braces `{ ... }`, compiles to list with implicit `do` when grouped.

### 3.2 Function & Macro Definition

- `(define (name param1 param2 ... ...rest) body)`
- Variadic via `...rest`
- AST: `Expr::Define`

### 3.3 Control Flow

| Construct  | Arity | Example                          | Description              | Impl.       | Status |
| :--------- | :---- | :------------------------------- | :----------------------- | :---------- | :----- |
| `if`       | 3     | `(if condition then else)`       | Conditional              | `Expr::If`  | impl.  |
| `cond`     | 3..   | `(cond ((cond1) then) ...)`      | Branching Conditional    | Macro       | impl.  |
| `do`       | 0..   | `(do expr1 expr2 ...)`           | Sequence, returns last   | Atom: `do`  | impl.  |
| `when`     | 2..   | `(when condition ...)`           | Conditional `if` w/ `do` | Macro       | impl.  |
| `let`      | 2     | `(let ((var val) ...) body ...)` | Lexical bindings         | SpecialForm | impl.  |
| `lambda`   | 2..   | `(lambda (params) body ...)`     | Anonymous function       | SpecialForm | impl.  |
| `for-each` | 3..   | `(for-each ...)`                 | Looping construct        | Macro       | impl.  |

### 3.4 Arity (Function & Macro Arguments)

Arity refers to the number of arguments a function or macro accepts. Sutra supports several kinds of arity:

- **Fixed Arity:** The function takes an exact number of arguments (e.g., `(lt? a b)` has an arity of 2).
- **Variadic Arity:** The function can take a variable number of arguments. This is indicated by `..` in the arity column.
  - `N..` means at least N arguments are required (e.g., `+` requires at least two arguments).
  - `..N` means at most N arguments are allowed.
- **Optional Arguments:** While not a formal feature, optionality can be implemented within a macro or function's logic.

In function definitions, a variadic parameter is specified using `...` before the parameter name (e.g., `(define (my-func ...args) ...)`) which collects all subsequent arguments into a single list named `args`.

---

## 4. First-Class Functions: `lambda`

Sutra supports first-class, lexically scoped anonymous functions via the `lambda` special form.

### Syntax

```sutra
(lambda (param1 param2 ...) body1 body2 ...)
```

- Parameters are listed in a vector (e.g., `(x y)`), supporting both fixed and variadic arity (with `...rest`).
- The body may contain one or more expressions; the value of the last is returned.

### Semantics

- `lambda` returns a function value that can be called like any other function.
- Lambdas capture their lexical environment (closures).
- Arity is checked at call time; errors are raised for mismatches.

### Example

```sutra
(define add
  (lambda (x y)
    (+ x y)))
(add 2 3) ; => 5

(define make-adder
  (lambda (n)
    (lambda (x)
      (+ x n))))
(define add5 (make-adder 5))
(add5 10) ; => 15
```

---

## 5. Lexical Bindings: `let`

The `let` special form introduces new lexical bindings for the duration of its body.

### Syntax

```sutra
(let ((var1 val1) (var2 val2) ...) body1 body2 ...)
```

- Each binding is a pair `(var val)`.
- The body may contain one or more expressions; the value of the last is returned.

### Semantics

- Bindings are evaluated sequentially and are visible in the body and subsequent bindings.
- `let` creates a new lexical scope; variables shadow outer bindings.

### Example

```sutra
(let ((x 2)
      (y 3))
  (* x y)) ; => 6

(let ((x 1))
  (let ((x 2)
        (y x))
    (+ x y))) ; => 3
```

---

## 3. Block Style Transformation Rules

While List style is the canonical ground truth syntax, Sutra provides Block style as authoring sugar that compiles losslessly to List style. Both syntaxes are unified by the fundamental s-expression grammar that underlies all Verse language constructs.

### 3.1 The Fundamental S-Expression Grammar

At its core, every Verse construct—whether written in List or Block style—is an **s-expression**: a symbolic expression consisting of atoms and nested lists. This is the unifying grammatical foundation:

- **Atoms**: Numbers, strings, symbols, booleans, nil
- **Lists**: Ordered sequences of atoms and/or nested lists, denoted `(element1 element2 ...)`
- **Blocks**: Syntactic sugar for lists where newlines serve as expression boundaries

The key insight is that **newlines in Block style serve the same delimiting function as parentheses in List style**. Where List style uses explicit parentheses to group expressions, Block style uses newlines to separate sequential expressions and braces to group them.

### 3.2 Newline-Based Expression Parsing

In Block style, **newlines are the primary expression delimiters**:

```sutra
storylet "kitchen" {
  print "You enter the kitchen"
  set! player.location "kitchen"
  inc! player.steps
}
```

Each newline-terminated line is parsed as a separate s-expression. This transforms to:

```sutra
(storylet "kitchen"
  (do
    (print "You enter the kitchen")
    (set! player.location "kitchen")
    (inc! player.steps)))
```

This is why Block style doesn't require parentheses around each expression—the newlines provide the necessary syntactic boundaries.

### 3.3 Core Transformation Rules

#### Rule 1: Block-to-List Wrapping

Any construct `identifier { statements }` becomes `(identifier (do statements))` where `statements` are newline-separated expressions.

#### Rule 2: Newline Expression Separation

Newlines within blocks create separate s-expressions that are wrapped in an implicit `(do ...)` sequence.

#### Rule 3: Conditional Block Transformation

- `if condition { statements }` → `(if condition (do statements))`
- `if condition { then-statements } else { else-statements }` → `(if condition (do then-statements) (do else-statements))`
- `unless condition { statements }` → `(unless condition (do statements))`

#### Rule 4: Nested Block Recursion

Each `{ }` block recursively applies these rules, maintaining the s-expression tree structure.

#### Rule 5: Context-Sensitive Preservation

- Parenthesized pairs `(key value)` within `tags` and `state` contexts represent literal s-expressions and are preserved
- Exclamation suffixes `!` are part of the symbol atom itself
- Arrow syntax `->` exists only within `hub` constructs as a special operator

### 3.4 Specific Construct Transformations

The s-expression foundation means all constructs follow the same underlying pattern, regardless of their specific semantics:

#### 3.4.1 Storylet Constructs

```sutra
storylet "duel" {
  tag combat
  weight agent.rivalry
  print "Swords clash!"
}
```

→ `(storylet "duel"
     (do
       (tag combat)
       (weight agent.rivalry)
       (print "Swords clash!")))`

#### 3.4.2 Thread Constructs

```sutra
define exploration thread {
  start entrance
  state { (visited false) }
}
```

→ `(define exploration thread
     (do
       (start entrance)
       (state
         (do
           (visited false)))))`

#### 3.4.3 Conditional Constructs

```sutra
if player.hungry {
  print "Your stomach growls"
  dec! player.energy
}
```

→ `(if player.hungry
     (do
       (print "Your stomach growls")
       (dec! player.energy)))`

#### 3.4.4 Choice Constructs

```sutra
choices {
  "Enter tavern" { set! player.location "tavern" }
  "Continue walking" { inc! player.steps }
}
```

→ `(choices
     (do
       ("Enter tavern"
         (do
           (set! player.location "tavern")))
       ("Continue walking"
         (do
           (inc! player.steps)))))`

### 3.5 The Unifying S-Expression Principle

Every Verse construct, whether written in List or Block style, ultimately becomes an s-expression tree. This fundamental unity means:

1. **Semantic Equivalence**: Both styles produce identical s-expression trees and execute identically
2. **Lossless Transformation**: The newline-to-parentheses mapping preserves all semantic information
3. **Compositional Grammar**: Complex nested structures follow the same recursive s-expression rules
4. **Canonical Representation**: The List style s-expression is always the canonical form for evaluation

The power of this design is that authors can write in the more natural Block style while the engine operates on the mathematically precise s-expression representation—unified by the same underlying grammar.

---

## 5. Design Notes & Edge Cases

This section highlights specific behaviors and design choices in Sutra that might be non-obvious but are intentional. Understanding these can help in writing more robust and idiomatic Sutra code.

- **Identity Values for Arithmetic Atoms:** When called with no arguments, `+` returns its identity value `0`, and `*` returns its identity value `1`. This is a common convention in Lisp-family languages.

  - `(+)` => `0`
  - `(*)` => `1`

- **Unary Negation and Reciprocal:** The `-` and `/` atoms can be called with a single argument.

  - `(- x)` returns the negation of `x`.
  - `(/ x)` returns the reciprocal `1/x`.

- **Comparison Atom Arity:** Comparison atoms (`eq?`, `gt?`, `lt?`, `gte?`, `lte?`) require at least 2 arguments. Providing fewer arguments will raise an arity error.

  - `(gt? 5 3)` => `true`
  - `(gt? 5)` => error: gt? expects at least 2 arguments, got 1

- **`print` Arity:** The `print` atom strictly requires one argument. Providing a different number of arguments will raise an arity mismatch error. For multi-argument printing, use the `display` macro.

---

## 6. Assignment & State

| Macro     | Arity | Expansion Pattern                            | Impl.       | Status            |
| :-------- | :---- | :------------------------------------------- | :---------- | :---------------- |
| `set!`    | 2     | `(core/set! (path ...) value)`               | Macro, Atom | impl.             |
| `del!`    | 1     | `(core/del! (path ...))`                     | Macro, Atom | impl.             |
| `add!`    | 2     | `(core/set! (path ...) (+ (get ...) value))` | Macro, Atom | impl.             |
| `sub!`    | 2     | `(core/set! (path ...) (- (get ...) value))` | Macro, Atom | impl.             |
| `inc!`    | 1     | `(core/set! (path ...) (+ (get ...) 1))`     | Macro, Atom | impl.             |
| `dec!`    | 1     | `(core/set! (path ...) (- (get ...) 1))`     | Macro, Atom | impl.             |
| `mul!`    | 2     | `(core/set! (path ...) (* (get ...) value))` | Macro, Atom | impl.             |
| `div!`    | 2     | `(core/set! (path ...) (/ (get ...) value))` | Macro, Atom | impl.             |
| `push!`   | 1..   | `(core/push! (path ...) value ...)`          | Macro, Atom | impl. via core/\* |
| `pull!`   | 1..   | `(core/pull! (path ...) value ...)`          | Macro, Atom | impl. via core/\* |
| `get`     | 1     | `(core/get (path ...))`                      | Macro, Atom | impl.             |
| `exists?` | 1     | `(core/exists? (path ...))`                  | Macro, Atom | impl.             |

---

## 7. Predicates & Logic

| Macro     | Arity | Expands to     | Purpose                  | Impl. | Status | Macro Aliases     |
| :-------- | :---- | :------------- | :----------------------- | :---- | :----- | :---------------- |
| `eq?`     | 2..   | —              | Equality                 | Atom  | impl.  | `=`, `is?`        |
| `gt?`     | 2..   | —              | Greater than             | Atom  | impl.  | `>`, `over?`      |
| `lt?`     | 2..   | —              | Less than                | Atom  | impl.  | `<`, `under?`     |
| `gte?`    | 2..   | —              | Greater/equal            | Atom  | impl.  | `>=`, `at-least?` |
| `lte?`    | 2..   | —              | Less/equal               | Atom  | impl.  | `<=`, `at-most?`  |
| `not`     | 1     | —              | Negation                 | Atom  | impl.  | —                 |
| `has?`    | 2..   | —              | Membership in collection | Atom  | impl.  | —                 |
| `exists?` | 1     | `core/exists?` | Path/value existence     | Macro | impl.  | —                 |
| `and`     | 0..   | `(if ...)`     | Logical AND              | Macro | impl.  | —                 |
| `or`      | 0..   | `(if ...)`     | Logical OR               | Macro | impl.  | —                 |
| `empty?`  | 1     | `eq?` + `len`  | Collection is empty      | Macro | impl.  | —                 |
| `null?`   | 1     | `eq?` + `len`  | List is empty            | Macro | impl.  | —                 |

---

## 8. Math & Value Operations

| Atom   | Arity | Purpose                      | Impl. | Status |
| :----- | :---- | :--------------------------- | :---- | :----- |
| `+`    | 0..   | Addition                     | Atom  | impl.  |
| `-`    | 1..   | Subtraction/Negation         | Atom  | impl.  |
| `*`    | 0..   | Multiplication               | Atom  | impl.  |
| `/`    | 1..   | Division                     | Atom  | impl.  |
| `mod`  | 2     | Modulo (int)                 | Atom  | impl.  |
| `len`  | 1     | Length (list/string)         | Atom  | impl.  |
| `car`  | 1     | First element of list        | Atom  | impl.  |
| `cdr`  | 1     | Tail of list (all but first) | Atom  | impl.  |
| `cons` | 2     | Prepend element to list      | Atom  | impl.  |
| `min`  | 1..   | Minimum                      | Atom  | impl.  |
| `max`  | 1..   | Maximum                      | Atom  | impl.  |
| `abs`  | 1     | Absolute value               | Atom  | impl.  |

### List Operations: `car`, `cdr`, `cons`

- **`car`**

  - **Signature:** `(car <list>)`
  - **Purpose:** Returns the first element of a list.
  - **Arity:** 1
  - **Example:**
    ```sutra
    (car (list 1 2 3)) ; => 1
    (car (list "a" "b")) ; => "a"
    (car (list)) ; => error: car: empty list
    ```

- **`cdr`**

  - **Signature:** `(cdr <list>)`
  - **Purpose:** Returns the tail of a list (all but the first element).
  - **Arity:** 1
  - **Example:**
    ```sutra
    (cdr (list 1 2 3)) ; => (2 3)
    (cdr (list "a" "b")) ; => ("b")
    (cdr (list)) ; => error: cdr: empty list
    ```

- **`cons`**
  - **Signature:** `(cons <element> <list>)`
  - **Purpose:** Prepends an element to a list, returning a new list.
  - **Arity:** 2
  - **Example:**
    ```sutra
    (cons 1 (list 2 3)) ; => (1 2 3)
    (cons "a" (list "b" "c")) ; => ("a" "b" "c")
    (cons 1 2) ; => error: cons expects second argument to be a List
    ```

---

## 9. String Utilities

| Macro/Atom  | Arity | Signature                 | Purpose                       | Impl. | Status |
| :---------- | :---- | :------------------------ | :---------------------------- | :---- | :----- |
| `display`   | 0..   | `(display a b ...)`       | Print multiple values         | Macro | impl.  |
| `str`       | 1     | `(str x)`                 | Typecast to string            | Atom  | impl.  |
| `str+`      | 0..   | `(str+ a b ...)`          | Concatenate strings           | Atom  | impl.  |
| `core/str+` | 0..   | `(core/str+ a b ...)`     | Concatenate strings (core)    | Atom  | impl.  |
| `join-str+` | 2..   | `(join-str+ sep a b ...)` | Join strings with a separator | Macro | impl.  |

---

## 10. Additional Utility Macros

| Macro      | Arity | Purpose                          | Usage Example                  | Status |
| :--------- | :---- | :------------------------------- | :----------------------------- | :----- |
| `when`     | 2..   | Execute body when condition true | `(when cond ...body)`          | impl.  |
| `cond`     | 3..   | Multi-branch conditional         | `(cond ((test1) expr1) ...)`   | impl.  |
| `test`     | 3..   | Define test case                 | `(test "name" expect body...)` | impl.  |
| `expect`   | 0..   | Declare test expectations        | `(expect ...conditions)`       | impl.  |
| `cadr`     | 1     | Second element of list           | `(cadr (list 1 2 3))`          | impl.  |
| `null?`    | 1     | Check if list is empty           | `(null? (list))`               | impl.  |
| `append`   | 2     | Append two lists                 | `(append l1 l2)`               | impl.  |
| `map`      | 2     | Map function over list           | `(map f lst)`                  | impl.  |
| `for-each` | 3..   | Loop over collection             | `(for-each var coll ...body)`  | impl.  |

---

## 11. World Interaction

| Atom           | Arity | Purpose                         | Impl. |
| :------------- | :---- | :------------------------------ | :---- |
| `core/set!`    | 2     | Set value at path               | Atom  |
| `core/get`     | 1     | Get value at path               | Atom  |
| `core/del!`    | 1     | Delete value at path            | Atom  |
| `core/exists?` | 1     | Path existence                  | Atom  |
| `path`         | 1     | Create path from string         | Atom  |
| `core/map`     | 0..   | Create map from key-value pairs | Atom  |

---

## 12. I/O & Random

| Atom/Macro | Arity | Purpose                     | Impl. | Status  |
| :--------- | :---- | :-------------------------- | :---- | :------ |
| `print`    | 1     | Output single value         | Atom  | impl.   |
| `output`   | 1     | Output single value (alias) | Atom  | impl.   |
| `display`  | 0..   | Output multiple values      | Macro | impl.   |
| `rand`     | 0     | Random float                | Atom  | impl.   |
| `chance?`  | 1     | Macro: true with X% chance  | Macro | planned |

---

## 13. Special Forms

| Special Form | Arity | Purpose                             | Impl.       | Status |
| :----------- | :---- | :---------------------------------- | :---------- | :----- |
| `if`         | 3     | Conditional evaluation              | SpecialForm | impl.  |
| `lambda`     | 2..   | Anonymous function                  | SpecialForm | impl.  |
| `let`        | 2..   | Lexical bindings                    | SpecialForm | impl.  |
| `do`         | 0..   | Sequential evaluation               | SpecialForm | impl.  |
| `apply`      | 2..   | Function application with list args | SpecialForm | impl.  |
| `error`      | 1     | Raise error with message            | SpecialForm | impl.  |

---

## 14. Test Atoms (Debug/Test Only)

| Test Atom            | Arity | Purpose                    | Impl.       | Status |
| :------------------- | :---- | :------------------------- | :---------- | :----- |
| `register-test!`     | 4..   | Register test definition   | SpecialForm | impl.  |
| `value`              | 1     | Test expected value        | SpecialForm | impl.  |
| `tags`               | 0..   | Test tags                  | SpecialForm | impl.  |
| `test/echo`          | 1     | Echo value for testing     | SpecialForm | impl.  |
| `test/borrow_stress` | 2     | Stress test borrow checker | SpecialForm | impl.  |
| `assert`             | 1     | Assert condition is true   | SpecialForm | impl.  |
| `assert-eq`          | 2     | Assert two values equal    | SpecialForm | impl.  |

**Note:** Test atoms are only available when compiled with debug assertions or the `test-atom` feature.

---

## 15. Error Handling

- **Parse-time:** Invalid tokens, unmatched delimiters, bad escapes.
- **Validation:** Unknown macros/atoms, arity/type errors, invalid paths.
- **Runtime:** Type errors, division by zero, invalid world access.

---

## 16. Comments

- Start with `;` and continue to end of line.

---

## 17. Example Program

```sutra
; Factorial function
(define (fact n)
  (if (lte? n 1)
      1
      (* n (fact (- n 1)))))
(fact 5) ; => 120
```

---

## 18. Grammar Summary

- See `src/syntax/grammar.pest` for the full PEG grammar.
- All syntax is formally specified and enforced by the parser.

---

## 19. Macro System

- All author-facing macros are defined in `src/macros/macros.sutra` and registered at startup.
- Macro expansion is canonical and deterministic.
- Macro environment is built by `build_canonical_macro_env()`.

---

## 20. Validation & Evaluation

- All code is parsed, macroexpanded, validated, and then evaluated.
- Validation checks for unknown macros/atoms, arity, and type errors.
- Evaluation is recursive, with world state and output managed by the runtime.

---

## 21. Value Types

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

## 22. CLI & Tooling

- CLI supports: run, macroexpand, macrotrace, validate, format, test, list-macros, list-atoms, ast, gen-expected.
- Output is pretty-printed and colorized.

---

## 23. Extensibility

- New atoms/macros can be added via Rust or Sutra macro files.
- All macro and atom registration is centralized and canonical.

---

## 24. Not Yet impl. (Planned)

- Tier 2+ high-level macros (e.g., `requires`, `threshold`, `hub`, `select`).
- Map literals and advanced collection utilities.
- Advanced error handling and debugging macros.

---

## 25. References

- Macro library: `src/macros/macros.sutra`
- Grammar: `src/syntax/grammar.pest`
- Atoms: `src/atoms/`
- AST: `src/ast/`
- Runtime: `src/runtime/`

---

This document is fully synchronized with the codebase and spec as of **17 July 2025**. For any ambiguity or missing feature, consult the canonical spec and the relevant module in the codebase.
