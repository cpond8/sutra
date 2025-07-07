# Sutra Parsing Pipeline Plan (Canonical Archival Document)

## Purpose
This document archives the full, detailed plan for the modular, interface-driven parsing pipeline refactor, including all context, rationale, refinements, and best practices. It is the single source of truth for this refactor and must be kept up to date as the project evolves.

## Context and Rationale
- The current parser, while functional, has proven difficult to maintain, debug, and extend.
- After extensive review, critique, and synthesis, a new architecture has been adopted: modular, interface-first, and maximally explicit.
- This plan incorporates best practices from language tooling, Rust ergonomics, and Sutra's core values (compositionality, transparency, extensibility).
- The plan and its context are referenced in all memory bank files and the system reference.

## Canonical Pipeline Architecture
- Each stage (CST parser, AST builder, macroexpander, validator, etc.) is a pure, swappable module with a documented contract.
- Data, error, and span information are explicit and preserved throughout.
- Opaque boundaries and explicit interfaces ensure maintainability and testability.

## Best Practices and Refinements
- Prefer enums for core types and error types; use trait objects only for extensibility.
- Macroexpander context starts minimal (registry, hygiene scope) and expands as needed.
- Unified, serializable diagnostic type with severity (Error, Warning, Info) for all validators.
- CST is opaque for core logic but provides a read-only traversal API for tooling.
- All public types and errors are serde-compatible for CLI/tools/debugging.
- Prefix all public types/traits with `Sutra` or a namespace for discoverability.
- Design for incremental/partial parsing at CST/AST stages for future editor integration.
- Use real-world narrative/game snippets for golden tests.
- Ship interfaces and trivial implementations first, with golden tests and contract documentation.
- Review and test each module in isolation before integration.
- Always favor explicitness and testability over clever abstraction.

## Migration Strategy
1. Draft and document all interfaces and types.
2. Implement trivial versions and golden tests for each module.
3. Incrementally port/refactor existing logic into the new modules.
4. Plug-and-play test each module in isolation.
5. Integrate modules and migrate the pipeline incrementally.
6. Maintain exhaustive documentation and changelogs.
7. Evolve each phase independently and with confidence.

## Cross-References
- All memory bank files (`memory-bank/`) reference this plan and summarize its context.
- `system-reference.md` contains a summary and migration strategy.
- See also: `docs/architecture/architecture.md`, `memory-bank/activeContext.md`, `memory-bank/systemPatterns.md`, `memory-bank/techContext.md`.

## Changelog
- 2025-07-04: Initial creation. Full plan, context, and best practices archived.
- 2025-07-05: Macro System, CLI, and Test Harness Refactor (Changelog)
  - Removed references to legacy macroexpander types and updated all macroexpander interface documentation.
  - Documented recursion depth enforcement (limit: 128) in macro expansion.
  - Updated macro system patterns and CLI output documentation.
  - All interface and contract sections now reflect the current implementation.
- 2024-07-08: **Status/progress section added to reflect current codebase.**
- [Add future updates here.]

## Canonical Interface Contracts

**Summary of Final Refinements (2025-07-04):**
- CST traversal is provided as both an `Iterator<Item=&SutraCstNode>` and a visitor pattern, supporting both depth-first and breadth-first traversals for tooling and editor integration.
- The AST Builder contract explicitly defines "normalized" AST, listing canonical forms and specifying what syntactic sugar is removed.
- `SutraMacroContext` includes a `get_macro(name: &str)` method for macro lookup; macro expansion is explicitly bounded and deterministic.
- Validator contracts note that future extensions may include an auto-fix interface, and diagnostics are designed for aggregation and chaining.
- All error messages must start with the rule name and describe expected vs. found, ensuring standardized, actionable diagnostics.
- For future pipeline phases, expected invariants (e.g., variable declaration before use) should be specified early in the contract.
- Example usage and golden tests will be expanded for complex AST and macro cases as the system evolves.

---

### 1. CST Parser Module Contract

**Purpose:**
- Parse source input (`&str`) into a concrete syntax tree (CST) using a PEG grammar (e.g., pest).
- Provide a read-only, traversable CST for downstream processing and tooling.

**Trait and Types:**
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraSpan { pub start: usize, pub end: usize }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraCstNode {
    pub rule: SutraRule, // Enum for grammar rules
    pub children: Vec<SutraCstNode>,
    pub span: SutraSpan,
    // Optionally: text, parent, etc.
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraCstParseError {
    Syntax { span: SutraSpan, message: String },
    Incomplete { span: SutraSpan, message: String },
    // ...
}

pub trait SutraCstParser {
    fn parse(&self, input: &str) -> Result<SutraCstNode, SutraCstParseError>;
    fn traverse<'a>(&'a self, node: &'a SutraCstNode) -> SutraCstTraversal<'a>;
    fn visit<'a, F: FnMut(&'a SutraCstNode)>(&'a self, node: &'a SutraCstNode, visitor: F, order: TraversalOrder);
}

pub struct SutraCstTraversal<'a> { /* Iterator<Item=&'a SutraCstNode> supporting DFS/BFS */ }

pub enum TraversalOrder { DepthFirst, BreadthFirst }
```

**Inputs:**
- `&str` (valid UTF-8 source code)

**Outputs:**
- `Result<SutraCstNode, SutraCstParseError>`
  - On success: root CST node representing the entire input
  - On error: parse error with precise span and message

**Invariants and Guarantees:**
- Always consumes the entire input or returns an error with the span of the first failing token.
- CST is immutable and read-only; no mutation allowed.
- All nodes and errors are serializable (serde-compatible).
- All spans are byte offsets into the original input.
- Traversal API is read-only and safe for tooling (syntax highlighting, etc.).
- Both depth-first and breadth-first traversal are supported.

**Error and Edge Case Behavior:**
- Syntax errors: return `Syntax` variant with span and message.
- Incomplete input: return `Incomplete` variant with span and message.
- All errors must include a span and a human-readable message.
- **Error message format:** All error messages must start with the rule name and describe expected vs. found.

**Example Usage:**
```rust
let parser = SutraPestCstParser::default();
let source = "(print \"Hello\")";
let cst = parser.parse(source)?;
assert_eq!(cst.rule, SutraRule::Program);
for node in parser.traverse(&cst) {
    println!("Rule: {:?}, Span: {:?}", node.rule, node.span);
}
parser.visit(&cst, |node| println!("Visiting: {:?}", node.rule), TraversalOrder::DepthFirst);
```

**Serialization/Diagnostics:**
- All types derive `Serialize`/`Deserialize` for CLI/editor integration and debugging.
- CST can be pretty-printed or exported for tooling.

**Extensibility Points:**
- To add new grammar rules, extend `SutraRule` and update the PEG grammar.
- To support new traversal patterns, extend `SutraCstTraversal`.
- To support incremental/partial parsing, extend the trait with new methods (future).

**Contract Changelog:**
- 2025-07-04: Initial contract drafted and approved.

---

### 2. AST Builder Module Contract

**Purpose:**
- Transform the CST (concrete syntax tree) into a canonical AST (abstract syntax tree), normalizing forms and discarding syntactic sugar.
- Preserve all span information for error reporting and debugging.

**Trait and Types:**
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraAstNode {
    List(Vec<WithSpan<SutraAstNode>>),
    Symbol(String),
    Number(f64),
    String(String),
    Bool(bool),
    Path(Vec<String>),
    If {
        condition: Box<WithSpan<SutraAstNode>>,
        then_branch: Box<WithSpan<SutraAstNode>>,
        else_branch: Box<WithSpan<SutraAstNode>>,
    },
    // ... extend as needed ...
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraAstBuildError {
    InvalidShape { span: SutraSpan, message: String },
    UnknownRule { span: SutraSpan, rule: String },
    // ...
}

pub trait SutraAstBuilder {
    fn build_ast(&self, cst: &SutraCstNode) -> Result<WithSpan<SutraAstNode>, SutraAstBuildError>;
}
```

**Normalization Guarantee:**
- The output AST is *normalized*: all syntactic sugar (e.g., `cond`, `choice`, custom forms) is transformed into canonical forms (e.g., nested `If`, explicit `List`, etc.). Only canonical AST node types remain. See the canonical forms list in `SutraAstNode`.
- Documented canonical forms: `List`, `Symbol`, `Number`, `String`, `Bool`, `Path`, `If` (and any future primitives).

**Inputs:**
- `&SutraCstNode` (root CST node)

**Outputs:**
- `Result<WithSpan<SutraAstNode>, SutraAstBuildError>`
  - On success: canonical AST node with span
  - On error: build error with precise span and message

**Invariants and Guarantees:**
- All AST nodes carry valid, non-overlapping spans.
- Output AST is canonical and normalized (no syntactic sugar remains).
- Never mutates the input CST.
- All nodes and errors are serializable (serde-compatible).

**Error and Edge Case Behavior:**
- Malformed CST: return `InvalidShape` with span and message.
- Unknown or unsupported rule: return `UnknownRule` with span and rule name.
- All errors must include a span and a human-readable message.
- **Error message format:** All error messages must start with the rule name and describe expected vs. found.

**Example Usage:**
```rust
let builder = SutraDefaultAstBuilder::default();
let ast = builder.build_ast(&cst)?;
assert!(matches!(ast.value, SutraAstNode::List(_)));
```

**Serialization/Diagnostics:**
- All types derive `Serialize`/`Deserialize` for CLI/editor integration and debugging.
- AST can be pretty-printed or exported for tooling.

**Extensibility Points:**
- To add new AST node types, extend `SutraAstNode` and update the builder logic.
- To support new normalization or desugaring patterns, extend the builder implementation.

**Contract Changelog:**
- 2025-07-04: Initial contract drafted and approved.

---

### 3. Macroexpander Module Contract

**Purpose:**
- Expand macros in the AST, supporting both built-in and user-defined macros.
- Transform AST to AST, preserving all span and semantic information.

**Trait and Types:**
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraMacroContext {
    pub registry: SutraMacroRegistry,
    pub hygiene_scope: Option<SutraHygieneScope>,
    // Extend as needed for user macros, environment, etc.
}

impl SutraMacroContext {
    pub fn get_macro(&self, name: &str) -> Option<&MacroDef> { /* ... */ }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraMacroError {
    Expansion { span: SutraSpan, macro_name: String, message: String },
    RecursionLimit { span: SutraSpan, macro_name: String },
    // ...
}

pub trait SutraMacroExpander {
    fn expand_macros(&self, ast: WithSpan<SutraAstNode>, context: &SutraMacroContext) -> Result<WithSpan<SutraAstNode>, SutraMacroError>;
}
```

**Inputs:**
- `WithSpan<SutraAstNode>` (AST node, possibly containing macro forms)
- `&SutraMacroContext` (macro registry, hygiene scope, etc.)

**Outputs:**
- `Result<WithSpan<SutraAstNode>, SutraMacroError>`
  - On success: macroexpanded AST node
  - On error: macro error with precise span, macro name, and message

**Invariants and Guarantees:**
- Never mutates the input AST.
- All output AST nodes preserve or update spans as appropriate.
- Macro expansion is pure, deterministic, and bounded (recursion/expansion depth is limited).
- All nodes and errors are serializable (serde-compatible).

**Error and Edge Case Behavior:**
- Expansion errors: return `Expansion` with span, macro name, and message.
- Recursion limit exceeded: return `RecursionLimit` with span and macro name.
- All errors must include a span and a human-readable message.
- **Error message format:** All error messages must start with the macro name and describe expected vs. found.

**Example Usage:**
```rust
let expander = SutraMacroExpander::default();
let expanded = expander.expand_macros(ast, &macro_context)?;
```

**Serialization/Diagnostics:**
- All types derive `Serialize`/`Deserialize` for CLI/editor integration and debugging.
- Macroexpansion trace can be exported for tooling.

**Extensibility Points:**
- To add new macro forms, extend the macro registry and context.
- To support new hygiene or user macro features, extend `SutraMacroContext` and expander logic.

**Contract Changelog:**
- 2025-07-04: Initial contract drafted and approved.

---

### 4. Validator Module Contract

**Purpose:**
- Perform all semantic and macro-level checks on the macroexpanded AST.
- Return a list of diagnostics (errors, warnings, info) for author feedback.

**Trait and Types:**
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraSeverity { Error, Warning, Info }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraDiagnostic {
    pub severity: SutraSeverity,
    pub message: String,
    pub span: SutraSpan,
    // Optionally: code, suggestion, etc.
}

pub trait SutraValidator {
    fn validate(&self, ast: &WithSpan<SutraAstNode>) -> Vec<SutraDiagnostic>;
}
```

**Inputs:**
- `&WithSpan<SutraAstNode>` (macroexpanded AST)

**Outputs:**
- `Vec<SutraDiagnostic>`
  - Each diagnostic includes severity, message, and span

**Invariants and Guarantees:**
- Never mutates the input AST.
- All diagnostics are serializable (serde-compatible).
- Validators may be chained or composed for different authoring modes.
- All diagnostics include a span and a human-readable message.
- Diagnostics are designed for aggregation and chaining.
- **Future extensibility:** Validators may provide an optional `auto_fix` interface (e.g., `fn fix(&self, ast: &mut WithSpan<SutraAstNode>)`).

**Error and Edge Case Behavior:**
- All errors, warnings, and info are returned as diagnostics; no panics or exceptions.
- Validators must not fail or abort on malformed input; always return diagnostics.
- **Error message format:** All error messages must start with the rule name and describe expected vs. found.

**Example Usage:**
```rust
let validator = SutraValidator::default();
let diagnostics = validator.validate(&expanded);
for diag in diagnostics {
    println!("{:?}: {} at {:?}", diag.severity, diag.message, diag.span);
}
```

**Serialization/Diagnostics:**
- All types derive `Serialize`/`Deserialize` for CLI/editor integration and debugging.
- Diagnostics can be exported for tooling and author feedback.

**Extensibility Points:**
- To add new validation rules, implement additional validator types and chain them.
- To support new diagnostic severities or codes, extend `SutraSeverity` and `SutraDiagnostic`.

**Contract Changelog:**
- 2025-07-04: Initial contract drafted and approved.

---

### 5. Placeholder: Future Phases (Lowering, Type Checking, Optimization)

**Purpose:**
- Support additional pipeline phases as the engine evolves (e.g., lowering, type checking, optimization).

**Contract Guidance:**
- Each new phase must:
  - Define clear trait(s) and public types
  - Specify input/output types and invariants
  - Be pure, stateless, and serializable
  - Document error/diagnostic behavior and extensibility points
  - Provide example usage and changelog
  - **Specify expected invariants early** (e.g., "All variables are declared before use", "AST is fully normalized").

---

## IV. Full Proposal: Canonical Modular Parsing Pipeline for Sutra

### 1. Architectural Overview

The Sutra parsing pipeline is a strictly modular, interface-driven sequence of pure, swappable stages. Each stage is a black box with a documented contract, responsible for a single transformation. All data, error, and span information is explicit and preserved throughout. The pipeline is designed for composability, testability, and future extensibility (e.g., type checking, optimization).

#### Pipeline Diagram

```
Source Input (&str)
   │
   ▼
CST Parser (CstParser)
   │
   ▼
AST Builder (AstBuilder)
   │
   ▼
Macroexpander (MacroExpander)
   │
   ▼
Validator(s) (Validator)
   │
   ▼
(Optional: Lowering, Type Checking, etc.)
   │
   ▼
Evaluator
```

### 2. Core Principles

- **Separation of Concerns:** Each stage is responsible for one transformation only.
- **Explicit Data Flow:** All state, error, and span information is passed explicitly.
- **Opaque Boundaries:** Each module exposes only its interface; internals are hidden.
- **Composability:** Modules can be swapped, composed, or extended without breaking contracts.
- **Testability:** Every module is exhaustively unit tested with golden inputs/outputs and error injection.
- **Auditability:** All transformations are small, pure, and easy to reason about.
- **Minimal Duplication:** Shared logic (e.g., span extraction, error formatting) is centralized in stateless helpers.
- **Documentation:** Every interface and contract is documented with examples, invariants, and error cases.

### 3. Data Types and Contracts

#### A. Span and WithSpan

```rust
pub struct Span { pub start: usize, pub end: usize }
pub struct WithSpan<T> { pub value: T, pub span: Span }
```
- All nodes (CST, AST) and errors carry span information for robust error reporting.

#### B. CST Parser Module

**Role**
- Parses source input (`&str`) into a concrete syntax tree (CST) using a PEG grammar (e.g., pest).
- No semantic validation, macroexpansion, or AST construction.

**Interface**
```rust
pub struct SutraCstNode { /* opaque: rule, children, span, etc. */ }
pub enum SutraCstParseError { /* includes span, error kind, stack trace */ }

pub trait SutraCstParser {
    fn parse(&self, input: &str) -> Result<SutraCstNode, SutraCstParseError>;
    fn traverse<'a>(&'a self, node: &'a SutraCstNode) -> SutraCstTraversal<'a>; // read-only traversal for tooling
}
```
- CST is opaque; only traversable via provided interface methods.
- Debug/inspection utilities are provided for testing and tooling.

#### C. AST Builder Module

**Role**
- Transforms CST into canonical AST nodes, normalizing forms and discarding syntactic sugar.
- No macroexpansion or semantic validation.

**Interface**
```rust
pub enum SutraAstNode { List(Vec<WithSpan<SutraAstNode>>), Symbol(String), Number(f64), /* ... */ }
pub enum SutraAstBuildError { /* always includes CST span, error kind */ }

pub trait SutraAstBuilder {
    fn build_ast(&self, cst: &SutraCstNode) -> Result<WithSpan<SutraAstNode>, SutraAstBuildError>;
}
```
- All AST nodes carry span info.
- Warnings or "soft errors" may be emitted for suspicious constructs (optional).

#### D. Macroexpander Module

**Role**
- Expands macros in the AST (AST → AST), supporting both built-in and user-defined macros.
- Pure transformation; never mutates input AST.

**Interface**
```rust
pub struct SutraMacroContext { /* registry, hygiene scope, ... */ }
pub enum SutraMacroError { /* includes AstNode span, macro name, error message */ }

pub trait SutraMacroExpander {
    fn expand_macros(&self, ast: WithSpan<SutraAstNode>, context: &SutraMacroContext) -> Result<WithSpan<SutraAstNode>, SutraMacroError>;
}
```
- `SutraMacroContext` provides registry, hygiene, and environment for expansion (start minimal, expand as needed).
- Macroexpander is reentrant and supports staged/recursive expansion.

#### E. Validator Module

**Role**
- Performs all semantic and macro-level checks (arity, forbidden constructs, duplicate bindings, etc.).
- Validators are composable and may emit errors or warnings.

**Interface**
```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraSeverity { Error, Warning, Info }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraDiagnostic {
    pub severity: SutraSeverity,
    pub message: String,
    pub span: Span,
    // Optionally: code, suggestion, etc.
}

pub trait SutraValidator {
    fn validate(&self, ast: &WithSpan<SutraAstNode>) -> Vec<SutraDiagnostic>;
}
```
- Multiple validators can be chained or composed for different authoring modes.
- Diagnostics are serializable for CLI/editor integration.

#### F. (Optional) Lowering, Type Checking, Optimization

- Additional phases can be inserted as needed, following the same interface discipline.

### 4. Shared Utilities

- **Span Extraction:** Centralized helpers for extracting and propagating spans.
- **Error Construction:** Unified error types, all serializable and including full context.
- **Debug/Inspection:** Pretty-printers and traversals for CST/AST for use in tests, REPLs, and tooling.
- **Tracing:** Optional, stateless tracing utilities for debugging and performance analysis.
- **Serialization:** All public types and errors must derive or implement `Serialize`/`Deserialize` (serde-compatible).

### 5. Example Usage: End-to-End Pipeline

```rust
let cst_parser = SutraPestCstParser::default();
let ast_builder = SutraDefaultAstBuilder::default();
let macroexpander = SutraMacroExpander::default();
let validator = SutraValidator::default();
let macro_context = SutraMacroContext::default();

let source: &str = "...";

let cst = cst_parser.parse(source)?;
let ast = ast_builder.build_ast(&cst)?;
let expanded = macroexpander.expand_macros(ast, &macro_context)?;
let diagnostics = validator.validate(&expanded);

// expanded is now the fully-checked, macroexpanded AST ready for evaluation
// diagnostics contains all errors/warnings for author feedback
```
- Each module can be swapped for testing, optimization, or experimentation.

### 6. Implementation and Migration Strategy

1. **Draft Interfaces:** Implement the trait and type definitions for each module.
2. **Trivial Implementations:** Start with "identity" or pass-through versions for each stage.
3. **Golden Tests:** Write golden input→output and error-path tests for each module, using real-world narrative/game content.
4. **Incremental Refactor:** Port existing parser logic into the new modules, extracting combinators and policies only where duplication is real.
5. **Plug-and-Play Testing:** Use mock/fake implementations to test each module in isolation.
6. **Iterative Expansion:** Add new rules, macro forms, and validators incrementally, always via the modular API.
7. **Documentation:** Document every interface, contract, and error case with examples and invariants.
8. **Versioning:** Track interface changes and breaking changes in module-level changelogs.

### 7. Key Refinements and Rationale

- **Hybrid Combinator/Direct Handler Approach:** Use combinators for regular rules, but allow direct handlers for complex or performance-critical cases.
- **Static Dispatch Where Possible:** Prefer enums and pattern matching for AST nodes; use trait objects only where necessary.
- **Unified Error/Diagnostic Type:** Use a top-level diagnostic type for all errors/warnings, making it easier to propagate and surface issues.
- **Debuggability:** All opaque types must provide debug/inspection methods for tests and tooling.
- **Extensible Macroexpander:** Design for future hygiene and user-defined macro support, even if not implemented immediately.
- **Incremental Parsing (Future):** Consider supporting incremental or partial parsing for editor integration.
- **Warnings as First-Class:** Validators and AST builder may emit warnings as well as errors, for better author feedback.
- **Serialization:** All public types and errors must be serde-compatible for CLI/editor integration and debugging.
- **Naming/Discoverability:** Prefix all public types/traits with `Sutra` or a namespace to avoid collisions and enhance discoverability.

### 8. Anti-Patterns to Avoid

- **Leaking Implementation:** Never expose module internals; always wrap outputs.
- **Hidden Global State:** All config and helpers are explicit and passed in.
- **Misplaced Logic:** Never perform macroexpansion or validation in the AST builder.
- **Over-Abstraction:** Only generalize when duplication is real and persistent.
- **Opaque Without Debug:** Opaque types must always be inspectable for debugging and tooling.

### 9. Documentation and Contracts

- **Every interface is documented** with:
  - Input/output types and invariants
  - Error and edge case behavior
  - Example usage and golden tests
- **All error types** include span and context info, and are serializable.
- **Changelogs** are maintained for all interface changes.

### 10. Example: Unified Diagnostic Type

```rust
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SutraSeverity { Error, Warning, Info }

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SutraDiagnostic {
    pub severity: SutraSeverity,
    pub message: String,
    pub span: Span,
    // Optionally: code, suggestion, etc.
}
```
- Validators and other modules return `Vec<SutraDiagnostic>`.
- CLI and authoring tools can filter, display, or escalate as needed.

### 11. Summary Table: Final Best Practices

| Area                | Best Practice                                                                 |
|---------------------|-------------------------------------------------------------------------------|
| Dispatch            | Prefer enums for small, stable sets; trait objects for extensibility          |
| Macro Context       | Start minimal, expand only as needed                                          |
| Diagnostics         | Use unified, serializable diagnostic type with severity                       |
| CST Traversal       | Provide read-only traversal for tooling                                       |
| Serialization       | All public types and errors are serde-compatible                              |
| Naming              | Prefix all public types/traits with `Sutra` or a namespace                    |
| Incremental Parsing | Design for partial/incremental parsing at CST/AST stages                      |
| Golden Tests        | Use real-world narrative/game snippets                                        |
| Implementation      | Ship interfaces + trivial impls + golden tests first; review contracts early  |
| Explicitness        | Always favor explicit, testable code over clever abstraction                  |

### 12. Example: Interface Documentation (Sample)

#### SutraCstParser

> `parse(&self, input: &str) -> Result<SutraCstNode, SutraCstParseError>`
>
> - *input*: Any valid UTF-8 string.
> - *output*: Root CST node on success; `SutraCstParseError` with source span and message on failure.
> - *Guarantee*: Always consumes the entire input or returns an error with span of first failing token.

#### SutraAstBuilder

> `build_ast(&self, cst: &SutraCstNode) -> Result<WithSpan<SutraAstNode>, SutraAstBuildError>`
>
> - *input*: Root CST node (from SutraCstParser).
> - *output*: Canonical AST node (with span) on success; error with CST span on failure.
> - *Guarantee*: Output AST contains full span information for all nodes. Never mutates CST.

#### SutraMacroExpander

> `expand_macros(&self, ast: WithSpan<SutraAstNode>, context: &SutraMacroContext) -> Result<WithSpan<SutraAstNode>, SutraMacroError>`
>
> - *input*: AST node (possibly containing macro forms), macro context.
> - *output*: Macroexpanded AST; errors always include macro name and input AST span.
> - *Guarantee*: Never mutates input; output AST is functionally equivalent except for expanded macros.

#### SutraValidator

> `validate(&self, ast: &WithSpan<SutraAstNode>) -> Vec<SutraDiagnostic>`
>
> - *input*: Macroexpanded AST.
> - *output*: List of diagnostics (errors, warnings, info) with span and context.
> - *Guarantee*: Never mutates AST; may be chained/composed with other validators.

---

*This document is canonical. All contributors must review and update it as the parsing pipeline evolves.*

## July 2025: Macroexpander Refactor & AST Invariant Migration

- Migrated Expr::List to Vec<WithSpan<Expr>> in all core modules (ast.rs, macros_std.rs, macros.rs, parser.rs, eval.rs, validate.rs).
- Macroexpander logic, helpers, and registry now operate exclusively on WithSpan<Expr>.
- Issues encountered:
  - Linter/type errors from mixed Expr/WithSpan<Expr> usage, especially in pattern matches and list construction.
  - Macro_rules! and error helper macros in atoms_std.rs require explicit, line-by-line fixes for delimiter and type safety.
  - Some macro contexts and test helpers still need a final audit for span-carrying compliance.
- Current status: Macroexpander and helpers are type-safe and span-carrying. Atoms_std.rs and some macro contexts need a final audit.
- Remaining work:
  - Complete audit and fix in atoms_std.rs (especially macro_rules! and error helpers).
  - Update all tests and doc examples for new AST invariant.
  - Perform a final integration test of the pipeline.
- Lessons learned:
  - Automated batch edits are insufficient for macro_rules! and error helpers; manual review is required.
  - Enforcing span-carrying invariants across all modules is nontrivial and must be maintained going forward.
- See memory bank and .cursorrules for canonical context and cross-references.

# Status as of 2024-07-08

## Parsing Pipeline Implementation Progress

- **CST Parser Module:**
  - Contract and scaffolding are present and mostly implemented (`src/cst_parser.rs`, `src/parser.rs`).
  - `PestCstParser` uses the canonical PEG grammar and produces a traversable CST.
  - Traversal/visitor APIs are stubbed or minimal, but the contract is present.
- **AST Builder Module:**
  - Contract and implementation are present and functional (`src/ast_builder.rs`, `src/ast.rs`).
  - Canonical builder normalizes CST to AST, with span-carrying nodes.
  - Error types and invariants match the plan.
- **Macroexpander Module:**
  - Macroexpansion logic is present and robust (`src/macros.rs`, `src/macros_std.rs`).
  - No explicit `SutraMacroExpander` trait, but the logic is modular and testable.
  - Registry and hygiene scope are present in the environment.
- **Validator Module:**
  - Contract and implementation are present and extensible (`src/validate.rs`, `src/validator.rs`).
  - Diagnostics are span-carrying, serializable, and match the contract.
  - Registry pattern allows chaining/composing validators.
- **Pipeline Integration:**
  - The modular pipeline (parse → macroexpand → validate → evaluate) is implemented in `src/lib.rs`.
  - All data flows are explicit and modular.
- **Documentation and Contracts:**
  - All modules have doc comments referencing the canonical plan, and the contracts are present in code and documentation.
- **Outstanding/Partial Areas:**
  - Macroexpander trait is not explicitly defined as in the plan, but the logic is present and modular.
  - CST traversal/visitor APIs are stubbed or minimal.
  - Some advanced features (auto-fix in validator, incremental parsing, full hygiene in macroexpander) are not yet implemented.
  - Golden tests and example usage are present but could be expanded.

**Conclusion:**
- The codebase is largely up to date with the canonical parsing pipeline plan. All major modules, contracts, and data types are present and implemented. Some interfaces (notably the macroexpander trait and advanced CST traversal) could be further formalized for full protocol compliance, but the core architecture and modularity are in place. Documentation and code comments are well-aligned with the plan.