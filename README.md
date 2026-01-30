# 🔍 OLX Price Tracker

Fast Rust CLI to track OLX.pt listings and alert on good deals.

## ⚡ Quick Start

```bash
# Build
cargo build --release

# Setup config
cp config.example.toml config.toml
# Edit config.toml with your bearer token (from browser DevTools)

# Add a search
./target/release/olx-tracker add -n "PS2 cheap" -k "playstation 2" -p 200 --city Porto

# Run once
./target/release/olx-tracker run

# Run as daemon (every 30 min)
./target/release/olx-tracker daemon -i 30
```

## 📋 Commands

| Command | Description |
|---------|-------------|
| `add` | Add a new search to track |
| `list` | List all saved searches |
| `run` | Run a one-shot check |
| `daemon` | Start daemon mode |
| `deals` | Show listings below avg price |
| `stats` | Show price statistics |
| `remove` | Remove a search |
| `toggle` | Toggle search active/inactive |

## 🗄️ Database

Default: `olx_tracker.db` in current directory.

Override with:
```bash
olx-tracker --db /path/to/my.db list
# or
export OLX_TRACKER_DB=/path/to/my.db
```

## 🔧 Config

See `config.example.toml` for all options:
- Bearer token (required)
- Proxy support (socks5/http)
- Webhook notifications
- Rate limiting

## 📦 Features

- 🚀 Fast Rust implementation
- 💾 SQLite storage with price history
- 🔔 Webhook notifications
- 🎯 Deal detection (price < avg)
- 🔄 Daemon mode
- 🌐 Proxy support
