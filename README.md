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
olx-tracker list
olx-tracker toggle -s 1
olx-tracker remove -s 1
olx-tracker stats -s 1
olx-tracker deals
```

## 🔄 Run & Daemon

```bash
olx-tracker run
olx-tracker run -s 1 -m 50
olx-tracker daemon
olx-tracker daemon -i 15 -m 100
```

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
olx-tracker --deal-threshold 30 run
olx-tracker --target-price 299 run
```

## 🌐 Proxy

```bash
olx-tracker --proxy "socks5://127.0.0.1:1080" search "iphone"
olx-tracker --proxy "http://user:pass@proxy.com:8080" run
```

## 🔧 User Agent

```bash
olx-tracker --user-agent "Mozilla/5.0 (Windows NT 10.0; Win64; x64)" search "iphone"
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
- 🌍 Multi-country support (7 OLX regions)
- 📍 Location + radius filtering
- 🔄 Daemon mode
- 🌐 Proxy support (SOCKS5/HTTP)

## 📄 License

MIT
