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
            _ => Err(format!(
                "Unknown format: {s}. Valid: table, json, markdown"
            )),
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

/// Format search results based on output format
pub fn format_results(
    format: OutputFormat,
    query: &str,
    sort: &str,
    offers: &[OfferData],
    min_price: Option<f64>,
    max_price: Option<f64>,
    city: Option<String>,
    radius: Option<i32>,
) -> String {
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

    out.push_str(&format!(
        "{:<10} {:<45} {:<12} {:<20}\n",
        "ID", "Title", "Price", "Location"
    ));
    out.push_str(&format!("{}\n", "-".repeat(90)));

    for offer in offers {
        let price = offer
            .get_price()
            .map_or_else(|| "-".to_string(), |p| format!("{p:.2} €"));
        let city = offer.get_city().unwrap_or_else(|| "-".to_string());
        let region = offer.get_region();
        let location = match region {
            Some(r) => format!("{}, {}", city, r),
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
        filters: SearchFilters {
            min_price,
            max_price,
            city,
            radius_km: radius,
        },
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
    out.push_str(&format!("# Search Results: \"{}\"\n\n", query));

    // Metadata
    out.push_str(&format!("**Found:** {} listings\n", offers.len()));
    out.push_str(&format!("**Sorted by:** {}\n", sort));

    if min_price.is_some() || max_price.is_some() {
        let filter = match (min_price, max_price) {
            (Some(min), Some(max)) => format!("{:.0}€ - {:.0}€", min, max),
            (Some(min), None) => format!("≥ {:.0}€", min),
            (None, Some(max)) => format!("≤ {:.0}€", max),
            (None, None) => String::new(),
        };
        out.push_str(&format!("**Price filter:** {}\n", filter));
    }

    out.push_str("\n---\n\n");

    // Listings
    for (i, offer) in offers.iter().enumerate() {
        let price = offer
            .get_price()
            .map_or_else(|| "Price not listed".to_string(), |p| format!("{:.2}€", p));

        let location = match (offer.get_city(), offer.get_region()) {
            (Some(city), Some(region)) => format!("{}, {}", city, region),
            (Some(city), None) => city,
            (None, Some(region)) => region,
            (None, None) => "Unknown location".to_string(),
        };

        out.push_str(&format!("## {}. {}\n\n", i + 1, offer.title));

        // Add main image if available
        if let Some(image_url) = offer.get_thumbnail() {
            out.push_str(&format!("![{}]({})\n\n", offer.title, image_url));
        }

        out.push_str(&format!("- **Price:** {}\n", price));
        out.push_str(&format!("- **Location:** {}\n", location));

        if let Some(seller) = offer.get_seller_name() {
            out.push_str(&format!("- **Seller:** {}\n", seller));
        }

        out.push_str(&format!("- **ID:** {}\n", offer.id));
        out.push_str(&format!("- **Link:** [View listing]({})\n", offer.url));
        out.push_str("\n");
    }

    // Summary table for quick reference
    out.push_str("---\n\n## Quick Reference\n\n");
    out.push_str("| # | Title | Price | Location |\n");
    out.push_str("|---|-------|-------|----------|\n");

    for (i, offer) in offers.iter().enumerate() {
        let price = offer
            .get_price()
            .map_or_else(|| "-".to_string(), |p| format!("{:.0}€", p));
        let city = offer.get_city().unwrap_or_else(|| "-".to_string());
        let title = truncate(&offer.title, 40);

        out.push_str(&format!("| {} | {} | {} | {} |\n", i + 1, title, price, city));
    }

    out
}

fn truncate(s: &str, max_len: usize) -> String {
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

    #[test]
    fn test_output_format_parse() {
        assert_eq!(OutputFormat::from_str("table").unwrap(), OutputFormat::Table);
        assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("markdown").unwrap(), OutputFormat::Markdown);
        assert_eq!(OutputFormat::from_str("md").unwrap(), OutputFormat::Markdown);
        assert_eq!(OutputFormat::from_str("llm").unwrap(), OutputFormat::Markdown);
        assert!(OutputFormat::from_str("invalid").is_err());
    }
}
