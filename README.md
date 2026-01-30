# 🔍 OLX Price Tracker

Fast Rust CLI to track OLX listings, monitor prices, and alert on good deals.

## ⚡ Quick Start

```bash
# Build
cargo build --release

# Quick search (no database, just display results)
./target/release/olx-tracker search "playstation 5"

# Add a tracked search
./target/release/olx-tracker add -n "PS5 deals" -k "playstation 5" --min-price 300

# Run checks on all saved searches
./target/release/olx-tracker run
```

## 🔎 Quick Search (One-off)

Search OLX without saving to database - great for testing queries:

```bash
# Basic search
olx-tracker search "iphone 14"

# Limit results
olx-tracker search "macbook" --max 10

# Sort options: newest, cheapest, expensive, relevance
olx-tracker search "nintendo switch" --sort cheapest
olx-tracker search "AYN odin 2" --sort relevance

# Filter by price (removes junk results)
olx-tracker search "playstation 5" --min-price 300
olx-tracker search "iphone 14" --max-price 800
olx-tracker search "macbook pro" --min-price 500 --max-price 1500

# Filter by location
olx-tracker search "bicicleta" --city "Porto" --radius 30

# Combine all filters
olx-tracker search "AYN odin 2" \
  --sort relevance \
  --min-price 200 \
  --max 10 \
  --city "Lisboa" \
  --radius 50
```

## 📋 Tracked Searches

### Add a Search

```bash
# Basic search
olx-tracker add -n "PS5" -k "playstation 5"

# With price filters
olx-tracker add -n "PS5 deals" -k "playstation 5" --min-price 300 --max-price 450

# With location
olx-tracker add -n "PS5 Porto" -k "playstation 5" --city "Porto" --radius 30

# With sort order (newest, cheapest, expensive, relevance)
olx-tracker add -n "PS5 cheap" -k "playstation 5" --sort cheapest

# Full example with all options
olx-tracker add \
  --name "AYN Odin 2" \
  --keyword "AYN odin 2" \
  --min-price 200 \
  --max-price 500 \
  --city "Lisboa" \
  --radius 50 \
  --sort relevance
```

### List Searches

```bash
# List active searches
olx-tracker list

# Include inactive searches
olx-tracker list --all
```

### Run Checks

```bash
# Run all active searches
olx-tracker run

# Run specific search by ID
olx-tracker run --search-id 1

# Limit results per search
olx-tracker run --max-results 50
```

### Daemon Mode

```bash
# Check every 30 minutes (default)
olx-tracker daemon

# Custom interval
olx-tracker daemon --interval 15

# With max results per search
olx-tracker daemon --interval 30 --max-results 100
```

### Manage Searches

```bash
# View deals found
olx-tracker deals
olx-tracker deals --search-id 1

# View price statistics
olx-tracker stats --search-id 1

# Toggle search active/inactive
olx-tracker toggle --search-id 1

# Remove a search
olx-tracker remove --search-id 1
```

## 🌍 Multi-Country Support

Supports multiple OLX regions:

```bash
# Via CLI flag
olx-tracker --country pl search "iphone"    # Poland
olx-tracker --country ua search "iphone"    # Ukraine
olx-tracker --country ro search "iphone"    # Romania
olx-tracker --country bg search "iphone"    # Bulgaria

# Via environment variable
export OLX_COUNTRY=pl
olx-tracker search "playstation"

# Available countries: pt, pl, ua, ro, bg, kz, uz
```

## 🔔 Notifications

### Discord Webhooks

```bash
# Via CLI flag
olx-tracker --discord "https://discord.com/api/webhooks/ID/TOKEN" run

# Via environment variable
export OLX_DISCORD_WEBHOOK="https://discord.com/api/webhooks/ID/TOKEN"
olx-tracker daemon
```

### Generic Webhooks

```bash
# Via CLI flag
olx-tracker --webhook "https://your-server.com/notify" run

# Via environment variable
export OLX_WEBHOOK="https://your-server.com/notify"
```

### Notification Flags

```bash
# Enable specific notifications
olx-tracker --notify-new run       # New listings
olx-tracker --notify-drops run     # Price drops
olx-tracker --notify-deals run     # Deals (below average)

# Combine with webhooks
olx-tracker --discord "URL" --notify-deals --notify-drops daemon
```

## 🎯 Deal Detection

Configure what counts as a "deal":

```bash
# Deal = 30% below average price
olx-tracker --deal-threshold 30 run

# Deal = any listing at or below target price
olx-tracker --target-price 299 run

# Via environment variables
export OLX_DEAL_THRESHOLD=25
export OLX_TARGET_PRICE=350
olx-tracker run
```

## 🗄️ Database

```bash
# Default: olx_tracker.db in current directory

# Custom path via CLI
olx-tracker --db /path/to/custom.db list

# Via environment variable
export OLX_TRACKER_DB=/path/to/custom.db
```

## 🔧 Configuration File

All CLI options can also be set in `config.toml`:

```bash
cp config.example.toml config.toml
```

```toml
[api]
country = "pt"

[deals]
threshold_pct = 20.0
target_price = 299.0

[notifications]
discord_webhook_url = "https://discord.com/api/webhooks/..."
notify_on_new_listing = true
notify_on_price_drop = true
notify_on_deal = true

[database]
path = "olx_tracker.db"
```

CLI flags override config file values.

## 📦 All CLI Options

```bash
olx-tracker --help
olx-tracker search --help
olx-tracker add --help
```

### Global Options

| Option | Env Var | Description |
|--------|---------|-------------|
| `-c, --config` | - | Config file path (default: config.toml) |
| `-d, --db` | `OLX_TRACKER_DB` | Database file path |
| `--country` | `OLX_COUNTRY` | OLX country (pt, pl, ua, ro, bg, kz, uz) |
| `--discord` | `OLX_DISCORD_WEBHOOK` | Discord webhook URL |
| `--webhook` | `OLX_WEBHOOK` | Generic webhook URL |
| `--deal-threshold` | `OLX_DEAL_THRESHOLD` | % below avg to be a deal |
| `--target-price` | `OLX_TARGET_PRICE` | Max price to be a deal |
| `--notify-new` | - | Notify on new listings |
| `--notify-drops` | - | Notify on price drops |
| `--notify-deals` | - | Notify on deals |

### Search Options

| Option | Description |
|--------|-------------|
| `--max, -m` | Max results (default: 20) |
| `--sort, -s` | Sort: newest, cheapest, expensive, relevance |
| `--min-price` | Minimum price filter |
| `--max-price` | Maximum price filter |
| `--city` | City to search in |
| `--radius, -r` | Search radius in km |

### Add Options

| Option | Description |
|--------|-------------|
| `-n, --name` | Search name (required) |
| `-k, --keyword` | Search keyword (required) |
| `--min-price` | Minimum price filter |
| `-p, --max-price` | Maximum price (deal threshold) |
| `--city` | City to search in |
| `-r, --radius` | Search radius in km |
| `--category` | OLX category ID |
| `-s, --sort` | Sort order (default: newest) |

## 🚀 Features

- ⚡ Fast Rust implementation
- 🔎 Quick search without database
- 💾 SQLite storage with price history
- 🔔 Discord & webhook notifications
- 🎯 Smart deal detection
- 💰 Min/max price filtering
- 🌍 Multi-country support (7 OLX regions)
- 📍 Location + radius filtering
- 🔄 Daemon mode for continuous monitoring
- 🌐 Proxy support (SOCKS5/HTTP)

## 📄 License

MIT
