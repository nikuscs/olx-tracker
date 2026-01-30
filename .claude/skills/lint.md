# Lint & Format

Check code quality and formatting.

## Usage
Run before committing to ensure code meets project standards.

## Commands

```bash
# Check formatting (no changes)
cargo fmt --check

# Apply formatting
cargo fmt

# Run clippy lints
cargo clippy --all-targets

# Fix clippy warnings automatically (where possible)
cargo clippy --fix --allow-dirty --allow-no-vcs
```

## Standards
- Pedantic + nursery clippy lints enabled
- Max line width: 100 chars
- No unsafe code allowed
