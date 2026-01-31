use anyhow::Result;
use tracing::{debug, info, warn};

use crate::api::{OlxClient, SearchClient, SearchParams};
use crate::config::DealConfig;
use crate::db::{Database, Listing, Search};
use crate::filters::FilterChain;

use super::price::PriceAnalyzer;

#[derive(Debug)]
pub struct SearchTracker<'a, C: SearchClient + ?Sized = OlxClient> {
    db: &'a Database,
    client: &'a C,
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

impl<'a, C: SearchClient + ?Sized> SearchTracker<'a, C> {
    pub fn new(db: &'a Database, client: &'a C, deal_config: DealConfig) -> Self {
        Self { db, client, filters: FilterChain::default(), deal_config }
    }

    pub fn with_filters(mut self, filters: FilterChain) -> Self {
        self.filters = filters;
        self
    }

    pub async fn run_search(&self, search: &Search, max_results: i32) -> Result<TrackResult> {
        info!(
            "Running search '{}' (id={}, keyword='{}', sort='{}')",
            search.name, search.id, search.keyword, search.sort_order
        );

        let sort = search.sort_order.parse().unwrap_or_default();

        // Lookup city ID if city name is set
        let city_id = if let Some(ref city_name) = search.city {
            match self.client.lookup_city(city_name).await? {
                Some(loc) => loc.city.id,
                None => None,
            }
        } else {
            None
        };

        let params = SearchParams {
            query: search.keyword.clone(),
            city_id,
            radius_km: search.radius_km,
            category_id: search.category_id,
            sort,
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
#[allow(clippy::float_cmp, clippy::option_if_let_else)]
mod tests {
    use super::*;
    use crate::api::models::{OfferData, OfferParam, ParamValue};
    use std::time::Duration;

    // Mock client for testing
    struct MockClient {
        offers: Vec<OfferData>,
        delay: Duration,
    }

    impl MockClient {
        fn new(offers: Vec<OfferData>) -> Self {
            Self { offers, delay: Duration::from_millis(0) }
        }
    }

    #[async_trait::async_trait]
    impl SearchClient for MockClient {
        async fn lookup_city(
            &self,
            _city_name: &str,
        ) -> Result<Option<crate::api::models::LocationResult>> {
            Ok(None) // Simple mock - no city lookup
        }

        async fn search_all(
            &self,
            _params: &SearchParams,
            _max_results: i32,
        ) -> Result<Vec<OfferData>> {
            Ok(self.offers.clone())
        }

        fn request_delay(&self) -> Duration {
            self.delay
        }
    }

    fn create_test_offer(id: i64, title: &str, price: Option<f64>) -> OfferData {
        OfferData {
            id,
            title: title.to_string(),
            url: format!("https://example.com/{id}"),
            created_time: Some("2024-01-01T00:00:00Z".to_string()),
            last_refresh_time: None,
            params: if let Some(p) = price {
                vec![OfferParam {
                    key: "price".to_string(),
                    name: "Price".to_string(),
                    value: Some(ParamValue::Numeric { value: p, label: None }),
                }]
            } else {
                vec![]
            },
            photos: vec![],
            location: None,
            user: None,
        }
    }

    #[tokio::test]
    async fn test_run_search_new_listings() {
        let db = Database::open_in_memory().unwrap();
        let search_id =
            db.create_search("Test", "iphone", None, None, None, None, None, None, None).unwrap();

        let offers = vec![
            create_test_offer(1, "iPhone 12", Some(400.0)),
            create_test_offer(2, "iPhone 13", Some(500.0)),
        ];

        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.new_listings.len(), 2);
        assert_eq!(result.updated_listings.len(), 0);
        assert_eq!(result.search_id, search_id);
    }

    #[tokio::test]
    async fn test_run_search_updated_listings() {
        let db = Database::open_in_memory().unwrap();
        let search_id =
            db.create_search("Test", "iphone", None, None, None, None, None, None, None).unwrap();

        // Insert initial listing
        db.upsert_listing(1, search_id, "iPhone 12", Some(400.0), "EUR", "url", None, None, None)
            .unwrap();

        let offers = vec![create_test_offer(1, "iPhone 12 Pro", Some(400.0))];
        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.new_listings.len(), 0);
        assert_eq!(result.updated_listings.len(), 1);
    }

    #[tokio::test]
    async fn test_run_search_price_drop() {
        let db = Database::open_in_memory().unwrap();
        let search_id =
            db.create_search("Test", "iphone", None, None, None, None, None, None, None).unwrap();

        // Insert initial listing with high price
        db.upsert_listing(1, search_id, "iPhone 12", Some(500.0), "EUR", "url", None, None, None)
            .unwrap();

        // Return same listing with lower price
        let offers = vec![create_test_offer(1, "iPhone 12", Some(400.0))];
        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.price_drops.len(), 1);
        assert_eq!(result.price_drops[0].1, 500.0); // old price
        assert_eq!(result.price_drops[0].2, 400.0); // new price
    }

    #[tokio::test]
    async fn test_run_search_deal_detection() {
        let db = Database::open_in_memory().unwrap();
        let search_id = db
            .create_search("Test", "iphone", None, Some(300.0), None, None, None, None, None)
            .unwrap();

        // Insert some listings to establish price stats
        db.upsert_listing(10, search_id, "iPhone X", Some(500.0), "EUR", "url1", None, None, None)
            .unwrap();
        db.upsert_listing(11, search_id, "iPhone 11", Some(600.0), "EUR", "url2", None, None, None)
            .unwrap();
        db.update_search_stats(search_id).unwrap();

        // Now add a deal (below max_price)
        let offers = vec![create_test_offer(1, "iPhone 12", Some(250.0))];
        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.deals.len(), 1);
        assert_eq!(result.deals[0].price, Some(250.0));
        assert!(result.deals[0].is_deal);
    }

    #[tokio::test]
    async fn test_run_search_empty_results() {
        let db = Database::open_in_memory().unwrap();
        let search_id = db
            .create_search("Test", "nonexistent", None, None, None, None, None, None, None)
            .unwrap();

        let client = MockClient::new(vec![]); // No offers
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.new_listings.len(), 0);
        assert_eq!(result.updated_listings.len(), 0);
        assert_eq!(result.deals.len(), 0);
    }

    #[tokio::test]
    async fn test_run_all_searches() {
        let db = Database::open_in_memory().unwrap();

        // Create multiple searches
        let _id1 = db
            .create_search("Search 1", "laptop", None, None, None, None, None, None, None)
            .unwrap();
        let _id2 = db
            .create_search("Search 2", "phone", None, None, None, None, None, None, None)
            .unwrap();

        let offers = vec![create_test_offer(1, "Laptop", Some(800.0))];
        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let results = tracker.run_all_searches(10).await.unwrap();

        assert_eq!(results.len(), 2); // Both searches ran
    }

    #[tokio::test]
    async fn test_run_search_with_price_filters() {
        let db = Database::open_in_memory().unwrap();
        let search_id = db
            .create_search("Test", "phone", Some(100.0), Some(200.0), None, None, None, None, None)
            .unwrap();

        let offers = vec![
            create_test_offer(1, "Cheap phone", Some(50.0)), // Below min
            create_test_offer(2, "Good phone", Some(150.0)), // In range
            create_test_offer(3, "Expensive phone", Some(500.0)), // Above max
        ];

        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default())
            .with_filters(FilterChain::with_defaults());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        // Only the phone in price range should be added
        assert_eq!(result.new_listings.len(), 1);
        assert_eq!(result.new_listings[0].price, Some(150.0));
    }

    // Mock client with city lookup
    struct MockClientWithCity {
        offers: Vec<OfferData>,
        city_id: Option<i64>,
    }

    impl MockClientWithCity {
        fn new(offers: Vec<OfferData>, city_id: Option<i64>) -> Self {
            Self { offers, city_id }
        }
    }

    #[async_trait::async_trait]
    impl SearchClient for MockClientWithCity {
        async fn lookup_city(
            &self,
            _city_name: &str,
        ) -> Result<Option<crate::api::models::LocationResult>> {
            use crate::api::models::{LocationCity, LocationRegion, LocationResult};
            if let Some(id) = self.city_id {
                Ok(Some(LocationResult {
                    city: LocationCity {
                        id: Some(id),
                        name: "Test City".to_string(),
                        normalized_name: Some("test-city".to_string()),
                    },
                    municipality: None,
                    region: Some(LocationRegion { id: Some(1), name: "Test Region".to_string() }),
                }))
            } else {
                Ok(None)
            }
        }

        async fn search_all(
            &self,
            _params: &SearchParams,
            _max_results: i32,
        ) -> Result<Vec<OfferData>> {
            Ok(self.offers.clone())
        }

        fn request_delay(&self) -> Duration {
            Duration::from_millis(0)
        }
    }

    #[tokio::test]
    async fn test_run_search_with_city_lookup_found() {
        let db = Database::open_in_memory().unwrap();
        let search_id = db
            .create_search("Test", "iphone", None, None, Some("Porto"), Some(10), None, None, None)
            .unwrap();

        let offers = vec![create_test_offer(1, "iPhone 12", Some(400.0))];
        let client = MockClientWithCity::new(offers, Some(12345));
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.new_listings.len(), 1);
    }

    #[tokio::test]
    async fn test_run_search_with_city_lookup_not_found() {
        let db = Database::open_in_memory().unwrap();
        let search_id = db
            .create_search(
                "Test",
                "iphone",
                None,
                None,
                Some("NonexistentCity"),
                None,
                None,
                None,
                None,
            )
            .unwrap();

        let offers = vec![create_test_offer(1, "iPhone 12", Some(400.0))];
        let client = MockClientWithCity::new(offers, None); // City not found
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        // Should still work, just without city filter
        assert_eq!(result.new_listings.len(), 1);
    }

    #[tokio::test]
    async fn test_run_search_price_increase() {
        let db = Database::open_in_memory().unwrap();
        let search_id =
            db.create_search("Test", "iphone", None, None, None, None, None, None, None).unwrap();

        // Insert initial listing with lower price
        db.upsert_listing(1, search_id, "iPhone 12", Some(400.0), "EUR", "url", None, None, None)
            .unwrap();

        // Return same listing with higher price (price increase, not drop)
        let offers = vec![create_test_offer(1, "iPhone 12", Some(500.0))];
        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        // No price drops (price went up, not down)
        assert_eq!(result.price_drops.len(), 0);
        assert_eq!(result.updated_listings.len(), 1);
    }

    #[tokio::test]
    async fn test_run_search_same_price() {
        let db = Database::open_in_memory().unwrap();
        let search_id =
            db.create_search("Test", "iphone", None, None, None, None, None, None, None).unwrap();

        // Insert initial listing
        db.upsert_listing(1, search_id, "iPhone 12", Some(400.0), "EUR", "url", None, None, None)
            .unwrap();

        // Return same listing with same price
        let offers = vec![create_test_offer(1, "iPhone 12", Some(400.0))];
        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        // No price drops
        assert_eq!(result.price_drops.len(), 0);
        assert_eq!(result.updated_listings.len(), 1);
    }

    // Mock client that returns error
    struct ErrorClient;

    #[async_trait::async_trait]
    impl SearchClient for ErrorClient {
        async fn lookup_city(
            &self,
            _city_name: &str,
        ) -> Result<Option<crate::api::models::LocationResult>> {
            Ok(None)
        }

        async fn search_all(
            &self,
            _params: &SearchParams,
            _max_results: i32,
        ) -> Result<Vec<OfferData>> {
            Err(anyhow::anyhow!("API error"))
        }

        fn request_delay(&self) -> Duration {
            Duration::from_millis(0)
        }
    }

    #[tokio::test]
    async fn test_run_all_searches_handles_errors() {
        let db = Database::open_in_memory().unwrap();
        db.create_search("Search1", "iphone", None, None, None, None, None, None, None).unwrap();
        db.create_search("Search2", "laptop", None, None, None, None, None, None, None).unwrap();

        let client = ErrorClient; // Returns error
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        // Should not panic, just return empty results
        let results = tracker.run_all_searches(10).await.unwrap();
        assert_eq!(results.len(), 0); // Both searches failed
    }

    #[tokio::test]
    async fn test_run_search_with_location_info() {
        use crate::api::models::{LocationCity, LocationRegion, OfferLocation};

        let db = Database::open_in_memory().unwrap();
        let search_id =
            db.create_search("Test", "iphone", None, None, None, None, None, None, None).unwrap();

        let mut offer = create_test_offer(1, "iPhone 12", Some(400.0));
        offer.location = Some(OfferLocation {
            city: Some(LocationCity {
                id: Some(1),
                name: "Porto".to_string(),
                normalized_name: Some("porto".to_string()),
            }),
            region: Some(LocationRegion { id: Some(1), name: "Norte".to_string() }),
        });

        let client = MockClient::new(vec![offer]);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.new_listings.len(), 1);
        assert_eq!(result.new_listings[0].city, Some("Porto".to_string()));
        assert_eq!(result.new_listings[0].region, Some("Norte".to_string()));
    }

    #[tokio::test]
    async fn test_run_search_with_seller_info() {
        use crate::api::models::OfferUser;

        let db = Database::open_in_memory().unwrap();
        let search_id =
            db.create_search("Test", "iphone", None, None, None, None, None, None, None).unwrap();

        let mut offer = create_test_offer(1, "iPhone 12", Some(400.0));
        offer.user = Some(OfferUser { id: Some(1), name: Some("John Seller".to_string()) });

        let client = MockClient::new(vec![offer]);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.new_listings.len(), 1);
        assert_eq!(result.new_listings[0].seller_name, Some("John Seller".to_string()));
    }

    #[tokio::test]
    async fn test_run_search_with_category() {
        let db = Database::open_in_memory().unwrap();
        let search_id = db
            .create_search("Test", "iphone", None, None, None, None, Some(123), None, None)
            .unwrap();

        let offers = vec![create_test_offer(1, "iPhone 12", Some(400.0))];
        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        assert_eq!(search.category_id, Some(123));

        let result = tracker.run_search(&search, 10).await.unwrap();
        assert_eq!(result.new_listings.len(), 1);
    }

    #[tokio::test]
    async fn test_run_search_no_price_listing() {
        let db = Database::open_in_memory().unwrap();
        let search_id =
            db.create_search("Test", "iphone", None, None, None, None, None, None, None).unwrap();

        // Offer without price
        let offers = vec![create_test_offer(1, "iPhone 12", None)];
        let client = MockClient::new(offers);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        let search = db.get_search(search_id).unwrap().unwrap();
        let result = tracker.run_search(&search, 10).await.unwrap();

        assert_eq!(result.new_listings.len(), 1);
        assert!(result.new_listings[0].price.is_none());
    }

    #[tokio::test]
    async fn test_track_result_clone() {
        let result = TrackResult {
            search_id: 1,
            new_listings: vec![],
            updated_listings: vec![],
            deals: vec![],
            price_drops: vec![],
        };

        let cloned = result.clone();
        assert_eq!(cloned.search_id, result.search_id);
    }

    #[tokio::test]
    async fn test_search_tracker_default_filters() {
        let db = Database::open_in_memory().unwrap();
        let client = MockClient::new(vec![]);
        let tracker = SearchTracker::new(&db, &client, DealConfig::default());

        // Default filter chain should be empty
        assert!(tracker.filters.is_empty());
    }
}
