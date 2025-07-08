# Canonical Test Suite (2025-07)

This directory now uses a small set of canonical test files, consolidating all previous tests for clarity, coverage, and maintainability.

## Canonical Test Files

| Canonical File           | Purpose/Contents                                                                 |
|-------------------------|---------------------------------------------------------------------------------|
| macro_basic.sutra        | Minimal macro definition, invocation, parameter passing.                         |
| macro_string_ops.sutra   | Macros involving string operations/interpolation.                                |
| atom_core_ops.sutra      | Canonical test for all core atom operations (arithmetic, comparison, path, etc). |
| error_cases.sutra        | Error handling, invalid input, and edge cases.                                   |
| parser_edge_cases.sutra  | Parser-specific edge cases (parameter list, etc). Optional.                      |

## Rationale

- **macro_basic.sutra**: Ensures parser and expander handle minimal and parameterized macros.
- **macro_string_ops.sutra**: Covers string interpolation and macro string operations.
- **atom_core_ops.sutra**: Comprehensive coverage of all built-in atoms and their macro patterns.
- **error_cases.sutra**: Ensures correct error handling for invalid input, arity, type, and unknown atoms.
- **parser_edge_cases.sutra**: Retained only if unique parser edge cases are not covered elsewhere.
