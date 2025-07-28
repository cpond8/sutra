# Sutra Language Reference

---

## What is Sutra?

Sutra is a minimal, expression-based programming language designed around a simple but powerful idea: everything is built from small, composable pieces that work together predictably.

If you've used Lisp or Scheme, you'll recognize the core concepts. If you haven't, think of Sutra as a language where you write instructions like mathematical expressions—everything has a clear structure, and complex behaviors emerge from combining simple operations.

### Why would you use Sutra?

Sutra excels at scenarios where you need:

- **Flexible data manipulation** with lists, maps, and hierarchical structures
- **Scriptable logic** that can be easily modified or extended
- **Compositional design** where small pieces combine into larger systems
- **Reliable state management** with clear paths to data

The language is particularly well-suited for configuration systems, data processing pipelines, and scenarios where you need both human-readable syntax and programmatic power.

---

## How to Read This Reference

This document explains Sutra from the ground up. Each section builds on the previous ones, so if you're new to the language, read it in order. If you're looking for specific information, the sections are organized by topic with clear headings.

Examples throughout show both the code you write and what it produces. When something might be confusing, we'll explain why it works the way it does.

---

## Core Concepts

### Everything is an Expression

In Sutra, every piece of code is an _expression_ that produces a _value_. Even operations that primarily perform side effects (like printing to the screen) are expressions that return values:

```sutra
42                    ; => 42
(+ 1 2)               ; => 3
(if true "yes" "no")  ; => "yes"
(print "hello")       ; => nil (prints "hello" as side effect)
```

This means you can use the results of any operation anywhere you need a value, even functions that are primarily called for their side effects.

### Two Ways to Write the Same Thing

Sutra has two syntaxes that produce identical results:

**List style** (the canonical form):

```sutra
(+ 1 2 3)               ; Simple expression
(if (> x 0)             ; Conditional expression
    (print "positive")      ; Then branch
    (print "not positive")) ; Else branch
```

**Block style** (more readable for complex structures):

```sutra
+ 1 2 3                 ; Simple expression
if (> x 0) {            ; Conditional expression
  print "positive"      ; Then branch
}
else {
  print "not positive"  ; Else branch
}
```

Both styles compile to the same internal representation. List style is the "ground truth"—when in doubt, think in terms of parentheses and you'll understand how Sutra works.

---

## Value Types

Sutra works with six fundamental types of values. Everything you compute produces one of these:

| Type        | Example                    | What It's For                           |
| ----------- | -------------------------- | --------------------------------------- |
| **Number**  | `42`, `3.14`, `-7`         | All numeric computation (64-bit floats) |
| **String**  | `"hello"`, `"world\n"`     | Text and labels                         |
| **Boolean** | `true`, `false`            | Logic and conditions                    |
| **List**    | `(1 2 3)`, `("a" "b")`     | Ordered collections of any values       |
| **Map**     | `{name: "Alice", age: 30}` | Key-value data structures               |
| **Nil**     | `nil`                      | Absence of a value                      |

Additionally, there are two special types you'll encounter:

- **Path** - References to data locations (like `player.health`)
- **Lambda** - User-defined functions

### Lists: The Heart of Sutra

Lists in Sutra are built like chains—each element points to the next, ending with `nil`. This makes some operations very fast:

```sutra
(cons 1 (list 2 3))     ; => (1 2 3) - adding to front is instant
(car (list 1 2 3))      ; => 1 - getting first element is instant
(cdr (list 1 2 3))      ; => (2 3) - getting "rest" is instant
```

You don't need to think about the implementation details—just know that Sutra lists are optimized for building up data piece by piece and taking it apart from the beginning.

---

## Basic Syntax

### Atoms (Simple Values)

Simple values evaluate to themselves:

```sutra
42          ; => 42
"hello"     ; => "hello"
true        ; => true
nil         ; => nil
```

### Lists (Function Calls and Data)

When you put expressions in parentheses, the first element is treated as a function and the rest as arguments:

```sutra
(+ 1 2 3)           ; Call + with arguments 1, 2, 3
(print "hello")     ; Call print with argument "hello"
(list 1 2 3)        ; Call list to create (1 2 3)
```

To create a list of data (not a function call), use `list`:

```sutra
(list 1 2 3)        ; => (1 2 3)
(list "a" "b" "c")  ; => ("a" "b" "c")
```

### Symbols and Variables

Symbols are names that refer to values:

```sutra
x                   ; The value stored in x
player.health       ; The value at path player.health
+                   ; The addition function
```

### Comments

Comments start with `;` and go to the end of the line:

```sutra
(+ 1 2)  ; This adds 1 and 2
; This whole line is a comment
```

---

## How Symbol Resolution Works

When Sutra encounters a symbol like `x`, it looks for the value in this order:

1. **Local variables** (from `let` or function parameters)
2. **World state** (global data storage)
3. **Built-in functions** (like `+`, `print`, etc.)

If it finds a built-in function, you must call it—you can't just reference it:

```sutra
(let ((x 42)) x)    ; => 42 (local variable)
player.health       ; => value from world state
(+ 1 2)             ; => 3 (function call)
+                   ; => Error: + must be called with arguments
```

This means local variables and world state can "shadow" built-in functions, which is usually what you want.

---

## Special Forms

Special forms are the building blocks of control flow and function definition. Unlike regular functions, they control when and how their arguments are evaluated.

### Conditionals

**`if`** evaluates one of two branches based on a condition:

```sutra
(if condition then-expression else-expression)

(if (> x 0) "positive" "not positive")
(if player.alive (continue-game) (game-over))
```

All values except `false`, `nil`, `0`, and empty strings/collections are considered "true" in conditions.

**`cond`** handles multiple conditions:

```sutra
(cond
  ((> x 0) "positive")
  ((< x 0) "negative")
  (else "zero"))
```

### Functions

**`lambda`** creates anonymous functions:

```sutra
(lambda (x y) (+ x y))                ; Function that adds two numbers
((lambda (x) (* x x)) 5)              ; => 25

(define square (lambda (x) (* x x)))  ; Store function in variable
(square 4)                            ; => 16
```

**`define`** creates named functions or variables:

```sutra
; Variable definition
(define pi 3.14159)

; Function definition
(define (add x y) (+ x y))
(define (greet name) (print "Hello, " name))
```

### Local Variables

**`let`** creates local variables:

```sutra
(let ((x 10)
      (y 20))
  (+ x y))          ; => 30

; Variables can use previous bindings
(let ((x 5)
      (y (* x 2)))
  y)                ; => 10
```

### Grouping

**`do`** evaluates multiple expressions and returns the last result:

```sutra
(do
  (print "Starting calculation")
  (define result (+ 1 2 3))
  (print "Done")
  result)           ; => 6
```

### Logic

**`and`** returns the first false value or the last value:

```sutra
(and true 42 "hello")      ; => "hello"
(and true false 42)        ; => false
```

**`or`** returns the first true value or the last value:

```sutra
(or false nil 42)          ; => 42
(or false nil)             ; => nil
```

---

## Arithmetic Operations

All arithmetic operations work with numbers and return numbers:

| Operation | Usage                    | Notes                                 |
| --------- | ------------------------ | ------------------------------------- |
| `+`       | `(+ a b ...)`            | Requires at least 2 arguments         |
| `-`       | `(- a b ...)` or `(- a)` | Single argument gives negation        |
| `*`       | `(* a b ...)`            | Requires at least 2 arguments         |
| `/`       | `(/ a b ...)` or `(/ a)` | Single argument gives reciprocal      |
| `mod`     | `(mod a b)`              | Modulo operation, exactly 2 arguments |
| `abs`     | `(abs n)`                | Absolute value                        |
| `min`     | `(min a b ...)`          | Minimum of all arguments              |
| `max`     | `(max a b ...)`          | Maximum of all arguments              |

Examples:

```sutra
(+ 1 2 3 4)         ; => 10
(- 10 3)            ; => 7
(- 5)               ; => -5
(* 2 3 4)           ; => 24
(/ 12 3)            ; => 4
(/ 2)               ; => 0.5
(mod 10 3)          ; => 1
(abs -7)            ; => 7
(min 3 1 4 1 5)     ; => 1
(max 3 1 4 1 5)     ; => 5
```

---

## Comparison and Logic

### Comparison Operations

All comparison operations require at least 2 arguments and can chain multiple comparisons:

| Operation | Aliases           | Usage                               |
| --------- | ----------------- | ----------------------------------- |
| `eq?`     | `=`, `is?`        | `(eq? a b ...)` - equality          |
| `gt?`     | `>`, `over?`      | `(gt? a b ...)` - greater than      |
| `lt?`     | `<`, `under?`     | `(lt? a b ...)` - less than         |
| `gte?`    | `>=`, `at-least?` | `(gte? a b ...)` - greater or equal |
| `lte?`    | `<=`, `at-most?`  | `(lte? a b ...)` - less or equal    |

Examples:

```sutra
(eq? 1 1)           ; => true
(= 1 1 1)           ; => true (all equal)
(> 5 3 1)           ; => true (5 > 3 > 1)
(< 1 2 3)           ; => true (1 < 2 < 3)
(>= 5 5 3)          ; => true (5 >= 5 >= 3)
```

### Logic Operations

| Operation | Usage           | Behavior          |
| --------- | --------------- | ----------------- |
| `not`     | `(not value)`   | Logical negation  |
| `and`     | `(and a b ...)` | Short-circuit AND |
| `or`      | `(or a b ...)`  | Short-circuit OR  |

```sutra
(not true)          ; => false
(not false)         ; => true
(and true true)     ; => true
(and true false)    ; => false
(or false true)     ; => true
(or false false)    ; => false
```

---

## Working with Lists

Lists are one of Sutra's most important data types. Here's how to create and manipulate them:

### Creating Lists

```sutra
(list)              ; => nil (empty list)
(list 1)            ; => (1)
(list 1 2 3)        ; => (1 2 3)
(list "a" 42 true)  ; => ("a" 42 true)
```

### Basic List Operations

| Operation | Usage              | Purpose                  |
| --------- | ------------------ | ------------------------ |
| `car`     | `(car list)`       | Get first element        |
| `cdr`     | `(cdr list)`       | Get rest of list         |
| `cons`    | `(cons item list)` | Add item to front        |
| `len`     | `(len list)`       | Get length               |
| `null?`   | `(null? list)`     | Check if empty           |
| `has?`    | `(has? list item)` | Check if item is in list |

Examples:

```sutra
(car (list 1 2 3))      ; => 1
(cdr (list 1 2 3))      ; => (2 3)
(cons 0 (list 1 2 3))   ; => (0 1 2 3)
(len (list 1 2 3))      ; => 3
(null? (list))          ; => true
(null? (list 1))        ; => false
(has? (list 1 2 3) 2)   ; => true
```

### Higher-Order List Operations

```sutra
(append (list 1 2) (list 3 4))  ; => (1 2 3 4)
(map (lambda (x) (* x 2))
     (list 1 2 3))              ; => (2 4 6)
```

---

## World State and Paths

Sutra provides a global state system for persistent data storage. You access this data using _paths_—dot-separated names that navigate nested structures.

### Basic State Operations

| Operation | Usage               | Purpose                 |
| --------- | ------------------- | ----------------------- |
| `get`     | `(get path)`        | Retrieve value          |
| `set!`    | `(set! path value)` | Store value             |
| `del!`    | `(del! path)`       | Delete value            |
| `exists?` | `(exists? path)`    | Check if path has value |

### Path Syntax

Paths use dot notation to navigate nested data:

```sutra
player.health           ; Direct path reference
player.inventory.gold   ; Nested path
world.current-room      ; Kebab-case works too
```

You can also create paths programmatically:

```sutra
(path "player" "health")        ; Same as player.health
(path "items" item-name)        ; Dynamic path creation
```

### State Examples

```sutra
; Set some initial values
(set! player.name "Alice")
(set! player.health 100)
(set! player.inventory.gold 50)

; Read values back
(get player.name)               ; => "Alice"
(get player.health)             ; => 100

; Check existence
(exists? player.health)         ; => true
(exists? player.mana)           ; => false

; Modify values
(set! player.health 90)
(del! player.inventory.gold)
```

### Mathematical State Operations

For numeric values, there are convenient operations:

| Operation | Usage                | Equivalent To                       |
| --------- | -------------------- | ----------------------------------- |
| `add!`    | `(add! path amount)` | `(set! path (+ (get path) amount))` |
| `sub!`    | `(sub! path amount)` | `(set! path (- (get path) amount))` |
| `inc!`    | `(inc! path)`        | `(add! path 1)`                     |
| `dec!`    | `(dec! path)`        | `(sub! path 1)`                     |

```sutra
(set! score 100)
(add! score 50)         ; score is now 150
(sub! score 25)         ; score is now 125
(inc! score)            ; score is now 126
(dec! score)            ; score is now 125
```

---

## String Operations

| Operation | Usage                        | Purpose                     |
| --------- | ---------------------------- | --------------------------- |
| `str`     | `(str value)`                | Convert any value to string |
| `str+`    | `(str+ string1 string2 ...)` | Concatenate strings         |

```sutra
(str 42)                        ; => "42"
(str true)                      ; => "true"
(str+ "Hello, " "world!")       ; => "Hello, world!"
(str+ "Score: " (str score))    ; => "Score: 100"
```

---

## Input and Output

| Operation | Usage            | Purpose                        |
| --------- | ---------------- | ------------------------------ |
| `print`   | `(print value)`  | Output a single value          |
| `output`  | `(output value)` | Alias for print                |
| `rand`    | `(rand)`         | Get random number (0.0 to 1.0) |

```sutra
(print "Hello, world!")
(print (+ 1 2 3))
(output player.name)

(rand)                  ; => 0.7834291 (example)
```

---

## Error Handling

Sutra has a structured error system that groups related failures into clear categories. When something goes wrong, you'll see specific error messages that help you understand exactly what happened.

### Error Categories

**Parse Errors** - Problems with syntax and structure:

- Invalid syntax, mismatched parentheses, malformed constructs
- Empty expressions where content is required
- Invalid literals (like malformed numbers or strings)

**Runtime Errors** - Problems during execution:

- Undefined symbols (referencing variables that don't exist)
- Type mismatches (like trying to add string + number)
- Wrong number of arguments to functions
- Invalid operations (like division by zero)
- Recursion limit exceeded

**Validation Errors** - Problems with program structure:

- Invalid macro definitions or usage
- Invalid path references
- Duplicate definitions of the same symbol

**Test Errors** - Problems in test assertions:

- Test assertion failures

### Example Errors

```sutra
(+ 1 "two")         ; Runtime error: Type mismatch, expected Number, got String
(/ 10 0)            ; Runtime error: Invalid operation 'division' on zero
(+ 1)               ; Runtime error: Arity mismatch, expected at least 2, got 1
unknown-var         ; Runtime error: Undefined symbol 'unknown-var'
()                  ; Parse error: Empty expression
```

### Creating Your Own Errors

The `error` function lets you signal problems in your code:

```sutra
(if (< health 0)
    (error "Player health cannot be negative")
    health)
```

When an error occurs, Sutra shows you exactly where the problem is in your code, what went wrong, and often suggests how to fix it.

---

## Functions and Closures

### Creating Functions

Functions in Sutra capture their environment, creating _closures_:

```sutra
(define (make-counter start)
  (lambda ()
    (do
      (set! start (+ start 1))
      start)))

(define counter (make-counter 10))
(counter)               ; => 11
(counter)               ; => 12
(counter)               ; => 13
```

### Variadic Functions

Functions can accept variable numbers of arguments using `...`:

```sutra
(define (sum ...numbers)
  (if (null? numbers)
      0
      (+ (car numbers) (apply sum (cdr numbers)))))

(sum 1 2 3 4)          ; => 10
```

### Function Application

`apply` calls a function with arguments from a list:

```sutra
(apply + (list 1 2 3 4))        ; => 10
(apply max (list 5 2 8 1))      ; => 8
```

---

## Advanced Features

### Loops

`for-each` iterates over lists:

```sutra
(for-each item (list "apple" "banana" "orange")
  (print item))
; Prints:
; apple
; banana
; orange
```

### Quoting

Use `'` to prevent evaluation:

```sutra
'(+ 1 2)                ; => (+ 1 2) (not evaluated)
(list '+ 1 2)           ; => (+ 1 2)
```

### Advanced List Utilities

```sutra
; Get second element
(define cadr (lambda (lst) (car (cdr lst))))
(cadr (list 1 2 3))     ; => 2

; Check if two lists are equal
(eq? (list 1 2) (list 1 2))     ; => true
```

---

## Testing

Sutra includes a built-in testing framework (available in debug builds):

```sutra
(test "addition works"
  (expect (value 5))
  (+ 2 3))

(test "string concatenation"
  (expect (value "hello world"))
  (str+ "hello" " " "world"))
```

Test assertions:

```sutra
(assert (> 5 3))        ; Passes silently
(assert (< 5 3))        ; Fails with error
(assert-eq 5 (+ 2 3))   ; Checks equality
```

---

## Complete Example

Here's a small program that demonstrates many of Sutra's features:

```sutra
; Define a player with initial state
(set! player.name "Alice")
(set! player.health 100)
(set! player.inventory (list "sword" "potion"))

; Define a function to take damage
(define (take-damage amount)
  (do
    (sub! player.health amount)
    (if (< (get player.health) 0)
        (set! player.health 0))
    (print (str+ (get player.name) " has " (str (get player.health)) " health"))))

; Define a function to use item
(define (use-item item-name)
  (if (has? (get player.inventory) item-name)
      (cond
        ((eq? item-name "potion")
         (do
           (add! player.health 20)
           (print "Used potion, gained 20 health")))
        (else
         (print (str+ "Don't know how to use " item-name))))
      (print (str+ "Don't have " item-name))))

; Simulate some gameplay
(take-damage 30)        ; Alice has 70 health
(use-item "potion")     ; Used potion, gained 20 health
(take-damage 50)        ; Alice has 40 health
```

---

## Summary

Sutra gives you:

- **Simple, consistent syntax** with two equivalent forms
- **Powerful data structures** for hierarchical information
- **Flexible state management** with paths and automatic operations
- **Functional programming** with closures and higher-order functions
- **Clear error reporting** when things go wrong

The key insight is that complex behaviors emerge from combining simple, predictable operations. Start with the basics—arithmetic, lists, and state operations—then build up more sophisticated logic as you become comfortable with the language's patterns.

When in doubt, remember that everything is an expression that produces a value, and parentheses group things together. The rest follows naturally from there.
