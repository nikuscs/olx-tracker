# OLX Price Tracker

Fast Rust CLI to track OLX listings and alert on good deals.

## Install

```bash
cargo build --release
```

## Quick Search

```bash
olx-tracker search "iphone 14"
olx-tracker search "macbook" --max 10 --sort cheapest
olx-tracker search "ps5" --min-price 300 --max-price 500
olx-tracker search "iphone" --city "Porto" --radius 30
olx-tracker search "iphone" --city "Paços de Ferreira" --radius 50
olx-tracker search "ps5" --format json
olx-tracker search "ps5" --format markdown
```

| Flag | Description |
|------|-------------|
| `--max` | Max results (default: 20) |
| `--sort` | newest, cheapest, expensive, relevance |
| `--min-price` | Minimum price filter |
| `--max-price` | Maximum price filter |
| `--city` | City/region name (auto-lookup) |
| `--radius` | Radius in km from city |
| `--format` | table, json, markdown (aliases: md, llm) |

## Tracked Searches

```bash
olx-tracker add --name "PS5" --keyword "playstation 5"
olx-tracker add --name "PS5 deals" --keyword "ps5" --min-price 300 --max-price 450 --sort cheapest
olx-tracker add --name "iPhone Porto" --keyword "iphone" --city "Porto" --radius 30
olx-tracker add --name "PS5 temp" --keyword "ps5" --days 7
olx-tracker list
olx-tracker toggle --search-id 1
olx-tracker remove --search-id 1
olx-tracker stats --search-id 1
olx-tracker deals
```

| Flag | Description |
|------|-------------|
| `--name` | Search name |
| `--keyword` | Search keyword |
| `--min-price` | Min price filter |
| `--max-price` | Max price filter |
| `--sort` | newest, cheapest, expensive, relevance |
| `--city` | City/region name |
| `--radius` | Radius in km |
| `--days` | Auto-expire after N days |

## Run & Daemon

```bash
olx-tracker run
olx-tracker run --search-id 1
olx-tracker run --max-results 50

olx-tracker daemon
olx-tracker daemon --interval 15
olx-tracker daemon --interval 60 --max-results 100
```

| Flag | Description |
|------|-------------|
| `--search-id` | Run specific search only |
| `--max-results` | Max results per search (default: 100) |
| `--interval` | Check interval in minutes (default: 30) |

## Serve (HTTP API)

```bash
olx-tracker serve
olx-tracker serve --host 0.0.0.0 --port 8080
olx-tracker serve --timeout 120
API_KEY=secret olx-tracker serve
```

| Flag | Description |
|------|-------------|
| `--host` | Bind address (default: 127.0.0.1) |
| `--port` | Port to listen on (default: 8080) |
| `--timeout` | Request timeout in seconds (default: 60) |
| `--api-key` | API key for authentication (or use `API_KEY` env) |

Endpoints (JSON):

- `GET /health` - Health check (no auth required)
- `POST /search` - Quick search
- `POST /searches/add` - Add a tracked search
- `POST /searches/list` - List tracked searches
- `POST /searches/run` - Run searches
- `POST /searches/daemon` - Start background daemon
- `POST /searches/daemon/stop` - Stop background daemon
- `POST /searches/deals` - Get deals
- `POST /searches/stats` - Get search statistics
- `POST /searches/toggle` - Toggle search active status
- `POST /searches/remove` - Remove a search

Auth: set `API_KEY` for the `serve` command and pass it via `x-api-key`, `api-key`, or `Authorization: Bearer ...` header.

### Security Notes

When deploying the HTTP API:

- **Use HTTPS in production** - Deploy behind a reverse proxy (nginx, caddy) with TLS
- **Set a strong API key** - Always use `API_KEY` when exposing to networks
- **Rate limiting** - Consider rate limiting at the reverse proxy level
- Request body size is limited to 1MB by default

## Countries

```bash
olx-tracker --country pl search "iphone"
```

Supported: `pt` `pl` `ua` `ro` `bg` `kz` `uz`

## Notifications

```bash
olx-tracker --discord "https://discord.com/api/webhooks/..." run
olx-tracker --webhook "https://your-server.com/notify" run
olx-tracker --notify-new --notify-drops --notify-deals run
```

## Deals

```bash
olx-tracker --deal-threshold 30 run     # 30% below average = deal
olx-tracker --target-price 299 run      # anything <=299 = deal
```

## Proxy

```bash
olx-tracker --proxy "socks5://127.0.0.1:1080" search "iphone"
olx-tracker --proxy "http://user:pass@proxy.com:8080" run
```

## User Agent

```bash
olx-tracker --user-agent "Mozilla/5.0..." search "iphone"
```

## Database

```bash
olx-tracker --db /path/to/custom.db list
```

## Global Flags

| Flag | Description |
|------|-------------|
| `--config` | Config file path |
| `--db` | Database path |
| `--country` | OLX country |
| `--proxy` | Proxy URL (socks5/http) |
| `--user-agent` | Custom user agent |
| `--discord` | Discord webhook URL |
| `--webhook` | Generic webhook URL |
| `--deal-threshold` | % below avg for deals |
| `--target-price` | Max price for deals |
| `--notify-new` | Notify new listings |
| `--notify-drops` | Notify price drops |
| `--notify-deals` | Notify deals |

## Full Workflow Examples

### Example 1: Track iPhones in Paços de Ferreira with Discord alerts

```bash
# 1. Quick search to see what's available
olx-tracker search "iphone 14" --city "Paços de Ferreira" --radius 30 --min-price 200 --format markdown

# 2. Create a tracked search (30% below avg = deal, notify Discord)
olx-tracker add \
  --name "iPhone PF" \
  --keyword "iphone 14" \
  --city "Paços de Ferreira" \
  --radius 30 \
  --min-price 200 \
  --max-price 600 \
  --sort cheapest \
  --days 30

# 3. Run once with Discord notifications
olx-tracker \
  --discord "https://discord.com/api/webhooks/YOUR_WEBHOOK" \
  --deal-threshold 30 \
  --notify-new \
  --notify-deals \
  run

# 4. Or run as daemon (check every 15 min)
olx-tracker \
  --discord "https://discord.com/api/webhooks/YOUR_WEBHOOK" \
  --deal-threshold 30 \
  --notify-new \
  --notify-deals \
  daemon --interval 15
```

### Example 2: Track PS5 deals in Porto with webhook

```bash
# Create search
olx-tracker add \
  --name "PS5 Porto" \
  --keyword "playstation 5" \
  --city "Porto" \
  --radius 50 \
  --min-price 300 \
  --max-price 450 \
  --sort cheapest

# Run with webhook and target price
olx-tracker \
  --webhook "https://your-api.com/olx-notify" \
  --target-price 350 \
  --notify-deals \
  run --search-id 1
```

### Example 3: Quick JSON search for API/LLM integration

```bash
# Get results as JSON (includes images)
olx-tracker search "macbook pro" --max 5 --format json

# Get results as markdown (for LLMs)
olx-tracker search "macbook pro" --max 5 --format markdown
```

### Example 4: Full tracking with proxy

```bash
# Search through proxy
olx-tracker --proxy "socks5://127.0.0.1:1080" search "gaming laptop"

# Track with all bells and whistles
olx-tracker \
  --proxy "socks5://127.0.0.1:1080" \
  --discord "https://discord.com/api/webhooks/..." \
  --deal-threshold 25 \
  --notify-new \
  --notify-drops \
  --notify-deals \
  daemon --interval 30 --max-results 50
```

## Output Formats

| Format | Flag | Use Case |
|--------|------|----------|
| Table | `--format table` | CLI output (default) |
| JSON | `--format json` | APIs, scripts, includes images |
| Markdown | `--format markdown` | LLMs, docs, includes images |

JSON output includes: `id`, `title`, `price`, `city`, `region`, `seller`, `url`, `image`, `images[]`, `created_at`

## Features

- Fast Rust implementation
- Quick search without database
- SQLite storage with price history
- Discord & webhook notifications
- Smart deal detection
- Min/max price filtering
- Image URLs in JSON/markdown output
- Search TTL (auto-expire)
- Multi-country support (7 OLX regions)
- Location + radius filtering
- Daemon mode
- Proxy support (SOCKS5/HTTP)
- Random user agent rotation

## License

MIT
