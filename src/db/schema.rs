use anyhow::Result;
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};

/// Returns all migrations in order.
fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("../../migrations/V1__initial_schema.sql")),
    ])
}

pub fn run_migrations(conn: &mut Connection) -> Result<()> {
    migrations().to_latest(conn)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_valid() {
        // Validate that migrations are well-formed
        assert!(migrations().validate().is_ok());
    }

    #[test]
    fn test_migrations_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut conn).unwrap();
        run_migrations(&mut conn).unwrap();

        // Verify tables exist
        let count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='searches'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);
    }
}
