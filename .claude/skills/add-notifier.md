# Add New Notifier

Create a new notification backend (e.g., Telegram, Email).

## Steps

1. Create new file `src/notify/my_notifier.rs`:

```rust
use anyhow::Result;
use async_trait::async_trait;
use crate::db::Listing;
use super::Notifier;

pub struct MyNotifier {
    // config fields
}

impl MyNotifier {
    pub fn new(/* config */) -> Self {
        Self { /* ... */ }
    }
}

#[async_trait]
impl Notifier for MyNotifier {
    async fn notify_new_listings(&self, listings: &[Listing]) -> Result<()> {
        // Send notification for new listings
        Ok(())
    }

    async fn notify_price_drops(&self, drops: &[(Listing, f64, f64)]) -> Result<()> {
        // drops: (listing, old_price, new_price)
        Ok(())
    }

    async fn notify_deals(&self, deals: &[Listing], avg_price: Option<f64>) -> Result<()> {
        // Notify about deals below average
        Ok(())
    }
}
```

2. Export in `src/notify/mod.rs`:

```rust
mod my_notifier;
pub use my_notifier::MyNotifier;
```

3. Add config options to `src/config.rs` if needed.

4. Use in `main.rs` alongside or instead of `WebhookNotifier`.
