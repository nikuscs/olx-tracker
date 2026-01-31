---
name: add-notifier
description: Create a new notification backend (e.g., Telegram, Email, Slack). Use when adding custom notification channels.
user-invocable: false
---

# Add New Notifier

Create a new notification backend for olx-tracker (e.g., Telegram, Email, Slack).

## Notifier Structure

Notifiers implement the `Notifier` trait using `async_trait` and send notifications for three event types:
- New listings
- Price drops
- Deal detection

## Steps

### 1. Create the notifier file

Create `src/notify/$ARGUMENTS.rs` or `src/notify/my_notifier.rs`:

```rust
use anyhow::Result;
use async_trait::async_trait;
use crate::db::Listing;
use super::Notifier;

pub struct MyNotifier {
    // Configuration fields (webhook URL, API key, etc.)
    api_url: String,
}

impl MyNotifier {
    pub fn new(api_url: String) -> Self {
        Self { api_url }
    }
}

#[async_trait]
impl Notifier for MyNotifier {
    async fn notify_new_listings(&self, listings: &[Listing]) -> Result<()> {
        // Send notification for new listings
        if listings.is_empty() {
            return Ok(());
        }

        // TODO: Send HTTP request or call API
        Ok(())
    }

    async fn notify_price_drops(&self, drops: &[(Listing, f64, f64)]) -> Result<()> {
        // drops: Vec<(listing, old_price, new_price)>
        if drops.is_empty() {
            return Ok(());
        }

        // TODO: Send price drop notification
        Ok(())
    }

    async fn notify_deals(&self, deals: &[Listing], avg_price: Option<f64>) -> Result<()> {
        // deals: listings below average or target price
        if deals.is_empty() {
            return Ok(());
        }

        // TODO: Send deal notification
        Ok(())
    }
}
```

### 2. Export in notify module

Add to `src/notify/mod.rs`:

```rust
mod my_notifier;
pub use my_notifier::MyNotifier;
```

### 3. Add configuration (optional)

Add config fields to `src/config.rs` if your notifier needs configuration:

```rust
#[derive(Debug, Deserialize, Clone)]
pub struct NotificationConfig {
    // ... existing fields ...
    pub my_notifier_url: Option<String>,
}
```

### 4. Use in MultiNotifier

Add to `MultiNotifier::from_config()` in `src/notify/mod.rs`:

```rust
if let Some(url) = config.my_notifier_url {
    notifiers.push(Box::new(MyNotifier::new(url)));
}
```

### 5. Add tests

Add tests under `#[cfg(test)]` module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_my_notifier() {
        let notifier = MyNotifier::new("https://api.example.com".to_string());
        // Test notification logic
    }
}
```

## Existing Notifiers

- `DiscordNotifier`: Sends rich embeds to Discord webhooks
- `WebhookNotifier`: Sends JSON payloads to generic webhooks

## Notifier Trait

```rust
#[async_trait]
pub trait Notifier {
    async fn notify_new_listings(&self, listings: &[Listing]) -> Result<()>;
    async fn notify_price_drops(&self, drops: &[(Listing, f64, f64)]) -> Result<()>;
    async fn notify_deals(&self, deals: &[Listing], avg_price: Option<f64>) -> Result<()>;
}
```

All methods are async and should handle empty inputs gracefully.
