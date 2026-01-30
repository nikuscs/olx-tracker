//! Background daemon for periodic search execution

use std::time::Duration;
use tokio::sync::oneshot;
use tracing::{error, info, warn};

use crate::api::client::SearchClient;
use crate::{Config, Database, FilterChain, MultiNotifier, Notifier, OlxClient, SearchTracker};

use super::state::{AppState, DaemonHandle, MAX_DAEMON_ERRORS};

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

/// Execute searches and send notifications (generic over client for testing)
async fn run_searches_with_client<C: SearchClient>(
    db: &Database,
    client: &C,
    config: &Config,
    search_id: Option<i64>,
    max_results: i32,
) -> anyhow::Result<(usize, usize)> {
    let filters = FilterChain::with_defaults();
    let tracker = SearchTracker::new(db, client, config.deals.clone()).with_filters(filters);
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

/// Execute searches and send notifications (production version with real `OlxClient`)
async fn run_searches(
    db: &Database,
    config: &Config,
    search_id: Option<i64>,
    max_results: i32,
) -> anyhow::Result<(usize, usize)> {
    let client = OlxClient::new(config)?;
    run_searches_with_client(db, &client, config, search_id, max_results).await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::SearchParams;
    use crate::api::models::{LocationResult, OfferData};
    use std::sync::Arc;

    // Mock client for testing (NEVER makes real API calls)
    struct MockClient;

    #[async_trait::async_trait]
    impl SearchClient for MockClient {
        async fn lookup_city(&self, _city_name: &str) -> anyhow::Result<Option<LocationResult>> {
            Ok(None)
        }

        async fn search_all(
            &self,
            _params: &SearchParams,
            _max_results: i32,
        ) -> anyhow::Result<Vec<OfferData>> {
            Ok(vec![]) // Return empty results for tests
        }

        fn request_delay(&self) -> Duration {
            Duration::from_millis(0)
        }
    }

    fn make_test_config() -> Config {
        Config::minimal()
    }

    fn make_test_state() -> AppState {
        AppState {
            db_path: ":memory:".to_string(),
            config: Config::minimal(),
            api_key: Some("test_key".to_string()),
            timeout: std::time::Duration::from_secs(30),
            daemon: Arc::new(tokio::sync::Mutex::new(None)),
        }
    }

    #[tokio::test]
    async fn test_stop_daemon_when_none_running() {
        let state = make_test_state();
        let result = stop_daemon(&state).await;
        assert!(!result); // Should return false when no daemon is running
    }

    #[tokio::test]
    async fn test_stop_daemon_when_running() {
        let db_path = ":memory:".to_string();
        let config = make_test_config();

        let daemon = start_daemon(db_path, config, 60, 10);

        let state = make_test_state();
        *state.daemon.lock().await = Some(daemon);

        let result = stop_daemon(&state).await;
        assert!(result); // Should return true when daemon was stopped

        // Verify daemon is actually stopped
        let guard = state.daemon.lock().await;
        assert!(guard.is_none());
    }

    #[test]
    fn test_run_searches_blocking_with_empty_db() {
        let _db = Database::open_in_memory().unwrap();
        let db_path = ":memory:";
        let config = make_test_config();

        // This should succeed but return (0, 0) since there are no searches
        // Note: This still uses real OlxClient but with empty DB, so no actual API calls happen
        let result = run_searches_blocking(db_path, &config, None, 10);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_searches_with_empty_db() {
        let db = Database::open_in_memory().unwrap();
        let config = make_test_config();
        let client = MockClient; // Use mock client - NO REAL API CALLS

        let result = run_searches_with_client(&db, &client, &config, None, 10).await;
        assert!(result.is_ok());
        let (new, deals) = result.unwrap();
        assert_eq!(new, 0);
        assert_eq!(deals, 0);
    }

    #[tokio::test]
    async fn test_run_searches_specific_search_not_found() {
        let db = Database::open_in_memory().unwrap();
        let config = make_test_config();
        let client = MockClient; // Use mock client - NO REAL API CALLS

        // Try to run a search that doesn't exist
        let result = run_searches_with_client(&db, &client, &config, Some(999), 10).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_daemon_handle_creation() {
        let db_path = ":memory:".to_string();
        let config = make_test_config();

        let daemon = start_daemon(db_path, config, 60, 10);

        // Signal stop immediately
        let _ = daemon.stop_tx.send(());

        // Wait for thread to complete
        let join_result = daemon.handle.join();
        assert!(join_result.is_ok());
    }

    #[tokio::test]
    async fn test_daemon_stops_on_signal() {
        use std::time::Duration;

        let db_path = ":memory:".to_string();
        let config = make_test_config();

        let daemon = start_daemon(db_path, config, 60, 10);

        // Signal stop
        let _ = daemon.stop_tx.send(());

        // Wait briefly for the thread to stop
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Thread should complete without panicking
        let join_result = daemon.handle.join();
        assert!(join_result.is_ok());
    }

    #[test]
    fn test_run_searches_blocking_specific_search() {
        let _db = Database::open_in_memory().unwrap();
        let config = make_test_config();

        // Try to run a search that doesn't exist (tests error path)
        // Note: This still uses real OlxClient but fails before making API calls
        // because the search doesn't exist in the database
        let result = run_searches_blocking(":memory:", &config, Some(999), 10);
        assert!(result.is_err()); // Search won't exist
    }
}
