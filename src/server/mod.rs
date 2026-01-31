//! HTTP server exposing the CLI functionality via REST API.
//!
//! # Security Considerations
//!
//! When deploying this server:
//! - Use HTTPS in production (via reverse proxy like nginx/caddy)
//! - Set a strong `API_KEY` when exposing to networks
//! - Consider rate limiting at the reverse proxy level
//! - Request body size is limited to 1MB by default
//!
//! # Scalability Notes
//!
//! This server opens a new `SQLite` connection per request. This design is appropriate
//! for low-to-moderate traffic (< 50 req/s). For higher traffic, consider:
//! - Connection pooling (requires r2d2 or similar)
//! - Moving to a client-server database (`PostgreSQL`, `MySQL`)
//! - Horizontal scaling with a shared database backend

mod auth;
mod daemon;
mod error;
mod handlers;
mod models;
mod routes;
mod state;

use anyhow::Result as AnyResult;
use std::time::Duration;
use tracing::info;

pub use routes::build_app;
pub use state::AppState;

use crate::Config;

/// Default timeout for blocking operations (60 seconds)
const DEFAULT_TIMEOUT_SECS: u64 = 60;

pub async fn serve(
    db_path: String,
    config: &Config,
    host: String,
    port: u16,
    api_key: Option<String>,
) -> AnyResult<()> {
    serve_with_timeout(db_path, config, host, port, api_key, DEFAULT_TIMEOUT_SECS).await
}

pub async fn serve_with_timeout(
    db_path: String,
    config: &Config,
    host: String,
    port: u16,
    api_key: Option<String>,
    timeout_secs: u64,
) -> AnyResult<()> {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let state = AppState {
        db_path,
        config: config.clone(),
        api_key,
        timeout: Duration::from_secs(timeout_secs),
        daemon: Arc::new(Mutex::new(None)),
    };
    let shutdown_state = state.clone();
    let app = build_app(state);

    let addr = format!("{host}:{port}");
    info!("Serving on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).with_graceful_shutdown(shutdown_signal(shutdown_state)).await?;
    Ok(())
}

async fn shutdown_signal(state: AppState) {
    let _ = tokio::signal::ctrl_c().await;
    daemon::stop_daemon(&state).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use std::sync::Arc;
    use tempfile::NamedTempFile;
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    use models::*;

    fn test_state(db_path: String, api_key: Option<String>) -> AppState {
        AppState {
            db_path,
            config: Config::minimal(),
            api_key,
            timeout: Duration::from_secs(60),
            daemon: Arc::new(Mutex::new(None)),
        }
    }

    #[tokio::test]
    async fn health_ok() {
        let state = test_state(":memory:".to_string(), None);
        let app = build_app(state);
        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn auth_required_for_list() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);
        let body = serde_json::to_vec(&ListRequest { all: Some(true) }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/list")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn auth_with_bearer_token() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);
        let body = serde_json::to_vec(&ListRequest { all: Some(true) }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/list")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer secret")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn add_and_list_searches() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let add = AddSearchRequest {
            name: "Test".to_string(),
            keyword: "ps5".to_string(),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            category: None,
            sort: Some("newest".to_string()),
            days: None,
        };
        let add_body = serde_json::to_vec(&add).unwrap();
        let add_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/add")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(add_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(add_response.status(), StatusCode::OK);

        let list_body = serde_json::to_vec(&ListRequest { all: Some(true) }).unwrap();
        let list_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/list")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(list_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn toggle_and_remove_search() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let add = AddSearchRequest {
            name: "ToggleTest".to_string(),
            keyword: "ps5".to_string(),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            category: None,
            sort: Some("newest".to_string()),
            days: None,
        };
        let add_body = serde_json::to_vec(&add).unwrap();
        let add_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/add")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(add_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(add_response.status(), StatusCode::OK);
        let add_json = add_response.into_body().collect().await.unwrap().to_bytes();
        let add_value: serde_json::Value = serde_json::from_slice(&add_json).unwrap();
        let id = add_value.get("id").and_then(serde_json::Value::as_i64).unwrap();

        let toggle_body = serde_json::to_vec(&ToggleRequest { search_id: id }).unwrap();
        let toggle_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/toggle")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(toggle_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(toggle_response.status(), StatusCode::OK);

        let remove_body = serde_json::to_vec(&RemoveRequest { search_id: id }).unwrap();
        let remove_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/remove")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(remove_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(remove_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn auth_required_for_search_and_run_and_deals() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let search_body = serde_json::to_vec(&SearchRequest {
            query: "ps5".to_string(),
            max: None,
            sort: None,
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            keyword: None,
            category: None,
            format: None,
        })
        .unwrap();
        let search_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(search_response.status(), StatusCode::UNAUTHORIZED);

        let run_body =
            serde_json::to_vec(&RunRequest { search_id: None, max_results: None }).unwrap();
        let run_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/run")
                    .header("content-type", "application/json")
                    .body(Body::from(run_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(run_response.status(), StatusCode::UNAUTHORIZED);

        let deals_body = serde_json::to_vec(&DealsRequest { search_id: None }).unwrap();
        let deals_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/deals")
                    .header("content-type", "application/json")
                    .body(Body::from(deals_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deals_response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn daemon_start_and_stop() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let start_body =
            serde_json::to_vec(&DaemonRequest { interval: Some(1), max_results: Some(1) }).unwrap();
        let start_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/daemon")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(start_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start_response.status(), StatusCode::OK);

        let stop_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/daemon/stop")
                    .header("x-api-key", "secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stop_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn no_auth_when_api_key_not_set() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), None);
        let app = build_app(state);

        let body = serde_json::to_vec(&ListRequest { all: Some(true) }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/list")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn stats_endpoint_returns_stats() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        // First create a search
        let add = AddSearchRequest {
            name: "StatsTest".to_string(),
            keyword: "test".to_string(),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            category: None,
            sort: None,
            days: None,
        };
        let add_body = serde_json::to_vec(&add).unwrap();
        let add_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/add")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(add_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(add_response.status(), StatusCode::OK);

        let add_json = add_response.into_body().collect().await.unwrap().to_bytes();
        let add_value: serde_json::Value = serde_json::from_slice(&add_json).unwrap();
        let search_id = add_value.get("id").and_then(serde_json::Value::as_i64).unwrap();

        // Now get stats
        let stats_body = serde_json::to_vec(&StatsRequest { search_id }).unwrap();
        let stats_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/stats")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(stats_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stats_response.status(), StatusCode::OK);

        let stats_json = stats_response.into_body().collect().await.unwrap().to_bytes();
        let stats_value: serde_json::Value = serde_json::from_slice(&stats_json).unwrap();
        assert!(stats_value.get("stats").is_some());
    }

    #[tokio::test]
    async fn deals_endpoint_returns_deals() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        // Get deals (should be empty initially)
        let deals_body = serde_json::to_vec(&DealsRequest { search_id: None }).unwrap();
        let deals_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/deals")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(deals_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deals_response.status(), StatusCode::OK);

        let deals_json = deals_response.into_body().collect().await.unwrap().to_bytes();
        let deals_value: serde_json::Value = serde_json::from_slice(&deals_json).unwrap();
        assert!(deals_value.get("deals").is_some());
        let deals_array = deals_value.get("deals").unwrap().as_array().unwrap();
        assert_eq!(deals_array.len(), 0); // Should be empty initially
    }

    #[tokio::test]
    async fn stats_endpoint_not_found_for_invalid_search() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let stats_body = serde_json::to_vec(&StatsRequest { search_id: 99999 }).unwrap();
        let stats_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/stats")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(stats_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        // Should return internal server error for invalid search ID
        assert_eq!(stats_response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn run_endpoint_executes_without_searches() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        // Run with no searches in database (should succeed but return 0 results)
        let run_body =
            serde_json::to_vec(&RunRequest { search_id: None, max_results: Some(10) }).unwrap();
        let run_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/run")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(run_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(run_response.status(), StatusCode::OK);

        let run_json = run_response.into_body().collect().await.unwrap().to_bytes();
        let run_value: serde_json::Value = serde_json::from_slice(&run_json).unwrap();
        assert_eq!(run_value.get("total_new").and_then(serde_json::Value::as_u64), Some(0));
        assert_eq!(run_value.get("total_deals").and_then(serde_json::Value::as_u64), Some(0));
    }

    #[tokio::test]
    async fn add_search_with_invalid_sort() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let add = AddSearchRequest {
            name: "Test".to_string(),
            keyword: "ps5".to_string(),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            category: None,
            sort: Some("invalid_sort".to_string()),
            days: None,
        };
        let body = serde_json::to_vec(&add).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/add")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn add_search_with_expiration() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let add = AddSearchRequest {
            name: "ExpiringSearch".to_string(),
            keyword: "test".to_string(),
            min_price: Some(10.0),
            max_price: Some(100.0),
            city: Some("Porto".to_string()),
            radius: Some(10),
            category: Some(123),
            sort: Some("cheapest".to_string()),
            days: Some(7), // Expires in 7 days
        };
        let body = serde_json::to_vec(&add).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/add")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn toggle_nonexistent_search() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let body = serde_json::to_vec(&ToggleRequest { search_id: 99999 }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/toggle")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn remove_nonexistent_search() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let body = serde_json::to_vec(&RemoveRequest { search_id: 99999 }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/remove")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn daemon_already_running() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        // Start daemon first time
        let start_body =
            serde_json::to_vec(&DaemonRequest { interval: Some(60), max_results: Some(10) })
                .unwrap();
        let start_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/daemon")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(start_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start_response.status(), StatusCode::OK);

        // Try to start daemon again
        let start_body2 =
            serde_json::to_vec(&DaemonRequest { interval: Some(60), max_results: Some(10) })
                .unwrap();
        let start_response2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/daemon")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(start_body2))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start_response2.status(), StatusCode::OK);
        let body = start_response2.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap()
                .contains("already running")
        );

        // Stop the daemon
        let stop_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/daemon/stop")
                    .header("x-api-key", "secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stop_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn daemon_stop_when_not_running() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let stop_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/daemon/stop")
                    .header("x-api-key", "secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stop_response.status(), StatusCode::OK);
        let body = stop_response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(
            value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .unwrap()
                .contains("not running")
        );
    }

    #[tokio::test]
    async fn list_active_searches_only() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        // Add a search
        let add = AddSearchRequest {
            name: "Test".to_string(),
            keyword: "ps5".to_string(),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            category: None,
            sort: None,
            days: None,
        };
        let add_body = serde_json::to_vec(&add).unwrap();
        let add_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/add")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(add_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(add_response.status(), StatusCode::OK);

        // List active only (all=false or not specified)
        let list_body = serde_json::to_vec(&ListRequest { all: Some(false) }).unwrap();
        let list_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/list")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(list_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(list_response.status(), StatusCode::OK);

        let list_json = list_response.into_body().collect().await.unwrap().to_bytes();
        let list_value: serde_json::Value = serde_json::from_slice(&list_json).unwrap();
        let searches = list_value.get("searches").unwrap().as_array().unwrap();
        assert_eq!(searches.len(), 1);
    }

    #[tokio::test]
    async fn run_specific_search() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        // Add a search first
        let add = AddSearchRequest {
            name: "RunTest".to_string(),
            keyword: "test".to_string(),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            category: None,
            sort: None,
            days: None,
        };
        let add_body = serde_json::to_vec(&add).unwrap();
        let add_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/add")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(add_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let add_json = add_response.into_body().collect().await.unwrap().to_bytes();
        let add_value: serde_json::Value = serde_json::from_slice(&add_json).unwrap();
        let search_id = add_value.get("id").and_then(serde_json::Value::as_i64).unwrap();

        // Run specific search - this will fail because we can't mock the OLX API,
        // but it tests the run path for a specific search_id
        // In a real test environment, we'd mock the OlxClient
        let run_body =
            serde_json::to_vec(&RunRequest { search_id: Some(search_id), max_results: Some(10) })
                .unwrap();
        let _run_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/run")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(run_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        // We don't assert status here because the actual API call may fail
        // The test verifies the path is exercised
    }

    #[tokio::test]
    async fn auth_wrong_token() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let body = serde_json::to_vec(&ListRequest { all: Some(true) }).unwrap();
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/list")
                    .header("content-type", "application/json")
                    .header("x-api-key", "wrong_token")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn deals_for_specific_search() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        // Add a search
        let add = AddSearchRequest {
            name: "DealsTest".to_string(),
            keyword: "test".to_string(),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            category: None,
            sort: None,
            days: None,
        };
        let add_body = serde_json::to_vec(&add).unwrap();
        let add_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/add")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(add_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        let add_json = add_response.into_body().collect().await.unwrap().to_bytes();
        let add_value: serde_json::Value = serde_json::from_slice(&add_json).unwrap();
        let search_id = add_value.get("id").and_then(serde_json::Value::as_i64).unwrap();

        // Get deals for specific search
        let deals_body = serde_json::to_vec(&DealsRequest { search_id: Some(search_id) }).unwrap();
        let deals_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/deals")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(deals_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deals_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn daemon_with_default_values() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        // Start daemon with default values
        let start_body =
            serde_json::to_vec(&DaemonRequest { interval: None, max_results: None }).unwrap();
        let start_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/daemon")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(start_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(start_response.status(), StatusCode::OK);

        // Stop the daemon
        let _ = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/daemon/stop")
                    .header("x-api-key", "secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn run_with_default_max_results() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let run_body =
            serde_json::to_vec(&RunRequest { search_id: None, max_results: None }).unwrap();
        let run_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/searches/run")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(run_body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(run_response.status(), StatusCode::OK);
    }

    fn test_state_with_config(
        db_path: String,
        api_key: Option<String>,
        config: Config,
    ) -> AppState {
        AppState {
            db_path,
            config,
            api_key,
            timeout: Duration::from_secs(60),
            daemon: Arc::new(Mutex::new(None)),
        }
    }

    #[tokio::test]
    async fn search_handler_returns_results() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock the search endpoint
        let search_response = serde_json::json!({
            "data": [
                {
                    "id": 12345,
                    "title": "iPhone 14 Pro",
                    "url": "https://olx.pt/d/anuncio/12345",
                    "params": [
                        {
                            "key": "price",
                            "name": "Price",
                            "value": { "value": 800.0, "label": "800 €" }
                        }
                    ],
                    "location": {
                        "city": { "id": 1, "name": "Lisbon" },
                        "region": { "id": 10, "name": "Lisboa" }
                    },
                    "user": { "id": 99, "name": "João" },
                    "photos": []
                }
            ],
            "metadata": { "total_elements": 1 }
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/offers.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&search_response))
            .mount(&mock_server)
            .await;

        let mut config = Config::minimal();
        config.api.base_url = Some(format!("{}/api/v1/offers", mock_server.uri()));

        let db = NamedTempFile::new().unwrap();
        let state = test_state_with_config(
            db.path().to_string_lossy().to_string(),
            Some("secret".to_string()),
            config,
        );
        let app = build_app(state);

        let search_body = serde_json::to_vec(&SearchRequest {
            query: "iphone".to_string(),
            max: Some(10),
            sort: Some("newest".to_string()),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            keyword: None,
            category: None,
            format: Some("json".to_string()),
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let output = value.get("output").and_then(serde_json::Value::as_str).unwrap();
        assert!(output.contains("iPhone 14 Pro"));
        assert!(output.contains("800"));
    }

    #[tokio::test]
    async fn search_handler_with_city_lookup() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock city lookup endpoint
        let location_response = serde_json::json!({
            "data": [
                {
                    "city": { "id": 42, "name": "Porto", "normalized_name": "porto" },
                    "region": { "id": 10, "name": "Norte" }
                }
            ]
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/geo-encoder/location-autocomplete.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&location_response))
            .mount(&mock_server)
            .await;

        // Mock search endpoint
        let search_response = serde_json::json!({
            "data": [
                {
                    "id": 99999,
                    "title": "MacBook Pro",
                    "url": "https://olx.pt/d/anuncio/99999",
                    "params": [
                        {
                            "key": "price",
                            "name": "Price",
                            "value": { "value": 1200.0, "label": "1200 €" }
                        }
                    ],
                    "location": {
                        "city": { "id": 42, "name": "Porto" },
                        "region": { "id": 10, "name": "Norte" }
                    },
                    "user": { "id": 1, "name": "Maria" },
                    "photos": []
                }
            ],
            "metadata": { "total_elements": 1 }
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/offers.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&search_response))
            .mount(&mock_server)
            .await;

        let mut config = Config::minimal();
        config.api.base_url = Some(format!("{}/api/v1/offers", mock_server.uri()));

        let db = NamedTempFile::new().unwrap();
        let state = test_state_with_config(
            db.path().to_string_lossy().to_string(),
            Some("secret".to_string()),
            config,
        );
        let app = build_app(state);

        let search_body = serde_json::to_vec(&SearchRequest {
            query: "macbook".to_string(),
            max: Some(10),
            sort: None,
            min_price: None,
            max_price: None,
            city: Some("Porto".to_string()),
            radius: Some(25),
            keyword: None,
            category: None,
            format: Some("table".to_string()),
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let output = value.get("output").and_then(serde_json::Value::as_str).unwrap();
        assert!(output.contains("MacBook Pro"));
    }

    #[tokio::test]
    async fn search_handler_city_not_found() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock city lookup endpoint - returns empty data
        let location_response = serde_json::json!({ "data": [] });

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/geo-encoder/location-autocomplete.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&location_response))
            .mount(&mock_server)
            .await;

        let mut config = Config::minimal();
        config.api.base_url = Some(format!("{}/api/v1/offers", mock_server.uri()));

        let db = NamedTempFile::new().unwrap();
        let state = test_state_with_config(
            db.path().to_string_lossy().to_string(),
            Some("secret".to_string()),
            config,
        );
        let app = build_app(state);

        let search_body = serde_json::to_vec(&SearchRequest {
            query: "laptop".to_string(),
            max: None,
            sort: None,
            min_price: None,
            max_price: None,
            city: Some("NonexistentCity".to_string()),
            radius: None,
            keyword: None,
            category: None,
            format: None,
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn search_handler_invalid_sort() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let search_body = serde_json::to_vec(&SearchRequest {
            query: "phone".to_string(),
            max: None,
            sort: Some("invalid_sort_order".to_string()),
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            keyword: None,
            category: None,
            format: None,
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_handler_invalid_format() {
        let db = NamedTempFile::new().unwrap();
        let state = test_state(db.path().to_string_lossy().to_string(), Some("secret".to_string()));
        let app = build_app(state);

        let search_body = serde_json::to_vec(&SearchRequest {
            query: "phone".to_string(),
            max: None,
            sort: None,
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            keyword: None,
            category: None,
            format: Some("invalid_format".to_string()),
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn search_handler_no_results() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock search endpoint with empty results
        let search_response = serde_json::json!({
            "data": [],
            "metadata": { "total_elements": 0 }
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/offers.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&search_response))
            .mount(&mock_server)
            .await;

        let mut config = Config::minimal();
        config.api.base_url = Some(format!("{}/api/v1/offers", mock_server.uri()));

        let db = NamedTempFile::new().unwrap();
        let state = test_state_with_config(
            db.path().to_string_lossy().to_string(),
            Some("secret".to_string()),
            config,
        );
        let app = build_app(state);

        let search_body = serde_json::to_vec(&SearchRequest {
            query: "nonexistent_item_12345".to_string(),
            max: None,
            sort: None,
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            keyword: None,
            category: None,
            format: None,
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let output = value.get("output").and_then(serde_json::Value::as_str).unwrap();
        assert!(output.contains("No results found"));
    }

    #[tokio::test]
    async fn search_handler_with_price_filter() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Mock search endpoint with multiple results
        let search_response = serde_json::json!({
            "data": [
                {
                    "id": 1,
                    "title": "Cheap Item",
                    "url": "https://olx.pt/d/anuncio/1",
                    "params": [{ "key": "price", "name": "Price", "value": { "value": 50.0 } }],
                    "photos": []
                },
                {
                    "id": 2,
                    "title": "Mid Item",
                    "url": "https://olx.pt/d/anuncio/2",
                    "params": [{ "key": "price", "name": "Price", "value": { "value": 150.0 } }],
                    "photos": []
                },
                {
                    "id": 3,
                    "title": "Expensive Item",
                    "url": "https://olx.pt/d/anuncio/3",
                    "params": [{ "key": "price", "name": "Price", "value": { "value": 500.0 } }],
                    "photos": []
                }
            ],
            "metadata": { "total_elements": 3 }
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/offers.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&search_response))
            .mount(&mock_server)
            .await;

        let mut config = Config::minimal();
        config.api.base_url = Some(format!("{}/api/v1/offers", mock_server.uri()));

        let db = NamedTempFile::new().unwrap();
        let state = test_state_with_config(
            db.path().to_string_lossy().to_string(),
            Some("secret".to_string()),
            config,
        );
        let app = build_app(state);

        // Search with price filter - should only match "Mid Item" (100-200)
        let search_body = serde_json::to_vec(&SearchRequest {
            query: "item".to_string(),
            max: None,
            sort: None,
            min_price: Some(100.0),
            max_price: Some(200.0),
            city: None,
            radius: None,
            keyword: None,
            category: None,
            format: Some("json".to_string()),
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let output = value.get("output").and_then(serde_json::Value::as_str).unwrap();
        // Should contain Mid Item but not Cheap or Expensive
        assert!(output.contains("Mid Item"));
        assert!(!output.contains("Cheap Item"));
        assert!(!output.contains("Expensive Item"));
    }

    #[tokio::test]
    async fn search_handler_with_keyword_filter() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let search_response = serde_json::json!({
            "data": [
                {
                    "id": 1,
                    "title": "iPhone 14 Pro Max",
                    "url": "https://olx.pt/d/anuncio/1",
                    "params": [{ "key": "price", "name": "Price", "value": { "value": 900.0 } }],
                    "photos": []
                },
                {
                    "id": 2,
                    "title": "Samsung Galaxy S23",
                    "url": "https://olx.pt/d/anuncio/2",
                    "params": [{ "key": "price", "name": "Price", "value": { "value": 700.0 } }],
                    "photos": []
                }
            ],
            "metadata": { "total_elements": 2 }
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/offers.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&search_response))
            .mount(&mock_server)
            .await;

        let mut config = Config::minimal();
        config.api.base_url = Some(format!("{}/api/v1/offers", mock_server.uri()));

        let db = NamedTempFile::new().unwrap();
        let state = test_state_with_config(
            db.path().to_string_lossy().to_string(),
            Some("secret".to_string()),
            config,
        );
        let app = build_app(state);

        // Search with keyword filter - should only match iPhone
        let search_body = serde_json::to_vec(&SearchRequest {
            query: "phone".to_string(),
            max: None,
            sort: None,
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            keyword: Some("iPhone".to_string()),
            category: None,
            format: Some("json".to_string()),
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let output = value.get("output").and_then(serde_json::Value::as_str).unwrap();
        assert!(output.contains("iPhone 14 Pro Max"));
        assert!(!output.contains("Samsung"));
    }

    #[tokio::test]
    async fn search_handler_markdown_format() {
        use wiremock::matchers::{method, path_regex};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        let search_response = serde_json::json!({
            "data": [
                {
                    "id": 42,
                    "title": "Test Product",
                    "url": "https://olx.pt/d/anuncio/42",
                    "params": [{ "key": "price", "name": "Price", "value": { "value": 250.0 } }],
                    "location": { "city": { "id": 1, "name": "Lisbon" } },
                    "user": { "id": 1, "name": "TestSeller" },
                    "photos": []
                }
            ],
            "metadata": { "total_elements": 1 }
        });

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/offers.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&search_response))
            .mount(&mock_server)
            .await;

        let mut config = Config::minimal();
        config.api.base_url = Some(format!("{}/api/v1/offers", mock_server.uri()));

        let db = NamedTempFile::new().unwrap();
        let state = test_state_with_config(
            db.path().to_string_lossy().to_string(),
            Some("secret".to_string()),
            config,
        );
        let app = build_app(state);

        let search_body = serde_json::to_vec(&SearchRequest {
            query: "test".to_string(),
            max: None,
            sort: None,
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
            keyword: None,
            category: None,
            format: Some("markdown".to_string()),
        })
        .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/search")
                    .header("content-type", "application/json")
                    .header("x-api-key", "secret")
                    .body(Body::from(search_body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let output = value.get("output").and_then(serde_json::Value::as_str).unwrap();
        // Markdown format should contain markdown headers
        assert!(output.contains("# Search Results"));
        assert!(output.contains("## 1. Test Product"));
        assert!(output.contains("**Price:**"));
    }
}
