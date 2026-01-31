use anyhow::Result;
use chrono::{Duration as ChronoDuration, Utc};

use olx_tracker::{Database, SortOrder};

#[allow(clippy::too_many_arguments)]
pub fn cmd_add(
    db: &Database,
    name: &str,
    keyword: &str,
    min_price: Option<f64>,
    max_price: Option<f64>,
    city: Option<String>,
    radius: Option<i32>,
    category: Option<i64>,
    sort: &str,
    days: Option<i64>,
) -> Result<()> {
    // Validate sort order
    let _: SortOrder = sort.parse().map_err(|e: String| anyhow::anyhow!("{e}"))?;

    // Calculate expires_at if days is specified
    let expires_at = days.map(|d| {
        let expires = Utc::now() + ChronoDuration::days(d);
        expires.to_rfc3339()
    });

    let id = db.create_search(
        name,
        keyword,
        min_price,
        max_price,
        city.as_deref(),
        radius,
        category,
        Some(sort),
        expires_at.as_deref(),
    )?;

    let price_info = match (min_price, max_price) {
        (Some(min), Some(max)) => format!(", price: {min:.0}€-{max:.0}€"),
        (Some(min), None) => format!(", min: {min:.0}€"),
        (None, Some(max)) => format!(", max: {max:.0}€"),
        (None, None) => String::new(),
    };
    let ttl_info = days.map_or(String::new(), |d| format!(", expires in {d} days"));
    println!("Created search '{name}' with ID {id} (sort: {sort}{price_info}{ttl_info})");
    Ok(())
}
