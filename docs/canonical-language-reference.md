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

| Type    | Example         | Description                                  | Implementation  |
| :------ | :-------------- | :------------------------------------------- | :-------------- |
| Number  | `42`, `-3.14`   | 64-bit float                                 | `Value::Number` |
| Boolean | `true`, `false` | Boolean                                      | `Value::Bool`   |
| String  | `"foo\nbar"`    | String, escapes: `\\n`, `\\t`, `\\"`, `\\\\` | `Value::String` |
| Symbol  | `foo`, `+`      | Variable/function names                      | `Value::Symbol` |
| Nil     | `nil`           | Absence of value                             | `Value::Nil`    |

### 2.2 Collections

| Type | Example            | Description            | Implementation |
| :--- | :----------------- | :--------------------- | :------------- |
| List | `(1 2 "a" true)`   | Ordered, heterogeneous | `Value::List`  |
| Map  | `{foo: 1, bar: 2}` | Key-value, string keys | `Value::Map`   |

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

| Construct  | Arity | Purpose                  | Example                                           |
| :--------- | :---- | :----------------------- | :------------------------------------------------ |
| `if`       | 3     | Conditional evaluation   | `(if true "then" "else")`                         |
| `cond`     | 3..   | Multi-branch conditional | `(cond ((gt? x 0) "positive") (else "negative"))` |
| `do`       | 0..   | Sequential evaluation    | `(do (set! x 1) (set! y 2) (+ x y))`              |
| `when`     | 2..   | Conditional with do      | `(when (gt? x 0) (print "positive"))`             |
| `let`      | 2..   | Lexical bindings         | `(let ((x 1) (y 2)) (+ x y))`                     |
| `lambda`   | 2..   | Anonymous function       | `(lambda (x y) (+ x y))`                          |
| `for-each` | 3..   | Loop over collection     | `(for-each x (list 1 2 3) (print x))`             |

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

While List style is the canonical ground truth syntax, Sutra provides Block style as authoring sugar that compiles losslessly to List style. Both syntaxes are unified by the fundamental s-expression grammar that underlies all Sutra language constructs.

### 3.1 The Fundamental S-Expression Grammar

At its core, every Sutra construct—whether written in List or Block style—is an **s-expression**: a symbolic expression consisting of atoms and nested lists. This is the unifying grammatical foundation:

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

**Simple if statement:**

```sutra
if condition { statements }
```

↓ transforms to ↓

```sutra
(if condition (do statements))
```

**If-else statement:**

```sutra
if condition { then-statements } else { else-statements }
```

↓ transforms to ↓

```sutra
(if condition (do then-statements) (do else-statements))
```

**Unless statement:**

```sutra
unless condition { statements }
```

↓ transforms to ↓

```sutra
(unless condition (do statements))
```

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

Every Sutra construct, whether written in List or Block style, ultimately becomes an s-expression tree. This fundamental unity means:

1. **Semantic Equivalence**: Both styles produce identical s-expression trees and execute identically
2. **Lossless Transformation**: The newline-to-parentheses mapping preserves all semantic information
3. **Compositional Grammar**: Complex nested structures follow the same recursive s-expression rules
4. **Canonical Representation**: The List style s-expression is always the canonical form for evaluation

The power of this design is that authors can write in the more natural Block style while the engine operates on the mathematically precise s-expression representation—unified by the same underlying grammar.

---

## 5. Design Notes & Edge Cases

This section highlights specific behaviors and design choices in Sutra that might be non-obvious but are intentional. Understanding these can help in writing more robust and idiomatic Sutra code.

- **Unary Negation and Reciprocal:** The `-` and `/` atoms can be called with a single argument.

  - `(- x)` returns the negation of `x`.
  - `(/ x)` returns the reciprocal `1/x`.

- **Comparison Atom Arity:** Comparison atoms (`eq?`, `gt?`, `lt?`, `gte?`, `lte?`) require at least 2 arguments. Providing fewer arguments will raise an arity error.

  - `(gt? 5 3)` => `true`
  - `(gt? 5)` => error: gt? expects at least 2 arguments, got 1

- **`print` Arity:** The `print` atom strictly requires one argument. Providing a different number of arguments will raise an arity mismatch error. For multi-argument printing, use the `display` macro.

---

## 6. Assignment & State

| Macro     | Arity | Purpose                     | Example                    |
| :-------- | :---- | :-------------------------- | :------------------------- |
| `set!`    | 2     | Set value at path           | `(set! player.health 100)` |
| `del!`    | 1     | Delete value at path        | `(del! temp.value)`        |
| `add!`    | 2     | Add to value at path        | `(add! player.score 10)`   |
| `sub!`    | 2     | Subtract from value at path | `(sub! player.mana 5)`     |
| `inc!`    | 1     | Increment value at path     | `(inc! counter)`           |
| `dec!`    | 1     | Decrement value at path     | `(dec! lives)`             |
| `mul!`    | 2     | Multiply value at path      | `(mul! player.damage 2)`   |
| `div!`    | 2     | Divide value at path        | `(div! player.health 2)`   |
| `push!`   | 1..   | Push to list at path        | `(push! items "sword")`    |
| `pull!`   | 1..   | Pull from list at path      | `(pull! items "sword")`    |
| `get`     | 1     | Get value at path           | `(get player.health)`      |
| `exists?` | 1     | Check path existence        | `(exists? player.health)`  |

---

## 7. Predicates & Logic

| Predicate | Arity | Purpose                  | Example                   |
| :-------- | :---- | :----------------------- | :------------------------ |
| `eq?`     | 2..   | Equality                 | `(eq? 1 1)`               |
| `gt?`     | 2..   | Greater than             | `(gt? 5 3)`               |
| `lt?`     | 2..   | Less than                | `(lt? 3 5)`               |
| `gte?`    | 2..   | Greater/equal            | `(gte? 5 5)`              |
| `lte?`    | 2..   | Less/equal               | `(lte? 3 5)`              |
| `not`     | 1     | Negation                 | `(not false)`             |
| `has?`    | 2..   | Membership in collection | `(has? (list 1 2 3) 2)`   |
| `exists?` | 1     | Path/value existence     | `(exists? player.health)` |
| `and`     | 0..   | Logical AND              | `(and true false)`        |
| `or`      | 0..   | Logical OR               | `(or true false)`         |
| `empty?`  | 1     | Collection is empty      | `(empty? (list))`         |
| `null?`   | 1     | List is empty            | `(null? (list))`          |

### Comparison Aliases

| Alias       | Maps to | Example                 |
| :---------- | :------ | :---------------------- |
| `=`         | `eq?`   | `(= 1 1)`               |
| `is?`       | `eq?`   | `(is? "hello" "hello")` |
| `>`         | `gt?`   | `(> 2 1)`               |
| `over?`     | `gt?`   | `(over? 10 0)`          |
| `<`         | `lt?`   | `(< 1 2)`               |
| `under?`    | `lt?`   | `(under? 0 10)`         |
| `>=`        | `gte?`  | `(>= 2 2)`              |
| `at-least?` | `gte?`  | `(at-least? 10 10)`     |
| `<=`        | `lte?`  | `(<= 2 2)`              |
| `at-most?`  | `lte?`  | `(at-most? 10 10)`      |

---

## 8. Math & Value Operations

| Operation | Arity | Purpose                      | Example               |
| :-------- | :---- | :--------------------------- | :-------------------- |
| `+`       | 2..   | Addition                     | `(+ 1 2 3)`           |
| `-`       | 1..   | Subtraction/Negation         | `(- 10 5)`            |
| `*`       | 2..   | Multiplication               | `(* 2 3 4)`           |
| `/`       | 1..   | Division                     | `(/ 10 4)`            |
| `mod`     | 2     | Modulo (int)                 | `(mod 10 3)`          |
| `len`     | 1     | Length (list/string)         | `(len (list 1 2 3))`  |
| `car`     | 1     | First element of list        | `(car (list 1 2 3))`  |
| `cdr`     | 1     | Tail of list (all but first) | `(cdr (list 1 2 3))`  |
| `cons`    | 2     | Prepend element to list      | `(cons 1 (list 2 3))` |
| `min`     | 1..   | Minimum                      | `(min 1 2 3)`         |
| `max`     | 1..   | Maximum                      | `(max 1 2 3)`         |
| `abs`     | 1     | Absolute value               | `(abs -5)`            |

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

| Utility     | Arity | Purpose                     | Example                       |
| :---------- | :---- | :-------------------------- | :---------------------------- |
| `display`   | 0..   | Print multiple values       | `(display "hello" 123 true)`  |
| `str`       | 1     | Typecast to string          | `(str 42)`                    |
| `str+`      | 0..   | Concatenate strings         | `(str+ "hello" " " "world")`  |
| `join-str+` | 2..   | Join strings with separator | `(join-str+ " " "a" "b" "c")` |

---

## 10. Additional Utility Macros

| Macro      | Arity | Purpose                          | Example                                           |
| :--------- | :---- | :------------------------------- | :------------------------------------------------ |
| `when`     | 2..   | Execute body when condition true | `(when (gt? x 0) (print "positive"))`             |
| `cond`     | 3..   | Multi-branch conditional         | `(cond ((gt? x 0) "positive") (else "negative"))` |
| `test`     | 3..   | Define test case                 | `(test "name" (expect (value 5)) (+ 2 3))`        |
| `expect`   | 0..   | Declare test expectations        | `(expect (value 5) (tags "math"))`                |
| `cadr`     | 1     | Second element of list           | `(cadr (list 1 2 3))`                             |
| `null?`    | 1     | Check if list is empty           | `(null? (list))`                                  |
| `append`   | 2     | Append two lists                 | `(append (list 1 2) (list 3 4))`                  |
| `map`      | 2     | Map function over list           | `(map (lambda (x) (* x 2)) (list 1 2 3))`         |
| `for-each` | 3..   | Loop over collection             | `(for-each x (list 1 2 3) (print x))`             |

---

## 11. World Interaction

| Macro     | Arity | Purpose                     | Example                    |
| :-------- | :---- | :-------------------------- | :------------------------- |
| `set!`    | 2     | Set value at path           | `(set! player.health 100)` |
| `get`     | 1     | Get value at path           | `(get player.health)`      |
| `del!`    | 1     | Delete value at path        | `(del! temp.value)`        |
| `exists?` | 1     | Path existence              | `(exists? player.health)`  |
| `path`    | 1     | Create path from string     | `(path "player" "health")` |
| `add!`    | 2     | Add to value at path        | `(add! player.score 10)`   |
| `sub!`    | 2     | Subtract from value at path | `(sub! player.mana 5)`     |
| `inc!`    | 1     | Increment value at path     | `(inc! counter)`           |
| `dec!`    | 1     | Decrement value at path     | `(dec! lives)`             |

### Internal Atom Mappings

The following table shows which internal `core/` atoms each macro expands to:

| Macro     | Expands to                                      | Internal Atom           |
| :-------- | :---------------------------------------------- | :---------------------- |
| `set!`    | `(core/set! (path ...) ...)`                    | `core/set!`             |
| `get`     | `(core/get (path ...))`                         | `core/get`              |
| `del!`    | `(core/del! (path ...))`                        | `core/del!`             |
| `exists?` | `(core/exists? (path ...))`                     | `core/exists?`          |
| `add!`    | `(core/set! (path ...) (+ (core/get ...) ...))` | `core/set!`, `core/get` |
| `sub!`    | `(core/set! (path ...) (- (core/get ...) ...))` | `core/set!`, `core/get` |
| `inc!`    | `(core/set! (path ...) (+ (core/get ...) 1))`   | `core/set!`, `core/get` |
| `dec!`    | `(core/set! (path ...) (- (core/get ...) 1))`   | `core/set!`, `core/get` |

**Note:** The `core/` prefixed atoms are internal implementation details used by the macro system and should not be used directly in user code.

---

## 12. I/O & Random

| Operation | Arity | Purpose                     | Example                      |
| :-------- | :---- | :-------------------------- | :--------------------------- |
| `print`   | 1     | Output single value         | `(print "hello")`            |
| `output`  | 1     | Output single value (alias) | `(output "hello")`           |
| `display` | 0..   | Output multiple values      | `(display "hello" 123 true)` |
| `rand`    | 0     | Random float                | `(rand)`                     |

---

## 13. Special Forms

| Special Form | Arity | Purpose                             | Example                              |
| :----------- | :---- | :---------------------------------- | :----------------------------------- |
| `if`         | 3     | Conditional evaluation              | `(if true "then" "else")`            |
| `lambda`     | 2..   | Anonymous function                  | `(lambda (x y) (+ x y))`             |
| `let`        | 2..   | Lexical bindings                    | `(let ((x 1) (y 2)) (+ x y))`        |
| `do`         | 0..   | Sequential evaluation               | `(do (set! x 1) (set! y 2) (+ x y))` |
| `apply`      | 2..   | Function application with list args | `(apply + (list 1 2 3))`             |
| `error`      | 1     | Raise error with message            | `(error "Something went wrong")`     |

---

## 14. Test Atoms (Debug/Test Only)

Test atoms provide a comprehensive testing framework for Sutra code. These atoms are only available when compiled with debug assertions or the `test-atom` feature.

### Core Test Framework

| Test Atom        | Arity | Purpose                   | Usage                                        |
| :--------------- | :---- | :------------------------ | :------------------------------------------- |
| `register-test!` | 4..   | Register test definition  | `(register-test! name expect body metadata)` |
| `test`           | 3..   | Define test case          | `(test "name" expect body...)`               |
| `expect`         | 0..   | Declare test expectations | `(expect ...conditions)`                     |

### Test Assertions

| Test Atom   | Arity | Purpose                  | Usage                         |
| :---------- | :---- | :----------------------- | :---------------------------- |
| `value`     | 1     | Test expected value      | `(value expected-result)`     |
| `tags`      | 0..   | Test tags                | `(tags "tag1" "tag2" ...)`    |
| `assert`    | 1     | Assert condition is true | `(assert condition)`          |
| `assert-eq` | 2     | Assert two values equal  | `(assert-eq expected actual)` |

### Internal Test Atom Mappings

The following table shows which internal test atoms are available in debug/test builds:

| Public Atom | Internal Atom        | Purpose                    |
| :---------- | :------------------- | :------------------------- |
| `value`     | `value`              | Test expected value        |
| `tags`      | `tags`               | Test tags                  |
| `assert`    | `assert`             | Basic assertion            |
| `assert-eq` | `assert-eq`          | Equality assertion         |
| -           | `test/echo`          | Echo value for debugging   |
| -           | `test/borrow_stress` | Stress test borrow checker |

**Note:** The `test/` prefixed atoms (`test/echo`, `test/borrow_stress`) are internal debugging utilities only available in debug/test builds and should not be used in production code.

### Test Structure

A complete test follows this pattern:

```sutra
(test "test name"
  (expect
    (value expected-result)
    (tags "tag1" "tag2"))
  (do
    ;; test body
    (define (add x y) (+ x y))
    (add 2 3)))
```

### Value Assertions

The `value` atom expects a specific return value:

```sutra
(test "addition"
  (expect (value 5))
  (+ 2 3))
```

### Error Assertions

Tests can expect specific error types:

```sutra
(test "division by zero"
  (expect (error DivisionByZero))
  (/ 10 0))

(test "type error"
  (expect (error TypeError))
  (+ 1 "two"))

(test "arity error"
  (expect (error Eval))
  (+ 1))
```

### Tag System

Tests can be tagged for organization:

```sutra
(test "string concatenation"
  (expect
    (value "hello world")
    (tags "string" "concatenation"))
  (str+ "hello" " " "world"))
```

### Assertion Atoms

Direct assertions for testing:

```sutra
(assert true)                   ; => nil (success)
(assert (eq? 1 1))              ; => nil (success)
(assert false)                  ; => TestFailure

(assert-eq 1 1)                 ; => nil (success)
(assert-eq "a" "a")             ; => nil (success)
(assert-eq 1 2)                 ; => TestFailure
```

### Test Utilities

Echo for debugging:

```sutra
(test/echo "hello")             ; => "hello" (also emits "hello")
```

Borrow checker stress testing:

```sutra
(test/borrow_stress 2 "test")   ; => "depth:2;msg:test"
```

**Note:** Test atoms are only available when compiled with debug assertions or the `test-atom` feature.

---

## 15. Error Handling

Sutra uses a structured error system with specific error types for different failure modes:

### Error Types

| Error Type       | Description                                       | Usage                  |
| :--------------- | :------------------------------------------------ | :--------------------- |
| `Parse`          | Invalid syntax, unmatched delimiters, bad escapes | Parse-time errors      |
| `Validation`     | Unknown macros/atoms, arity errors, invalid paths | Validation-time errors |
| `Eval`           | Runtime evaluation errors, arity mismatches       | Runtime errors         |
| `TypeError`      | Type mismatches (e.g., string + number)           | Runtime errors         |
| `DivisionByZero` | Division by zero operations                       | Runtime errors         |
| `Internal`       | Internal engine errors                            | System errors          |
| `TestFailure`    | Test assertion failures                           | Test-time errors       |

### Error Context

All errors include context information:

- **Source**: The source code that caused the error
- **Span**: The exact location in the source code
- **Help**: Optional help message
- **Related**: Additional labeled spans for multi-label diagnostics

### Error Examples

```sutra
(+ 1 "two")      ; => TypeError: Type error
(/ 10 0)         ; => DivisionByZero: division by zero
(undefined-func) ; => Eval: undefined symbol: 'undefined-func'
(+ 1)            ; => Eval: Arity error
```

### Error Testing

Test atoms can expect specific error types:

```sutra
(test "division by zero"
  (expect (error DivisionByZero))
  (/ 10 0))

(test "type error"
  (expect (error TypeError))
  (+ 1 "two"))
```

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

- All author-facing macros are defined in `src/macros/std_macros.sutra` and registered at startup.
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

### Non-Implemented Constructs

The following constructs are **not implemented** in the current engine:

#### Error Testing Constructs

- `try` - No try/catch mechanism exists
- `assert-error` - No direct error assertion atom

#### Error Type Names

- `ArityError` - Use `Eval` error type instead
- `ParseError` - Use `Parse` error type instead

#### Test Constructs

- `assert-error` - Use `(expect (error ErrorType))` pattern instead

#### Internal Constructs (Not Public API)

- `core/*` atoms - Internal implementation details used by macros
- `test/*` atoms - Internal debugging utilities only available in debug/test builds

#### Examples of Correct Usage

Instead of:

```sutra
(try (car (list)))  ; Not implemented
(assert-error ...)  ; Not implemented
(core/set! foo bar) ; Internal API
```

Use:

```sutra
(test "car empty list"
  (expect (error Eval))
  (car (list)))     ; Correct pattern

(set! foo bar)      ; Public API
```

---

## 25. References

- Macro library: `src/macros/std_macros.sutra`
- Grammar: `src/syntax/grammar.pest`
- Atoms: `src/atoms/`
- AST: `src/ast/`
- Runtime: `src/runtime/`

---

This document is fully synchronized with the codebase and spec as of **19 July 2025**. For any ambiguity or missing feature, consult the canonical spec and the relevant module in the codebase.
