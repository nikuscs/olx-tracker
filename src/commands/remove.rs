use anyhow::Result;

use olx_tracker::Database;

pub fn cmd_remove(db: &Database, search_id: i64) -> Result<()> {
    if db.delete_search(search_id)? {
        println!("Removed search with ID {search_id}");
    } else {
        println!("Search with ID {search_id} not found");
    }
    Ok(())
}
