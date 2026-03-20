use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;

pub fn list(conn: &Connection, home_only: bool, format: &OutputFormat) -> Result<()> {
    let games = queries::list_games(conn, home_only)?;

    let actions: Vec<Action> = games.iter().map(|g| {
        Action::new("show", &format!("pokedex game show {}", g.name))
    }).collect();

    let cmd = if home_only { "pokedex game list --home-only" } else { "pokedex game list" };
    let response = Response::new(games, actions, Meta::simple(cmd));
    response.print(format)
}

pub fn show(conn: &Connection, game: &str, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_game(conn, game)?;
    let (_, game_name) = match resolved {
        Some(r) => r,
        None => {
            let all_games = queries::list_games(conn, false)?;
            let suggestions: Vec<Action> = all_games.iter().map(|g| {
                Action::new("did_you_mean", &format!("pokedex game show {}", g.name))
            }).collect();
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
