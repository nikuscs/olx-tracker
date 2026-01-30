use anyhow::{Context, Result};
use fake_user_agent::get_rua;
use reqwest::header::{
    ACCEPT, ACCEPT_ENCODING, ACCEPT_LANGUAGE, CACHE_CONTROL, CONNECTION, HOST, HeaderMap,
    HeaderValue, REFERER,
};
use reqwest::{Client, Proxy};
use std::str::FromStr;
use std::time::Duration;
use tracing::{debug, info};

use crate::config::Config;

use super::models::{LocationResponse, LocationResult, OfferData, SearchResponse};

/// Mask credentials in proxy URL for safe logging
fn mask_proxy_credentials(url: &str) -> String {
    // Try to parse and mask credentials like "user:pass@host"
    if let Some(at_pos) = url.find('@') {
        // Find the protocol separator
        if let Some(proto_end) = url.find("://") {
            let proto = &url[..proto_end + 3];
            let after_at = &url[at_pos + 1..];
            return format!("{proto}***:***@{after_at}");
        }
    }
    url.to_string()
}

/// Sort order for search results
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SortOrder {
    /// Most recent first (default)
    #[default]
    Newest,
    /// Cheapest first
    PriceAsc,
    /// Most expensive first
    PriceDesc,
    /// Most relevant first
    Relevance,
}

impl SortOrder {
    /// Returns the OLX API parameter value
    pub const fn as_api_param(self) -> &'static str {
        match self {
            Self::Newest => "created_at:desc",
            Self::PriceAsc => "filter_float_price:asc",
            Self::PriceDesc => "filter_float_price:desc",
            Self::Relevance => "relevance:desc",
        }
    }
}

impl FromStr for SortOrder {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "newest" | "new" | "recent" => Ok(Self::Newest),
            "price_asc" | "price-asc" | "cheap" | "cheapest" => Ok(Self::PriceAsc),
            "price_desc" | "price-desc" | "expensive" => Ok(Self::PriceDesc),
            "relevance" | "relevant" => Ok(Self::Relevance),
            _ => Err(format!(
                "Unknown sort order: {s}. Valid: newest, cheapest, expensive, relevance"
            )),
        }
    }
}

#[derive(Debug)]
pub struct OlxClient {
    client: Client,
    base_url: String,
    bearer_token: Option<String>,
    request_delay: Duration,
}

#[derive(Debug, Clone)]
pub struct SearchParams {
    pub query: String,
    pub city_id: Option<i64>,
    pub radius_km: Option<i32>,
    pub category_id: Option<i64>,
    pub sort: SortOrder,
    pub offset: i32,
    pub limit: i32,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query: String::new(),
            city_id: None,
            radius_km: None,
            category_id: None,
            sort: SortOrder::default(),
            offset: 0,
            limit: 50,
        }
    }
}

impl OlxClient {
    pub fn new(config: &Config) -> Result<Self> {
        // Use custom user agent or generate a random realistic one
        let user_agent = if config.api.user_agent.contains("Mozilla") {
            config.api.user_agent.clone()
        } else {
            get_rua().to_string()
        };

        // Build headers that mimic a real browser
        let mut headers = HeaderMap::new();
        headers.insert(ACCEPT, HeaderValue::from_static("application/json, text/plain, */*"));
        headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.9,pt;q=0.8"));
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, deflate, br"));
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-cache"));
        headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));

        // Extract host from base URL for proper Host header
        let host = config
            .api
            .get_base_url()
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("www.olx.pt");
        if let Ok(host_value) = HeaderValue::from_str(host) {
            headers.insert(HOST, host_value);
        }

        // Referer makes it look like we came from the website
        let referer = format!("https://{host}/");
        if let Ok(referer_value) = HeaderValue::from_str(&referer) {
            headers.insert(REFERER, referer_value);
        }

        // Sec-Fetch headers (modern browsers send these)
        headers.insert("Sec-Fetch-Dest", HeaderValue::from_static("empty"));
        headers.insert("Sec-Fetch-Mode", HeaderValue::from_static("cors"));
        headers.insert("Sec-Fetch-Site", HeaderValue::from_static("same-origin"));
        headers.insert("Sec-CH-UA-Mobile", HeaderValue::from_static("?0"));
        headers.insert("Sec-CH-UA-Platform", HeaderValue::from_static("\"macOS\""));

        let mut client_builder = Client::builder()
            .user_agent(&user_agent)
            .default_headers(headers)
            .timeout(Duration::from_secs(30))
            .gzip(true)
            .brotli(true);

        if config.proxy.enabled {
            if let Some(proxy_url) = &config.proxy.url {
                let proxy = Proxy::all(proxy_url).with_context(|| {
                    format!("Invalid proxy URL: {}", mask_proxy_credentials(proxy_url))
                })?;
                client_builder = client_builder.proxy(proxy);
                info!("Using proxy: {}", mask_proxy_credentials(proxy_url));
            }
        }

        let client = client_builder.build()?;

        debug!("HTTP client initialized with user agent: {}", user_agent);

        Ok(Self {
            client,
            base_url: config.api.get_base_url().to_string(),
            bearer_token: config.auth.bearer_token.clone(),
            request_delay: Duration::from_millis(config.api.request_delay_ms),
        })
    }

    /// Lookup city ID from city name using autocomplete API
    pub async fn lookup_city(&self, city_name: &str) -> Result<Option<LocationResult>> {
        let base = self.base_url.replace("/api/v1/offers", "").replace("/api/v1/offer", "");
        let url = format!(
            "{}/api/v1/geo-encoder/location-autocomplete/?query={}",
            base,
            urlencoding::encode(city_name)
        );

        debug!("Looking up city: {}", url);

        let response = self.client.get(&url).send().await.context("Failed to lookup city")?;

        let status = response.status();
        if !status.is_success() {
            let body =
                response.text().await.unwrap_or_else(|e| format!("<failed to read body: {e}>"));
            anyhow::bail!("City lookup failed with status {status}: {body}");
        }

        let result: LocationResponse =
            response.json().await.context("Failed to parse location response")?;

        // Smart matching: prioritize exact city match, then region match
        let query_lower = city_name.to_lowercase();

        // 1. Exact city name match
        if let Some(loc) =
            result.data.iter().find(|loc| loc.city.name.to_lowercase() == query_lower)
        {
            return Ok(Some(loc.clone()));
        }

        // 2. Region/municipality match (e.g., "Porto" matches any city in Porto region)
        if let Some(loc) = result.data.iter().find(|loc| {
            loc.region.as_ref().map(|r| r.name.to_lowercase()) == Some(query_lower.clone())
                || loc.municipality.as_ref().map(|m| m.name.to_lowercase())
                    == Some(query_lower.clone())
        }) {
            return Ok(Some(loc.clone()));
        }

        // 3. Fallback to first result
        Ok(result.data.into_iter().next())
    }

    pub async fn search(&self, params: &SearchParams) -> Result<SearchResponse> {
        let mut url = format!("{}?query={}", self.base_url, urlencoding::encode(&params.query));

        url.push_str(&format!("&offset={}&limit={}", params.offset, params.limit));
        url.push_str(&format!("&sort_by={}", params.sort.as_api_param()));

        if let Some(city_id) = params.city_id {
            url.push_str(&format!("&city_id={city_id}"));
        }

        if let Some(radius) = params.radius_km {
            url.push_str(&format!("&distance={radius}"));
        }

        if let Some(category) = params.category_id {
            url.push_str(&format!("&category_id={category}"));
        }

        debug!("Searching OLX: {}", url);

        let mut request = self.client.get(&url);

        // Add bearer token only if provided (OLX search is public)
        if let Some(token) = &self.bearer_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request.send().await.context("Failed to send search request")?;

        let status = response.status();
        if !status.is_success() {
            let body =
                response.text().await.unwrap_or_else(|e| format!("<failed to read body: {e}>"));
            anyhow::bail!("API request failed with status {status}: {body}");
        }

        let result: SearchResponse =
            response.json().await.context("Failed to parse search response")?;

        debug!(
            "Found {} listings (total: {:?})",
            result.data.len(),
            result.metadata.total_elements
        );

        Ok(result)
    }

    pub async fn search_all(
        &self,
        params: &SearchParams,
        max_results: i32,
    ) -> Result<Vec<OfferData>> {
        let mut all_results = Vec::new();
        let mut offset = 0;
        let limit = params.limit.min(50);

        loop {
            let search_params = SearchParams {
                query: params.query.clone(),
                city_id: params.city_id,
                radius_km: params.radius_km,
                category_id: params.category_id,
                sort: params.sort,
                offset,
                limit,
            };

            let response = self.search(&search_params).await?;
            let count = response.data.len();
            all_results.extend(response.data);

            offset += count as i32;

            // Stop if we've reached max results or no more results
            if count == 0 || all_results.len() as i32 >= max_results {
                break;
            }

            // Rate limiting
            tokio::time::sleep(self.request_delay).await;
        }

        // Truncate to max results
        all_results.truncate(max_results as usize);
        Ok(all_results)
    }

    pub const fn request_delay(&self) -> Duration {
        self.request_delay
    }
}

/// Trait for search client operations, enabling testing via mocking
#[async_trait::async_trait]
pub trait SearchClient {
    async fn lookup_city(&self, city_name: &str) -> Result<Option<LocationResult>>;
    async fn search_all(&self, params: &SearchParams, max_results: i32) -> Result<Vec<OfferData>>;
    fn request_delay(&self) -> Duration;
}

#[async_trait::async_trait]
impl SearchClient for OlxClient {
    async fn lookup_city(&self, city_name: &str) -> Result<Option<LocationResult>> {
        self.lookup_city(city_name).await
    }

    async fn search_all(&self, params: &SearchParams, max_results: i32) -> Result<Vec<OfferData>> {
        self.search_all(params, max_results).await
    }

    fn request_delay(&self) -> Duration {
        self.request_delay()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_default() {
        let params = SearchParams::default();
        assert_eq!(params.offset, 0);
        assert_eq!(params.limit, 50);
    }

    #[test]
    fn test_mask_proxy_credentials_with_auth() {
        let url = "socks5://user:password@proxy.example.com:1080";
        let masked = mask_proxy_credentials(url);
        assert_eq!(masked, "socks5://***:***@proxy.example.com:1080");
    }

    #[test]
    fn test_mask_proxy_credentials_http_with_auth() {
        let url = "http://admin:secret123@10.0.0.1:8080";
        let masked = mask_proxy_credentials(url);
        assert_eq!(masked, "http://***:***@10.0.0.1:8080");
    }

    #[test]
    fn test_mask_proxy_credentials_https_with_auth() {
        let url = "https://myuser:mypass@proxy.corp.net:3128/path";
        let masked = mask_proxy_credentials(url);
        assert_eq!(masked, "https://***:***@proxy.corp.net:3128/path");
    }

    #[test]
    fn test_mask_proxy_credentials_no_auth() {
        let url = "socks5://proxy.example.com:1080";
        let masked = mask_proxy_credentials(url);
        assert_eq!(masked, url);
    }

    #[test]
    fn test_mask_proxy_credentials_no_protocol() {
        // Edge case: @ symbol but no protocol - should return unchanged
        let url = "user:pass@host:port";
        let masked = mask_proxy_credentials(url);
        assert_eq!(masked, url);
    }

    #[test]
    fn test_mask_proxy_credentials_empty() {
        let url = "";
        let masked = mask_proxy_credentials(url);
        assert_eq!(masked, url);
    }
}
