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
