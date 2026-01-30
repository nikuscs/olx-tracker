# 🔍 OLX Price Tracker

Fast Rust CLI to track OLX listings and alert on good deals.

## ⚡ Install

```bash
cargo build --release
```

## 🔎 Quick Search

```bash
olx-tracker search "iphone 14"
olx-tracker search "macbook" -m 10 -s cheapest
olx-tracker search "ps5" --min-price 300 --max-price 500
olx-tracker search "bike" --city Porto -r 30
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
olx-tracker add -n "PS5" -k "playstation 5"
olx-tracker add -n "PS5 deals" -k "ps5" --min-price 300 -p 450 -s cheapest
olx-tracker add -n "PS5 Porto" -k "ps5" --city Porto -r 30
olx-tracker add -n "PS5 temp" -k "ps5" --days 7              # expires in 7 days
olx-tracker list
olx-tracker toggle -s 1
olx-tracker remove -s 1
olx-tracker stats -s 1
olx-tracker deals
```

| Flag | Description |
|------|-------------|
| `-n, --name` | Search name |
| `-k, --keyword` | Search keyword |
| `--min-price` | Min price filter |
| `-p, --max-price` | Max price filter |
| `-s, --sort` | newest, cheapest, expensive, relevance |
| `--city` | City name |
| `-r, --radius` | Radius in km |
| `--days` | Auto-expire after N days |

## 🔄 Run & Daemon

```bash
olx-tracker run                         # run all searches once
olx-tracker run -s 1                    # run search ID 1 only
olx-tracker run -m 50                   # max 50 results per search

olx-tracker daemon                      # check every 30 min
olx-tracker daemon -i 15                # check every 15 min
olx-tracker daemon -i 60 -m 100         # every 60 min, max 100 results
```

| Flag | Description |
|------|-------------|
| `-s, --search-id` | Run specific search only |
| `-m, --max-results` | Max results per search (default: 100) |
| `-i, --interval` | Check interval in minutes (default: 30) |

## 🌍 Countries

```bash
olx-tracker --country pl search "iphone"
```

Supported: `pt` `pl` `ua` `ro` `bg` `kz` `uz`

## 🔔 Notifications

```bash
olx-tracker --discord "https://discord.com/api/webhooks/..." run
olx-tracker --webhook "https://your-server.com/notify" run
olx-tracker --notify-new --notify-drops --notify-deals run
```

## 🎯 Deals

```bash
olx-tracker --deal-threshold 30 run     # 30% below average = deal
olx-tracker --target-price 299 run      # anything ≤299€ = deal
```

## 🌐 Proxy

```bash
olx-tracker --proxy "socks5://127.0.0.1:1080" search "iphone"
olx-tracker --proxy "http://user:pass@proxy.com:8080" run
```

## 🔧 User Agent

```bash
olx-tracker --user-agent "Mozilla/5.0..." search "iphone"
```

## 🗄️ Database

```bash
olx-tracker --db /path/to/custom.db list
```

## 📖 Global Flags

| Flag | Description |
|------|-------------|
| `-c, --config` | Config file path |
| `-d, --db` | Database path |
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

## 📦 Features

- ⚡ Fast Rust implementation
- 🔎 Quick search without database
- 💾 SQLite storage with price history
- 🔔 Discord & webhook notifications
- 🎯 Smart deal detection
- 💰 Min/max price filtering
- ⏰ Search TTL (auto-expire)
- 🌍 Multi-country support (7 OLX regions)
- 📍 Location + radius filtering
- 🔄 Daemon mode
- 🌐 Proxy support (SOCKS5/HTTP)

## 📄 License

MIT
