pub mod collection;
pub mod db_cmd;
pub mod dex;
pub mod game;
pub mod home;
pub mod pokemon;
pub mod type_cmd;

use anyhow::Result;
use rusqlite::Connection;
use crate::db::queries;
use crate::output::*;

pub fn validate_game_filter(conn: &Connection, game: &str, command_prefix: &str) -> Result<()> {
    // Check games table first (HOME-era games)
    if queries::resolve_game(conn, game)?.is_some() {
        return Ok(());
    }
    // Also check versions table (pre-HOME games like red, gold, ruby, etc.)
    let version_exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM versions WHERE LOWER(name) = LOWER(?1))",
        rusqlite::params![game],
        |row| row.get::<_, i64>(0).map(|v| v != 0),
    ).unwrap_or(false);
    if version_exists {
        return Ok(());
    }
    // Neither found - error with suggestions from both tables
    let all = queries::list_games(conn, false)?;
    let mut suggestions: Vec<Action> = all.iter()
        .filter(|g| strsim::jaro_winkler(&game.to_lowercase(), &g.name.to_lowercase()) > 0.5)
        .take(5)
        .map(|g| Action::new("did_you_mean", &format!("{command_prefix} --game={}", g.name)))
        .collect();
    // Also check versions table for suggestions
    let mut ver_stmt = conn.prepare(
        "SELECT name FROM versions WHERE ?1 != '' ORDER BY id"
    ).ok();
    if let Some(ref mut stmt) = ver_stmt {
        let ver_names: Vec<String> = stmt.query_map(rusqlite::params![game], |row| row.get(0))
            .ok()
            .map(|rows| rows.filter_map(|r| r.ok()).collect())
            .unwrap_or_default();
        for vn in &ver_names {
            if strsim::jaro_winkler(&game.to_lowercase(), &vn.to_lowercase()) > 0.5 {
                let action = Action::new("did_you_mean", &format!("{command_prefix} --game={vn}"));
                if !suggestions.iter().any(|s| s.cmd == action.cmd) {
                    suggestions.push(action);
                }
                if suggestions.len() >= 5 { break; }
            }
        }
    }
    ErrorResponse::not_found(
        &format!("No game named '{game}'"),
        if suggestions.is_empty() {
            vec![Action::new("list", "pokedex game list")]
        } else { suggestions },
    ).print()?;
    Ok(())
}
