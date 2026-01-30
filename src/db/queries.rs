use anyhow::{Context, Result};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

use super::schema::run_migrations;

#[derive(Debug, Clone)]
pub struct Search {
    pub id: i64,
    pub name: String,
    pub keyword: String,
    pub max_price: Option<f64>,
    pub city: Option<String>,
    pub radius_km: Option<i32>,
    pub category_id: Option<i64>,
    pub active: bool,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct Listing {
    pub id: i64,
    pub search_id: i64,
    pub title: String,
    pub price: Option<f64>,
    pub currency: String,
    pub url: String,
    pub city: Option<String>,
    pub region: Option<String>,
    pub seller_name: Option<String>,
    pub first_seen_at: String,
    pub last_seen_at: Option<String>,
    pub is_deal: bool,
}

#[derive(Debug, Clone)]
pub struct SearchStats {
    pub search_id: i64,
    pub avg_price: Option<f64>,
    pub min_price: Option<f64>,
    pub max_price: Option<f64>,
    pub total_listings: i64,
    pub last_updated_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PriceHistory {
    pub id: i64,
    pub listing_id: i64,
    pub price: Option<f64>,
    pub recorded_at: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref())
            .with_context(|| format!("Failed to open database: {:?}", path.as_ref()))?;

        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        run_migrations(&conn)?;

        Ok(Self { conn })
    }

    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    // Search CRUD operations

    pub fn create_search(
        &self,
        name: &str,
        keyword: &str,
        max_price: Option<f64>,
        city: Option<&str>,
        radius_km: Option<i32>,
        category_id: Option<i64>,
    ) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO searches (name, keyword, max_price, city, radius_km, category_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![name, keyword, max_price, city, radius_km, category_id],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn get_search(&self, id: i64) -> Result<Option<Search>> {
        self.conn
            .query_row(
                "SELECT id, name, keyword, max_price, city, radius_km, category_id, active, created_at
                 FROM searches WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Search {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        keyword: row.get(2)?,
                        max_price: row.get(3)?,
                        city: row.get(4)?,
                        radius_km: row.get(5)?,
                        category_id: row.get(6)?,
                        active: row.get::<_, i32>(7)? != 0,
                        created_at: row.get(8)?,
                    })
                },
            )
            .optional()
            .context("Failed to get search")
    }

    pub fn list_searches(&self, active_only: bool) -> Result<Vec<Search>> {
        let sql = if active_only {
            "SELECT id, name, keyword, max_price, city, radius_km, category_id, active, created_at
             FROM searches WHERE active = 1 ORDER BY id"
        } else {
            "SELECT id, name, keyword, max_price, city, radius_km, category_id, active, created_at
             FROM searches ORDER BY id"
        };

        let mut stmt = self.conn.prepare(sql)?;
        let searches = stmt
            .query_map([], |row| {
                Ok(Search {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    keyword: row.get(2)?,
                    max_price: row.get(3)?,
                    city: row.get(4)?,
                    radius_km: row.get(5)?,
                    category_id: row.get(6)?,
                    active: row.get::<_, i32>(7)? != 0,
                    created_at: row.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(searches)
    }

    pub fn delete_search(&self, id: i64) -> Result<bool> {
        let rows = self.conn.execute("DELETE FROM searches WHERE id = ?1", params![id])?;
        Ok(rows > 0)
    }

    pub fn set_search_active(&self, id: i64, active: bool) -> Result<bool> {
        let rows = self.conn.execute(
            "UPDATE searches SET active = ?2 WHERE id = ?1",
            params![id, i32::from(active)],
        )?;
        Ok(rows > 0)
    }

    // Listing operations

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_listing(
        &self,
        id: i64,
        search_id: i64,
        title: &str,
        price: Option<f64>,
        currency: &str,
        url: &str,
        city: Option<&str>,
        region: Option<&str>,
        seller_name: Option<&str>,
    ) -> Result<bool> {
        let existing = self.get_listing(id)?;
        let now = Utc::now().to_rfc3339();

        if let Some(existing) = existing {
            // Update existing listing
            self.conn.execute(
                "UPDATE listings SET title = ?2, price = ?3, currency = ?4, url = ?5,
                 city = ?6, region = ?7, seller_name = ?8, last_seen_at = ?9
                 WHERE id = ?1",
                params![id, title, price, currency, url, city, region, seller_name, now],
            )?;

            // Record price change if different
            if existing.price != price {
                self.record_price(id, price)?;
            }

            Ok(false) // Not new
        } else {
            // Insert new listing
            self.conn.execute(
                "INSERT INTO listings (id, search_id, title, price, currency, url, city, region, seller_name, first_seen_at, last_seen_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)",
                params![id, search_id, title, price, currency, url, city, region, seller_name, now],
            )?;

            // Record initial price
            self.record_price(id, price)?;

            Ok(true) // Is new
        }
    }

    pub fn get_listing(&self, id: i64) -> Result<Option<Listing>> {
        self.conn
            .query_row(
                "SELECT id, search_id, title, price, currency, url, city, region, seller_name,
                 first_seen_at, last_seen_at, is_deal FROM listings WHERE id = ?1",
                params![id],
                |row| {
                    Ok(Listing {
                        id: row.get(0)?,
                        search_id: row.get(1)?,
                        title: row.get(2)?,
                        price: row.get(3)?,
                        currency: row.get(4)?,
                        url: row.get(5)?,
                        city: row.get(6)?,
                        region: row.get(7)?,
                        seller_name: row.get(8)?,
                        first_seen_at: row.get(9)?,
                        last_seen_at: row.get(10)?,
                        is_deal: row.get::<_, i32>(11)? != 0,
                    })
                },
            )
            .optional()
            .context("Failed to get listing")
    }

    pub fn get_listings_for_search(&self, search_id: i64) -> Result<Vec<Listing>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, search_id, title, price, currency, url, city, region, seller_name,
             first_seen_at, last_seen_at, is_deal FROM listings WHERE search_id = ?1
             ORDER BY first_seen_at DESC",
        )?;

        let listings = stmt
            .query_map(params![search_id], |row| {
                Ok(Listing {
                    id: row.get(0)?,
                    search_id: row.get(1)?,
                    title: row.get(2)?,
                    price: row.get(3)?,
                    currency: row.get(4)?,
                    url: row.get(5)?,
                    city: row.get(6)?,
                    region: row.get(7)?,
                    seller_name: row.get(8)?,
                    first_seen_at: row.get(9)?,
                    last_seen_at: row.get(10)?,
                    is_deal: row.get::<_, i32>(11)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(listings)
    }

    pub fn get_deals(&self, search_id: Option<i64>) -> Result<Vec<Listing>> {
        let sql = match search_id {
            Some(_) => {
                "SELECT id, search_id, title, price, currency, url, city, region, seller_name,
                 first_seen_at, last_seen_at, is_deal FROM listings
                 WHERE is_deal = 1 AND search_id = ?1
                 ORDER BY price ASC"
            }
            None => {
                "SELECT id, search_id, title, price, currency, url, city, region, seller_name,
                 first_seen_at, last_seen_at, is_deal FROM listings
                 WHERE is_deal = 1
                 ORDER BY price ASC"
            }
        };

        let mut stmt = self.conn.prepare(sql)?;

        let listings = if let Some(sid) = search_id {
            stmt.query_map(params![sid], |row| {
                Ok(Listing {
                    id: row.get(0)?,
                    search_id: row.get(1)?,
                    title: row.get(2)?,
                    price: row.get(3)?,
                    currency: row.get(4)?,
                    url: row.get(5)?,
                    city: row.get(6)?,
                    region: row.get(7)?,
                    seller_name: row.get(8)?,
                    first_seen_at: row.get(9)?,
                    last_seen_at: row.get(10)?,
                    is_deal: row.get::<_, i32>(11)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], |row| {
                Ok(Listing {
                    id: row.get(0)?,
                    search_id: row.get(1)?,
                    title: row.get(2)?,
                    price: row.get(3)?,
                    currency: row.get(4)?,
                    url: row.get(5)?,
                    city: row.get(6)?,
                    region: row.get(7)?,
                    seller_name: row.get(8)?,
                    first_seen_at: row.get(9)?,
                    last_seen_at: row.get(10)?,
                    is_deal: row.get::<_, i32>(11)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        Ok(listings)
    }

    pub fn mark_as_deal(&self, listing_id: i64, is_deal: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE listings SET is_deal = ?2 WHERE id = ?1",
            params![listing_id, i32::from(is_deal)],
        )?;
        Ok(())
    }

    // Price history operations

    pub fn record_price(&self, listing_id: i64, price: Option<f64>) -> Result<()> {
        self.conn.execute(
            "INSERT INTO price_history (listing_id, price) VALUES (?1, ?2)",
            params![listing_id, price],
        )?;
        Ok(())
    }

    pub fn get_price_history(&self, listing_id: i64) -> Result<Vec<PriceHistory>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, listing_id, price, recorded_at FROM price_history
             WHERE listing_id = ?1 ORDER BY recorded_at DESC",
        )?;

        let history = stmt
            .query_map(params![listing_id], |row| {
                Ok(PriceHistory {
                    id: row.get(0)?,
                    listing_id: row.get(1)?,
                    price: row.get(2)?,
                    recorded_at: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(history)
    }

    // Search stats operations

    pub fn update_search_stats(&self, search_id: i64) -> Result<SearchStats> {
        let stats: (Option<f64>, Option<f64>, Option<f64>, i64) = self.conn.query_row(
            "SELECT AVG(price), MIN(price), MAX(price), COUNT(*)
             FROM listings WHERE search_id = ?1 AND price IS NOT NULL",
            params![search_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        let now = Utc::now().to_rfc3339();

        self.conn.execute(
            "INSERT INTO search_stats (search_id, avg_price, min_price, max_price, total_listings, last_updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(search_id) DO UPDATE SET
             avg_price = excluded.avg_price,
             min_price = excluded.min_price,
             max_price = excluded.max_price,
             total_listings = excluded.total_listings,
             last_updated_at = excluded.last_updated_at",
            params![search_id, stats.0, stats.1, stats.2, stats.3, now],
        )?;

        Ok(SearchStats {
            search_id,
            avg_price: stats.0,
            min_price: stats.1,
            max_price: stats.2,
            total_listings: stats.3,
            last_updated_at: Some(now),
        })
    }

    pub fn get_search_stats(&self, search_id: i64) -> Result<Option<SearchStats>> {
        self.conn
            .query_row(
                "SELECT search_id, avg_price, min_price, max_price, total_listings, last_updated_at
                 FROM search_stats WHERE search_id = ?1",
                params![search_id],
                |row| {
                    Ok(SearchStats {
                        search_id: row.get(0)?,
                        avg_price: row.get(1)?,
                        min_price: row.get(2)?,
                        max_price: row.get(3)?,
                        total_listings: row.get(4)?,
                        last_updated_at: row.get(5)?,
                    })
                },
            )
            .optional()
            .context("Failed to get search stats")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_crud() {
        let db = Database::open_in_memory().unwrap();

        let id =
            db.create_search("Test", "iphone", Some(500.0), Some("Porto"), Some(30), None).unwrap();

        let search = db.get_search(id).unwrap().unwrap();
        assert_eq!(search.name, "Test");
        assert_eq!(search.keyword, "iphone");
        assert_eq!(search.max_price, Some(500.0));
        assert!(search.active);

        let searches = db.list_searches(true).unwrap();
        assert_eq!(searches.len(), 1);

        db.delete_search(id).unwrap();
        assert!(db.get_search(id).unwrap().is_none());
    }

    #[test]
    fn test_listing_upsert() {
        let db = Database::open_in_memory().unwrap();

        let search_id = db.create_search("Test", "iphone", None, None, None, None).unwrap();

        // Insert new listing
        let is_new = db
            .upsert_listing(
                12345,
                search_id,
                "iPhone 12",
                Some(400.0),
                "EUR",
                "https://olx.pt/12345",
                Some("Porto"),
                None,
                Some("John"),
            )
            .unwrap();
        assert!(is_new);

        // Update existing listing
        let is_new = db
            .upsert_listing(
                12345,
                search_id,
                "iPhone 12 Pro",
                Some(350.0),
                "EUR",
                "https://olx.pt/12345",
                Some("Porto"),
                None,
                Some("John"),
            )
            .unwrap();
        assert!(!is_new);

        let listing = db.get_listing(12345).unwrap().unwrap();
        assert_eq!(listing.title, "iPhone 12 Pro");
        assert_eq!(listing.price, Some(350.0));

        // Check price history
        let history = db.get_price_history(12345).unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_search_stats() {
        let db = Database::open_in_memory().unwrap();

        let search_id = db.create_search("Test", "iphone", None, None, None, None).unwrap();

        db.upsert_listing(1, search_id, "A", Some(100.0), "EUR", "url1", None, None, None).unwrap();
        db.upsert_listing(2, search_id, "B", Some(200.0), "EUR", "url2", None, None, None).unwrap();
        db.upsert_listing(3, search_id, "C", Some(300.0), "EUR", "url3", None, None, None).unwrap();

        let stats = db.update_search_stats(search_id).unwrap();
        assert_eq!(stats.avg_price, Some(200.0));
        assert_eq!(stats.min_price, Some(100.0));
        assert_eq!(stats.max_price, Some(300.0));
        assert_eq!(stats.total_listings, 3);
    }
}
