use anyhow::Result;

use olx_tracker::Database;

pub fn cmd_stats(db: &Database, search_id: i64) -> Result<()> {
    let search = db
        .get_search(search_id)?
        .ok_or_else(|| anyhow::anyhow!("Search with ID {search_id} not found"))?;

    let stats = db.update_search_stats(search_id)?;

    println!("Statistics for '{}' (ID: {})", search.name, search.id);
    println!("{}", "-".repeat(40));
    println!("Keyword:         {}", search.keyword);
    println!("Max price:       {}", fmt_price(search.max_price));
    println!("City:            {}", search.city.as_deref().unwrap_or("-"));
    println!(
        "Radius:          {}",
        search.radius_km.map_or_else(|| "-".to_string(), |r| format!("{r} km"))
    );
    println!();
    println!("Total listings:  {}", stats.total_listings);
    println!("Average price:   {}", fmt_price(stats.avg_price));
    println!("Min price:       {}", fmt_price(stats.min_price));
    println!("Max price:       {}", fmt_price(stats.max_price));
    println!("Last updated:    {}", stats.last_updated_at.as_deref().unwrap_or("-"));

    Ok(())
}

fn fmt_price(price: Option<f64>) -> String {
    price.map_or_else(|| "-".to_string(), |p| format!("{p:.2} €"))
}
