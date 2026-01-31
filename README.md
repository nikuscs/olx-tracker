# olx-tracker

![CI](https://github.com/nikuscs/olx-tracker/actions/workflows/ci.yml/badge.svg)
![Release](https://img.shields.io/github/v/release/nikuscs/olx-tracker)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

**Fast Rust CLI to track OLX.pt listings and get alerts on deals.**

Search products, track price drops, filter by location, and receive notifications via Discord or webhooks when good deals appear.

## Why?

- **Fast** — Native Rust. Searches complete in milliseconds.
- **Location-aware** — Filter by city/region with radius. Auto-resolves city names to OLX location IDs.
- **Deal Detection** — Track searches over time and get notified when prices drop or new listings match your criteria.
- **Notifications** — Discord webhooks, generic webhooks, or run your own alerting.
- **Daemon Mode** — Run in background, checks periodically, alerts automatically.

> **Disclaimer:** This project is for **educational purposes and AI automation research only**.
> The authors are not responsible for any misuse or for any damages resulting from the use of this tool.
> Users are solely responsible for ensuring compliance with applicable laws and the terms of service
> of any websites accessed. This software is provided "as-is" without warranty of any kind.
>
> If you are a rights holder and wish to have this project removed, please [contact me](https://github.com/nikuscs).

> **Note:** This project was partially developed with AI assistance and may contain bugs or unexpected behavior. Use at your own risk.

## Install

```bash
# From source (requires Rust)
cargo install --git https://github.com/nikuscs/olx-tracker

# Or clone and build
git clone https://github.com/nikuscs/olx-tracker
cd olx-tracker
cargo build --release
```

Pre-built binaries available in [Releases](https://github.com/nikuscs/olx-tracker/releases).

## Usage

### Search Listings

```bash
olx-tracker search "iphone 14"
olx-tracker search "macbook" --max 10 --sort cheapest
olx-tracker search "ps5" --min-price 300 --max-price 500
olx-tracker search "nintendo switch" --city "Porto" --radius 30
olx-tracker search "bicicleta" --city "Lisboa" --radius 50 --sort newest
```

**Output:**
```
OLX Search Results: "iphone 14"
================================================================================
  Price    Title                                              Location
--------------------------------------------------------------------------------
  €450     iPhone 14 Pro 128GB como novo                      Porto
  €380     iPhone 14 128GB c/ caixa                           Vila Nova de Gaia
  €520     iPhone 14 Pro Max 256GB                            Lisboa
  
💡 Found 3 listings
```

### Track Searches

Save searches to monitor over time:

```bash
# Add a tracked search
olx-tracker add --name "PS5 Deals" --keyword "ps5" --max-price 400 --city "Porto"

# Add with auto-expire (temporary tracking)
olx-tracker add --name "iPhone temp" --keyword "iphone 15" --days 7

# List all tracked searches
olx-tracker list

# Check for new deals across all tracked searches
olx-tracker deals

# View stats for a specific search
olx-tracker stats --search-id 1

# Toggle search on/off
olx-tracker toggle --search-id 1

# Remove a search
olx-tracker remove --search-id 1
```

### Daemon Mode

Run in background with periodic checks:

```bash
# Start daemon (checks every 15 minutes by default)
olx-tracker daemon

# Custom interval
olx-tracker daemon --interval 30  # Check every 30 minutes

# With Discord notifications
olx-tracker daemon --discord-webhook "https://discord.com/api/webhooks/..."
```

### Notifications

Configure alerts for new listings matching your tracked searches:

```bash
# Discord webhook
olx-tracker daemon --discord-webhook "https://discord.com/api/webhooks/xxx/yyy"

# Generic webhook (POST JSON)
olx-tracker daemon --webhook "https://your-server.com/olx-alerts"
```

**Discord alert example:**
```
🔔 New OLX Listing!
PS5 Console + 2 Controllers
€420 · Porto
https://olx.pt/d/anuncio/...
```

## Options

### Search Filters

| Flag | Description |
|------|-------------|
| `--max` | Max results (default: 20) |
| `--sort` | Sort: newest, cheapest, expensive, relevance |
| `--min-price` | Minimum price |
| `--max-price` | Maximum price |
| `--city` | City/region name (auto-resolved) |
| `--radius` | Radius from city in km |
| `--keyword` | Additional keyword filter |
| `--category` | OLX category ID |

### Output Formats

| Flag | Description |
|------|-------------|
| `--format table` | Human-readable table (default) |
| `--format json` | JSON output for scripts |
| `--format markdown` | Markdown for LLMs/docs |

### Daemon Options

| Flag | Description |
|------|-------------|
| `--interval` | Check interval in minutes (default: 15) |
| `--discord-webhook` | Discord webhook URL for alerts |
| `--webhook` | Generic webhook URL |

## Configuration

Tracked searches and history are stored in `~/.config/olx-tracker/`:

```
~/.config/olx-tracker/
├── searches.json     # Tracked searches
├── history.json      # Seen listings (deduplication)
└── config.toml       # Global settings
```

**config.toml:**
```toml
default_city = "Porto"
default_radius = 30
check_interval_minutes = 15
discord_webhook = "https://discord.com/api/webhooks/..."
```

## Examples

### Find cheap PS5 near Porto

```bash
olx-tracker search "ps5" --max-price 400 --city "Porto" --radius 50 --sort cheapest
```

### Track iPhone deals for a week

```bash
olx-tracker add --name "iPhone deals" --keyword "iphone 15" --max-price 700 --days 7
olx-tracker daemon --discord-webhook "https://discord.com/..."
```

### Export to JSON for processing

```bash
olx-tracker search "macbook" --format json | jq '.[] | select(.price < 800)'
```

### Multiple cities

```bash
olx-tracker search "bicicleta" --city "Porto" --radius 100  # Porto + surrounding
olx-tracker search "bicicleta" --city "Lisboa"              # Lisboa area
```

## How It Works

1. **Search** — Queries OLX.pt API with your filters
2. **Parse** — Extracts listings, prices, locations, URLs
3. **Track** — Saves seen listings to avoid duplicate alerts
4. **Notify** — Sends webhooks when new matching listings appear

## Portugal Coverage

Works with all OLX.pt regions:
- Major cities: Lisboa, Porto, Braga, Coimbra, Faro, etc.
- Districts and municipalities
- Radius-based filtering from any location

## License

MIT
