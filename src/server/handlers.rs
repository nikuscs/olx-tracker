//! HTTP request handlers

use axum::{Json, extract::State};
use chrono::{Duration as ChronoDuration, Utc};

use crate::api::SearchParams;
use crate::{Database, FormatParams, OlxClient, OutputFormat, SortOrder, format_results};

use super::daemon::{run_searches_blocking, start_daemon, stop_daemon};
use super::error::{ApiError, ErrorKind};
use super::models::{
    AddSearchRequest, AddSearchResponse, DaemonRequest, DealsRequest, DealsResponse, ListRequest,
    MessageResponse, RemoveRequest, RunRequest, RunResponse, SearchListResponse, SearchRequest,
    SearchResponseBody, StatsRequest, StatsResponse, ToggleRequest,
};
use super::state::AppState;

/// Opens a database connection.
///
/// Note: We open a new connection per request because `SQLite` connections are
/// not thread-safe and rusqlite doesn't support connection pooling. For this
/// use case (low-traffic API), per-request connections are acceptable.
fn open_db(db_path: &str) -> Result<Database, ApiError> {
    Database::open(db_path).map_err(ApiError::from_db)
}

pub async fn health() -> Json<MessageResponse> {
    Json(MessageResponse { message: "ok".to_string() })
}

pub async fn add_search_handler(
    State(state): State<AppState>,
    Json(payload): Json<AddSearchRequest>,
) -> Result<Json<AddSearchResponse>, ApiError> {
    let sort = payload.sort.as_deref().unwrap_or("newest");
    let _: SortOrder = sort.parse().map_err(ApiError::bad_request)?;

    let expires_at = payload.days.map(|d| {
        let expires = Utc::now() + ChronoDuration::days(d);
        expires.to_rfc3339()
    });

    let db = open_db(&state.db_path)?;
    let id = db
        .create_search(
            &payload.name,
            &payload.keyword,
            payload.min_price,
            payload.max_price,
            payload.city.as_deref(),
            payload.radius,
            payload.category,
            Some(sort),
            expires_at.as_deref(),
        )
        .map_err(ApiError::from_db)?;

    Ok(Json(AddSearchResponse { id }))
}

pub async fn list_searches_handler(
    State(state): State<AppState>,
    Json(payload): Json<ListRequest>,
) -> Result<Json<SearchListResponse>, ApiError> {
    let all = payload.all.unwrap_or(false);
    let db = open_db(&state.db_path)?;
    let searches = db.list_searches(!all).map_err(ApiError::from_db)?;

    Ok(Json(SearchListResponse { searches }))
}

pub async fn toggle_search_handler(
    State(state): State<AppState>,
    Json(payload): Json<ToggleRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    let db = open_db(&state.db_path)?;
    let search = db
        .get_search(payload.search_id)
        .map_err(ApiError::from_db)?
        .ok_or_else(|| ApiError::not_found("search not found"))?;

    let new_status = !search.active;
    db.set_search_active(payload.search_id, new_status).map_err(ApiError::from_db)?;

    Ok(Json(MessageResponse {
        message: format!(
            "search {} is now {}",
            search.id,
            if new_status { "active" } else { "inactive" }
        ),
    }))
}

pub async fn remove_search_handler(
    State(state): State<AppState>,
    Json(payload): Json<RemoveRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    let db = open_db(&state.db_path)?;
    let removed = db.delete_search(payload.search_id).map_err(ApiError::from_db)?;

    if removed {
        Ok(Json(MessageResponse { message: "removed".to_string() }))
    } else {
        Err(ApiError::not_found("search not found"))
    }
}

pub async fn run_handler(
    State(state): State<AppState>,
    Json(payload): Json<RunRequest>,
) -> Result<Json<RunResponse>, ApiError> {
    let max_results = payload.max_results.unwrap_or(100);
    let db_path = state.db_path.clone();
    let config = state.config.clone();
    let search_id = payload.search_id;
    let timeout = state.timeout;

    // Use spawn_blocking because Database (rusqlite) is not Send.
    // The blocking task creates its own single-threaded tokio runtime for async operations.
    // This has some overhead (~1-2ms per request) but is necessary given rusqlite's constraints.
    // For high-frequency operations (>100 req/s), consider using an actor pattern or
    // migrating to a client-server database that supports connection pooling.
    let task = tokio::task::spawn_blocking(move || {
        run_searches_blocking(&db_path, &config, search_id, max_results)
    });

    let (total_new, total_deals) = tokio::time::timeout(timeout, task)
        .await
        .map_err(|_| ApiError::timeout())?
        .map_err(|e| ApiError::internal(format!("task failed: {e}")))?
        .map_err(|e| ApiError::from_anyhow(e, ErrorKind::Internal))?;

    Ok(Json(RunResponse { total_new, total_deals }))
}

pub async fn daemon_handler(
    State(state): State<AppState>,
    Json(payload): Json<DaemonRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    let interval = payload.interval.unwrap_or(30);
    let max_results = payload.max_results.unwrap_or(100);

    let mut guard = state.daemon.lock().await;
    if guard.is_some() {
        return Ok(Json(MessageResponse { message: "daemon already running".to_string() }));
    }

    let daemon = start_daemon(state.db_path.clone(), state.config.clone(), interval, max_results);

    *guard = Some(daemon);
    drop(guard);

    Ok(Json(MessageResponse { message: "daemon started".to_string() }))
}

pub async fn daemon_stop_handler(
    State(state): State<AppState>,
) -> Result<Json<MessageResponse>, ApiError> {
    let stopped = stop_daemon(&state).await;

    if stopped {
        Ok(Json(MessageResponse { message: "daemon stopped".to_string() }))
    } else {
        Ok(Json(MessageResponse { message: "daemon not running".to_string() }))
    }
}

pub async fn deals_handler(
    State(state): State<AppState>,
    Json(payload): Json<DealsRequest>,
) -> Result<Json<DealsResponse>, ApiError> {
    let db = open_db(&state.db_path)?;
    let deals = db.get_deals(payload.search_id).map_err(ApiError::from_db)?;

    Ok(Json(DealsResponse { deals }))
}

pub async fn stats_handler(
    State(state): State<AppState>,
    Json(payload): Json<StatsRequest>,
) -> Result<Json<StatsResponse>, ApiError> {
    let db = open_db(&state.db_path)?;
    let search_stats = db.update_search_stats(payload.search_id).map_err(ApiError::from_db)?;

    Ok(Json(StatsResponse { stats: search_stats }))
}

pub async fn search_handler(
    State(state): State<AppState>,
    Json(payload): Json<SearchRequest>,
) -> Result<Json<SearchResponseBody>, ApiError> {
    let sort = payload.sort.unwrap_or_else(|| "relevance".to_string());
    let format = payload.format.unwrap_or_else(|| "table".to_string());

    let sort_order: SortOrder = sort.parse().map_err(ApiError::bad_request)?;
    let output_format: OutputFormat = format.parse().map_err(ApiError::bad_request)?;

    let client = OlxClient::new(&state.config).map_err(ApiError::internal)?;

    let city_id = if let Some(ref city_name) = payload.city {
        let location = client.lookup_city(city_name).await.map_err(|e| {
            // Distinguish between "not found" (404) and API failure (502)
            let err_msg = e.to_string();
            if err_msg.contains("not found") || err_msg.contains("No results") {
                ApiError::not_found(format!("City not found: {city_name}"))
            } else {
                ApiError::upstream_error(format!("City lookup failed: {e}"))
            }
        })?;
        match location {
            Some(loc) => loc.city.id,
            None => {
                return Err(ApiError::not_found(format!("City not found: {city_name}")));
            }
        }
    } else {
        None
    };

    let params = SearchParams {
        query: payload.query.clone(),
        city_id,
        radius_km: payload.radius,
        category_id: None,
        sort: sort_order,
        offset: 0,
        limit: 50,
    };

    let max_results = payload.max.unwrap_or(20);
    let fetch_count = max_results * 3;
    let all_offers = client.search_all(&params, fetch_count).await.map_err(ApiError::internal)?;

    let offers: Vec<_> = all_offers
        .into_iter()
        .filter(|o| {
            let price = o.get_price();
            match (price, payload.min_price, payload.max_price) {
                (Some(p), Some(min), Some(max)) => p >= min && p <= max,
                (Some(p), Some(min), None) => p >= min,
                (Some(p), None, Some(max)) => p <= max,
                (None, _, _) | (Some(_), None, None) => true,
            }
        })
        .take(max_results as usize)
        .collect();

    if offers.is_empty() {
        return Ok(Json(SearchResponseBody {
            output: format!("No results found for '{}'", payload.query),
        }));
    }

    let output = format_results(FormatParams {
        format: output_format,
        query: &payload.query,
        sort: &sort,
        offers: &offers,
        min_price: payload.min_price,
        max_price: payload.max_price,
        city: payload.city,
        radius: payload.radius,
    });

    Ok(Json(SearchResponseBody { output }))
}
