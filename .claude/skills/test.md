# Run Tests

Execute the test suite for olx-tracker.

## Usage
Run before committing changes to ensure nothing is broken.

## Commands

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run only lib tests
cargo test --lib
```

## Expected Output
All 26 tests should pass.
