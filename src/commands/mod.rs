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

pub fn validate_game_filter(conn: &Connection, game: &str) -> Result<()> {
    if queries::resolve_game(conn, game)?.is_none() {
        let all = queries::list_games(conn, false)?;
        let suggestions: Vec<Action> = all.iter()
            .filter(|g| strsim::jaro_winkler(&game.to_lowercase(), &g.name.to_lowercase()) > 0.5)
            .take(5)
            .map(|g| Action::new("did_you_mean", &format!("... --game={}", g.name)))
            .collect();
        ErrorResponse::not_found(
            &format!("No game named '{game}'"),
            if suggestions.is_empty() {
                vec![Action::new("list", "pokedex game list")]
            } else { suggestions },
        ).print()?;
    }
    Ok(())
}
