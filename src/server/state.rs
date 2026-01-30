//! Application state and daemon management

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};

use crate::Config;

/// Maximum consecutive errors before daemon stops (circuit breaker)
pub const MAX_DAEMON_ERRORS: u32 = 10;

/// Handle to a running daemon thread.
///
/// Note: We use a native thread with its own Tokio runtime because rusqlite's
/// Database is not Send/Sync. This allows async operations while keeping
/// the database connection thread-local.
pub struct DaemonHandle {
    pub stop_tx: oneshot::Sender<()>,
    pub handle: std::thread::JoinHandle<()>,
}

#[derive(Clone)]
pub struct AppState {
    pub db_path: String,
    pub config: Config,
    pub api_key: Option<String>,
    pub timeout: Duration,
    pub daemon: Arc<Mutex<Option<DaemonHandle>>>,
}
