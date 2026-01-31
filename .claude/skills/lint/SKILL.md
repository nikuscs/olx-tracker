---
name: lint
description: Check code quality and formatting. Use when checking code style, running pre-commit checks, or when the user asks to lint or format code.
argument-hint: [--fix to auto-fix]
disable-model-invocation: true
allowed-tools: Bash(cargo *)
---

# Lint & Format Code

Run linting and formatting checks before committing code.

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
- No unsafe code allowed (`unsafe_code = "forbid"`)

## When $ARGUMENTS contains "--fix"

Run both format and clippy fixes:

```bash
cargo fmt
cargo clippy --fix --allow-dirty --allow-no-vcs --all-targets
```

Otherwise, just check without modifying files:

```bash
cargo fmt --check
cargo clippy --all-targets
```
