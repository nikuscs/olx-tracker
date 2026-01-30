use std::time::Duration;

use anyhow::Result;
use tracing::{error, info};

use olx_tracker::{
    Config, Database, FilterChain, MultiNotifier, Notifier, OlxClient, SearchTracker,
};

pub async fn cmd_run(
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

pub async fn cmd_daemon(
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
