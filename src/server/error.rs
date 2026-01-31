//! Error types and handling for the HTTP API

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use super::models::ErrorResponse;

/// Categorized error types for better HTTP status code mapping
#[derive(Debug)]
pub enum ErrorKind {
    NotFound,
    InvalidInput,
    DatabaseError,
    UpstreamError,
    Internal,
}

/// API error type with consistent formatting and `IntoResponse` implementation
pub struct ApiError {
    pub status: StatusCode,
    pub message: String,
}

impl ApiError {
    pub fn new(kind: ErrorKind, msg: impl ToString) -> Self {
        let status = match kind {
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::InvalidInput => StatusCode::BAD_REQUEST,
            ErrorKind::UpstreamError => StatusCode::BAD_GATEWAY,
            ErrorKind::DatabaseError | ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self { status, message: msg.to_string() }
    }

    pub fn internal(msg: impl ToString) -> Self {
        Self::new(ErrorKind::Internal, msg)
    }

    pub fn not_found(msg: impl ToString) -> Self {
        Self::new(ErrorKind::NotFound, msg)
    }

    pub fn bad_request(msg: impl ToString) -> Self {
        Self::new(ErrorKind::InvalidInput, msg)
    }

    pub fn upstream_error(msg: impl ToString) -> Self {
        Self::new(ErrorKind::UpstreamError, msg)
    }

    pub fn unauthorized() -> Self {
        Self { status: StatusCode::UNAUTHORIZED, message: "unauthorized".to_string() }
    }

    pub fn timeout() -> Self {
        Self { status: StatusCode::GATEWAY_TIMEOUT, message: "operation timed out".to_string() }
    }

    pub fn from_anyhow(err: anyhow::Error, kind: ErrorKind) -> Self {
        Self::new(kind, err.to_string())
    }

    pub fn from_db(err: anyhow::Error) -> Self {
        Self::new(ErrorKind::DatabaseError, format!("database error: {err}"))
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(ErrorResponse { error: self.message })).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_kind_not_found() {
        let err = ApiError::not_found("resource not found");
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert_eq!(err.message, "resource not found");
    }

    #[test]
    fn test_error_kind_bad_request() {
        let err = ApiError::bad_request("invalid input");
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.message, "invalid input");
    }

    #[test]
    fn test_error_kind_upstream() {
        let err = ApiError::upstream_error("upstream API failed");
        assert_eq!(err.status, StatusCode::BAD_GATEWAY);
        assert_eq!(err.message, "upstream API failed");
    }

    #[test]
    fn test_error_kind_internal() {
        let err = ApiError::internal("something went wrong");
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.message, "something went wrong");
    }

    #[test]
    fn test_error_kind_database() {
        let err = ApiError::new(ErrorKind::DatabaseError, "db connection failed");
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.message, "db connection failed");
    }

    #[test]
    fn test_error_unauthorized() {
        let err = ApiError::unauthorized();
        assert_eq!(err.status, StatusCode::UNAUTHORIZED);
        assert_eq!(err.message, "unauthorized");
    }

    #[test]
    fn test_error_timeout() {
        let err = ApiError::timeout();
        assert_eq!(err.status, StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(err.message, "operation timed out");
    }

    #[test]
    fn test_from_anyhow_internal() {
        let anyhow_err = anyhow::anyhow!("test error");
        let err = ApiError::from_anyhow(anyhow_err, ErrorKind::Internal);
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(err.message, "test error");
    }

    #[test]
    fn test_from_anyhow_not_found() {
        let anyhow_err = anyhow::anyhow!("not found");
        let err = ApiError::from_anyhow(anyhow_err, ErrorKind::NotFound);
        assert_eq!(err.status, StatusCode::NOT_FOUND);
        assert_eq!(err.message, "not found");
    }

    #[test]
    fn test_from_db() {
        let anyhow_err = anyhow::anyhow!("connection failed");
        let err = ApiError::from_db(anyhow_err);
        assert_eq!(err.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(err.message.contains("database error"));
        assert!(err.message.contains("connection failed"));
    }

    #[test]
    fn test_into_response() {
        let err = ApiError::not_found("test");
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_all_error_kinds() {
        let test_cases = vec![
            (ErrorKind::NotFound, StatusCode::NOT_FOUND),
            (ErrorKind::InvalidInput, StatusCode::BAD_REQUEST),
            (ErrorKind::UpstreamError, StatusCode::BAD_GATEWAY),
            (ErrorKind::DatabaseError, StatusCode::INTERNAL_SERVER_ERROR),
            (ErrorKind::Internal, StatusCode::INTERNAL_SERVER_ERROR),
        ];

        for (kind, expected_status) in test_cases {
            let err = ApiError::new(kind, "test message");
            assert_eq!(err.status, expected_status);
            assert_eq!(err.message, "test message");
        }
    }
}
