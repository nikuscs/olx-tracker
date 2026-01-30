pub mod webhook;

pub use webhook::WebhookNotifier;

use anyhow::Result;
use async_trait::async_trait;

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
