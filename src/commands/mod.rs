pub mod collection;
pub mod db_cmd;
pub mod dex;
pub mod game;
pub mod home;
pub mod location;
pub mod pokemon;
pub mod type_cmd;

use anyhow::Result;
use rusqlite::Connection;
use crate::db::queries;
use crate::output::*;

pub fn validate_limit(limit: u64) -> Result<u64> {
    if limit == 0 {
        ErrorResponse::invalid_parameter(
            "Invalid limit '0'. Limit must be at least 1.",
            vec![Action::new("discover", "pokedex --discover")],
        ).print()?;
    }
    Ok(limit)
}

pub fn validate_game_filter(conn: &Connection, game: &str, command_prefix: &str) -> Result<()> {
    // Check games table (now includes both HOME-era and pre-HOME games)
    if queries::resolve_game(conn, game)?.is_some() {
        return Ok(());
    }
    // Also check versions table directly (catches any version not in games)
    let version_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM versions WHERE LOWER(name) = LOWER(?1))",
        rusqlite::params![game],
        |row| row.get::<_, i64>(0).map(|v| v != 0),
    ).unwrap_or(false);
    if version_exists {
        return Ok(());
    }
    // Not found — fuzzy match against games table only (which has all games now)
    let all = queries::list_games(conn, false)?;
    let mut scored: Vec<_> = all.iter()
        .map(|g| (strsim::jaro_winkler(&game.to_lowercase(), &g.name.to_lowercase()), g))
        .filter(|(score, _)| *score > 0.6)
        .collect();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(5);
    let suggestions: Vec<Action> = scored.iter()
        .map(|(_, g)| Action::new("did_you_mean", &format!("{command_prefix} --game={}", g.name)))
        .collect();
    ErrorResponse::not_found(
        &format!("No game named '{game}'"),
        if suggestions.is_empty() {
            vec![Action::new("list", "pokedex game list")]
        } else { suggestions },
    ).print()?;
    Ok(())
}
