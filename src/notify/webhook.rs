use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use tracing::{debug, info, warn};

use crate::config::NotificationConfig;
use crate::db::Listing;

use super::Notifier;

pub struct WebhookNotifier {
    client: Client,
    config: NotificationConfig,
}

#[derive(Debug, Serialize)]
struct WebhookPayload {
    event: String,
    listings: Vec<ListingPayload>,
    #[serde(skip_serializing_if = "Option::is_none")]
    avg_price: Option<f64>,
}

#[derive(Debug, Serialize)]
struct ListingPayload {
    id: i64,
    title: String,
    price: Option<f64>,
    currency: String,
    url: String,
    city: Option<String>,
    seller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discount_pct: Option<f64>,
}

impl WebhookNotifier {
    pub fn new(config: NotificationConfig) -> Self {
        Self { client: Client::new(), config }
    }

    async fn send_webhook(&self, payload: &WebhookPayload) -> Result<()> {
        let Some(url) = &self.config.webhook_url else {
            debug!("No webhook URL configured, skipping notification");
            return Ok(());
        };

        info!("Sending webhook: {} ({} listings)", payload.event, payload.listings.len());

        let response =
            self.client.post(url).json(payload).send().await.context("Failed to send webhook")?;

        if !response.status().is_success() {
            warn!("Webhook returned non-success status: {}", response.status());
        }

        Ok(())
    }
}

#[async_trait]
impl Notifier for WebhookNotifier {
    async fn notify_new_listings(&self, listings: &[Listing]) -> Result<()> {
        if !self.config.notify_on_new_listing || listings.is_empty() {
            return Ok(());
        }

        let payload = WebhookPayload {
            event: "new_listings".to_string(),
            listings: listings
                .iter()
                .map(|l| ListingPayload {
                    id: l.id,
                    title: l.title.clone(),
                    price: l.price,
                    currency: l.currency.clone(),
                    url: l.url.clone(),
                    city: l.city.clone(),
                    seller: l.seller_name.clone(),
                    old_price: None,
                    discount_pct: None,
                })
                .collect(),
            avg_price: None,
        };

        self.send_webhook(&payload).await
    }

    async fn notify_price_drops(&self, drops: &[(Listing, f64, f64)]) -> Result<()> {
        if !self.config.notify_on_price_drop || drops.is_empty() {
            return Ok(());
        }

        let payload = WebhookPayload {
            event: "price_drops".to_string(),
            listings: drops
                .iter()
                .map(|(l, old, new)| {
                    let discount =
                        if *old > 0.0 { Some(((old - new) / old) * 100.0) } else { None };

                    ListingPayload {
                        id: l.id,
                        title: l.title.clone(),
                        price: Some(*new),
                        currency: l.currency.clone(),
                        url: l.url.clone(),
                        city: l.city.clone(),
                        seller: l.seller_name.clone(),
                        old_price: Some(*old),
                        discount_pct: discount,
                    }
                })
                .collect(),
            avg_price: None,
        };

        self.send_webhook(&payload).await
    }

    async fn notify_deals(&self, deals: &[Listing], avg_price: Option<f64>) -> Result<()> {
        if !self.config.notify_on_deal || deals.is_empty() {
            return Ok(());
        }

        let payload = WebhookPayload {
            event: "deals".to_string(),
            listings: deals
                .iter()
                .map(|l| {
                    let discount = match (l.price, avg_price) {
                        (Some(price), Some(avg)) if avg > 0.0 => {
                            Some(((avg - price) / avg) * 100.0)
                        }
                        _ => None,
                    };

                    ListingPayload {
                        id: l.id,
                        title: l.title.clone(),
                        price: l.price,
                        currency: l.currency.clone(),
                        url: l.url.clone(),
                        city: l.city.clone(),
                        seller: l.seller_name.clone(),
                        old_price: None,
                        discount_pct: discount,
                    }
                })
                .collect(),
            avg_price,
        };

        self.send_webhook(&payload).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listing_payload_serialization() {
        let payload = ListingPayload {
            id: 123,
            title: "Test Item".to_string(),
            price: Some(100.0),
            currency: "EUR".to_string(),
            url: "https://olx.pt/123".to_string(),
            city: Some("Porto".to_string()),
            seller: Some("John".to_string()),
            old_price: None,
            discount_pct: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("Test Item"));
        assert!(!json.contains("old_price")); // Should be skipped
    }
}
