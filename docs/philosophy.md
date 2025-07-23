---
status: authoritative
last-reviewed: 2025-07-21
summary: Design philosophy and guiding principles for the Sutra Engine.
---

# Sutra Engine Design Philosophy

> **Living Document**
> This document captures the foundational design philosophy and principles that guide every aspect of the Sutra Engine project. It serves as the north star for all technical decisions, from architecture to code style to process.

---

## Vision and Purpose

Sutra aspires to be a **universal substrate for compositional, emergent, and narrative-rich game systems**. The engine enables designers to build everything from interactive fiction to deep simulations from a minimal, maximally compositional core.

**Core Goals:**
- Model any gameplay or narrative system via composition of simple parts
- Enable transparent, extensible, and infinitely flexible authoring
- Maintain a core simple enough to be fully understood, yet powerful enough to encode anything

This philosophy draws inspiration from the lambda calculus, Lisp/Scheme minimalism, and the emergent complexity of digital logic—where simple rules combine to create unlimited possibility.

---

## Foundational Principles

### 1. Minimalism and Composition

**Atoms and Macros Model:**
- **Atoms:** Irreducible micro-operations (queries, mutations, control flow, output, randomness)
- **Macros:** All higher-level constructs built as compositions of atoms and other macros
- **Everything is a macro:** From "storylet" to "loop" to "choice"—no privileged constructs

**Why:** This ensures arbitrary abstraction and extension while maintaining a minimal, stable foundation.

### 2. Explicit and Intentional Design

**Explicit Value Lookup:**
- All value access requires explicit `(get ...)` calls—no implicit resolution
- Paths are not automatically dereferenced in value contexts
- **Rationale:** Eliminates ambiguity, makes state access visible and debuggable

**Explicit Mutation:**
- State-changing operations are marked with `!` suffix (`set!`, `add!`, `sub!`)
- Pure operations (`+`, `-`, `*`, `/`) never modify state
- **Rationale:** Clear separation between computation and side effects

**Consistent Predicates:**
- All boolean-returning functions end with `?` (`is?`, `has?`, `gt?`)
- **Rationale:** Conditional logic is immediately identifiable

### 3. Functional Purity and Immutability

**Pure Functions by Default:**
- Operations are pure expressions unless explicitly marked as mutating
- World state is deeply immutable; mutations yield new state
- **Rationale:** Predictable, debuggable, and composable behavior

**Flat Code Structure:**
- Aggressive use of guard clauses and early returns
- Minimal indentation and nesting
- Function decomposition over deep branching
- **Rationale:** Readable, maintainable, and comprehensible code

### 4. Separation of Concerns and Encapsulation

**Strict Layering:**
- Parsing, macro expansion, validation, evaluation, and presentation are isolated
- No cross-layer dependencies or leakage
- **Rationale:** Modular, testable, and maintainable architecture

**Tight Encapsulation:**
- Components expose minimal, well-defined interfaces
- Implementation details are rigorously hidden
- **Rationale:** Prevents coupling, enables independent evolution

### 5. Transparency and Traceability

**Complete Introspection:**
- All computation is inspectable down to the atom level
- Macro expansion and world diffs are always available
- Code is convertible between surface syntaxes for debugging

**Unified Error System:**
- All errors use the `err_msg!` or `err_ctx!` macros for consistency
- Precise, actionable diagnostics with context and spans
- **Rationale:** Clear feedback accelerates development and debugging

---

## Architecture Philosophy

### Uniform Syntax and Representation

**Canonical AST:**
- All code is internally represented as s-expression trees
- Surface syntax options preserve full semantic fidelity

**Dual Surface Syntax:**
- **Brace-block syntax:** Accessible, newline-driven format
- **S-expression syntax:** Explicit, power-user format
- **One-to-one mapping:** Complete structural equivalence between forms

**Example Equivalence:**
```
storylet "find-key" {
  and {
    is? player.location "cellar"
    has? player.items "rusty-key"
  }
  do {
    print "You unlock the door."
    set! world.door.unlocked true
  }
}
```

```lisp
(storylet "find-key"
  (and
    (is? player.location "cellar")
    (has? player.items "rusty-key"))
  (do
    (print "You unlock the door.")
    (set! world.door.unlocked true)))
```

### Validation as a First-Class Concern

**Decoupled Validation:**
- Validation logic is separate from parsing and evaluation
- Modular, testable, and reusable validation components
- **Rationale:** Author safety and feedback without architectural coupling

**Comprehensive Coverage:**
- Grammar consistency checking
- Rule reference validation
- Critical pattern coverage
- **Rationale:** Prevents common authoring errors before runtime

---

## Development Philosophy

### Quality Gates and Process

**Non-Negotiable Standards:**
- Automated formatting, linting, and test coverage
- All code must pass quality gates before integration
- **Rationale:** Consistency and reliability over convenience

**Rigorous Review:**
- Every change reviewed for minimalism and philosophy alignment
- Technical debt tracked and reviewed quarterly
- **Rationale:** Prevents accumulation of complexity and drift

### Testing and Verification

**Comprehensive Test Coverage:**
- Unit tests for all validation logic
- Integration tests for end-to-end workflows
- **Rationale:** Confidence in correctness and regression prevention

**Behavior-Focused Testing:**
- Tests verify behavior, not implementation details
- Meaningful assertions over trivial coverage
- **Rationale:** Tests that actually catch real problems

### Documentation Standards

**Intent-Focused Documentation:**
- Concise, meaningful comments on "why" not "how"
- Public APIs documented with examples
- No boilerplate or structural redundancy
- **Rationale:** Documentation that adds value, not noise

---

## Evolution and Pragmatism

### Pragmatic Minimalism

**Start Minimal, Extend Carefully:**
- Begin with the smallest possible atom set
- Build everything else as macros and test in real scenarios
- Promote to atoms only when composition proves insufficient

**Empirical Process:**
- All decisions subject to revision based on real usage
- Author experience guides architectural evolution
- **Rationale:** Dogma serves the project, not vice versa

### Continuous Refinement

**Living Philosophy:**
- Principles evolve as the project matures
- Document rationale for all major changes
- **Rationale:** Adaptive philosophy that learns from experience

**Author Empowerment:**
- Surface syntax designed for human comfort
- Macro system enables unlimited extension
- Tooling supports introspection and debugging
- **Rationale:** The engine serves creators, not implementers

---

## Technical Standards

### Code Quality

**Rust Idiomatic Patterns:**
- Ownership-aware design with minimal cloning
- Result types for error handling
- Pattern matching over conditional branching
- **Rationale:** Leverage Rust's strengths for safety and performance

**Functional Bias:**
- Preference for expressions over statements
- Composition over inheritance
- Immutability over mutation
- **Rationale:** Predictable, composable, and debuggable systems

### Error Handling

**Unified Error Construction:**
- All errors must be constructed using the canonical error API (e.g., `errors::runtime_general`, `errors::validation_arity`, `errors::type_mismatch`, etc.)
- All error constructors must be called with exactly the required arguments (typically 4: message, context, source, span).
- All extra fields and method chaining (e.g., `.with_*()`) are forbidden.
- Span information must be provided for precise location; use a real span whenever possible, or `SourceSpan::from(0..0)` with a comment if not.
- **Rationale:** Uniform, helpful error experience, and maintainability.

**Example:**
```rust
// Canonical pattern (required)
return Err(errors::runtime_general(
    "Division by zero".to_string(),
    "math.sutra".to_string(),
    source_code, // or a string describing the context/source
    SourceSpan::new(offset, length),
));
```

**Graceful Degradation:**
- Validation errors don't crash the system
- Clear recovery paths for common failures
- **Rationale:** Robust author experience

---

## Inspiration and Context

**Foundational Influences:**
- **Scheme/Lisp:** Minimal core with macro extensibility
- **Lambda Calculus:** Universal computation from simple rules
- **Logic Gates:** Emergent complexity from basic operations
- **Quality-Based Narrative:** Compositional story systems

**Modern Practices:**
- **Rust Safety:** Memory safety without garbage collection
- **Functional Programming:** Predictable, composable design
- **Test-Driven Development:** Confidence through verification

---

## Code Review and Refactoring Prompts

**Purpose:** These prompts help identify opportunities for consolidation, simplification, and improved readability during code reviews. They focus on aligning code with the philosophy's core principles.

### Structural Simplification

**Redundant Parameter Structs:**
- Look for multiple similar parameter structs that could be unified
- Identify functions that create temporary objects just to pass parameters
- Find public/internal function pairs that add unnecessary complexity

**Example Prompts:**
```
"@philosophy.md review @[filename] -- identify any redundant parameter structs, unnecessary function wrappers, or violations of the flat code structure principle"
```

### Philosophy-Specific Violations

**Flat Code Structure:**
- Functions with depth > 3 that need guard clauses or early returns
- Deep nesting that could be flattened
- Complex conditional chains that could be simplified

**Error Handling Consistency:**
- Mix of `map_err` chains vs early returns with `?`
- Inconsistent use of `err_msg!` vs `err_ctx!` macros
- Error messages that don't follow the unified error system

**Example Prompts:**
```
"@philosophy.md analyze @[filename] -- find functions that could be simplified by removing temporary parameter objects or consolidating similar operations"
```

### Code Quality Issues

**Antipatterns:**
- Excessive `.clone()` calls that could be avoided with better ownership design
- Functions that clone just to pass parameters
- Complex functions that do multiple things (violating single responsibility)

**Type Aliases:**
- Complex return types that could benefit from type aliases
- Repeated complex parameter combinations
- Missing type aliases for complex results and contexts

**Example Prompts:**
```
"@philosophy.md check @[filename] -- look for inconsistent error handling patterns, excessive cloning, or functions that violate the 'one responsibility' principle"
```

### Domain-Specific Patterns

**AST Traversal:**
- Look for similar traversal logic across different modules
- Functions that could be unified into generic traversal patterns
- Redundant tree-walking code

**Macro Expansion:**
- Complex macro substitution logic that could be simplified
- Redundant parameter binding code
- Overly complex expansion patterns

**Validation Logic:**
- Validation functions that do multiple checks
- Validation that could be composed from smaller validators
- Non-modular validation components

**Example Prompts:**
```
"@philosophy.md review @[filename] -- find AST traversal patterns that could be unified or macro expansion logic that could be simplified"
```

### General Review Prompts

**Comprehensive Code Review:**
```
"@philosophy.md comprehensive-review @[filename] -- check for structural simplification opportunities, philosophy violations, code quality issues, and domain-specific patterns that could be improved"
```

**Architecture Review:**
```
"@philosophy.md architecture-review @[filename] -- analyze for separation of concerns, encapsulation quality, error handling consistency, and opportunities for better composition"
```

**Refactoring Assessment:**
```
"@philosophy.md refactor-check @[filename] -- identify functions that could be simplified, redundant structures to consolidate, and patterns that violate minimalism principles"
```

**Quality Gate Review:**
```
"@philosophy.md quality-review @[filename] -- check for flat code structure, consistent error handling, proper type aliases, and adherence to functional programming principles"
```

### Review Process

**When to Use:**
- During code reviews for new features
- When refactoring existing modules
- Before merging complex changes
- During architectural reviews

**Expected Outcomes:**
- Simplified function signatures
- Reduced code duplication
- Improved readability
- Better alignment with philosophy principles

---

## Changelog

- **2025-07-18:** Added comprehensive "Code Review and Refactoring Prompts" section with specific and general review prompts for maintaining code quality and philosophy alignment
- **2025-07-16:** Comprehensive rewrite reflecting current architecture, validation system, error handling, quality gates, and development process
- **2025-07-05:** Updated to require explicit value lookup (no more auto-get)

---

This philosophy guides every technical decision in Sutra, from the smallest function to the largest architectural choice. It evolves as we learn, but the core commitment to minimalism, compositionality, and transparency remains constant.
