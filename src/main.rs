use std::path::Path;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use olx_tracker::Config;
use olx_tracker::Database;
use olx_tracker::server;

mod commands;

#[derive(Parser)]
#[command(name = "olx-tracker")]
#[command(about = "Track OLX listings, monitor prices, and alert on good deals")]
#[command(version)]
struct Cli {
    /// Path to config file (optional if using CLI flags)
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Path to database file
    #[arg(short, long, env = "OLX_TRACKER_DB")]
    db: Option<String>,

    /// OLX country (pt, pl, ua, ro, bg, kz, uz)
    #[arg(long, env = "OLX_COUNTRY")]
    country: Option<String>,

    /// Discord webhook URL for notifications
    #[arg(long, env = "OLX_DISCORD_WEBHOOK")]
    discord: Option<String>,

    /// Generic webhook URL for notifications
    #[arg(long, env = "OLX_WEBHOOK")]
    webhook: Option<String>,

    /// Deal threshold percentage below average (e.g., 30 = 30% below avg)
    #[arg(long, env = "OLX_DEAL_THRESHOLD")]
    deal_threshold: Option<f64>,

    /// Target price - any listing at or below this is a deal
    #[arg(long, env = "OLX_TARGET_PRICE")]
    target_price: Option<f64>,

    /// Notify on new listings
    #[arg(long)]
    notify_new: bool,

    /// Notify on price drops
    #[arg(long)]
    notify_drops: bool,

    /// Notify on deals
    #[arg(long)]
    notify_deals: bool,

    /// Proxy URL (socks5://host:port or http://host:port)
    #[arg(long)]
    proxy: Option<String>,

    /// Custom user agent string
    #[arg(long)]
    user_agent: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new search to track
    Add {
        /// Name for this search (e.g., "PS2 cheap")
        #[arg(long)]
        name: String,

        /// Search keyword (e.g., "playstation 2")
        #[arg(long)]
        keyword: String,

        /// Minimum price filter (ignore cheaper junk results)
        #[arg(long)]
        min_price: Option<f64>,

        /// Maximum price threshold for deals (per-search)
        #[arg(long)]
        max_price: Option<f64>,

        /// City to search in
        #[arg(long)]
        city: Option<String>,

        /// Search radius in km (requires city)
        #[arg(long)]
        radius: Option<i32>,

        /// OLX category ID
        #[arg(long)]
        category: Option<i64>,

        /// Sort order: newest, cheapest, expensive, relevance
        #[arg(long, default_value = "newest")]
        sort: String,

        /// Expire search after N days (stops scanning, keeps data)
        #[arg(long)]
        days: Option<i64>,
    },

    /// List all saved searches
    List {
        /// Show all searches including inactive ones
        #[arg(short, long)]
        all: bool,
    },

    /// Run a check on searches
    Run {
        /// Run only a specific search by ID
        #[arg(long)]
        search_id: Option<i64>,

        /// Maximum results per search
        #[arg(long, default_value = "100")]
        max_results: i32,
    },

    /// Start daemon mode (checks periodically)
    Daemon {
        /// Check interval in minutes
        #[arg(long, default_value = "30")]
        interval: u64,

        /// Maximum results per search
        #[arg(long, default_value = "100")]
        max_results: i32,
    },

    /// Show deals (listings below average price or target price)
    Deals {
        /// Filter by search ID
        #[arg(long)]
        search_id: Option<i64>,
    },

    /// Show price statistics for a search
    Stats {
        /// Search ID to show stats for
        #[arg(long)]
        search_id: i64,
    },

    /// Remove a search
    Remove {
        /// Search ID to remove
        #[arg(long)]
        search_id: i64,
    },

    /// Toggle search active status
    Toggle {
        /// Search ID to toggle
        #[arg(long)]
        search_id: i64,
    },

    /// Quick search OLX (no database, just display results)
    Search {
        /// Search query (e.g., "playstation 2")
        query: String,

        /// Maximum results to show
        #[arg(long, default_value = "20")]
        max: i32,

        /// Sort order: newest, cheapest, expensive, relevance
        #[arg(long, default_value = "relevance")]
        sort: String,

        /// Minimum price filter (ignore cheaper junk)
        #[arg(long)]
        min_price: Option<f64>,

        /// Maximum price filter
        #[arg(long)]
        max_price: Option<f64>,

        /// City to search in
        #[arg(long)]
        city: Option<String>,

        /// Search radius in km (requires city)
        #[arg(long)]
        radius: Option<i32>,

        /// Additional keyword filter (must appear in title)
        #[arg(long)]
        keyword: Option<String>,

        /// OLX category ID (filter by category)
        #[arg(long)]
        category: Option<i64>,

        /// Output format: table, json, markdown (or md/llm)
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Start HTTP server exposing the CLI actions
    Serve {
        /// Bind address (e.g., 127.0.0.1)
        #[arg(long, default_value = "127.0.0.1")]
        host: String,

        /// Port to listen on
        #[arg(long, default_value = "8080")]
        port: u16,

        /// API key required for requests (optional). If not provided, no auth is required.
        #[arg(long, env = "API_KEY", value_name = "API_KEY")]
        api_key: Option<String>,

        /// Request timeout in seconds for long-running operations
        #[arg(long, default_value = "60")]
        timeout: u64,
    },
}

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    // Load config from file if it exists, otherwise use defaults
    let mut config = if Path::new(&cli.config).exists() {
        Config::load(&cli.config)?
    } else {
        Config::minimal()
    };

    // Override config with CLI flags
    if let Some(country_str) = &cli.country {
        config.api.country = country_str.parse().map_err(|e: String| anyhow::anyhow!("{e}"))?;
    }

    if let Some(discord_url) = &cli.discord {
        config.notifications.discord_webhook_url = Some(discord_url.clone());
    }

    if let Some(webhook_url) = &cli.webhook {
        config.notifications.webhook_url = Some(webhook_url.clone());
    }

    if let Some(threshold) = cli.deal_threshold {
        config.deals.threshold_pct = threshold;
    }

    if let Some(target) = cli.target_price {
        config.deals.target_price = Some(target);
    }

    // CLI flags override config for notifications
    if cli.notify_new {
        config.notifications.notify_on_new_listing = true;
    }
    if cli.notify_drops {
        config.notifications.notify_on_price_drop = true;
    }
    if cli.notify_deals {
        config.notifications.notify_on_deal = true;
    }

    // Proxy and user-agent
    if let Some(proxy_url) = &cli.proxy {
        config.proxy.enabled = true;
        config.proxy.url = Some(proxy_url.clone());
    }
    if let Some(ua) = &cli.user_agent {
        config.api.user_agent = ua.clone();
    }

    // Use CLI db path if provided, otherwise use config
    let db_path = cli.db.as_deref().unwrap_or(&config.database.path);
    let db = Database::open(db_path)?;

    match cli.command {
        Commands::Add {
            name,
            keyword,
            min_price,
            max_price,
            city,
            radius,
            category,
            sort,
            days,
        } => {
            commands::cmd_add(
                &db, &name, &keyword, min_price, max_price, city, radius, category, &sort, days,
            )?;
        }
        Commands::List { all } => {
            commands::cmd_list(&db, all)?;
        }
        Commands::Run { search_id, max_results } => {
            commands::cmd_run(&db, &config, search_id, max_results).await?;
        }
        Commands::Daemon { interval, max_results } => {
            commands::cmd_daemon(&db, &config, interval, max_results).await?;
        }
        Commands::Deals { search_id } => {
            commands::cmd_deals(&db, search_id)?;
        }
        Commands::Stats { search_id } => {
            commands::cmd_stats(&db, search_id)?;
        }
        Commands::Remove { search_id } => {
            commands::cmd_remove(&db, search_id)?;
        }
        Commands::Toggle { search_id } => {
            commands::cmd_toggle(&db, search_id)?;
        }
        Commands::Search {
            query,
            max,
            sort,
            min_price,
            max_price,
            city,
            radius,
            keyword,
            category,
            format,
        } => {
            commands::cmd_search(
                &config, &query, max, &sort, min_price, max_price, city, radius, keyword, category,
                &format,
            )
            .await?;
        }
        Commands::Serve { host, port, api_key, timeout } => {
            server::serve_with_timeout(db_path.to_string(), &config, host, port, api_key, timeout)
                .await?;
        }
    }

    Ok(())
}
