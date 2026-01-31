use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use tracing::{info, warn};

use crate::db::Listing;

use super::Notifier;

#[derive(Debug)]
pub struct DiscordNotifier {
    client: Client,
    webhook_url: String,
}

#[derive(Debug, Serialize)]
struct DiscordWebhook {
    content: Option<String>,
    embeds: Vec<DiscordEmbed>,
}

#[derive(Debug, Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    fields: Vec<DiscordField>,
    url: Option<String>,
}

#[derive(Debug, Serialize)]
struct DiscordField {
    name: String,
    value: String,
    inline: bool,
}

impl DiscordNotifier {
    pub fn new(webhook_url: String) -> Self {
        Self { client: Client::new(), webhook_url }
    }

    async fn send_webhook(&self, payload: &DiscordWebhook) -> Result<()> {
        info!("Sending Discord notification ({} embeds)", payload.embeds.len());

        let response = self
            .client
            .post(&self.webhook_url)
            .json(payload)
            .send()
            .await
            .context("Failed to send Discord webhook")?;

        if !response.status().is_success() {
            let status = response.status();
            let body =
                response.text().await.unwrap_or_else(|e| format!("<failed to read body: {e}>"));
            warn!("Discord webhook returned non-success status: {} - {}", status, body);
        }

        Ok(())
    }

    fn listing_to_embed(listing: &Listing, color: u32, extra_info: Option<String>) -> DiscordEmbed {
        let price_str = listing
            .price
            .map_or_else(|| "N/A".to_string(), |p| format!("{:.2} {}", p, listing.currency));

        let location = listing.city.as_ref().map_or_else(
            || "Unknown".to_string(),
            |c| listing.region.as_ref().map_or_else(|| c.clone(), |r| format!("{c}, {r}")),
        );

        let mut description = format!("**Price:** {price_str}\n**Location:** {location}");

        if let Some(seller) = &listing.seller_name {
            description.push_str(&format!("\n**Seller:** {seller}"));
        }

        if let Some(info) = extra_info {
            description.push_str(&format!("\n{info}"));
        }

        DiscordEmbed {
            title: listing.title.clone(),
            description,
            color,
            fields: vec![],
            url: Some(listing.url.clone()),
        }
    }
}

#[async_trait]
impl Notifier for DiscordNotifier {
    async fn notify_new_listings(&self, listings: &[Listing]) -> Result<()> {
        if listings.is_empty() {
            return Ok(());
        }

        // Discord has a limit of 10 embeds per message
        for chunk in listings.chunks(10) {
            let embeds: Vec<_> = chunk
                .iter()
                .map(|l| Self::listing_to_embed(l, 0x0034_98db, None)) // Blue
                .collect();

            let payload = DiscordWebhook {
                content: Some(format!("🆕 **{} new listing(s) found!**", chunk.len())),
                embeds,
            };

            self.send_webhook(&payload).await?;
        }

        Ok(())
    }

    async fn notify_price_drops(&self, drops: &[(Listing, f64, f64)]) -> Result<()> {
        if drops.is_empty() {
            return Ok(());
        }

        for chunk in drops.chunks(10) {
            let embeds: Vec<_> = chunk
                .iter()
                .map(|(listing, old_price, new_price)| {
                    let discount = ((old_price - new_price) / old_price) * 100.0;
                    let info = format!(
                        "📉 **Price dropped:** {old_price:.2} € → {new_price:.2} € (-{discount:.1}%)"
                    );
                    Self::listing_to_embed(listing, 0x002e_cc71, Some(info)) // Green
                })
                .collect();

            let payload = DiscordWebhook {
                content: Some(format!("📉 **{} price drop(s) detected!**", chunk.len())),
                embeds,
            };

            self.send_webhook(&payload).await?;
        }

        Ok(())
    }

    async fn notify_deals(&self, deals: &[Listing], avg_price: Option<f64>) -> Result<()> {
        if deals.is_empty() {
            return Ok(());
        }

        for chunk in deals.chunks(10) {
            let embeds: Vec<_> = chunk
                .iter()
                .map(|listing| {
                    let info = avg_price.and_then(|avg| {
                        listing.price.map(|price| {
                            let discount = ((avg - price) / avg) * 100.0;
                            format!("🔥 **{discount:.1}% below average** (avg: {avg:.2} €)")
                        })
                    });
                    Self::listing_to_embed(listing, 0x00e7_4c3c, info) // Red
                })
                .collect();

            let payload = DiscordWebhook {
                content: Some(format!("🔥 **{} deal(s) found!**", chunk.len())),
                embeds,
            };

            self.send_webhook(&payload).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    fn make_listing() -> Listing {
        Listing {
            id: 123,
            search_id: 1,
            title: "Test Item".to_string(),
            price: Some(100.0),
            currency: "EUR".to_string(),
            url: "https://olx.pt/123".to_string(),
            city: Some("Porto".to_string()),
            region: Some("Norte".to_string()),
            seller_name: Some("John".to_string()),
            first_seen_at: "2024-01-01".to_string(),
            last_seen_at: None,
            is_deal: false,
        }
    }

    #[test]
    fn test_listing_to_embed() {
        let listing = make_listing();
        let embed = DiscordNotifier::listing_to_embed(&listing, 0x0034_98db, None);

        assert_eq!(embed.title, "Test Item");
        assert!(embed.description.contains("100.00 EUR"));
        assert!(embed.description.contains("Porto, Norte"));
        assert_eq!(embed.url, Some("https://olx.pt/123".to_string()));
    }

    #[test]
    fn test_listing_to_embed_no_price() {
        let mut listing = make_listing();
        listing.price = None;

        let embed = DiscordNotifier::listing_to_embed(&listing, 0x0034_98db, None);

        assert!(embed.description.contains("N/A"));
    }

    #[test]
    fn test_listing_to_embed_no_location() {
        let mut listing = make_listing();
        listing.city = None;
        listing.region = None;

        let embed = DiscordNotifier::listing_to_embed(&listing, 0x0034_98db, None);

        assert!(embed.description.contains("Unknown"));
    }

    #[test]
    fn test_listing_to_embed_city_only() {
        let mut listing = make_listing();
        listing.region = None;

        let embed = DiscordNotifier::listing_to_embed(&listing, 0x0034_98db, None);

        assert!(embed.description.contains("Porto"));
        assert!(!embed.description.contains(','));
    }

    #[test]
    fn test_listing_to_embed_no_seller() {
        let mut listing = make_listing();
        listing.seller_name = None;

        let embed = DiscordNotifier::listing_to_embed(&listing, 0x0034_98db, None);

        assert!(!embed.description.contains("Seller:"));
    }

    #[test]
    fn test_listing_to_embed_with_extra_info() {
        let listing = make_listing();
        let embed = DiscordNotifier::listing_to_embed(
            &listing,
            0x0034_98db,
            Some("Extra Info Here".to_string()),
        );

        assert!(embed.description.contains("Extra Info Here"));
    }

    #[test]
    fn test_discord_notifier_creation() {
        let notifier = DiscordNotifier::new("https://discord.com/webhook".to_string());
        assert_eq!(notifier.webhook_url, "https://discord.com/webhook");
    }

    #[test]
    fn test_new_listings_embed_creation() {
        let listings = [make_listing()];
        let embeds: Vec<_> = listings
            .iter()
            .map(|l| DiscordNotifier::listing_to_embed(l, 0x0034_98db, None))
            .collect();

        assert_eq!(embeds.len(), 1);
        assert_eq!(embeds[0].color, 0x0034_98db);
    }

    #[test]
    fn test_price_drop_discount_calculation() {
        let old_price = 100.0;
        let new_price = 80.0;
        let discount = ((old_price - new_price) / old_price) * 100.0;

        assert_eq!(discount, 20.0);
    }

    #[test]
    fn test_price_drop_embed_creation() {
        let listing = make_listing();
        let old_price = 120.0;
        let new_price = 100.0;
        let discount = ((old_price - new_price) / old_price) * 100.0;

        let info =
            format!("📉 **Price dropped:** {old_price:.2} € → {new_price:.2} € (-{discount:.1}%)");
        let embed = DiscordNotifier::listing_to_embed(&listing, 0x002e_cc71, Some(info.clone()));

        assert!(embed.description.contains(&info));
        assert_eq!(embed.color, 0x002e_cc71);
    }

    #[test]
    fn test_deals_embed_with_avg_price() {
        let listing = make_listing(); // price = 100.0
        let avg_price = 150.0;
        let discount = ((avg_price - 100.0) / avg_price) * 100.0;

        let info = format!("🔥 **{discount:.1}% below average** (avg: {avg_price:.2} €)");
        let embed = DiscordNotifier::listing_to_embed(&listing, 0x00e7_4c3c, Some(info.clone()));

        assert!(embed.description.contains(&info));
        assert_eq!(embed.color, 0x00e7_4c3c);
    }

    #[test]
    fn test_deals_embed_without_avg_price() {
        let listing = make_listing();
        let embed = DiscordNotifier::listing_to_embed(&listing, 0x00e7_4c3c, None);

        assert_eq!(embed.color, 0x00e7_4c3c);
        assert!(!embed.description.contains("below average"));
    }

    #[test]
    fn test_chunking_large_list() {
        let listings: Vec<Listing> = (0..25)
            .map(|i| {
                let mut l = make_listing();
                l.id = i;
                l
            })
            .collect();

        // Test that chunks work correctly (Discord limit is 10 embeds per message)
        let chunks: Vec<_> = listings.chunks(10).collect();
        assert_eq!(chunks.len(), 3); // 10, 10, 5
        assert_eq!(chunks[0].len(), 10);
        assert_eq!(chunks[1].len(), 10);
        assert_eq!(chunks[2].len(), 5);
    }

    #[test]
    fn test_embed_fields() {
        let embed = DiscordEmbed {
            title: "Test".to_string(),
            description: "Desc".to_string(),
            color: 0x0012_3456,
            fields: vec![],
            url: Some("https://example.com".to_string()),
        };

        assert_eq!(embed.title, "Test");
        assert_eq!(embed.fields.len(), 0);
    }

    #[test]
    fn test_discord_webhook_payload_structure() {
        let embed = DiscordNotifier::listing_to_embed(&make_listing(), 0x0034_98db, None);
        let payload =
            DiscordWebhook { content: Some("Test content".to_string()), embeds: vec![embed] };

        assert_eq!(payload.content, Some("Test content".to_string()));
        assert_eq!(payload.embeds.len(), 1);
    }

    #[tokio::test]
    async fn test_notify_new_listings_sends_webhook() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let notifier = DiscordNotifier::new(mock_server.uri());
        let listing = make_listing();

        let result = notifier.notify_new_listings(&[listing]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_new_listings_empty() {
        let notifier = DiscordNotifier::new("https://discord.com/webhook".to_string());
        let result = notifier.notify_new_listings(&[]).await;
        assert!(result.is_ok()); // Should return Ok without sending
    }

    #[tokio::test]
    async fn test_notify_price_drops_sends_webhook() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let notifier = DiscordNotifier::new(mock_server.uri());
        let listing = make_listing();
        let drops = [(listing, 120.0, 100.0)];

        let result = notifier.notify_price_drops(&drops).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_price_drops_empty() {
        let notifier = DiscordNotifier::new("https://discord.com/webhook".to_string());
        let result = notifier.notify_price_drops(&[]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_deals_sends_webhook() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let notifier = DiscordNotifier::new(mock_server.uri());
        let listing = make_listing();

        let result = notifier.notify_deals(&[listing], Some(150.0)).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_deals_empty() {
        let notifier = DiscordNotifier::new("https://discord.com/webhook".to_string());
        let result = notifier.notify_deals(&[], None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_deals_no_avg_price() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let notifier = DiscordNotifier::new(mock_server.uri());
        let listing = make_listing();

        let result = notifier.notify_deals(&[listing], None).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_webhook_non_success_status() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limited"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let notifier = DiscordNotifier::new(mock_server.uri());
        let listing = make_listing();

        // Should not error, but will log a warning
        let result = notifier.notify_new_listings(&[listing]).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_send_webhook_connection_error() {
        let notifier = DiscordNotifier::new("http://127.0.0.1:1".to_string()); // Invalid port
        let listing = make_listing();

        let result = notifier.notify_new_listings(&[listing]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_notify_new_listings_multiple_chunks() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Expect 2 requests (12 listings = 10 + 2)
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(2)
            .mount(&mock_server)
            .await;

        let notifier = DiscordNotifier::new(mock_server.uri());
        let listings: Vec<Listing> = (0..12)
            .map(|i| {
                let mut l = make_listing();
                l.id = i;
                l
            })
            .collect();

        let result = notifier.notify_new_listings(&listings).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_price_drops_multiple_chunks() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(2)
            .mount(&mock_server)
            .await;

        let notifier = DiscordNotifier::new(mock_server.uri());
        let drops: Vec<(Listing, f64, f64)> = (0..15)
            .map(|i| {
                let mut l = make_listing();
                l.id = i;
                (l, 100.0, 80.0)
            })
            .collect();

        let result = notifier.notify_price_drops(&drops).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notify_deals_multiple_chunks() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(2)
            .mount(&mock_server)
            .await;

        let notifier = DiscordNotifier::new(mock_server.uri());
        let deals: Vec<Listing> = (0..11)
            .map(|i| {
                let mut l = make_listing();
                l.id = i;
                l
            })
            .collect();

        let result = notifier.notify_deals(&deals, Some(150.0)).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_discord_field_struct() {
        let field =
            DiscordField { name: "Price".to_string(), value: "100 EUR".to_string(), inline: true };

        assert_eq!(field.name, "Price");
        assert_eq!(field.value, "100 EUR");
        assert!(field.inline);
    }

    #[test]
    fn test_listing_to_embed_no_price_no_location() {
        let mut listing = make_listing();
        listing.price = None;
        listing.city = None;
        listing.region = None;
        listing.seller_name = None;

        let embed = DiscordNotifier::listing_to_embed(&listing, 0x0034_98db, None);

        assert!(embed.description.contains("N/A"));
        assert!(embed.description.contains("Unknown"));
        assert!(!embed.description.contains("Seller:"));
    }
}
