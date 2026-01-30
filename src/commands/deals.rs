use anyhow::Result;

use olx_tracker::{truncate, Database};

pub fn cmd_deals(db: &Database, search_id: Option<i64>) -> Result<()> {
    let deals = db.get_deals(search_id)?;

    if deals.is_empty() {
        println!("No deals found.");
        return Ok(());
    }

    println!("Found {} deal(s):\n", deals.len());
    println!("{:<8} {:<40} {:<10} {:<15}", "ID", "Title", "Price", "City");
    println!("{}", "-".repeat(75));

    for deal in deals {
        let price = deal.price.map_or_else(|| "-".to_string(), |p| format!("{p:.2} €"));
        let city = deal.city.as_deref().unwrap_or("-");

        println!(
            "{:<8} {:<40} {:<10} {:<15}",
            deal.id,
            truncate(&deal.title, 38),
            price,
            truncate(city, 13)
        );
        println!("         {}", deal.url);
    }

    Ok(())
}
