use anyhow::Result;

use olx_tracker::{Database, truncate};

pub fn cmd_list(db: &Database, include_inactive: bool) -> Result<()> {
    let searches = db.list_searches(!include_inactive)?;

    if searches.is_empty() {
        println!("No searches found. Use 'olx-tracker add' to create one.");
        return Ok(());
    }

    println!(
        "{:<4} {:<18} {:<18} {:<12} {:<12} {:<10} {:<6}",
        "ID", "Name", "Keyword", "Price", "City", "Sort", "Active"
    );
    println!("{}", "-".repeat(90));

    for search in searches {
        let price_range = match (search.min_price, search.max_price) {
            (Some(min), Some(max)) => format!("{min:.0}-{max:.0}€"),
            (Some(min), None) => format!(">{min:.0}€"),
            (None, Some(max)) => format!("<{max:.0}€"),
            (None, None) => "-".to_string(),
        };
        let city = search.city.as_deref().unwrap_or("-");
        let active = if search.active { "Yes" } else { "No" };

        println!(
            "{:<4} {:<18} {:<18} {:<12} {:<12} {:<10} {:<6}",
            search.id,
            truncate(&search.name, 16),
            truncate(&search.keyword, 16),
            truncate(&price_range, 10),
            truncate(city, 10),
            truncate(&search.sort_order, 8),
            active
        );
    }

    Ok(())
}
