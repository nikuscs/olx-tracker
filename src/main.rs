use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};
use clap::{Parser, Subcommand};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use olx_tracker::api::SearchParams;
use olx_tracker::{
    format_results, Config, Database, FilterChain, MultiNotifier, Notifier, OlxClient,
    OutputFormat, SearchTracker, SortOrder,
};

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

        /// Output format: table, json, markdown (or md/llm)
        #[arg(long, default_value = "table")]
        format: String,
    },
}

#[tokio::main]
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
        Commands::Add { name, keyword, min_price, max_price, city, radius, category, sort, days } => {
            cmd_add(&db, &name, &keyword, min_price, max_price, city, radius, category, &sort, days)?;
        }
        Commands::List { all } => {
            cmd_list(&db, all)?;
        }
        Commands::Run { search_id, max_results } => {
            cmd_run(&db, &config, search_id, max_results).await?;
        }
        Commands::Daemon { interval, max_results } => {
            cmd_daemon(&db, &config, interval, max_results).await?;
        }
        Commands::Deals { search_id } => {
            cmd_deals(&db, search_id)?;
        }
        Commands::Stats { search_id } => {
            cmd_stats(&db, search_id)?;
        }
        Commands::Remove { search_id } => {
            cmd_remove(&db, search_id)?;
        }
        Commands::Toggle { search_id } => {
            cmd_toggle(&db, search_id)?;
        }
        Commands::Search { query, max, sort, min_price, max_price, city, radius, format } => {
            cmd_search(&config, &query, max, &sort, min_price, max_price, city, radius, &format).await?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_add(
    db: &Database,
    name: &str,
    keyword: &str,
    min_price: Option<f64>,
    max_price: Option<f64>,
    city: Option<String>,
    radius: Option<i32>,
    category: Option<i64>,
    sort: &str,
    days: Option<i64>,
) -> Result<()> {
    // Validate sort order
    let _: SortOrder = sort.parse().map_err(|e: String| anyhow::anyhow!("{e}"))?;

    // Calculate expires_at if days is specified
    let expires_at = days.map(|d| {
        let expires = Utc::now() + ChronoDuration::days(d);
        expires.to_rfc3339()
    });

    let id = db.create_search(
        name,
        keyword,
        min_price,
        max_price,
        city.as_deref(),
        radius,
        category,
        Some(sort),
        expires_at.as_deref(),
    )?;

    let price_info = match (min_price, max_price) {
        (Some(min), Some(max)) => format!(", price: {min:.0}€-{max:.0}€"),
        (Some(min), None) => format!(", min: {min:.0}€"),
        (None, Some(max)) => format!(", max: {max:.0}€"),
        (None, None) => String::new(),
    };
    let ttl_info = days.map_or(String::new(), |d| format!(", expires in {d} days"));
    println!("Created search '{name}' with ID {id} (sort: {sort}{price_info}{ttl_info})");
    Ok(())
}

fn cmd_list(db: &Database, include_inactive: bool) -> Result<()> {
    let searches = db.list_searches(!include_inactive)?;

    if searches.is_empty() {
        println!("No searches found. Use 'olx-tracker add' to create one.");
        return Ok(());
    }

    println!(
        "{:<4} {:<18} {:<18} {:<12} {:<12} {:<10} {:<6}",
        "ID", "Name", "Keyword", "Price", "City", "Sort", "Active"
    );
    println!("{}", "-".repeat(90));

    for search in searches {
        let price_range = match (search.min_price, search.max_price) {
            (Some(min), Some(max)) => format!("{min:.0}-{max:.0}€"),
            (Some(min), None) => format!(">{min:.0}€"),
            (None, Some(max)) => format!("<{max:.0}€"),
            (None, None) => "-".to_string(),
        };
        let city = search.city.as_deref().unwrap_or("-");
        let active = if search.active { "Yes" } else { "No" };

        println!(
            "{:<4} {:<18} {:<18} {:<12} {:<12} {:<10} {:<6}",
            search.id,
            truncate(&search.name, 16),
            truncate(&search.keyword, 16),
            truncate(&price_range, 10),
            truncate(city, 10),
            truncate(&search.sort_order, 8),
            active
        );
    }

    Ok(())
}

async fn cmd_run(
    db: &Database,
    config: &Config,
    search_id: Option<i64>,
    max_results: i32,
) -> Result<()> {
    let client = OlxClient::new(config)?;
    let filters = FilterChain::with_defaults();
    let tracker = SearchTracker::new(db, &client, config.deals.clone()).with_filters(filters);
    let notifier = MultiNotifier::from_config(config.notifications.clone());

    let results = if let Some(id) = search_id {
        let search =
            db.get_search(id)?.ok_or_else(|| anyhow::anyhow!("Search with ID {id} not found"))?;
        vec![tracker.run_search(&search, max_results).await?]
    } else {
        tracker.run_all_searches(max_results).await?
    };

    // Process results and send notifications
    for result in &results {
        let stats = db.get_search_stats(result.search_id)?;
        let avg_price = stats.and_then(|s| s.avg_price);

        if !result.new_listings.is_empty() {
            println!(
                "Found {} new listing(s) for search {}",
                result.new_listings.len(),
                result.search_id
            );
            notifier.notify_new_listings(&result.new_listings).await?;
        }

        if !result.price_drops.is_empty() {
            println!(
                "Found {} price drop(s) for search {}",
                result.price_drops.len(),
                result.search_id
            );
            notifier.notify_price_drops(&result.price_drops).await?;
        }

        if !result.deals.is_empty() {
            println!("Found {} deal(s) for search {}", result.deals.len(), result.search_id);
            notifier.notify_deals(&result.deals, avg_price).await?;
        }
    }

    let total_new: usize = results.iter().map(|r| r.new_listings.len()).sum();
    let total_deals: usize = results.iter().map(|r| r.deals.len()).sum();
    println!("\nRun complete: {total_new} new listings, {total_deals} deals found");

    Ok(())
}

async fn cmd_daemon(
    db: &Database,
    config: &Config,
    interval_mins: u64,
    max_results: i32,
) -> Result<()> {
    info!("Starting daemon mode, checking every {} minutes", interval_mins);
    let interval = Duration::from_secs(interval_mins * 60);

    loop {
        match cmd_run(db, config, None, max_results).await {
            Ok(()) => info!("Check complete"),
            Err(e) => error!("Check failed: {}", e),
        }

        info!("Sleeping for {} minutes...", interval_mins);
        tokio::time::sleep(interval).await;
    }
}

fn cmd_deals(db: &Database, search_id: Option<i64>) -> Result<()> {
    let deals = db.get_deals(search_id)?;

    if deals.is_empty() {
        println!("No deals found.");
        return Ok(());
    }

    println!("Found {} deal(s):\n", deals.len());
    println!("{:<8} {:<40} {:<10} {:<15}", "ID", "Title", "Price", "City");
    println!("{}", "-".repeat(75));

    for deal in deals {
        let price = deal.price.map_or_else(|| "-".to_string(), |p| format!("{p:.2} €"));
        let city = deal.city.as_deref().unwrap_or("-");

        println!(
            "{:<8} {:<40} {:<10} {:<15}",
            deal.id,
            truncate(&deal.title, 38),
            price,
            truncate(city, 13)
        );
        println!("         {}", deal.url);
    }

    Ok(())
}

fn cmd_stats(db: &Database, search_id: i64) -> Result<()> {
    let search = db
        .get_search(search_id)?
        .ok_or_else(|| anyhow::anyhow!("Search with ID {search_id} not found"))?;

    let stats = db.update_search_stats(search_id)?;

    println!("Statistics for '{}' (ID: {})", search.name, search.id);
    println!("{}", "-".repeat(40));
    println!("Keyword:         {}", search.keyword);
    println!("Max price:       {}", fmt_price(search.max_price));
    println!("City:            {}", search.city.as_deref().unwrap_or("-"));
    println!(
        "Radius:          {}",
        search.radius_km.map_or_else(|| "-".to_string(), |r| format!("{r} km"))
    );
    println!();
    println!("Total listings:  {}", stats.total_listings);
    println!("Average price:   {}", fmt_price(stats.avg_price));
    println!("Min price:       {}", fmt_price(stats.min_price));
    println!("Max price:       {}", fmt_price(stats.max_price));
    println!("Last updated:    {}", stats.last_updated_at.as_deref().unwrap_or("-"));

    Ok(())
}

fn cmd_remove(db: &Database, search_id: i64) -> Result<()> {
    if db.delete_search(search_id)? {
        println!("Removed search with ID {search_id}");
    } else {
        println!("Search with ID {search_id} not found");
    }
    Ok(())
}

fn cmd_toggle(db: &Database, search_id: i64) -> Result<()> {
    let search = db
        .get_search(search_id)?
        .ok_or_else(|| anyhow::anyhow!("Search with ID {search_id} not found"))?;

    let new_status = !search.active;
    db.set_search_active(search_id, new_status)?;

    println!("Search '{}' is now {}", search.name, if new_status { "active" } else { "inactive" });
    Ok(())
}

async fn cmd_search(
    config: &Config,
    query: &str,
    max_results: i32,
    sort: &str,
    min_price: Option<f64>,
    max_price: Option<f64>,
    city: Option<String>,
    radius: Option<i32>,
    format: &str,
) -> Result<()> {
    let sort_order: SortOrder = sort.parse().map_err(|e: String| anyhow::anyhow!("{e}"))?;
    let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!("{e}"))?;

    let client = OlxClient::new(config)?;

    // Lookup city ID if city name provided
    let city_id = if let Some(ref city_name) = city {
        let location = client.lookup_city(city_name).await?;
        match location {
            Some(loc) => {
                info!("Found city: {} (ID: {})", loc.city.name, loc.city.id.unwrap_or(0));
                loc.city.id
            }
            None => {
                anyhow::bail!("City not found: {city_name}");
            }
        }
    } else {
        None
    };

    let params = SearchParams {
        query: query.to_string(),
        city_id,
        radius_km: radius,
        category_id: None,
        sort: sort_order,
        offset: 0,
        limit: 50,
    };

    // Fetch more results to account for filtering
    let fetch_count = max_results * 3; // Fetch 3x to have enough after filtering
    let all_offers = client.search_all(&params, fetch_count).await?;

    // Apply price filters
    let offers: Vec<_> = all_offers
        .into_iter()
        .filter(|o| {
            let price = o.get_price();
            match (price, min_price, max_price) {
                (None, _, _) => true, // Keep items without price
                (Some(p), Some(min), Some(max)) => p >= min && p <= max,
                (Some(p), Some(min), None) => p >= min,
                (Some(p), None, Some(max)) => p <= max,
                (Some(_), None, None) => true,
            }
        })
        .take(max_results as usize)
        .collect();

    if offers.is_empty() {
        println!("No results found for '{query}'");
        if min_price.is_some() || max_price.is_some() {
            println!(
                "(price filter: {} - {})",
                min_price.map_or("any".to_string(), |p| format!("{p:.0}€")),
                max_price.map_or("any".to_string(), |p| format!("{p:.0}€"))
            );
        }
        return Ok(());
    }

    let output = format_results(
        output_format,
        query,
        sort,
        &offers,
        min_price,
        max_price,
        city,
        radius,
    );

    print!("{output}");

    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{truncated}...")
    }
}

fn fmt_price(price: Option<f64>) -> String {
    price.map_or_else(|| "-".to_string(), |p| format!("{p:.2} €"))
}
