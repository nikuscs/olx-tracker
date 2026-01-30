mod keyword;
mod radius;

pub use keyword::KeywordFilter;
pub use radius::RadiusFilter;

use crate::api::OfferData;
use crate::db::Search;

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

impl FilterChain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a filter chain with default filters (keyword, radius)
    pub fn with_defaults() -> Self {
        let mut chain = Self::new();
        chain.add(Box::new(KeywordFilter));
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
            created_time: None,
            last_refresh_time: None,
        }
    }

    fn make_test_search() -> Search {
        Search {
            id: 1,
            name: "Test".to_string(),
            keyword: "test".to_string(),
            max_price: None,
            city: None,
            radius_km: None,
            category_id: None,
            sort_order: "newest".to_string(),
            active: true,
            created_at: "2024-01-01".to_string(),
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
}
