//! Router setup and route configuration

use axum::{
    Router, middleware,
    routing::{get, post},
};

use super::auth::auth_middleware;
use super::handlers::{
    add_search_handler, daemon_handler, daemon_stop_handler, deals_handler, health,
    list_searches_handler, remove_search_handler, run_handler, search_handler, stats_handler,
    toggle_search_handler,
};
use super::state::AppState;

/// Maximum request body size (1MB)
const MAX_BODY_SIZE: usize = 1024 * 1024;

pub fn build_app(state: AppState) -> Router {
    // Routes that require authentication
    let protected_routes = Router::new()
        .route("/search", post(search_handler))
        .route("/searches/add", post(add_search_handler))
        .route("/searches/list", post(list_searches_handler))
        .route("/searches/toggle", post(toggle_search_handler))
        .route("/searches/remove", post(remove_search_handler))
        .route("/searches/run", post(run_handler))
        .route("/searches/daemon", post(daemon_handler))
        .route("/searches/daemon/stop", post(daemon_stop_handler))
        .route("/searches/deals", post(deals_handler))
        .route("/searches/stats", post(stats_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Public routes (health check)
    let public_routes = Router::new().route("/health", get(health));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_SIZE))
        .with_state(state)
}
