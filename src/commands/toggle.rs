use anyhow::Result;

use olx_tracker::Database;

pub fn cmd_toggle(db: &Database, search_id: i64) -> Result<()> {
    let search = db
        .get_search(search_id)?
        .ok_or_else(|| anyhow::anyhow!("Search with ID {search_id} not found"))?;

    let new_status = !search.active;
    db.set_search_active(search_id, new_status)?;

    println!("Search '{}' is now {}", search.name, if new_status { "active" } else { "inactive" });
    Ok(())
}
