use crate::db::SearchStats;

pub struct PriceAnalyzer {
    avg_price: Option<f64>,
    max_price_threshold: Option<f64>,
}

impl PriceAnalyzer {
    pub const fn new(stats: &SearchStats, max_price_threshold: Option<f64>) -> Self {
        Self { avg_price: stats.avg_price, max_price_threshold }
    }

    /// Check if a price qualifies as a "deal"
    /// A deal is when:
    /// - Price is below the user's `max_price` threshold, OR
    /// - Price is below the average price for this search
    pub fn is_deal(&self, price: Option<f64>) -> bool {
        let Some(price) = price else {
            return false;
        };

        // Check against max price threshold
        if let Some(max) = self.max_price_threshold {
            if price <= max {
                return true;
            }
        }

        // Check against average
        if let Some(avg) = self.avg_price {
            if price < avg {
                return true;
            }
        }

        false
    }

    /// Check if price is significantly below average (good deal)
    pub fn is_good_deal(&self, price: Option<f64>, threshold_pct: f64) -> bool {
        let Some(price) = price else {
            return false;
        };

        if let Some(avg) = self.avg_price {
            let threshold = avg * (1.0 - threshold_pct / 100.0);
            return price < threshold;
        }

        false
    }

    /// Calculate how much below average a price is (as percentage)
    pub fn discount_percentage(&self, price: Option<f64>) -> Option<f64> {
        let price = price?;
        let avg = self.avg_price?;

        if avg > 0.0 { Some(((avg - price) / avg) * 100.0) } else { None }
    }

    pub const fn avg_price(&self) -> Option<f64> {
        self.avg_price
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats(avg: Option<f64>) -> SearchStats {
        SearchStats {
            search_id: 1,
            avg_price: avg,
            min_price: None,
            max_price: None,
            total_listings: 10,
            last_updated_at: None,
        }
    }

    #[test]
    fn test_is_deal_below_max_price() {
        let stats = make_stats(Some(500.0));
        let analyzer = PriceAnalyzer::new(&stats, Some(300.0));

        assert!(analyzer.is_deal(Some(250.0))); // Below max
        assert!(analyzer.is_deal(Some(300.0))); // Equal to max
        assert!(analyzer.is_deal(Some(400.0))); // Below avg
        assert!(!analyzer.is_deal(Some(600.0))); // Above both
    }

    #[test]
    fn test_is_deal_below_average() {
        let stats = make_stats(Some(500.0));
        let analyzer = PriceAnalyzer::new(&stats, None);

        assert!(analyzer.is_deal(Some(400.0))); // Below avg
        assert!(!analyzer.is_deal(Some(500.0))); // Equal to avg
        assert!(!analyzer.is_deal(Some(600.0))); // Above avg
    }

    #[test]
    fn test_is_deal_no_price() {
        let stats = make_stats(Some(500.0));
        let analyzer = PriceAnalyzer::new(&stats, Some(300.0));

        assert!(!analyzer.is_deal(None));
    }

    #[test]
    fn test_good_deal_threshold() {
        let stats = make_stats(Some(100.0));
        let analyzer = PriceAnalyzer::new(&stats, None);

        assert!(analyzer.is_good_deal(Some(70.0), 20.0)); // 30% below avg
        assert!(!analyzer.is_good_deal(Some(90.0), 20.0)); // 10% below avg
    }

    #[test]
    fn test_discount_percentage() {
        let stats = make_stats(Some(100.0));
        let analyzer = PriceAnalyzer::new(&stats, None);

        assert_eq!(analyzer.discount_percentage(Some(80.0)), Some(20.0));
        assert_eq!(analyzer.discount_percentage(Some(50.0)), Some(50.0));
        assert_eq!(analyzer.discount_percentage(Some(100.0)), Some(0.0));
        assert!(analyzer.discount_percentage(Some(120.0)).unwrap() < 0.0);
    }
}
