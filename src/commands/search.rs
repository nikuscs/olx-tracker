use anyhow::Result;
use tracing::info;

use olx_tracker::api::SearchParams;
use olx_tracker::{Config, FormatParams, OlxClient, OutputFormat, SortOrder, format_results};

#[allow(clippy::too_many_arguments)]
pub async fn cmd_search(
    config: &Config,
    query: &str,
    max_results: i32,
    sort: &str,
    min_price: Option<f64>,
    max_price: Option<f64>,
    city: Option<String>,
    radius: Option<i32>,
    format: &str,
) -> Result<()> {
    let sort_order: SortOrder = sort.parse().map_err(|e: String| anyhow::anyhow!("{e}"))?;
    let output_format: OutputFormat = format.parse().map_err(|e: String| anyhow::anyhow!("{e}"))?;

    let client = OlxClient::new(config)?;

    // Lookup city ID if city name provided
    let city_id = if let Some(ref city_name) = city {
        let location = client.lookup_city(city_name).await?;
        match location {
            Some(loc) => {
                info!("Found city: {} (ID: {})", loc.city.name, loc.city.id.unwrap_or(0));
                loc.city.id
            }
            None => {
                anyhow::bail!("City not found: {city_name}");
            }
        }
    } else {
        None
    };

    let params = SearchParams {
        query: query.to_string(),
        city_id,
        radius_km: radius,
        category_id: None,
        sort: sort_order,
        offset: 0,
        limit: 50,
    };

    // Fetch more results to account for filtering
    let fetch_count = max_results * 3; // Fetch 3x to have enough after filtering
    let all_offers = client.search_all(&params, fetch_count).await?;

    // Apply price filters
    let offers: Vec<_> = all_offers
        .into_iter()
        .filter(|o| {
            let price = o.get_price();
            match (price, min_price, max_price) {
                (Some(p), Some(min), Some(max)) => p >= min && p <= max,
                (Some(p), Some(min), None) => p >= min,
                (Some(p), None, Some(max)) => p <= max,
                (None, _, _) | (Some(_), None, None) => true, // Keep items without price
            }
        })
        .take(max_results as usize)
        .collect();

    if offers.is_empty() {
        println!("No results found for '{query}'");
        if min_price.is_some() || max_price.is_some() {
            println!(
                "(price filter: {} - {})",
                min_price.map_or_else(|| "any".to_string(), |p| format!("{p:.0}€")),
                max_price.map_or_else(|| "any".to_string(), |p| format!("{p:.0}€"))
            );
        }
        return Ok(());
    }

    let output = format_results(FormatParams {
        format: output_format,
        query,
        sort,
        offers: &offers,
        min_price,
        max_price,
        city,
        radius,
    });

    print!("{output}");

    Ok(())
}
