use crate::api::OfferData;
use crate::db::Search;

use super::Filter;

/// Filter that ensures the listing title contains the search keyword
/// (case-insensitive)
pub struct KeywordFilter;

impl Filter for KeywordFilter {
    fn apply(&self, offer: &OfferData, search: &Search) -> bool {
        let title_lower = offer.title.to_lowercase();
        let keywords: Vec<&str> = search.keyword.split_whitespace().collect();

        // All keywords must be present in the title
        keywords.iter().all(|kw: &&str| title_lower.contains(&kw.to_lowercase()))
    }

    fn name(&self) -> &'static str {
        "KeywordFilter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_offer(title: &str) -> OfferData {
        OfferData {
            id: 1,
            title: title.to_string(),
            url: "https://test.com".to_string(),
            params: vec![],
            location: None,
            user: None,
            created_time: None,
            last_refresh_time: None,
        }
    }

    fn make_search(keyword: &str) -> Search {
        Search {
            id: 1,
            name: "Test".to_string(),
            keyword: keyword.to_string(),
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
    fn test_keyword_match() {
        let filter = KeywordFilter;

        let offer = make_offer("PlayStation 2 with controllers");
        let search = make_search("playstation 2");

        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_keyword_case_insensitive() {
        let filter = KeywordFilter;

        let offer = make_offer("PLAYSTATION 2");
        let search = make_search("playstation");

        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_keyword_partial_match_fails() {
        let filter = KeywordFilter;

        let offer = make_offer("PlayStation 3");
        let search = make_search("playstation 2");

        assert!(!filter.apply(&offer, &search)); // Missing "2"
    }

    #[test]
    fn test_keyword_no_match() {
        let filter = KeywordFilter;

        let offer = make_offer("Xbox One");
        let search = make_search("playstation");

        assert!(!filter.apply(&offer, &search));
    }
}
