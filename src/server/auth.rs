//! Authentication middleware

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use super::error::ApiError;
use super::state::AppState;

/// Authentication middleware that checks for valid API key
pub async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Skip auth check if no API key is configured
    let Some(expected) = state.api_key.as_ref() else {
        return Ok(next.run(request).await);
    };

    let provided = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .or_else(|| headers.get("api-key").and_then(|v| v.to_str().ok()))
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
        });

    if provided == Some(expected.as_str()) {
        Ok(next.run(request).await)
    } else {
        Err(ApiError::unauthorized())
    }
}
