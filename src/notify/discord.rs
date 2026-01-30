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
}
