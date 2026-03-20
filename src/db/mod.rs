pub mod models;
pub mod queries;
pub mod seed;

use anyhow::{Context, Result};
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    if let Ok(path) = std::env::var("POKEDEX_DB_PATH") {
        PathBuf::from(path)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".pokedex").join("db.sqlite")
    }
}

pub fn open() -> Result<Connection> {
    let path = db_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    let mut conn = Connection::open(&path)
        .with_context(|| format!("Failed to open database at {}", path.display()))?;

    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    migrations().to_latest(&mut conn)?;

    Ok(conn)
}

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(include_str!("migrations/001_schema.sql")),
        M::up(include_str!("migrations/002_encounter_details.sql")),
    ])
}

pub fn is_seeded(conn: &Connection) -> Result<bool> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM species", [], |row| row.get(0))?;
    Ok(count > 0)
}

/// Open an in-memory database with migrations applied. For tests.
pub fn open_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;
    migrations().to_latest(&mut conn)?;
    Ok(conn)
}
