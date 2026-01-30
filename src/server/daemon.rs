//! Background daemon for periodic search execution

use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use crate::{Config, Database, FilterChain, MultiNotifier, Notifier, OlxClient, SearchTracker};

use super::state::{DaemonHandle, MAX_DAEMON_ERRORS, AppState};

/// Start a background daemon thread
pub fn start_daemon(
    db_path: String,
    config: Config,
    interval: u64,
    max_results: i32,
) -> DaemonHandle {
    let (stop_tx, mut stop_rx) = oneshot::channel();

    // Use a native thread with its own Tokio runtime because rusqlite's Database
    // is not Send/Sync. This keeps the database connection thread-local while
    // still allowing async operations within the daemon.
    let handle = std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
            Ok(rt) => rt,
            Err(e) => {
                error!("Failed to create daemon runtime: {}", e);
                return;
            }
        };

        runtime.block_on(async move {
            let interval_duration = Duration::from_secs(interval * 60);
            info!("Daemon started, checking every {} minutes (fixed interval)", interval);

            let mut ticker = tokio::time::interval(interval_duration);
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            let mut consecutive_errors = 0u32;

            loop {
                tokio::select! {
                    _ = &mut stop_rx => {
                        info!("Daemon received stop signal");
                        break;
                    }
                    _ = ticker.tick() => {
                        match Database::open(&db_path) {
                            Ok(db) => {
                                match run_searches(&db, &config, None, max_results).await {
                                    Ok(_) => {
                                        consecutive_errors = 0; // Reset on success
                                    }
                                    Err(e) => {
                                        consecutive_errors += 1;
                                        error!(
                                            "Daemon search run failed ({}/{}): {}",
                                            consecutive_errors, MAX_DAEMON_ERRORS, e
                                        );

                                        if consecutive_errors >= MAX_DAEMON_ERRORS {
                                            error!(
                                                "Daemon stopping after {} consecutive errors (circuit breaker)",
                                                MAX_DAEMON_ERRORS
                                            );
                                            break;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                consecutive_errors += 1;
                                error!(
                                    "Daemon failed to open database ({}/{}): {}",
                                    consecutive_errors, MAX_DAEMON_ERRORS, e
                                );

                                if consecutive_errors >= MAX_DAEMON_ERRORS {
                                    error!(
                                        "Daemon stopping after {} consecutive database errors (circuit breaker)",
                                        MAX_DAEMON_ERRORS
                                    );
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });
    });

    DaemonHandle { stop_tx, handle }
}

/// Stop the daemon if running. Returns true if a daemon was stopped.
pub async fn stop_daemon(state: &AppState) -> bool {
    let mut guard = state.daemon.lock().await;
    let Some(daemon) = guard.take() else {
        return false;
    };
    drop(guard);

    // Signal the daemon to stop
    let _ = daemon.stop_tx.send(());

    // Wait for the thread to complete (blocking, but should be quick after signal)
    if daemon.handle.join().is_ok() {
        info!("Daemon stopped cleanly");
    } else {
        warn!("Daemon thread panicked");
    }

    true
}

/// Execute searches and send notifications
async fn run_searches(
    db: &Database,
    config: &Config,
    search_id: Option<i64>,
    max_results: i32,
) -> anyhow::Result<(usize, usize)> {
    let client = OlxClient::new(config)?;
    let filters = FilterChain::with_defaults();
    let tracker = SearchTracker::new(db, &client, config.deals.clone()).with_filters(filters);
    let notifier = MultiNotifier::from_config(config.notifications.clone());

    let results = if let Some(id) = search_id {
        let search = db.get_search(id)?.ok_or_else(|| anyhow::anyhow!("search not found"))?;
        vec![tracker.run_search(&search, max_results).await?]
    } else {
        tracker.run_all_searches(max_results).await?
    };

    for result in &results {
        let stats = db.get_search_stats(result.search_id)?;
        let avg_price = stats.and_then(|s| s.avg_price);

        if !result.new_listings.is_empty() {
            notifier.notify_new_listings(&result.new_listings).await?;
        }

        if !result.price_drops.is_empty() {
            notifier.notify_price_drops(&result.price_drops).await?;
        }

        if !result.deals.is_empty() {
            notifier.notify_deals(&result.deals, avg_price).await?;
        }
    }

    let total_new: usize = results.iter().map(|r| r.new_listings.len()).sum();
    let total_deals: usize = results.iter().map(|r| r.deals.len()).sum();
    Ok((total_new, total_deals))
}

/// Blocking wrapper for `run_searches` that creates its own runtime.
///
/// This is necessary because rusqlite's Database is not Send/Sync, so we can't
/// use it directly in `tokio::spawn`. Instead, we use `spawn_blocking` and create
/// a thread-local runtime for async operations.
pub fn run_searches_blocking(
    db_path: &str,
    config: &Config,
    search_id: Option<i64>,
    max_results: i32,
) -> anyhow::Result<(usize, usize)> {
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let db = Database::open(db_path)?;
        run_searches(&db, config, search_id, max_results).await
    })
}
