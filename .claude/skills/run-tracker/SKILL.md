---
name: run-tracker
description: Run olx-tracker CLI commands. Use when executing tracker operations, managing searches, or testing the application.
argument-hint: [command] [args...]
disable-model-invocation: true
allowed-tools: Bash(./target/*)
---

# Run OLX Tracker

Execute olx-tracker CLI commands for testing and operation.

## Quick Reference

```bash
# Show all commands
./target/release/olx-tracker --help

# Quick search (no database)
./target/release/olx-tracker search "$ARGUMENTS"
./target/release/olx-tracker search "$0" --format json
./target/release/olx-tracker search "$0" --city "Porto" --radius 30

# Manage tracked searches
./target/release/olx-tracker list
./target/release/olx-tracker add --name "$0" --keyword "$1"
./target/release/olx-tracker run
./target/release/olx-tracker daemon --interval 30

# HTTP API server
./target/release/olx-tracker serve --port 8080
API_KEY=secret ./target/release/olx-tracker serve
```

## Common Workflows

### Quick Search
```bash
./target/release/olx-tracker search "iphone 14"
./target/release/olx-tracker search "ps5" --max 10 --format json
./target/release/olx-tracker search "laptop" --city "Porto" --radius 30 --format markdown
```

### Add & Run Searches
```bash
# Add a new search
./target/release/olx-tracker add \
  --name "PS2 cheap" \
  --keyword "playstation 2" \
  --max-price 200 \
  --city Porto \
  --radius 30

# Run all searches
./target/release/olx-tracker run

# Run specific search
./target/release/olx-tracker run --search-id 1

# Start daemon (checks every 30 min)
./target/release/olx-tracker daemon --interval 30
```

### View Results
```bash
# Show all deals
./target/release/olx-tracker deals

# Show stats for search
./target/release/olx-tracker stats --search-id 1

# List searches
./target/release/olx-tracker list
```

### HTTP Server
```bash
# Start server on port 8080
./target/release/olx-tracker serve --port 8080

# With API key authentication
API_KEY=secret ./target/release/olx-tracker serve

# Custom timeout
./target/release/olx-tracker serve --timeout 120
```

## Requirements

- Build release first: `cargo build --release`
- `config.toml` must exist with valid OLX bearer token (for tracked searches)
- For quick searches, no config needed

## Custom Database

Use a different database file:
```bash
./target/release/olx-tracker --db /path/to/custom.db list
```
