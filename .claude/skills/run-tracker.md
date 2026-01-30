# Run Tracker

Execute the olx-tracker CLI commands.

## Usage
Use these commands to interact with the tracker.

## Commands

```bash
# Show help
./target/release/olx-tracker --help

# List all searches
./target/release/olx-tracker list

# Add a new search
./target/release/olx-tracker add \
  --name "PS2 cheap" \
  --keyword "playstation 2" \
  --max-price 200 \
  --city Porto \
  --radius 30

# Run all searches once
./target/release/olx-tracker run

# Run specific search
./target/release/olx-tracker run --search-id 1

# Show deals
./target/release/olx-tracker deals

# Show stats
./target/release/olx-tracker stats --search-id 1

# Start daemon (every 30 min)
./target/release/olx-tracker daemon --interval 30

# Use custom database
./target/release/olx-tracker --db /path/to/custom.db list
```

## Requirements
- `config.toml` must exist with valid bearer token
- Build release first: `cargo build --release`
