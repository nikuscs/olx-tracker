//! Request and response types for the HTTP API

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct AddSearchRequest {
    pub name: String,
    pub keyword: String,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub city: Option<String>,
    pub radius: Option<i32>,
    pub category: Option<i64>,
    pub sort: Option<String>,
    pub days: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListRequest {
    pub all: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RunRequest {
    pub search_id: Option<i64>,
    pub max_results: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DaemonRequest {
    pub interval: Option<u64>,
    pub max_results: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DealsRequest {
    pub search_id: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StatsRequest {
    pub search_id: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ToggleRequest {
    pub search_id: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RemoveRequest {
    pub search_id: i64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SearchRequest {
    pub query: String,
    pub max: Option<i32>,
    pub sort: Option<String>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub city: Option<String>,
    pub radius: Option<i32>,
    pub format: Option<String>,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize)]
pub struct AddSearchResponse {
    pub id: i64,
}

#[derive(Serialize)]
pub struct RunResponse {
    pub total_new: usize,
    pub total_deals: usize,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub message: String,
}

#[derive(Serialize)]
pub struct SearchListResponse {
    pub searches: Vec<crate::db::Search>,
}

#[derive(Serialize)]
pub struct DealsResponse {
    pub deals: Vec<crate::db::Listing>,
}

#[derive(Serialize)]
pub struct StatsResponse {
    pub stats: crate::db::SearchStats,
}

#[derive(Serialize)]
pub struct SearchResponseBody {
    pub output: String,
}
