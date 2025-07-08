# Engine Feature Tests

This directory contains tests for engine-specific features that are not part of the core Sutra language specification.

## Test Categories

### Test-Atom Feature Tests

- `test_echo.sutra` - Tests the `test/echo` atom (requires `test-atom` feature)
- `test_borrow_stress.sutra` - Tests the `test/borrow_stress` atom (requires `test-atom` feature)

## Running These Tests

These tests require special engine features to be enabled:

```bash
# Run with test-atom feature (if available)
cargo test --features test-atom

# Or use the CLI test command (gracefully skips unavailable features)
./target/debug/sutra test tests/scripts/engine/
```

## Purpose

These tests validate:

- Engine extensibility and plugin system
- Feature-gated functionality
- Integration with non-core atoms
- Proper error handling for missing features

## Distinction from Core Tests

Unlike tests in `atoms/`, `macros/`, `parser/`, etc., these tests:

- May depend on optional engine features
- Test engine capabilities rather than language semantics
- May not be runnable in all environments
- Validate extensibility and plugin architecture
