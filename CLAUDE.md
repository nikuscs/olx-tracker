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
│   ├── mod.rs       # Database struct
│   ├── schema.rs    # Migrations
│   └── queries.rs   # CRUD operations
├── api/             # OLX API client
│   ├── mod.rs       # Exports
│   ├── client.rs    # HTTP client (88.9% test coverage)
│   └── models.rs    # Response types
├── tracker/         # Core business logic
│   ├── mod.rs       # Tracker exports
│   ├── search.rs    # Search execution
│   └── price.rs     # Deal detection (100% test coverage)
├── filters/         # Extensible filter system
│   ├── mod.rs       # Filter trait (100% test coverage)
│   ├── keyword.rs   # Keyword matching
│   ├── price.rs     # Price filtering
│   └── radius.rs    # Location filtering (100% test coverage)
├── format/          # Output formatting
│   └── mod.rs       # Table/JSON/Markdown formatters
├── notify/          # Notifications
│   ├── mod.rs       # Multi-notifier (100% test coverage)
│   ├── discord.rs   # Discord webhooks
│   └── webhook.rs   # Generic webhooks
└── server/          # HTTP API server
    ├── mod.rs       # Server setup
    ├── auth.rs      # API key authentication (100% test coverage)
    ├── daemon.rs    # Background daemon
    ├── error.rs     # Error responses (100% test coverage)
    ├── handlers.rs  # HTTP handlers
    ├── models.rs    # Request/response models
    ├── routes.rs    # Route definitions
    └── state.rs     # Application state
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
- HTTP mocking with `wiremock` for API client tests
- All tests use mocks - NO real OLX API calls
- Current coverage: 65.4% overall, 201 tests passing
- High-coverage modules: api/client (88.9%), db/queries (100%), filters (100%)
- Integration tests would go in `tests/` directory

Run coverage report:
```bash
cargo install cargo-tarpaulin  # One-time install
cargo tarpaulin --out Stdout --engine llvm
```

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

## End-of-Task Workflow

**ALWAYS run these three commands at the end of ANY significant task:**

```bash
# 1. Format code (fixes style automatically)
cargo fmt

# 2. Check lints (must have 0 warnings for CI)
cargo clippy --all-targets

# 3. Run tests (must all pass)
cargo test
```

**Why this matters:**
- CI runs with `RUSTFLAGS="-Dwarnings"` which treats warnings as errors
- Formatting issues will fail the CI format check
- Broken tests block merging to main
- Running these locally catches issues before pushing

**When to run:**
- After writing new code
- After refactoring
- Before committing
- After resolving merge conflicts
- At the end of any development session

## Pre-Commit Checklist

**CRITICAL: Always run format, clippy, and tests before committing!**

Before committing changes, always run:

```bash
# 1. Format code (auto-fixes style issues)
cargo fmt

# 2. Check lints (MUST have 0 warnings - CI treats warnings as errors)
cargo clippy --all-targets

# 3. Run tests (MUST all pass)
cargo test

# 4. Build release (optional, ensures release build works)
cargo build --release
```

**Verification:** All three must succeed with no errors or warnings. CI will fail if any of these fail locally.

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

## Key Features

### HTTP Server (serve command)
- Axum-based HTTP API exposing all CLI functionality
- API key authentication via headers
- Background daemon support (starts/stops within server process)
- Request timeout configuration
- JSON request/response format
- See README.md for endpoint documentation

### Search Capabilities
- Quick search (no database, instant results)
- Tracked searches (saved to SQLite)
- Multi-country support (pt, pl, ua, ro, bg, kz, uz)
- City + radius filtering with auto-lookup
- Price range filtering (min/max)
- Search TTL (auto-expire after N days)
- Sort orders: newest, cheapest, expensive, relevance

### Output Formats
- Table (CLI default, pretty-printed)
- JSON (includes images array, for APIs/scripts)
- Markdown (includes images, optimized for LLMs)

### Deal Detection
- Threshold-based: X% below average price
- Target price: Any listing at/below specified price
- Per-search max price override
- Smart filtering with FilterChain

### Notifications
- Discord webhooks (rich embeds)
- Generic webhooks (JSON payload)
- Configurable per event type (new listings, price drops, deals)

### Proxy Support
- SOCKS5 and HTTP proxies
- Credential masking in logs for security
- Per-request configuration

## Dependencies Policy

- Prefer well-maintained crates with minimal transitive deps
- Use `default-features = false` when possible
- Keep deps up to date: `cargo upgrade`
- Check for security issues: `cargo audit` (install with `cargo install cargo-audit`)

## Test Dependencies

- `wiremock` - HTTP mock server for API client tests
- `tokio-test` - Async test utilities
- `tempfile` - Temporary file/directory creation for tests
- `tower` - Testing utilities for HTTP services
- `http-body-util` - HTTP body utilities for testing
