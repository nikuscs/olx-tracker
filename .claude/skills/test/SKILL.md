---
name: test
description: Run the test suite. Use when running tests, checking test results, or when the user asks to test code.
argument-hint: [test-name or --coverage]
allowed-tools: Bash(cargo *)
---

# Run Tests

Execute the test suite for olx-tracker.

## Commands

```bash
# Run all tests
cargo test

# Run specific test
cargo test $ARGUMENTS

# Run tests with output
cargo test -- --nocapture

# Run only lib tests (faster)
cargo test --lib
```

## Coverage

When $ARGUMENTS contains "--coverage" or "coverage", run coverage report:

```bash
# Check if tarpaulin is installed, install if needed
which cargo-tarpaulin || cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Stdout --engine llvm
```

Current coverage: ~65% overall, with high coverage in core modules:
- api/client.rs: 88.9%
- db/queries.rs: 100%
- filters: 100%
- server/auth.rs: 100%

## Expected Output

All 201 tests should pass (as of Jan 2026).

## Test Safety

All tests are properly mocked - NO real OLX API calls are made during testing.
