# 🔍 OLX Price Tracker

Fast Rust CLI to track OLX listings and alert on good deals.

## ⚡ Install

```bash
cargo build --release
cp config.example.toml config.toml
```

## 🔎 Quick Search

```bash
olx-tracker search "iphone 14"
olx-tracker search "macbook" -m 10 -s cheapest
olx-tracker search "ps5" --min-price 300 --max-price 500
olx-tracker search "bike" --city Porto -r 30
olx-tracker search "AYN odin" -s relevance --min-price 200 -m 10
```

| Flag | Description |
|------|-------------|
| `-m, --max` | Max results (default: 20) |
| `-s, --sort` | newest, cheapest, expensive, relevance |
| `--min-price` | Minimum price filter |
| `--max-price` | Maximum price filter |
| `--city` | City name |
| `-r, --radius` | Radius in km |

## 📋 Tracked Searches

```bash
# Add
olx-tracker add -n "PS5" -k "playstation 5"
olx-tracker add -n "PS5 deals" -k "ps5" --min-price 300 -p 450 -s cheapest
olx-tracker add -n "PS5 Porto" -k "ps5" --city Porto -r 30

# Manage
olx-tracker list
olx-tracker list -a                    # include inactive
olx-tracker toggle -s 1                # toggle active
olx-tracker remove -s 1
olx-tracker stats -s 1
olx-tracker deals
olx-tracker deals -s 1
```

| Flag | Description |
|------|-------------|
| `-n, --name` | Search name |
| `-k, --keyword` | Search keyword |
| `--min-price` | Min price filter |
| `-p, --max-price` | Max price (deal threshold) |
| `-s, --sort` | newest, cheapest, expensive, relevance |
| `--city` | City name |
| `-r, --radius` | Radius in km |
| `--category` | OLX category ID |

## 🔄 Run & Daemon

```bash
olx-tracker run
olx-tracker run -s 1 -m 50             # specific search, max 50 results
olx-tracker daemon                      # every 30 min
olx-tracker daemon -i 15 -m 100        # every 15 min, max 100 results
```

## 🌍 Countries

```bash
olx-tracker --country pl search "iphone"
export OLX_COUNTRY=ua
```

Supported: `pt` `pl` `ua` `ro` `bg` `kz` `uz`

## 🔔 Notifications

```bash
# Discord
olx-tracker --discord "https://discord.com/api/webhooks/ID/TOKEN" run
export OLX_DISCORD_WEBHOOK="https://discord.com/api/webhooks/..."

# Generic webhook
olx-tracker --webhook "https://your-server.com/notify" run
export OLX_WEBHOOK="https://..."

# Flags
olx-tracker --notify-new --notify-drops --notify-deals run
```

## 🎯 Deals

```bash
olx-tracker --deal-threshold 30 run    # 30% below average
olx-tracker --target-price 299 run     # anything ≤299€
export OLX_DEAL_THRESHOLD=25
export OLX_TARGET_PRICE=350
```

## 🌐 Proxy

In `config.toml`:

```toml
[proxy]
enabled = true
url = "socks5://127.0.0.1:1080"
# or
url = "http://user:pass@proxy.example.com:8080"
```

Supports: `socks5://`, `http://`, `https://`

## 🗄️ Database

```bash
olx-tracker --db /path/to/custom.db list
export OLX_TRACKER_DB=/path/to/custom.db
```

## 🔧 Config

```toml
[api]
country = "pt"
user_agent = "Mozilla/5.0..."
request_delay_ms = 1000

[proxy]
enabled = false
url = "socks5://127.0.0.1:1080"

[notifications]
webhook_url = "https://..."
discord_webhook_url = "https://discord.com/api/webhooks/..."
notify_on_new_listing = true
notify_on_price_drop = true
notify_on_deal = true

[deals]
threshold_pct = 20.0
target_price = 299.0

[database]
path = "olx_tracker.db"
```

## 📖 Global Flags

| Flag | Env | Description |
|------|-----|-------------|
| `-c, --config` | - | Config file |
| `-d, --db` | `OLX_TRACKER_DB` | Database path |
| `--country` | `OLX_COUNTRY` | OLX country |
| `--discord` | `OLX_DISCORD_WEBHOOK` | Discord webhook |
| `--webhook` | `OLX_WEBHOOK` | Generic webhook |
| `--deal-threshold` | `OLX_DEAL_THRESHOLD` | % below avg |
| `--target-price` | `OLX_TARGET_PRICE` | Target price |
| `--notify-new` | - | Notify new listings |
| `--notify-drops` | - | Notify price drops |
| `--notify-deals` | - | Notify deals |

## 📄 License

MIT
