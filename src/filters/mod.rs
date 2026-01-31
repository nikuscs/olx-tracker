mod keyword;
mod price;
mod radius;

pub use keyword::KeywordFilter;
pub use price::PriceFilter;
pub use radius::RadiusFilter;

use crate::api::OfferData;
use crate::db::Search;

// ============================================================================
// Standalone filter utilities (for quick search without database/Search struct)
// ============================================================================

/// Check if a title contains the keyword (case-insensitive substring match).
///
/// Returns true if keyword is None or if the title contains the keyword.
/// This is for the `--keyword` CLI flag, which is an additional filter.
/// Note: This differs from `KeywordFilter` which splits keywords by whitespace.
#[must_use]
pub fn matches_keyword_filter(title: &str, keyword: Option<&str>) -> bool {
    keyword.is_none_or(|kw| title.to_lowercase().contains(&kw.to_lowercase()))
}

/// Check if a price matches the min/max price range.
/// Returns true if price is within range, or if no filters are set.
/// Items without a price pass through (return true).
#[must_use]
pub fn matches_price_range(
    price: Option<f64>,
    min_price: Option<f64>,
    max_price: Option<f64>,
) -> bool {
    match (price, min_price, max_price) {
        (Some(p), Some(min), Some(max)) => p >= min && p <= max,
        (Some(p), Some(min), None) => p >= min,
        (Some(p), None, Some(max)) => p <= max,
        (None, _, _) | (Some(_), None, None) => true,
    }
}

/// Trait for implementing custom filters
pub trait Filter: Send + Sync {
    /// Return true if the offer should be included, false to exclude
    fn apply(&self, offer: &OfferData, search: &Search) -> bool;

    /// Human-readable name for this filter
    fn name(&self) -> &'static str;
}

/// A chain of filters that all must pass for an offer to be included
#[derive(Default)]
pub struct FilterChain {
    filters: Vec<Box<dyn Filter>>,
}

impl std::fmt::Debug for FilterChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FilterChain").field("filter_count", &self.filters.len()).finish()
    }
}

impl FilterChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a filter chain with default filters (keyword, price, radius)
    pub fn with_defaults() -> Self {
        let mut chain = Self::new();
        chain.add(Box::new(KeywordFilter));
        chain.add(Box::new(PriceFilter));
        chain.add(Box::new(RadiusFilter));
        chain
    }

    pub fn add(&mut self, filter: Box<dyn Filter>) {
        self.filters.push(filter);
    }

    /// Apply all filters to an offer. Returns true if all filters pass.
    pub fn apply(&self, offer: &OfferData, search: &Search) -> bool {
        for filter in &self.filters {
            if !filter.apply(offer, search) {
                tracing::debug!("Offer {} filtered out by {}", offer.id, filter.name());
                return false;
            }
        }
        true
    }

    pub fn len(&self) -> usize {
        self.filters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysPass;
    impl Filter for AlwaysPass {
        fn apply(&self, _: &OfferData, _: &Search) -> bool {
            true
        }
        fn name(&self) -> &'static str {
            "AlwaysPass"
        }
    }

    struct AlwaysFail;
    impl Filter for AlwaysFail {
        fn apply(&self, _: &OfferData, _: &Search) -> bool {
            false
        }
        fn name(&self) -> &'static str {
            "AlwaysFail"
        }
    }

    fn make_test_offer() -> OfferData {
        OfferData {
            id: 1,
            title: "Test".to_string(),
            url: "https://test.com".to_string(),
            params: vec![],
            location: None,
            user: None,
            photos: vec![],
            created_time: None,
            last_refresh_time: None,
        }
    }

    fn make_test_search() -> Search {
        Search {
            id: 1,
            name: "Test".to_string(),
            keyword: "test".to_string(),
            min_price: None,
            max_price: None,
            city: None,
            radius_km: None,
            category_id: None,
            sort_order: "newest".to_string(),
            active: true,
            created_at: "2024-01-01".to_string(),
            expires_at: None,
        }
    }

    #[test]
    fn test_filter_chain_empty() {
        let chain = FilterChain::new();
        let offer = make_test_offer();
        let search = make_test_search();

        assert!(chain.apply(&offer, &search));
    }

    #[test]
    fn test_filter_chain_all_pass() {
        let mut chain = FilterChain::new();
        chain.add(Box::new(AlwaysPass));
        chain.add(Box::new(AlwaysPass));

        let offer = make_test_offer();
        let search = make_test_search();

        assert!(chain.apply(&offer, &search));
    }

    #[test]
    fn test_filter_chain_one_fails() {
        let mut chain = FilterChain::new();
        chain.add(Box::new(AlwaysPass));
        chain.add(Box::new(AlwaysFail));
        chain.add(Box::new(AlwaysPass));

        let offer = make_test_offer();
        let search = make_test_search();

        assert!(!chain.apply(&offer, &search));
    }

    #[test]
    fn test_filter_chain_with_defaults() {
        let chain = FilterChain::with_defaults();
        assert_eq!(chain.len(), 3); // keyword, price, radius
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_filter_chain_len_and_empty() {
        let mut chain = FilterChain::new();
        assert_eq!(chain.len(), 0);
        assert!(chain.is_empty());

        chain.add(Box::new(AlwaysPass));
        assert_eq!(chain.len(), 1);
        assert!(!chain.is_empty());

        chain.add(Box::new(AlwaysPass));
        assert_eq!(chain.len(), 2);
    }

    #[test]
    fn test_filter_chain_debug() {
        let mut chain = FilterChain::new();
        chain.add(Box::new(AlwaysPass));
        chain.add(Box::new(AlwaysFail));

        let debug_str = format!("{chain:?}");
        assert!(debug_str.contains("FilterChain"));
        assert!(debug_str.contains("filter_count"));
    }

    #[test]
    fn test_filter_name() {
        let pass_filter = AlwaysPass;
        let fail_filter = AlwaysFail;

        assert_eq!(pass_filter.name(), "AlwaysPass");
        assert_eq!(fail_filter.name(), "AlwaysFail");
    }

    // Tests for standalone filter utilities

    #[test]
    fn test_matches_keyword_filter_none() {
        assert!(matches_keyword_filter("iPhone 14 Pro", None));
        assert!(matches_keyword_filter("", None));
    }

    #[test]
    fn test_matches_keyword_filter_match() {
        assert!(matches_keyword_filter("iPhone 14 Pro", Some("iPhone")));
        assert!(matches_keyword_filter("iPhone 14 Pro", Some("14")));
        assert!(matches_keyword_filter("iPhone 14 Pro", Some("Pro")));
    }

    #[test]
    fn test_matches_keyword_filter_case_insensitive() {
        assert!(matches_keyword_filter("iPhone 14 Pro", Some("iphone")));
        assert!(matches_keyword_filter("iPhone 14 Pro", Some("IPHONE")));
        assert!(matches_keyword_filter("AYN Odin 2 Portal", Some("ayn")));
        assert!(matches_keyword_filter("ayn odin 2 portal", Some("AYN")));
    }

    #[test]
    fn test_matches_keyword_filter_no_match() {
        assert!(!matches_keyword_filter("iPhone 14 Pro", Some("Samsung")));
        assert!(!matches_keyword_filter("PlayStation 5", Some("Xbox")));
    }

    #[test]
    fn test_matches_keyword_filter_empty_title() {
        assert!(!matches_keyword_filter("", Some("iPhone")));
    }

    #[test]
    fn test_matches_keyword_filter_substring() {
        assert!(matches_keyword_filter("iPhone 14 Pro Max", Some("14 Pro")));
        assert!(matches_keyword_filter("PlayStation 5", Some("Play")));
    }

    #[test]
    fn test_matches_price_range_no_filters() {
        assert!(matches_price_range(Some(500.0), None, None));
        assert!(matches_price_range(None, None, None));
    }

    #[test]
    fn test_matches_price_range_min_only() {
        assert!(matches_price_range(Some(500.0), Some(200.0), None));
        assert!(matches_price_range(Some(200.0), Some(200.0), None)); // equal
        assert!(!matches_price_range(Some(100.0), Some(200.0), None));
    }

    #[test]
    fn test_matches_price_range_max_only() {
        assert!(matches_price_range(Some(500.0), None, Some(800.0)));
        assert!(matches_price_range(Some(800.0), None, Some(800.0))); // equal
        assert!(!matches_price_range(Some(900.0), None, Some(800.0)));
    }

    #[test]
    fn test_matches_price_range_both() {
        assert!(matches_price_range(Some(500.0), Some(200.0), Some(800.0)));
        assert!(matches_price_range(Some(200.0), Some(200.0), Some(800.0))); // at min
        assert!(matches_price_range(Some(800.0), Some(200.0), Some(800.0))); // at max
        assert!(!matches_price_range(Some(100.0), Some(200.0), Some(800.0)));
        assert!(!matches_price_range(Some(900.0), Some(200.0), Some(800.0)));
    }

    #[test]
    fn test_matches_price_range_no_price() {
        // Items without price should pass through
        assert!(matches_price_range(None, Some(200.0), Some(800.0)));
        assert!(matches_price_range(None, Some(200.0), None));
        assert!(matches_price_range(None, None, Some(800.0)));
    }
}
