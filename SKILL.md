---
name: olx-tracker
description: Search OLX.pt listings, track prices, and detect deals. Use for finding products, monitoring prices, and getting alerts on good deals in Portugal and other OLX regions.
metadata: {"openclaw":{"emoji":"🛒","requires":{"bins":["olx-tracker","jq"]},"install":[{"id":"cargo","kind":"cargo","crate":"olx-tracker","bins":["olx-tracker"],"label":"Install via Cargo"},{"id":"source","kind":"custom","command":"cargo build --release && cp target/release/olx-tracker /usr/local/bin/","label":"Build from source"}]}}
---

# OLX Tracker Skill

Search OLX.pt marketplace listings and track prices. Supports quick searches, tracked searches with price history, deal detection, and notifications.

## When to use (trigger phrases)

Use this skill when the user asks:

- "search OLX for..."
- "find [product] on OLX"
- "what's the price of [item] in Portugal?"
- "track prices for..."
- "monitor OLX listings"
- "find deals on..."
- "how much does [item] cost on OLX?"
- "search for [item] near [city]"
- Any mention of OLX, Portuguese marketplace, or second-hand items in Portugal/Poland/Ukraine/Romania/Bulgaria

## Quick Search (no database)

For one-off searches, use `search` command:

```bash
# Basic search
olx-tracker search "iphone 14"

# With filters
olx-tracker search "macbook" --max 10 --sort cheapest --min-price 500 --max-price 1500

# With location
olx-tracker search "ps5" --city "Porto" --radius 30
```

## Output Formats

**Use `--format json` for programmatic access or `--format markdown` for LLM-friendly output:**

```bash
# JSON (includes images array)
olx-tracker search "macbook pro" --max 5 --format json

# Markdown (optimized for LLMs)
olx-tracker search "macbook pro" --max 5 --format markdown
```

JSON fields: `id`, `title`, `price`, `city`, `region`, `seller`, `url`, `image`, `images[]`, `created_at`

## Search Options

| Flag | Description | Example |
|------|-------------|---------|
| `--max` | Max results (default: 20) | `--max 50` |
| `--sort` | newest, cheapest, expensive, relevance | `--sort cheapest` |
| `--min-price` | Minimum price | `--min-price 200` |
| `--max-price` | Maximum price | `--max-price 800` |
| `--city` | City name (auto-lookup) | `--city "Lisboa"` |
| `--radius` | Radius in km from city | `--radius 30` |
| `--keyword` | Additional keyword filter (must appear in title) | `--keyword "pro"` |
| `--category` | OLX category ID (filter by category) | `--category 179` |
| `--format` | table, json, markdown | `--format json` |

## Tracked Searches (with database)

For ongoing monitoring with price history:

```bash
# Add a tracked search
olx-tracker add --name "PS5 deals" --keyword "playstation 5" --min-price 300 --max-price 450

# Add with location and auto-expire
olx-tracker add --name "iPhone Porto" --keyword "iphone" --city "Porto" --radius 30 --days 14

# List tracked searches
olx-tracker list

# Run all searches (check for new listings)
olx-tracker run

# Run specific search
olx-tracker run --search-id 1

# View detected deals
olx-tracker deals

# View stats for a search
olx-tracker stats --search-id 1

# Toggle search on/off
olx-tracker toggle --search-id 1

# Remove a search
olx-tracker remove --search-id 1
```

## Deal Detection

Detect listings priced below market:

```bash
# Threshold-based: 30% below average = deal
olx-tracker --deal-threshold 30 run

# Target price: anything at or below this price = deal
olx-tracker --target-price 299 run

# Combined with notifications
olx-tracker --deal-threshold 25 --notify-deals run
```

## Daemon Mode

Run continuously in background:

```bash
# Check every 30 min (default)
olx-tracker daemon

# Custom interval
olx-tracker daemon --interval 15

# With notifications
olx-tracker --discord "https://discord.com/api/webhooks/..." --notify-new --notify-deals daemon
```

## Notifications

Send alerts via Discord or webhook:

```bash
# Discord webhook
olx-tracker --discord "https://discord.com/api/webhooks/..." run

# Generic webhook (POST JSON)
olx-tracker --webhook "https://your-server.com/notify" run

# Choose what to notify
olx-tracker --notify-new --notify-drops --notify-deals run
```

## Multi-Country Support

```bash
# Portugal (default)
olx-tracker search "iphone"

# Poland
olx-tracker --country pl search "iphone"

# Other countries: ua, ro, bg, kz, uz
olx-tracker --country ro search "laptop"
```

## Common Workflows

### Find cheapest listings in a city

```bash
olx-tracker search "macbook pro m3" --city "Lisboa" --radius 50 --sort cheapest --max 20 --format markdown
```

### Track deals with Discord alerts

```bash
# 1. Create tracked search
olx-tracker add --name "PS5 Porto" --keyword "playstation 5" --city "Porto" --radius 30 --max-price 400

# 2. Run with deal detection and notifications
olx-tracker --discord "https://discord.com/api/webhooks/..." --deal-threshold 30 --notify-deals run
```

### Quick market research

```bash
# See price distribution
olx-tracker search "iphone 15 pro" --max 50 --sort cheapest --format json | jq '[.[] | .price] | {min: min, max: max, avg: (add/length)}'
```

### Background monitoring

```bash
# Start daemon with full notifications
olx-tracker \
  --discord "https://discord.com/api/webhooks/..." \
  --deal-threshold 25 \
  --notify-new \
  --notify-deals \
  daemon --interval 15
```

## HTTP API (serve command)

For programmatic access, run as HTTP server:

```bash
# Start server
API_KEY=mysecret olx-tracker serve --port 8080

# Search via API
curl -X POST http://localhost:8080/search \
  -H "x-api-key: mysecret" \
  -H "Content-Type: application/json" \
  -d '{"keyword": "iphone", "max_results": 10}'
```

Endpoints:
- `GET /health` - Health check
- `POST /search` - Quick search
- `POST /searches/add` - Add tracked search
- `POST /searches/list` - List searches
- `POST /searches/run` - Run searches
- `POST /searches/deals` - Get deals
- `POST /searches/daemon` - Start daemon
- `POST /searches/daemon/stop` - Stop daemon

## Tips

- Use `--format markdown` when feeding results to LLMs
- Use `--format json` for scripts and automation
- City names are auto-looked up, spaces work: `--city "Paços de Ferreira"`
- Combine `--min-price` and `--max-price` to filter noise
- `--days N` auto-expires tracked searches after N days
- Database defaults to `olx_tracker.db` in current directory, override with `--db`

## Proxy Support

```bash
# SOCKS5 proxy
olx-tracker --proxy "socks5://127.0.0.1:1080" search "laptop"

# HTTP proxy with auth
olx-tracker --proxy "http://user:pass@proxy.com:8080" search "laptop"
```

## Output Interpretation

### JSON Structure

Each listing in JSON output has this structure:

```json
{
  "id": "12345678",
  "title": "iPhone 14 Pro 128GB",
  "price": 750,
  "city": "Porto",
  "region": "Porto",
  "seller": "user123",
  "url": "https://www.olx.pt/d/anuncio/...",
  "image": "https://ireland.apollo.olxcdn.com/...",
  "images": ["https://...1.jpg", "https://...2.jpg"],
  "created_at": "2026-01-30T14:32:00Z"
}
```

### Parsing Examples

```bash
# Extract just prices for analysis
olx-tracker search "iphone" --format json | jq '[.[].price]'

# Get cheapest listing
olx-tracker search "ps5" --sort cheapest --max 1 --format json | jq '.[0]'

# Filter by price range in post-processing
olx-tracker search "macbook" --format json | jq '[.[] | select(.price >= 500 and .price <= 1000)]'

# Get URLs only
olx-tracker search "bike" --format json | jq -r '.[].url'

# Calculate price statistics
olx-tracker search "iphone 15" --max 50 --format json | jq '{
  count: length,
  min: [.[].price] | min,
  max: [.[].price] | max,
  avg: ([.[].price] | add / length) | floor,
  median: ([.[].price] | sort | .[length/2 | floor])
}'
```

## Error Handling

### Common Errors

| Error | Cause | Fix |
| ----- | ----- | --- |
| `No results found` | Search too specific or no listings | Broaden search terms, remove filters |
| `City not found` | Invalid city name | Check spelling, try region name |
| `Rate limited` | Too many requests | Wait 1-2 minutes, use proxy |
| `Connection refused` | Network/proxy issue | Check proxy settings, try without proxy |
| `Invalid bearer token` | Config issue | Check `config.toml` has valid token |

### Retry Strategy

If a search fails, try:

1. Simplify the search query (fewer words)
2. Remove location filters
3. Increase `--max` to get more results
4. Wait and retry (rate limiting)
5. Use a proxy if blocked

## Agent Guidelines

### Best Practices

1. **Always use `--format json`** for programmatic processing
2. **Use `--format markdown`** when presenting results to users in chat
3. **Limit results** with `--max 10-20` for quick responses
4. **Sort by relevance** first, then re-sort if user needs cheapest/newest
5. **Combine filters** to reduce noise: `--min-price` + `--max-price` + `--city`

### Response Formatting

When presenting results to users:

- Show top 3-5 most relevant listings
- Include: title, price, city, and URL
- Mention total results found
- Highlight any deals (unusually low prices)
- Offer to show more or refine search

### Price Intelligence

To help users find deals:

```bash
# 1. Get market overview (50 listings)
olx-tracker search "product" --max 50 --format json > /tmp/market.json

# 2. Calculate price distribution
cat /tmp/market.json | jq '{
  listings: length,
  price_range: "\([.[].price] | min) - \([.[].price] | max) EUR",
  average: ([.[].price] | add / length) | floor,
  deal_threshold: (([.[].price] | add / length) * 0.7) | floor
}'

# 3. Find listings below threshold
cat /tmp/market.json | jq --argjson threshold 500 '[.[] | select(.price <= $threshold)]'
```

## Ideas to Try

- **Price alerts**: Set up tracked search with `--target-price` for instant deal notifications
- **Market research**: Compare prices across cities with multiple searches
- **Flip opportunities**: Track items selling below market value
- **Inventory monitoring**: Watch for restocks of rare items
- **Competitor analysis**: Monitor seller listings over time
- **Seasonal tracking**: Use `--days 30` to track short-term price trends

## Rate Limits

- OLX API has undocumented rate limits (~100 requests/minute estimated)
- Use delays between batch searches
- Daemon mode has built-in intervals (default 30 min)
- If rate limited, wait 1-2 minutes before retrying
- Use proxy rotation for high-volume monitoring

## Configuration

Optional `config.toml` for persistent settings:

```toml
# Bearer token from OLX (get from browser DevTools)
bearer_token = "your-token-here"

# Default country
country = "pt"

# Notification settings
[notifications]
discord_webhook = "https://discord.com/api/webhooks/..."
notify_new = true
notify_deals = true

# Deal detection
[deals]
threshold = 30  # 30% below average
```

Place in current directory or specify with `--config /path/to/config.toml`.
