# Sutra Thread System — Canonical Specification

## Overview

**Sutra threads** are modular, encapsulated, compositional narrative/game flows.  
A thread is a centralized, declarative controller that:

- Defines the flow (“steps”) of a linear, branching, or hybrid narrative or gameplay module.
    
- Declares all local state and tags (metadata) in a single place.
    
- Keeps step implementations, state, and side-effects strictly encapsulated—no “narrative spaghetti.”
    

---

## Thread Header / Controller

### **Canonical Syntax**

```sutra
define <name> thread {
  tags {
    (key1 value1)
    (key2 value2)
    ...
  }
  start <start-step>
  end <end-step>
  state {
    (variable1 initial-value)
    (variable2 initial-value)
    ...
  }
  step <step1> [!] { ... }
  step <step2> [!] { ... }
  ...
  step <end-step> { ... }
}
```

### **Conventions**

- **tags:**
    
    - Declares key/value metadata for the thread (e.g., theme, difficulty, salience).
        
    - Always uses parenthesized `(key value)` pairs.
        
    - Tags are passive (for editor/system use) unless explicitly acted upon.
        
- **start / end:**
    
    - Names of the entry and terminal steps (must correspond to declared step blocks).
        
- **state:**
    
    - Declares all local thread variables (with initial values).
        
    - Always present (use `state {}` or `state none` if empty).
        
- **step \[!\]:**
    
    - Each step must be declared.
        
    - `!` denotes steps that mutate local state (for author/readability/conventions).
        
    - Step header contains no logic, only structure.
        

---

## Step Implementations

### **Canonical Syntax**

```sutra
step <step-name> [!] {
  # Narrative output or logic
  print "Description..."
  # State mutation (local or global)
  set! <local-var> <value>
  add! player.hp -5
  # Choices (optional, if not defined solely by thread)
  choices {
    "Label" -> next-step
    if (<predicate>) { "Label" -> next-step }
    cond {
      (<predicate>) { "Label" -> next-step }
      ...
    }
  }
}
```

### **Conventions**

- **Narrative, logic, and side-effects** all live here.
    
- **Choices** may be declared either in the thread header (preferred for API clarity) or within steps (for simple or override cases).
    
- **All step logic is scoped;** only the thread’s `end` step should touch global state (unless intentionally designed).
    

---

## Choices and Conditional Choices

### **Canonical Forms**

**1. Unconditional:**

```sutra
choices {
  "To the kitchen" -> kitchen!
}
```

**2. Inline If:**

```sutra
choices {
  if (<predicate>) { "To the kitchen" -> kitchen! }
}
```

**3. Cond Block:**

```sutra
choices {
  cond {
    (<predicate1>) { "To the garden" -> garden! }
    (<predicate2>) { "To the bedroom" -> bedroom! }
  }
}
```

- **All forms are valid.** Prefer inline `if` and block `cond` for parser and author clarity.
    

---

## State and Scoping

- **Local state:**
    
    - Only accessible/mutable within the thread’s steps.
        
    - Declared in `state { ... }` block in the header.
        
- **Global state:**
    
    - Modified only in the thread’s `end` step, or through explicit “export” blocks if designed.
        
- **No leakage:**
    
    - Steps cannot change global state unless intentionally coded in `end`.
        

---

## Tags

- **Tags can be attached to threads, steps, or choices** as `(key value)` pairs.
    
- Used for editor tooling, search, analytics, or system-driven logic.
    
- Semantics are up to author-defined macros/systems.
    

---

## Canonical Example

```sutra
define tour thread {
  tags {
    (theme "exploration")
    (area "indoor")
    (threat "safe")
  }

  start entrance
  end exit

  state {
    (visited-kitchen false)
    (visited-bedroom false)
    (visited-garden false)
    (progress 0)
  }

  step entrance! {
    print "Welcome to the grand tour!"
    choices {
      "Proceed to the living room" -> living-room!
    }
  }

  step living-room! {
    print "You enter the spacious living room."
    choices {
      "To the exit" -> exit
      cond {
        (not visited-kitchen) { "To the kitchen" -> kitchen! }
        (not visited-bedroom) { "To the bedroom" -> bedroom! }
        (not visited-garden)  { "To the garden" -> garden! }
      }
    }
  }

  step kitchen! {
    print "The kitchen smells of fresh bread."
    set! visited-kitchen true
    choices {
      "To the exit" -> exit
      "Back to living room" -> living-room!
      if (not visited-bedroom) { "To the bedroom" -> bedroom! }
      if (not visited-garden)  { "To the garden" -> garden! }
    }
  }

  step bedroom! {
    print "The bedroom is tranquil and well-kept."
    set! visited-bedroom true
    choices {
      "To the exit" -> exit
      "Back to living room" -> living-room!
      if (not visited-kitchen) { "To the kitchen" -> kitchen! }
      if (not visited-garden)  { "To the garden" -> garden! }
    }
  }

  step garden! {
    print "You stroll into the lush garden."
    set! visited-garden true
    choices {
      "To the exit" -> exit
      "Back to living room" -> living-room!
      if (not visited-kitchen) { "To the kitchen" -> kitchen! }
      if (not visited-bedroom) { "To the bedroom" -> bedroom! }
    }
  }

  step exit {
    print "Tour complete! You visited:"
    if visited-kitchen { print "- The kitchen" }
    if visited-bedroom { print "- The bedroom" }
    if visited-garden  { print "- The garden" }
    print "Thanks for touring!"
    ; Export or mutate global state here if needed
  }
}
```

---

## Macro Expansion / Engine Implementation

- **Thread** becomes a closure/data module, with its own state, registry of steps, and pointer to current step.
    
- **At each step:**
    
    - Executes step logic (`do` block with prints, state changes).
        
    - Presents choices as defined in the header (with guard macros).
        
    - On user selection, advances thread-local step pointer.
        
- **No “narrative spaghetti”:**
    
    - All flow and structure are visible and editable from a single location.
        
    - Step logic is never entangled with flow control.
        
    - State is strictly local unless exported intentionally.
        

---

## Best Practices

- **Declare state, tags, and all step names in the thread header.**
    
- **Keep business logic in steps, not in the thread controller.**
    
- **Use guard macros (`if`, `cond`) for conditional choices, as these are parser-friendly and author-readable.**
    
- **Export to global state only at end, if possible.**
    
- **Keep tags simple and parenthesized for future-proofing.**
    

---
