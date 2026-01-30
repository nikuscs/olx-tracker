use crate::api::OfferData;
use crate::db::Search;

use super::Filter;

/// Filter that ensures listing price is within the search's price range
pub struct PriceFilter;

impl Filter for PriceFilter {
    fn apply(&self, offer: &OfferData, search: &Search) -> bool {
        let price = offer.get_price();

        match (price, search.min_price, search.max_price) {
            // No price on listing - keep it (let user decide)
            (None, _, _) => true,
            // Both min and max set
            (Some(p), Some(min), Some(max)) => p >= min && p <= max,
            // Only min set
            (Some(p), Some(min), None) => p >= min,
            // Only max set
            (Some(p), None, Some(max)) => p <= max,
            // No price filters
            (Some(_), None, None) => true,
        }
    }

    fn name(&self) -> &'static str {
        "PriceFilter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{OfferParam, ParamValue};

    fn make_offer(price: Option<f64>) -> OfferData {
        let params = if let Some(p) = price {
            vec![OfferParam {
                key: "price".to_string(),
                name: "Price".to_string(),
                value: Some(ParamValue::Numeric { value: p, label: None }),
            }]
        } else {
            vec![]
        };

        OfferData {
            id: 1,
            title: "Test".to_string(),
            url: "https://test.com".to_string(),
            params,
            location: None,
            user: None,
            created_time: None,
            last_refresh_time: None,
        }
    }

    fn make_search(min: Option<f64>, max: Option<f64>) -> Search {
        Search {
            id: 1,
            name: "Test".to_string(),
            keyword: "test".to_string(),
            min_price: min,
            max_price: max,
            city: None,
            radius_km: None,
            category_id: None,
            sort_order: "newest".to_string(),
            active: true,
            created_at: "2024-01-01".to_string(),
        }
    }

    #[test]
    fn test_no_filters() {
        let filter = PriceFilter;
        let offer = make_offer(Some(100.0));
        let search = make_search(None, None);
        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_min_price_pass() {
        let filter = PriceFilter;
        let offer = make_offer(Some(200.0));
        let search = make_search(Some(100.0), None);
        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_min_price_fail() {
        let filter = PriceFilter;
        let offer = make_offer(Some(50.0));
        let search = make_search(Some(100.0), None);
        assert!(!filter.apply(&offer, &search));
    }

    #[test]
    fn test_max_price_pass() {
        let filter = PriceFilter;
        let offer = make_offer(Some(80.0));
        let search = make_search(None, Some(100.0));
        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_max_price_fail() {
        let filter = PriceFilter;
        let offer = make_offer(Some(150.0));
        let search = make_search(None, Some(100.0));
        assert!(!filter.apply(&offer, &search));
    }

    #[test]
    fn test_price_range() {
        let filter = PriceFilter;
        let search = make_search(Some(100.0), Some(500.0));

        assert!(filter.apply(&make_offer(Some(200.0)), &search));
        assert!(filter.apply(&make_offer(Some(100.0)), &search)); // boundary
        assert!(filter.apply(&make_offer(Some(500.0)), &search)); // boundary
        assert!(!filter.apply(&make_offer(Some(50.0)), &search));
        assert!(!filter.apply(&make_offer(Some(600.0)), &search));
    }

    #[test]
    fn test_no_price_on_listing() {
        let filter = PriceFilter;
        let offer = make_offer(None);
        let search = make_search(Some(100.0), Some(500.0));
        assert!(filter.apply(&offer, &search)); // Keep listings without price
    }
}
