pub mod models;
pub mod overrides;
pub mod queries;
pub mod seed;

use anyhow::{Context, Result};
use rusqlite::Connection;
use rusqlite_migration::{Migrations, M};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    resolve_db_path(
        std::env::var("POKEDEX_DB_PATH").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
}

/// Pure path resolution — testable without env var side effects.
pub fn resolve_db_path(env_path: Option<&str>, home: Option<&str>) -> PathBuf {
    if let Some(path) = env_path {
        PathBuf::from(path)
    } else {
        let home = home.unwrap_or(".");
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
        M::up(include_str!("migrations/003_collection_alpha.sql")),
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
