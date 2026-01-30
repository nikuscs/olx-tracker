use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use tracing::{debug, info, warn};

use crate::config::NotificationConfig;
use crate::db::Listing;

use super::Notifier;

#[derive(Debug)]
pub struct WebhookNotifier {
    client: Client,
    webhook_url: Option<String>,
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
        Self { client: Client::new(), webhook_url: config.webhook_url }
    }

    async fn send_webhook(&self, payload: &WebhookPayload) -> Result<()> {
        let Some(url) = &self.webhook_url else {
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
        if listings.is_empty() {
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
        if drops.is_empty() {
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
        if deals.is_empty() {
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

    fn make_test_listing(id: i64, title: &str, price: Option<f64>) -> Listing {
        Listing {
            id,
            search_id: 1,
            title: title.to_string(),
            price,
            currency: "EUR".to_string(),
            url: format!("https://olx.pt/{id}"),
            city: Some("Porto".to_string()),
            region: Some("Norte".to_string()),
            seller_name: Some("John".to_string()),
            first_seen_at: "2024-01-01T00:00:00Z".to_string(),
            last_seen_at: None,
            is_deal: false,
        }
    }

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

    #[test]
    fn test_webhook_payload_with_price_drop() {
        let listing = make_test_listing(1, "iPhone", Some(80.0));
        let drops = [(listing, 100.0, 80.0)];

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

        assert_eq!(payload.event, "price_drops");
        assert_eq!(payload.listings[0].old_price, Some(100.0));
        assert_eq!(payload.listings[0].discount_pct, Some(20.0)); // 20% off
    }

    #[test]
    fn test_webhook_payload_discount_calculation() {
        // Test discount from 100 to 80 = 20%
        let old = 100.0;
        let new = 80.0;
        let discount = ((old - new) / old) * 100.0;
        assert_eq!(discount, 20.0);

        // Test discount from 200 to 150 = 25%
        let old = 200.0;
        let new = 150.0;
        let discount = ((old - new) / old) * 100.0;
        assert_eq!(discount, 25.0);
    }

    #[test]
    fn test_webhook_payload_with_avg_price() {
        let listing = make_test_listing(1, "Deal", Some(50.0));
        let deals = [listing];
        let avg_price = Some(100.0);

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

        assert_eq!(payload.event, "deals");
        assert_eq!(payload.avg_price, Some(100.0));
        assert_eq!(payload.listings[0].discount_pct, Some(50.0)); // 50% below average
    }

    #[test]
    fn test_webhook_notifier_creation() {
        let config = NotificationConfig {
            webhook_url: Some("https://example.com/hook".to_string()),
            discord_webhook_url: None,
            notify_on_new_listing: true,
            notify_on_price_drop: true,
            notify_on_deal: true,
        };

        let notifier = WebhookNotifier::new(config);
        assert!(notifier.webhook_url.is_some());
    }

    #[test]
    fn test_webhook_notifier_no_url() {
        let config = NotificationConfig {
            webhook_url: None,
            discord_webhook_url: None,
            notify_on_new_listing: false,
            notify_on_price_drop: false,
            notify_on_deal: false,
        };

        let notifier = WebhookNotifier::new(config);
        assert!(notifier.webhook_url.is_none());
    }

    #[tokio::test]
    async fn test_notify_empty_listings() {
        let config = NotificationConfig {
            webhook_url: Some("https://example.com/hook".to_string()),
            discord_webhook_url: None,
            notify_on_new_listing: true,
            notify_on_price_drop: true,
            notify_on_deal: true,
        };

        let notifier = WebhookNotifier::new(config);
        let result = notifier.notify_new_listings(&[]).await;
        assert!(result.is_ok()); // Should return Ok without sending
    }

    #[tokio::test]
    async fn test_notify_empty_price_drops() {
        let config = NotificationConfig {
            webhook_url: Some("https://example.com/hook".to_string()),
            discord_webhook_url: None,
            notify_on_new_listing: true,
            notify_on_price_drop: true,
            notify_on_deal: true,
        };

        let notifier = WebhookNotifier::new(config);
        let result = notifier.notify_price_drops(&[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_empty_deals() {
        let config = NotificationConfig {
            webhook_url: Some("https://example.com/hook".to_string()),
            discord_webhook_url: None,
            notify_on_new_listing: true,
            notify_on_price_drop: true,
            notify_on_deal: true,
        };

        let notifier = WebhookNotifier::new(config);
        let result = notifier.notify_deals(&[], None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_webhook_no_url() {
        let config = NotificationConfig {
            webhook_url: None,
            discord_webhook_url: None,
            notify_on_new_listing: false,
            notify_on_price_drop: false,
            notify_on_deal: false,
        };

        let notifier = WebhookNotifier::new(config);
        let payload =
            WebhookPayload { event: "test".to_string(), listings: vec![], avg_price: None };

        let result = notifier.send_webhook(&payload).await;
        assert!(result.is_ok()); // Should return Ok without sending
    }

    #[test]
    fn test_webhook_payload_serialization() {
        let listing = make_test_listing(1, "Test", Some(100.0));
        let payload = WebhookPayload {
            event: "new_listings".to_string(),
            listings: vec![ListingPayload {
                id: listing.id,
                title: listing.title.clone(),
                price: listing.price,
                currency: listing.currency.clone(),
                url: listing.url.clone(),
                city: listing.city.clone(),
                seller: listing.seller_name,
                old_price: None,
                discount_pct: None,
            }],
            avg_price: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("new_listings"));
        assert!(json.contains("Test"));
        assert!(!json.contains("\"avg_price\"")); // Should be skipped when None
    }

    #[test]
    fn test_price_drop_zero_old_price() {
        // Edge case: old price is 0, discount should be None
        let old = 0.0;
        let new = 50.0;
        let discount = if old > 0.0 { Some(((old - new) / old) * 100.0) } else { None };
        assert_eq!(discount, None);
    }
}
