# Debug Files for Native .sutra File Loading Investigation

This directory contains test files created during the investigation of native `.sutra` file loading and user-defined macro functionality (2025-07-07).

## Purpose

These files were used to systematically debug and isolate issues in the macro definition and expansion pipeline, leading to the identification and resolution of critical parser bugs.

## Test Files

### Basic Functionality Tests

- `debug_simple.sutra` - Basic script execution test: `(print "hello")`
- `debug_builtin.sutra` - Built-in macro functionality test: `(print "Hello, world!")`
- `debug_paramlist.sutra` - Parameter list parsing test: `((test))`

### Macro Definition Tests

- `debug_minimal.sutra` - Minimal macro definition: `(define (f) x)`
- `debug_macro.sutra` - Simple macro with body: `(define (test) (print "test macro"))`
- `debug_def_only.sutra` - Macro definition without call: `(define (greet name) (print ...))`
- `test_macro_native.sutra` - Complete macro definition and usage test

## Key Findings

These tests revealed:

1. **Parser Bug**: Fixed critical issue in `build_define_form` where literal "define" was incorrectly expected as AST node
2. **Integration Issue**: Identified that user-defined macros are not being properly partitioned/expanded
3. **Pipeline Validation**: Confirmed that basic scripts and built-in macros work perfectly

## Usage

These files can be used with CLI commands:

```bash
./target/debug/sutra run debug/macro-testing/debug_simple.sutra
./target/debug/sutra macrotrace debug/macro-testing/test_macro_native.sutra
```

## Status

As of 2025-07-07:

- ✅ Basic script execution works
- ✅ Built-in macros work
- ✅ Parser correctly handles macro definitions (after fix)
- ❌ User-defined macro expansion still blocked (integration issue)

These files document the systematic approach to isolating and debugging the native file loading capabilities.
