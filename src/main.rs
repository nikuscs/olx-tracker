use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use olx_tracker::{
    Config, Database, FilterChain, Notifier, OlxClient, SearchTracker, WebhookNotifier,
};

#[derive(Parser)]
#[command(name = "olx-tracker")]
#[command(about = "Track OLX.pt listings, monitor prices, and alert on good deals")]
#[command(version)]
struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.toml")]
    config: String,

    /// Path to database file (overrides config)
    #[arg(short, long, env = "OLX_TRACKER_DB")]
    db: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Add a new search to track
    Add {
        /// Name for this search (e.g., "PS2 cheap")
        #[arg(short, long)]
        name: String,

        /// Search keyword (e.g., "playstation 2")
        #[arg(short, long)]
        keyword: String,

        /// Maximum price threshold for deals
        #[arg(short = 'p', long)]
        max_price: Option<f64>,

        /// City to search in
        #[arg(long)]
        city: Option<String>,

        /// Search radius in km (requires city)
        #[arg(short, long)]
        radius: Option<i32>,

        /// OLX category ID
        #[arg(long)]
        category: Option<i64>,
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
        #[arg(short, long)]
        search_id: Option<i64>,

        /// Maximum results per search
        #[arg(short, long, default_value = "100")]
        max_results: i32,
    },

    /// Start daemon mode (checks periodically)
    Daemon {
        /// Check interval in minutes
        #[arg(short, long, default_value = "30")]
        interval: u64,

        /// Maximum results per search
        #[arg(short, long, default_value = "100")]
        max_results: i32,
    },

    /// Show deals (listings below average price or `max_price`)
    Deals {
        /// Filter by search ID
        #[arg(short, long)]
        search_id: Option<i64>,
    },

    /// Show price statistics for a search
    Stats {
        /// Search ID to show stats for
        #[arg(short, long)]
        search_id: i64,
    },

    /// Remove a search
    Remove {
        /// Search ID to remove
        #[arg(short, long)]
        search_id: i64,
    },

    /// Toggle search active status
    Toggle {
        /// Search ID to toggle
        #[arg(short, long)]
        search_id: i64,
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

    // Load config
    let config = Config::load(&cli.config)?;

    // Use CLI db path if provided, otherwise use config
    let db_path = cli.db.as_deref().unwrap_or(&config.database.path);
    let db = Database::open(db_path)?;

    match cli.command {
        Commands::Add {
            name,
            keyword,
            max_price,
            city,
            radius,
            category,
        } => {
            cmd_add(&db, &name, &keyword, max_price, city, radius, category)?;
        }
        Commands::List { all } => {
            cmd_list(&db, all)?;
        }
        Commands::Run {
            search_id,
            max_results,
        } => {
            cmd_run(&db, &config, search_id, max_results).await?;
        }
        Commands::Daemon {
            interval,
            max_results,
        } => {
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
    }

    Ok(())
}

fn cmd_add(
    db: &Database,
    name: &str,
    keyword: &str,
    max_price: Option<f64>,
    city: Option<String>,
    radius: Option<i32>,
    category: Option<i64>,
) -> Result<()> {
    let id = db.create_search(name, keyword, max_price, city.as_deref(), radius, category)?;
    println!("Created search '{name}' with ID {id}");
    Ok(())
}

fn cmd_list(db: &Database, include_inactive: bool) -> Result<()> {
    let searches = db.list_searches(!include_inactive)?;

    if searches.is_empty() {
        println!("No searches found. Use 'olx-tracker add' to create one.");
        return Ok(());
    }

    println!(
        "{:<4} {:<20} {:<20} {:<10} {:<15} {:<8}",
        "ID", "Name", "Keyword", "Max €", "City", "Active"
    );
    println!("{}", "-".repeat(80));

    for search in searches {
        let max_price = search
            .max_price
            .map_or_else(|| "-".to_string(), |p| format!("{p:.2}"));
        let city = search.city.as_deref().unwrap_or("-");
        let active = if search.active { "Yes" } else { "No" };

        println!(
            "{:<4} {:<20} {:<20} {:<10} {:<15} {:<8}",
            search.id,
            truncate(&search.name, 18),
            truncate(&search.keyword, 18),
            max_price,
            truncate(city, 13),
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
    let tracker = SearchTracker::new(db, &client).with_filters(filters);
    let notifier = WebhookNotifier::new(config.notifications.clone());

    let results = if let Some(id) = search_id {
        let search = db
            .get_search(id)?
            .ok_or_else(|| anyhow::anyhow!("Search with ID {id} not found"))?;
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
            if let Err(e) = notifier.notify_new_listings(&result.new_listings).await {
                warn!("Failed to send new listings notification: {}", e);
            }
        }

        if !result.price_drops.is_empty() {
            println!(
                "Found {} price drop(s) for search {}",
                result.price_drops.len(),
                result.search_id
            );
            if let Err(e) = notifier.notify_price_drops(&result.price_drops).await {
                warn!("Failed to send price drop notification: {}", e);
            }
        }

        if !result.deals.is_empty() {
            println!(
                "Found {} deal(s) for search {}",
                result.deals.len(),
                result.search_id
            );
            if let Err(e) = notifier.notify_deals(&result.deals, avg_price).await {
                warn!("Failed to send deals notification: {}", e);
            }
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
    info!(
        "Starting daemon mode, checking every {} minutes",
        interval_mins
    );
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
        let price = deal
            .price
            .map_or_else(|| "-".to_string(), |p| format!("{p:.2} €"));
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
        search
            .radius_km
            .map_or_else(|| "-".to_string(), |r| format!("{r} km"))
    );
    println!();
    println!("Total listings:  {}", stats.total_listings);
    println!("Average price:   {}", fmt_price(stats.avg_price));
    println!("Min price:       {}", fmt_price(stats.min_price));
    println!("Max price:       {}", fmt_price(stats.max_price));
    println!(
        "Last updated:    {}",
        stats.last_updated_at.as_deref().unwrap_or("-")
    );

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

    println!(
        "Search '{}' is now {}",
        search.name,
        if new_status { "active" } else { "inactive" }
    );
    Ok(())
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len.saturating_sub(3)])
    }
}

fn fmt_price(price: Option<f64>) -> String {
    price.map_or_else(|| "-".to_string(), |p| format!("{p:.2} €"))
}
