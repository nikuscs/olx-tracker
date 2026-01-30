use anyhow::{Context, Result};
use reqwest::{Client, Proxy};
use std::time::Duration;
use tracing::{debug, info};

use crate::config::Config;

use super::models::{OfferData, SearchResponse};

pub struct OlxClient {
    client: Client,
    base_url: String,
    bearer_token: String,
    request_delay: Duration,
}

#[derive(Debug, Clone)]
pub struct SearchParams {
    pub query: String,
    pub city: Option<String>,
    pub radius_km: Option<i32>,
    pub category_id: Option<i64>,
    pub offset: i32,
    pub limit: i32,
}

impl Default for SearchParams {
    fn default() -> Self {
        Self {
            query: String::new(),
            city: None,
            radius_km: None,
            category_id: None,
            offset: 0,
            limit: 50,
        }
    }
}

impl OlxClient {
    pub fn new(config: &Config) -> Result<Self> {
        let mut client_builder =
            Client::builder().user_agent(&config.api.user_agent).timeout(Duration::from_secs(30));

        if config.proxy.enabled {
            if let Some(proxy_url) = &config.proxy.url {
                let proxy = Proxy::all(proxy_url)
                    .with_context(|| format!("Invalid proxy URL: {proxy_url}"))?;
                client_builder = client_builder.proxy(proxy);
                info!("Using proxy: {}", proxy_url);
            }
        }

        let client = client_builder.build()?;

        Ok(Self {
            client,
            base_url: config.api.base_url.clone(),
            bearer_token: config.auth.bearer_token.clone(),
            request_delay: Duration::from_millis(config.api.request_delay_ms),
        })
    }

    pub async fn search(&self, params: &SearchParams) -> Result<SearchResponse> {
        let mut url = format!("{}?query={}", self.base_url, urlencoding::encode(&params.query));

        url.push_str(&format!("&offset={}&limit={}", params.offset, params.limit));

        if let Some(city) = &params.city {
            url.push_str(&format!("&city={}", urlencoding::encode(city)));
        }

        if let Some(radius) = params.radius_km {
            url.push_str(&format!("&distance={radius}"));
        }

        if let Some(category) = params.category_id {
            url.push_str(&format!("&category_id={category}"));
        }

        debug!("Searching OLX: {}", url);

        let response = self
            .client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.bearer_token))
            .header("Accept", "application/json")
            .send()
            .await
            .context("Failed to send search request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
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
                city: params.city.clone(),
                radius_km: params.radius_km,
                category_id: params.category_id,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_params_default() {
        let params = SearchParams::default();
        assert_eq!(params.offset, 0);
        assert_eq!(params.limit, 50);
    }
}
