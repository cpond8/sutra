## **Phase 1: Introduce Type Alias and Migrate API**

### 1. Add Type Alias

- [x] In `src/ast/mod.rs`, add:
  ```rust
  /// Canonical AST node type with shared ownership for efficient macro expansion.
  pub type AstNode = WithSpan<std::sync::Arc<Expr>>;
  ```

### 2. Batch Replace Usages

- [x] Replace all `WithSpan<Expr>` with `AstNode` in:
  - [x] Function signatures (parameters and return types)
  - [x] Struct fields
  - [x] Type aliases and trait impls
  - [x] Test code and helpers

### 3. Update AST Construction

- [x] Update all AST node construction to wrap `Expr` in `Arc::new(...)`:
  - [x] Update `with_span` and similar helpers to do this automatically.
  - [x] Update all direct `WithSpan` constructions.

### 4. Update Pattern Matching and Access

- [x] Update all pattern matches and destructuring to dereference the `Arc`:
  - [x] Use `match &*node.value { ... }` or similar.
  - [x] Update traversal and utility functions as needed.

### 5. Run Tests and Lint

- [ ] Run `cargo check` and `cargo test` to validate the migration.
- [ ] Fix remaining type mismatches and construction issues in:
  - [ ] `src/atoms/std.rs`
  - [ ] `src/cli/mod.rs`
  - [ ] `src/runtime/eval.rs`
  - [ ] `src/lib.rs`
  - [ ] `src/atoms/test.rs`
- [ ] Run `cargo clippy` and fix any new warnings or errors.

---

## **Phase 2: Refactor Macro System and Optimize**

### 6. Remove Unnecessary Deep Clones

- [ ] Audit macro expansion, substitution, and trace logic for `.clone()` calls.
- [ ] Replace deep clones with `Arc::clone(&node.value)` or rely on `AstNode`’s `.clone()`.
- [ ] Remove any now-unnecessary deep cloning.

### 7. Update Documentation

- [ ] Update doc comments for all public APIs and helpers to reference `AstNode`.
- [ ] Add safety notes about shared ownership and `Arc` usage.

### 8. Benchmark and Profile

- [ ] Add or run benchmarks for macro-heavy code.
- [ ] Compare memory and CPU usage before and after the change.

### 9. (Optional) Experiment with Alternative Strategies

- [ ] If desired, try changing the type alias to use `Rc`, `Cow`, or arena allocation.
- [ ] Evaluate tradeoffs and document findings.

---

## **Current Progress Summary (2025-07-11)**

- Type alias and most API migration complete.
- All helpers and pattern matches updated.
- Remaining work: fix legacy usages and type mismatches in a handful of files, then validate and document.
- See checklist above for precise files and steps.

---

## **Final Review**

- [ ] Ensure all code, docs, and tests are up to date.
- [ ] Commit changes with a clear message about the migration and rationale.

---

**Tip:**
You can use your editor’s multi-file search/replace or a tool like `sed` for the batch replacement step.
Example (from project root):

```sh
find src/ -type f -name '*.rs' -exec sed -i '' 's/WithSpan<Expr>/AstNode/g' {} +
```

(Review changes before committing!)

---
