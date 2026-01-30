use crate::config::DealConfig;
use crate::db::SearchStats;

pub struct PriceAnalyzer {
    avg_price: Option<f64>,
    /// Per-search max price (from search settings)
    max_price_threshold: Option<f64>,
    /// Global target price (from deal config)
    target_price: Option<f64>,
    /// Percentage threshold for "good deal" (e.g., 30 = 30% below avg)
    threshold_pct: f64,
}

impl PriceAnalyzer {
    pub const fn new(
        stats: &SearchStats,
        max_price_threshold: Option<f64>,
        deal_config: &DealConfig,
    ) -> Self {
        Self {
            avg_price: stats.avg_price,
            max_price_threshold,
            target_price: deal_config.target_price,
            threshold_pct: deal_config.threshold_pct,
        }
    }

    /// Check if a price qualifies as a "deal"
    /// A deal is when:
    /// - Price is at or below the target price, OR
    /// - Price is at or below the per-search `max_price` threshold, OR
    /// - Price is X% below average (based on `threshold_pct`)
    pub fn is_deal(&self, price: Option<f64>) -> bool {
        let Some(price) = price else {
            return false;
        };

        // Check against global target price
        if let Some(target) = self.target_price {
            if price <= target {
                return true;
            }
        }

        // Check against per-search max price threshold
        if let Some(max) = self.max_price_threshold {
            if price <= max {
                return true;
            }
        }

        // Check against average with threshold percentage
        if let Some(avg) = self.avg_price {
            let threshold = avg * (1.0 - self.threshold_pct / 100.0);
            if price <= threshold {
                return true;
            }
        }

        false
    }

    /// Check if price is significantly below average (good deal)
    /// Uses custom threshold percentage
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

    pub const fn threshold_pct(&self) -> f64 {
        self.threshold_pct
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

    fn make_deal_config(threshold_pct: f64, target_price: Option<f64>) -> DealConfig {
        DealConfig { threshold_pct, target_price }
    }

    #[test]
    fn test_is_deal_below_max_price() {
        let stats = make_stats(Some(500.0));
        let config = make_deal_config(20.0, None);
        let analyzer = PriceAnalyzer::new(&stats, Some(300.0), &config);

        assert!(analyzer.is_deal(Some(250.0))); // Below max
        assert!(analyzer.is_deal(Some(300.0))); // Equal to max
        assert!(analyzer.is_deal(Some(350.0))); // Below 20% threshold (400)
        assert!(!analyzer.is_deal(Some(450.0))); // Above threshold
    }

    #[test]
    fn test_is_deal_target_price() {
        let stats = make_stats(Some(500.0));
        let config = make_deal_config(20.0, Some(299.0));
        let analyzer = PriceAnalyzer::new(&stats, None, &config);

        assert!(analyzer.is_deal(Some(250.0))); // Below target
        assert!(analyzer.is_deal(Some(299.0))); // Equal to target
        assert!(!analyzer.is_deal(Some(450.0))); // Above both target and threshold
    }

    #[test]
    fn test_is_deal_threshold_pct() {
        let stats = make_stats(Some(100.0));
        let config = make_deal_config(30.0, None); // 30% below avg
        let analyzer = PriceAnalyzer::new(&stats, None, &config);

        assert!(analyzer.is_deal(Some(70.0))); // Exactly 30% below
        assert!(analyzer.is_deal(Some(60.0))); // 40% below
        assert!(!analyzer.is_deal(Some(80.0))); // Only 20% below
    }

    #[test]
    fn test_is_deal_no_price() {
        let stats = make_stats(Some(500.0));
        let config = make_deal_config(20.0, Some(300.0));
        let analyzer = PriceAnalyzer::new(&stats, Some(300.0), &config);

        assert!(!analyzer.is_deal(None));
    }

    #[test]
    fn test_good_deal_custom_threshold() {
        let stats = make_stats(Some(100.0));
        let config = make_deal_config(20.0, None);
        let analyzer = PriceAnalyzer::new(&stats, None, &config);

        // Using custom 40% threshold (not the config's 20%)
        assert!(analyzer.is_good_deal(Some(50.0), 40.0)); // 50% below avg
        assert!(!analyzer.is_good_deal(Some(70.0), 40.0)); // Only 30% below
    }

    #[test]
    fn test_discount_percentage() {
        let stats = make_stats(Some(100.0));
        let config = make_deal_config(20.0, None);
        let analyzer = PriceAnalyzer::new(&stats, None, &config);

        assert_eq!(analyzer.discount_percentage(Some(80.0)), Some(20.0));
        assert_eq!(analyzer.discount_percentage(Some(50.0)), Some(50.0));
        assert_eq!(analyzer.discount_percentage(Some(100.0)), Some(0.0));
        assert!(analyzer.discount_percentage(Some(120.0)).unwrap() < 0.0);
    }

    #[test]
    fn test_is_good_deal_no_price() {
        let stats = make_stats(Some(100.0));
        let config = make_deal_config(20.0, None);
        let analyzer = PriceAnalyzer::new(&stats, None, &config);

        assert!(!analyzer.is_good_deal(None, 40.0));
    }

    #[test]
    fn test_is_good_deal_no_avg_price() {
        let stats = make_stats(None); // No average price
        let config = make_deal_config(20.0, None);
        let analyzer = PriceAnalyzer::new(&stats, None, &config);

        assert!(!analyzer.is_good_deal(Some(50.0), 40.0));
    }

    #[test]
    fn test_getters() {
        let stats = make_stats(Some(123.45));
        let config = make_deal_config(35.5, None);
        let analyzer = PriceAnalyzer::new(&stats, None, &config);

        assert_eq!(analyzer.avg_price(), Some(123.45));
        assert_eq!(analyzer.threshold_pct(), 35.5);
    }
}
