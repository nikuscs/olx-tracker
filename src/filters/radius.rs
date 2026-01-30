use crate::api::OfferData;
use crate::db::Search;

use super::Filter;

/// Filter based on city location
/// Note: OLX API handles radius filtering server-side via the distance param,
/// so this filter primarily checks if city matches when specified
pub struct RadiusFilter;

impl Filter for RadiusFilter {
    fn apply(&self, offer: &OfferData, search: &Search) -> bool {
        // If no city filter is set, allow all
        let Some(search_city) = &search.city else {
            return true;
        };

        // If search has a radius, the API already filters by distance
        // We trust the API's filtering in that case
        if search.radius_km.is_some() {
            return true;
        }

        // If no radius but city is specified, do exact city match
        if let Some(offer_city) = offer.get_city() {
            return offer_city.eq_ignore_ascii_case(search_city);
        }

        // No location info on offer, include it anyway
        true
    }

    fn name(&self) -> &'static str {
        "RadiusFilter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::models::{LocationCity, OfferLocation};

    fn make_offer(city: Option<&str>) -> OfferData {
        OfferData {
            id: 1,
            title: "Test".to_string(),
            url: "https://test.com".to_string(),
            params: vec![],
            location: city.map(|c| OfferLocation {
                city: Some(LocationCity {
                    id: Some(1),
                    name: c.to_string(),
                    normalized_name: None,
                }),
                region: None,
            }),
            user: None,
            photos: vec![],
            created_time: None,
            last_refresh_time: None,
        }
    }

    fn make_search(city: Option<&str>, radius: Option<i32>) -> Search {
        Search {
            id: 1,
            name: "Test".to_string(),
            keyword: "test".to_string(),
            min_price: None,
            max_price: None,
            city: city.map(std::string::ToString::to_string),
            radius_km: radius,
            category_id: None,
            sort_order: "newest".to_string(),
            active: true,
            created_at: "2024-01-01".to_string(),
            expires_at: None,
        }
    }

    #[test]
    fn test_no_city_filter() {
        let filter = RadiusFilter;
        let offer = make_offer(Some("Lisbon"));
        let search = make_search(None, None);

        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_exact_city_match() {
        let filter = RadiusFilter;
        let offer = make_offer(Some("Porto"));
        let search = make_search(Some("Porto"), None);

        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_city_mismatch_no_radius() {
        let filter = RadiusFilter;
        let offer = make_offer(Some("Lisbon"));
        let search = make_search(Some("Porto"), None);

        assert!(!filter.apply(&offer, &search));
    }

    #[test]
    fn test_city_with_radius_allows_all() {
        let filter = RadiusFilter;
        let offer = make_offer(Some("Lisbon"));
        let search = make_search(Some("Porto"), Some(100));

        // With radius, we trust the API's filtering
        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_case_insensitive() {
        let filter = RadiusFilter;
        let offer = make_offer(Some("PORTO"));
        let search = make_search(Some("porto"), None);

        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_offer_no_location_with_city_filter() {
        let filter = RadiusFilter;
        let offer = make_offer(None);
        let search = make_search(Some("Porto"), None);

        // Offer without location still passes (we include it)
        assert!(filter.apply(&offer, &search));
    }

    #[test]
    fn test_filter_name() {
        let filter = RadiusFilter;
        assert_eq!(filter.name(), "RadiusFilter");
    }
}
