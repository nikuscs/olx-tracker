use serde::Serialize;
use std::str::FromStr;

use crate::api::OfferData;

/// Output format for search results
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// ASCII table format (default)
    #[default]
    Table,
    /// JSON format for APIs/programmers
    Json,
    /// Markdown format for LLMs/documentation
    Markdown,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "table" | "t" => Ok(Self::Table),
            "json" | "j" => Ok(Self::Json),
            "markdown" | "md" | "llm" => Ok(Self::Markdown),
            _ => Err(format!("Unknown format: {s}. Valid: table, json, markdown")),
        }
    }
}

/// Simplified offer for JSON output
#[derive(Debug, Serialize)]
pub struct SearchResultItem {
    pub id: i64,
    pub title: String,
    pub price: Option<f64>,
    pub currency: String,
    pub city: Option<String>,
    pub region: Option<String>,
    pub seller: Option<String>,
    pub url: String,
    pub image: Option<String>,
    pub images: Vec<String>,
    pub created_at: Option<String>,
}

impl From<&OfferData> for SearchResultItem {
    fn from(offer: &OfferData) -> Self {
        Self {
            id: offer.id,
            title: offer.title.clone(),
            price: offer.get_price(),
            currency: "EUR".to_string(),
            city: offer.get_city(),
            region: offer.get_region(),
            seller: offer.get_seller_name(),
            url: offer.url.clone(),
            image: offer.get_thumbnail(),
            images: offer.get_all_thumbnails(),
            created_at: offer.created_time.clone(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub query: String,
    pub sort: String,
    pub total: usize,
    pub filters: SearchFilters,
    pub items: Vec<SearchResultItem>,
}

#[derive(Debug, Serialize)]
pub struct SearchFilters {
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub city: Option<String>,
    pub radius_km: Option<i32>,
}

/// Parameters for formatting search results
#[derive(Debug)]
pub struct FormatParams<'a> {
    pub format: OutputFormat,
    pub query: &'a str,
    pub sort: &'a str,
    pub offers: &'a [OfferData],
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub city: Option<String>,
    pub radius: Option<i32>,
}

/// Format search results based on output format
pub fn format_results(params: FormatParams<'_>) -> String {
    let FormatParams { format, query, sort, offers, min_price, max_price, city, radius } = params;
    match format {
        OutputFormat::Table => format_table(query, sort, offers, min_price, max_price),
        OutputFormat::Json => format_json(query, sort, offers, min_price, max_price, city, radius),
        OutputFormat::Markdown => format_markdown(query, sort, offers, min_price, max_price),
    }
}

fn format_table(
    query: &str,
    sort: &str,
    offers: &[OfferData],
    min_price: Option<f64>,
    max_price: Option<f64>,
) -> String {
    let mut out = String::new();

    let filter_info = match (min_price, max_price) {
        (Some(min), Some(max)) => format!(" [price: {min:.0}€ - {max:.0}€]"),
        (Some(min), None) => format!(" [min: {min:.0}€]"),
        (None, Some(max)) => format!(" [max: {max:.0}€]"),
        (None, None) => String::new(),
    };

    out.push_str(&format!(
        "Found {} result(s) for '{}' (sorted by {}{}):\n\n",
        offers.len(),
        query,
        sort,
        filter_info
    ));

    out.push_str(&format!("{:<10} {:<45} {:<12} {:<20}\n", "ID", "Title", "Price", "Location"));
    out.push_str(&format!("{}\n", "-".repeat(90)));

    for offer in offers {
        let price = offer.get_price().map_or_else(|| "-".to_string(), |p| format!("{p:.2} €"));
        let city = offer.get_city().unwrap_or_else(|| "-".to_string());
        let region = offer.get_region();
        let location = match region {
            Some(r) => format!("{city}, {r}"),
            None => city,
        };

        out.push_str(&format!(
            "{:<10} {:<45} {:<12} {:<20}\n",
            offer.id,
            truncate(&offer.title, 43),
            price,
            truncate(&location, 18)
        ));
    }

    out.push_str("\nURLs:\n");
    for offer in offers {
        out.push_str(&format!("  {} - {}\n", offer.id, offer.url));
    }

    out
}

fn format_json(
    query: &str,
    sort: &str,
    offers: &[OfferData],
    min_price: Option<f64>,
    max_price: Option<f64>,
    city: Option<String>,
    radius: Option<i32>,
) -> String {
    let result = SearchResult {
        query: query.to_string(),
        sort: sort.to_string(),
        total: offers.len(),
        filters: SearchFilters { min_price, max_price, city, radius_km: radius },
        items: offers.iter().map(SearchResultItem::from).collect(),
    };

    serde_json::to_string_pretty(&result).unwrap_or_else(|_| "{}".to_string())
}

fn format_markdown(
    query: &str,
    sort: &str,
    offers: &[OfferData],
    min_price: Option<f64>,
    max_price: Option<f64>,
) -> String {
    let mut out = String::new();

    // Header
    out.push_str(&format!("# Search Results: \"{query}\"\n\n"));

    // Metadata
    out.push_str(&format!("**Found:** {} listings\n", offers.len()));
    out.push_str(&format!("**Sorted by:** {sort}\n"));

    if min_price.is_some() || max_price.is_some() {
        let filter = match (min_price, max_price) {
            (Some(min), Some(max)) => format!("{min:.0}€ - {max:.0}€"),
            (Some(min), None) => format!("≥ {min:.0}€"),
            (None, Some(max)) => format!("≤ {max:.0}€"),
            (None, None) => String::new(),
        };
        out.push_str(&format!("**Price filter:** {filter}\n"));
    }

    out.push_str("\n---\n\n");

    // Listings
    for (i, offer) in offers.iter().enumerate() {
        let price = offer
            .get_price()
            .map_or_else(|| "Price not listed".to_string(), |p| format!("{p:.2}€"));

        let location = match (offer.get_city(), offer.get_region()) {
            (Some(city), Some(region)) => format!("{city}, {region}"),
            (Some(city), None) => city,
            (None, Some(region)) => region,
            (None, None) => "Unknown location".to_string(),
        };

        out.push_str(&format!("## {}. {}\n\n", i + 1, offer.title));

        // Add main image if available
        if let Some(image_url) = offer.get_thumbnail() {
            out.push_str(&format!("![{}]({})\n\n", offer.title, image_url));
        }

        out.push_str(&format!("- **Price:** {price}\n"));
        out.push_str(&format!("- **Location:** {location}\n"));

        if let Some(seller) = offer.get_seller_name() {
            out.push_str(&format!("- **Seller:** {seller}\n"));
        }

        out.push_str(&format!("- **ID:** {}\n", offer.id));
        out.push_str(&format!("- **Link:** [View listing]({})\n", offer.url));
        out.push('\n');
    }

    // Summary table for quick reference
    out.push_str("---\n\n## Quick Reference\n\n");
    out.push_str("| # | Title | Price | Location |\n");
    out.push_str("|---|-------|-------|----------|\n");

    for (i, offer) in offers.iter().enumerate() {
        let price = offer.get_price().map_or_else(|| "-".to_string(), |p| format!("{p:.0}€"));
        let city = offer.get_city().unwrap_or_else(|| "-".to_string());
        let title = truncate(&offer.title, 40);

        out.push_str(&format!("| {} | {} | {} | {} |\n", i + 1, title, price, city));
    }

    out
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_len.saturating_sub(3)).collect();
        format!("{truncated}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{LocationCity, OfferLocation, OfferParam, OfferPhoto, OfferUser, ParamValue};

    fn create_test_offer(id: i64, title: &str, price: Option<f64>, city: Option<&str>) -> OfferData {
        let params = if let Some(p) = price {
            vec![OfferParam {
                key: "price".to_string(),
                name: "Price".to_string(),
                value: Some(ParamValue::Numeric { value: p, label: None }),
            }]
        } else {
            vec![]
        };

        let location = city.map(|c| OfferLocation {
            city: Some(LocationCity {
                id: Some(1),
                name: c.to_string(),
                normalized_name: None,
            }),
            region: None,
        });

        OfferData {
            id,
            title: title.to_string(),
            url: format!("https://example.com/{id}"),
            params,
            location,
            user: Some(OfferUser {
                id: Some(100),
                name: Some("Test Seller".to_string()),
            }),
            photos: vec![OfferPhoto {
                id: 1,
                filename: "photo.jpg".to_string(),
                link: "https://example.com/photo.jpg".to_string(),
                width: Some(800),
                height: Some(600),
            }],
            created_time: None,
            last_refresh_time: None,
        }
    }

    #[test]
    fn test_output_format_parse() {
        assert_eq!(OutputFormat::from_str("table").unwrap(), OutputFormat::Table);
        assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("markdown").unwrap(), OutputFormat::Markdown);
        assert_eq!(OutputFormat::from_str("md").unwrap(), OutputFormat::Markdown);
        assert_eq!(OutputFormat::from_str("llm").unwrap(), OutputFormat::Markdown);
        assert!(OutputFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("short", 10), "short");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("exact", 5), "exact");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("this is a very long string", 10), "this is...");
    }

    #[test]
    fn test_format_table_with_offers() {
        let offers = vec![
            create_test_offer(1, "iPhone 14", Some(500.0), Some("Lisbon")),
            create_test_offer(2, "MacBook Pro", Some(1200.0), Some("Porto")),
        ];

        let output = format_table("iphone", "newest", &offers, None, None);

        assert!(output.contains("iPhone 14"));
        assert!(output.contains("MacBook Pro"));
        assert!(output.contains("500.00 €"));
        assert!(output.contains("1200.00 €"));
        assert!(output.contains("Lisbon"));
        assert!(output.contains("Porto"));
        assert!(output.contains("for 'iphone'"));
        assert!(output.contains("sorted by newest"));
    }

    #[test]
    fn test_format_table_with_price_filter() {
        let offers = vec![create_test_offer(1, "Test", Some(100.0), Some("City"))];
        let output = format_table("query", "newest", &offers, Some(50.0), Some(200.0));

        assert!(output.contains("50€ - 200€"));
    }

    #[test]
    fn test_format_table_no_price() {
        let offers = vec![create_test_offer(1, "Free Item", None, Some("Lisbon"))];
        let output = format_table("query", "newest", &offers, None, None);

        assert!(output.contains("Free Item"));
        assert!(output.contains("-")); // No price shown as "-"
    }

    #[test]
    fn test_format_json_structure() {
        let offers = vec![create_test_offer(1, "Test Item", Some(100.0), Some("Lisbon"))];
        let output = format_json("query", "newest", &offers, Some(50.0), Some(150.0), Some("Lisbon".to_string()), Some(10));

        let json: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");

        assert_eq!(json["query"], "query");
        assert_eq!(json["sort"], "newest");
        assert_eq!(json["total"], 1);
        assert_eq!(json["filters"]["min_price"], 50.0);
        assert_eq!(json["filters"]["max_price"], 150.0);
        assert_eq!(json["filters"]["city"], "Lisbon");
        assert_eq!(json["filters"]["radius_km"], 10);
        assert_eq!(json["items"][0]["title"], "Test Item");
        assert_eq!(json["items"][0]["price"], 100.0);
    }

    #[test]
    fn test_format_json_no_filters() {
        let offers = vec![create_test_offer(1, "Item", Some(100.0), None)];
        let output = format_json("query", "newest", &offers, None, None, None, None);

        let json: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");

        assert!(json["filters"]["min_price"].is_null());
        assert!(json["filters"]["max_price"].is_null());
        assert!(json["filters"]["city"].is_null());
        assert!(json["filters"]["radius_km"].is_null());
    }

    #[test]
    fn test_format_markdown_structure() {
        let offers = vec![
            create_test_offer(1, "First", Some(100.0), Some("Lisbon")),
            create_test_offer(2, "Second", Some(200.0), Some("Porto")),
        ];

        let output = format_markdown("query", "newest", &offers, Some(50.0), Some(250.0));

        assert!(output.contains("# Search Results: \"query\""));
        assert!(output.contains("**Found:** 2 listings"));
        assert!(output.contains("**Sorted by:** newest"));
        assert!(output.contains("**Price filter:** 50€ - 250€"));
        assert!(output.contains("## 1. First"));
        assert!(output.contains("## 2. Second"));
        assert!(output.contains("- **Price:** 100.00€"));
        assert!(output.contains("- **Price:** 200.00€"));
        assert!(output.contains("- **Location:** Lisbon"));
        assert!(output.contains("- **Seller:** Test Seller"));
        assert!(output.contains("![First](https://example.com/photo.jpg)"));
        assert!(output.contains("Quick Reference"));
    }

    #[test]
    fn test_format_markdown_min_price_only() {
        let offers = vec![create_test_offer(1, "Item", Some(100.0), None)];
        let output = format_markdown("query", "newest", &offers, Some(50.0), None);

        assert!(output.contains("≥ 50€"));
    }

    #[test]
    fn test_format_markdown_max_price_only() {
        let offers = vec![create_test_offer(1, "Item", Some(100.0), None)];
        let output = format_markdown("query", "newest", &offers, None, Some(200.0));

        assert!(output.contains("≤ 200€"));
    }

    #[test]
    fn test_format_results_table() {
        let offers = vec![create_test_offer(1, "Item", Some(100.0), Some("City"))];
        let output = format_results(FormatParams {
            format: OutputFormat::Table,
            query: "test",
            sort: "newest",
            offers: &offers,
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
        });

        assert!(output.contains("Item"));
        assert!(output.contains("100.00 €"));
    }

    #[test]
    fn test_format_results_json() {
        let offers = vec![create_test_offer(1, "Item", Some(100.0), Some("City"))];
        let output = format_results(FormatParams {
            format: OutputFormat::Json,
            query: "test",
            sort: "newest",
            offers: &offers,
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
        });

        let json: serde_json::Value = serde_json::from_str(&output).expect("Valid JSON");
        assert_eq!(json["query"], "test");
    }

    #[test]
    fn test_format_results_markdown() {
        let offers = vec![create_test_offer(1, "Item", Some(100.0), Some("City"))];
        let output = format_results(FormatParams {
            format: OutputFormat::Markdown,
            query: "test",
            sort: "newest",
            offers: &offers,
            min_price: None,
            max_price: None,
            city: None,
            radius: None,
        });

        assert!(output.contains("# Search Results"));
        assert!(output.contains("## 1. Item"));
    }

    #[test]
    fn test_search_result_item_from_offer() {
        let offer = create_test_offer(1, "Test", Some(123.45), Some("Lisbon"));
        let item = SearchResultItem::from(&offer);

        assert_eq!(item.id, 1);
        assert_eq!(item.title, "Test");
        assert_eq!(item.price, Some(123.45));
        assert_eq!(item.city, Some("Lisbon".to_string()));
        assert_eq!(item.url, "https://example.com/1");
        assert_eq!(item.image, Some("https://example.com/photo.jpg".to_string()));
    }
}
