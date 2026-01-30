# CLAUDE.md - Project Guidelines

## Project Overview
OLX Price Tracker - A Rust CLI tool for tracking OLX.pt listings and alerting on deals.

## Tech Stack
- **Language:** Rust (edition 2024, MSRV 1.85)
- **Async Runtime:** Tokio
- **HTTP Client:** Reqwest with socks proxy support
- **Database:** SQLite via rusqlite
- **CLI:** Clap with derive macros
- **Logging:** Tracing

## Architecture

```
src/
├── main.rs          # CLI entry point (~335 lines)
├── lib.rs           # Library exports
├── config.rs        # TOML config parsing
├── commands/        # CLI command handlers
│   ├── mod.rs       # Exports
│   ├── add.rs       # Add search command
│   ├── list.rs      # List searches command
│   ├── run.rs       # Run & daemon commands
│   ├── deals.rs     # Show deals command
│   ├── stats.rs     # Show stats command
│   ├── remove.rs    # Remove search command
│   ├── toggle.rs    # Toggle search command
│   └── search.rs    # Quick search command
├── db/              # SQLite database layer
│   ├── schema.rs    # Migrations
│   └── queries.rs   # CRUD operations
├── api/             # OLX API client
│   ├── client.rs    # HTTP client
│   └── models.rs    # Response types
├── tracker/         # Core business logic
│   ├── search.rs    # Search execution
│   └── price.rs     # Deal detection
├── filters/         # Extensible filter system
│   ├── mod.rs       # Filter trait
│   ├── keyword.rs   # Keyword matching
│   └── radius.rs    # Location filtering
└── notify/          # Notifications
    └── webhook.rs   # Webhook sender
```

## Code Standards

### Lints
- `unsafe_code = "forbid"` - No unsafe code allowed
- Clippy pedantic + nursery enabled
- Run `cargo clippy --all-targets` before committing

### Formatting
- Use `cargo fmt` before committing
- Max line width: 100 chars
- Edition 2024 style

### Error Handling
- Use `anyhow::Result` for application errors
- Use `thiserror` for library error types
- Always provide context with `.context()` or `.with_context()`

### Async
- Single-threaded runtime (rusqlite is not thread-safe)
- Futures don't need to be `Send`
- Use `tokio::time::sleep` for delays

### Testing
- Unit tests in same file with `#[cfg(test)]` module
- Run `cargo test` before committing
- Integration tests would go in `tests/` directory

## Common Commands

```bash
# Development
cargo build                    # Debug build
cargo build --release          # Release build

# Quality Checks (run these before committing)
cargo fmt                      # Format code
cargo clippy --all-targets     # Run linter (catch common mistakes and enforce style)
cargo test                     # Run all tests
cargo test --lib               # Run only library tests (faster)

# Running
./target/release/olx-tracker --help
./target/release/olx-tracker list
./target/release/olx-tracker run
```

## Pre-Commit Checklist

Before committing changes, always run:

```bash
# 1. Format code
cargo fmt

# 2. Check lints (should have no warnings)
cargo clippy --all-targets

# 3. Run tests (should all pass)
cargo test

# 4. Build release (optional, ensures release build works)
cargo build --release
```

## Adding New Features

### New Filter
1. Create `src/filters/my_filter.rs`
2. Implement the `Filter` trait
3. Export in `src/filters/mod.rs`
4. Add to `FilterChain::with_defaults()` if needed

### New Notification Backend
1. Create `src/notify/my_notifier.rs`
2. Implement the `Notifier` trait (async_trait)
3. Export in `src/notify/mod.rs`

### New CLI Command
1. Add variant to `Commands` enum in `main.rs`
2. Create `src/commands/my_command.rs` with `cmd_my_command()` function
3. Export in `src/commands/mod.rs`
4. Add match arm in `main.rs` calling `commands::cmd_my_command()`

## Database

- SQLite file: configurable via `--db` flag or `OLX_TRACKER_DB` env
- Default: `olx_tracker.db` in current directory
- Migrations run automatically on startup
- Schema version tracked in `schema_version` table

## Configuration

Required: `config.toml` with bearer token from OLX (get from browser DevTools).

See `config.example.toml` for all options.

## Dependencies Policy

- Prefer well-maintained crates with minimal transitive deps
- Use `default-features = false` when possible
- Keep deps up to date: `cargo upgrade`
- Check for security issues: `cargo audit` (install with `cargo install cargo-audit`)
