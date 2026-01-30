use anyhow::Result;
use tracing::{debug, info, warn};

use crate::api::{OlxClient, SearchParams};
use crate::config::DealConfig;
use crate::db::{Database, Listing, Search};
use crate::filters::FilterChain;

use super::price::PriceAnalyzer;

pub struct SearchTracker<'a> {
    db: &'a Database,
    client: &'a OlxClient,
    filters: FilterChain,
    deal_config: DealConfig,
}

#[derive(Debug, Clone)]
pub struct TrackResult {
    pub search_id: i64,
    pub new_listings: Vec<Listing>,
    pub updated_listings: Vec<Listing>,
    pub deals: Vec<Listing>,
    pub price_drops: Vec<(Listing, f64, f64)>, // (listing, old_price, new_price)
}

impl<'a> SearchTracker<'a> {
    pub fn new(db: &'a Database, client: &'a OlxClient, deal_config: DealConfig) -> Self {
        Self { db, client, filters: FilterChain::default(), deal_config }
    }

    pub fn with_filters(mut self, filters: FilterChain) -> Self {
        self.filters = filters;
        self
    }

    pub async fn run_search(&self, search: &Search, max_results: i32) -> Result<TrackResult> {
        info!("Running search '{}' (id={}, keyword='{}')", search.name, search.id, search.keyword);

        let params = SearchParams {
            query: search.keyword.clone(),
            city: search.city.clone(),
            radius_km: search.radius_km,
            category_id: search.category_id,
            offset: 0,
            limit: 50,
        };

        let offers = self.client.search_all(&params, max_results).await?;
        info!("Found {} listings from API", offers.len());

        // Apply filters
        let filtered_offers: Vec<_> =
            offers.into_iter().filter(|offer| self.filters.apply(offer, search)).collect();

        debug!("{} listings after filtering", filtered_offers.len());

        let mut result = TrackResult {
            search_id: search.id,
            new_listings: Vec::new(),
            updated_listings: Vec::new(),
            deals: Vec::new(),
            price_drops: Vec::new(),
        };

        // Get existing listings to check for price drops
        let existing_listings: std::collections::HashMap<i64, Listing> =
            self.db.get_listings_for_search(search.id)?.into_iter().map(|l| (l.id, l)).collect();

        for offer in filtered_offers {
            let price = offer.get_price();
            let city = offer.get_city();
            let region = offer.get_region();
            let seller = offer.get_seller_name();

            // Check for price drop before upsert
            if let Some(existing) = existing_listings.get(&offer.id) {
                if let (Some(old_price), Some(new_price)) = (existing.price, price) {
                    if new_price < old_price {
                        debug!(
                            "Price drop detected for '{}': {} -> {}",
                            offer.title, old_price, new_price
                        );
                    }
                }
            }

            let is_new = self.db.upsert_listing(
                offer.id,
                search.id,
                &offer.title,
                price,
                "EUR",
                &offer.url,
                city.as_deref(),
                region.as_deref(),
                seller.as_deref(),
            )?;

            if let Some(listing) = self.db.get_listing(offer.id)? {
                if is_new {
                    result.new_listings.push(listing);
                } else {
                    // Check for price drop
                    if let Some(existing) = existing_listings.get(&offer.id) {
                        if let (Some(old_price), Some(new_price)) = (existing.price, price) {
                            if new_price < old_price {
                                result.price_drops.push((listing.clone(), old_price, new_price));
                            }
                        }
                    }
                    result.updated_listings.push(listing);
                }
            }
        }

        // Update stats and detect deals
        let stats = self.db.update_search_stats(search.id)?;
        let analyzer = PriceAnalyzer::new(&stats, search.max_price, &self.deal_config);

        // Mark deals
        for listing in result.new_listings.iter().chain(result.updated_listings.iter()) {
            if analyzer.is_deal(listing.price) {
                self.db.mark_as_deal(listing.id, true)?;
                if let Some(updated) = self.db.get_listing(listing.id)? {
                    result.deals.push(updated);
                }
            }
        }

        info!(
            "Search '{}' complete: {} new, {} updated, {} deals",
            search.name,
            result.new_listings.len(),
            result.updated_listings.len(),
            result.deals.len()
        );

        Ok(result)
    }

    pub async fn run_all_searches(&self, max_results_per_search: i32) -> Result<Vec<TrackResult>> {
        let searches = self.db.list_searches(true)?;
        let mut results = Vec::new();

        for search in searches {
            match self.run_search(&search, max_results_per_search).await {
                Ok(result) => results.push(result),
                Err(e) => {
                    warn!("Failed to run search '{}' (id={}): {}", search.name, search.id, e);
                }
            }

            // Rate limiting between searches
            tokio::time::sleep(self.client.request_delay()).await;
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require mocking the API client
}
