pub mod discord;
pub mod webhook;

pub use discord::DiscordNotifier;
pub use webhook::WebhookNotifier;

use anyhow::Result;
use async_trait::async_trait;

use crate::config::NotificationConfig;
use crate::db::Listing;

/// Trait for implementing notification backends
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Notify about new listings
    async fn notify_new_listings(&self, listings: &[Listing]) -> Result<()>;

    /// Notify about price drops
    async fn notify_price_drops(&self, drops: &[(Listing, f64, f64)]) -> Result<()>;

    /// Notify about deals (listings below average price)
    async fn notify_deals(&self, deals: &[Listing], avg_price: Option<f64>) -> Result<()>;
}

/// Composite notifier that sends to multiple backends
pub struct MultiNotifier {
    notifiers: Vec<Box<dyn Notifier>>,
    config: NotificationConfig,
}

impl MultiNotifier {
    pub fn from_config(config: NotificationConfig) -> Self {
        let mut notifiers: Vec<Box<dyn Notifier>> = Vec::new();

        // Add generic webhook if configured
        if config.webhook_url.is_some() {
            notifiers.push(Box::new(WebhookNotifier::new(config.clone())));
        }

        // Add Discord webhook if configured
        if let Some(discord_url) = &config.discord_webhook_url {
            notifiers.push(Box::new(DiscordNotifier::new(discord_url.clone())));
        }

        Self { notifiers, config }
    }

    pub fn is_empty(&self) -> bool {
        self.notifiers.is_empty()
    }
}

#[async_trait]
impl Notifier for MultiNotifier {
    async fn notify_new_listings(&self, listings: &[Listing]) -> Result<()> {
        if !self.config.notify_on_new_listing || listings.is_empty() {
            return Ok(());
        }

        for notifier in &self.notifiers {
            if let Err(e) = notifier.notify_new_listings(listings).await {
                tracing::warn!("Failed to send new listings notification: {}", e);
            }
        }
        Ok(())
    }

    async fn notify_price_drops(&self, drops: &[(Listing, f64, f64)]) -> Result<()> {
        if !self.config.notify_on_price_drop || drops.is_empty() {
            return Ok(());
        }

        for notifier in &self.notifiers {
            if let Err(e) = notifier.notify_price_drops(drops).await {
                tracing::warn!("Failed to send price drop notification: {}", e);
            }
        }
        Ok(())
    }

    async fn notify_deals(&self, deals: &[Listing], avg_price: Option<f64>) -> Result<()> {
        if !self.config.notify_on_deal || deals.is_empty() {
            return Ok(());
        }

        for notifier in &self.notifiers {
            if let Err(e) = notifier.notify_deals(deals, avg_price).await {
                tracing::warn!("Failed to send deals notification: {}", e);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::Mutex;

    // Mock notifier that tracks calls for testing
    #[derive(Clone)]
    struct MockNotifier {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl MockNotifier {
        fn new() -> Self {
            Self { calls: Arc::new(Mutex::new(Vec::new())) }
        }

        fn get_calls(&self) -> Vec<String> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait]
    impl Notifier for MockNotifier {
        async fn notify_new_listings(&self, listings: &[Listing]) -> Result<()> {
            self.calls.lock().unwrap().push(format!("new_listings({})", listings.len()));
            Ok(())
        }

        async fn notify_price_drops(&self, drops: &[(Listing, f64, f64)]) -> Result<()> {
            self.calls.lock().unwrap().push(format!("price_drops({})", drops.len()));
            Ok(())
        }

        async fn notify_deals(&self, deals: &[Listing], avg_price: Option<f64>) -> Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push(format!("deals({}, {:?})", deals.len(), avg_price));
            Ok(())
        }
    }

    fn make_test_listing(id: i64, price: Option<f64>) -> Listing {
        Listing {
            id,
            search_id: 1,
            title: format!("Test Listing {id}"),
            price,
            currency: "EUR".to_string(),
            url: format!("https://example.com/{id}"),
            city: Some("Porto".to_string()),
            region: Some("Norte".to_string()),
            seller: Some("Test Seller".to_string()),
            first_seen_at: "2024-01-01T00:00:00Z".to_string(),
            last_seen_at: "2024-01-01T00:00:00Z".to_string(),
            is_deal: false,
        }
    }

    fn make_test_config(
        webhook_url: Option<String>,
        discord_url: Option<String>,
        notify_new: bool,
        notify_drops: bool,
        notify_deals: bool,
    ) -> NotificationConfig {
        NotificationConfig {
            webhook_url,
            discord_webhook_url: discord_url,
            notify_on_new_listing: notify_new,
            notify_on_price_drop: notify_drops,
            notify_on_deal: notify_deals,
        }
    }

    #[test]
    fn test_from_config_webhook_only() {
        let config =
            make_test_config(Some("https://example.com/hook".to_string()), None, true, true, true);
        let multi = MultiNotifier::from_config(config);
        assert_eq!(multi.notifiers.len(), 1);
        assert!(!multi.is_empty());
    }

    #[test]
    fn test_from_config_discord_only() {
        let config =
            make_test_config(None, Some("https://discord.com/hook".to_string()), true, true, true);
        let multi = MultiNotifier::from_config(config);
        assert_eq!(multi.notifiers.len(), 1);
        assert!(!multi.is_empty());
    }

    #[test]
    fn test_from_config_both() {
        let config = make_test_config(
            Some("https://example.com/hook".to_string()),
            Some("https://discord.com/hook".to_string()),
            true,
            true,
            true,
        );
        let multi = MultiNotifier::from_config(config);
        assert_eq!(multi.notifiers.len(), 2);
        assert!(!multi.is_empty());
    }

    #[test]
    fn test_from_config_empty() {
        let config = make_test_config(None, None, true, true, true);
        let multi = MultiNotifier::from_config(config);
        assert_eq!(multi.notifiers.len(), 0);
        assert!(multi.is_empty());
    }

    #[test]
    fn test_is_empty() {
        let empty_config = make_test_config(None, None, true, true, true);
        let empty_multi = MultiNotifier::from_config(empty_config);
        assert!(empty_multi.is_empty());

        let non_empty_config =
            make_test_config(Some("https://example.com/hook".to_string()), None, true, true, true);
        let non_empty_multi = MultiNotifier::from_config(non_empty_config);
        assert!(!non_empty_multi.is_empty());
    }

    #[tokio::test]
    async fn test_notify_new_listings_enabled() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, true, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        let listings = vec![make_test_listing(1, Some(100.0)), make_test_listing(2, Some(200.0))];

        multi.notify_new_listings(&listings).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "new_listings(2)");
    }

    #[tokio::test]
    async fn test_notify_new_listings_disabled() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, false, true, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        let listings = vec![make_test_listing(1, Some(100.0))];

        multi.notify_new_listings(&listings).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 0); // Should not call notifier
    }

    #[tokio::test]
    async fn test_notify_new_listings_empty() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, true, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        multi.notify_new_listings(&[]).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 0); // Should not call notifier for empty list
    }

    #[tokio::test]
    async fn test_notify_price_drops_enabled() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, true, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        let drops = vec![(make_test_listing(1, Some(80.0)), 100.0, 80.0)];

        multi.notify_price_drops(&drops).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "price_drops(1)");
    }

    #[tokio::test]
    async fn test_notify_price_drops_disabled() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, false, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        let drops = vec![(make_test_listing(1, Some(80.0)), 100.0, 80.0)];

        multi.notify_price_drops(&drops).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 0);
    }

    #[tokio::test]
    async fn test_notify_price_drops_empty() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, true, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        multi.notify_price_drops(&[]).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 0);
    }

    #[tokio::test]
    async fn test_notify_deals_enabled() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, true, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        let deals = vec![make_test_listing(1, Some(50.0))];

        multi.notify_deals(&deals, Some(100.0)).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "deals(1, Some(100.0))");
    }

    #[tokio::test]
    async fn test_notify_deals_disabled() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, true, false);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        let deals = vec![make_test_listing(1, Some(50.0))];

        multi.notify_deals(&deals, Some(100.0)).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 0);
    }

    #[tokio::test]
    async fn test_notify_deals_empty() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, true, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        multi.notify_deals(&[], Some(100.0)).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 0);
    }

    #[tokio::test]
    async fn test_notify_deals_no_avg_price() {
        let mock = MockNotifier::new();
        let config = make_test_config(None, None, true, true, true);
        let mut multi = MultiNotifier { notifiers: vec![Box::new(mock.clone())], config };

        let deals = vec![make_test_listing(1, Some(50.0))];

        multi.notify_deals(&deals, None).await.unwrap();

        let calls = mock.get_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0], "deals(1, None)");
    }

    #[tokio::test]
    async fn test_multiple_notifiers() {
        let mock1 = MockNotifier::new();
        let mock2 = MockNotifier::new();
        let config = make_test_config(None, None, true, true, true);
        let mut multi = MultiNotifier {
            notifiers: vec![Box::new(mock1.clone()), Box::new(mock2.clone())],
            config,
        };

        let listings = vec![make_test_listing(1, Some(100.0))];

        multi.notify_new_listings(&listings).await.unwrap();

        // Both notifiers should be called
        assert_eq!(mock1.get_calls().len(), 1);
        assert_eq!(mock2.get_calls().len(), 1);
    }
}
