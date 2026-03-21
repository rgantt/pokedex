use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;
use strsim;

pub fn list(conn: &Connection, home_compatible: bool, format: &OutputFormat) -> Result<()> {
    let games = queries::list_games(conn, home_compatible)?;

    let actions: Vec<Action> = games.iter().map(|g| {
        Action::new("show", &format!("pokedex game show {}", g.name))
    }).collect();

    let cmd = if home_compatible { "pokedex game list --home-compatible" } else { "pokedex game list" };
    let response = Response::new(games, actions, Meta::simple(cmd));
    response.print(format)
}

pub fn show(conn: &Connection, game: &str, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_game(conn, game)?;
    let (_, game_name) = match resolved {
        Some(r) => r,
        None => {
            // Check if it's a known version (pre-HOME game)
            let is_known_version: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM versions WHERE LOWER(name) = LOWER(?1))",
                rusqlite::params![game],
                |row| row.get::<_, i64>(0).map(|v| v != 0),
            ).unwrap_or(false);

            if is_known_version {
                let err = ErrorResponse::not_found(
                    &format!("'{game}' is a known game but not tracked in the games list. Use --game={game} with encounter/move commands to filter by this version."),
                    vec![
                        Action::new("encounters", &format!("pokedex pokemon encounters pikachu --game={game}")),
                        Action::new("game_list", "pokedex game list"),
                    ],
                );
                err.print()?;
                return Ok(());
            }

            let all_games = queries::list_games(conn, false)?;
            let mut suggestions: Vec<Action> = all_games.iter()
                .filter(|g| strsim::jaro_winkler(&game.to_lowercase(), &g.name.to_lowercase()) > 0.5)
                .take(5)
                .map(|g| Action::new("did_you_mean", &format!("pokedex game show {}", g.name)))
                .collect();
            if suggestions.is_empty() {
                suggestions = all_games.iter().map(|g| Action::new("did_you_mean", &format!("pokedex game show {}", g.name))).collect();
            }
            let err = ErrorResponse::not_found(
                &format!("No game named '{game}'"),
                suggestions,
            );
            err.print()?;
            return Ok(());
        }
    };

    let games = queries::list_games(conn, false)?;
    let game_info = games.into_iter().find(|g| g.name == game_name);

    if let Some(info) = game_info {
        let actions = vec![
            Action::new("collection_for_game", &format!("pokedex collection list --game={game_name}")),
            Action::new("all_games", "pokedex game list"),
        ];
        let response = Response::new(info, actions, Meta::simple(&format!("pokedex game show {game}")));
        response.print(format)?;
    }

    Ok(())
}
