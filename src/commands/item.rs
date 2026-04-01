use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;
use super::validate_game_filter;

pub fn show(conn: &Connection, item: &str, game: Option<&str>, format: &OutputFormat) -> Result<()> {
    if item.trim().is_empty() {
        ErrorResponse::invalid_parameter(
            "Item name or ID is required",
            vec![Action::new("discover", "pokedex --discover")],
        ).print()?;
        return Ok(());
    }

    if let Some(g) = game {
        validate_game_filter(conn, g, &format!("pokedex item show {item}"))?;
    }

    let resolved = queries::resolve_item(conn, item)?;
    let (item_id, _slug) = match resolved {
        Some(r) => r,
        None => {
            let suggestions = queries::search_items(conn, item);
            let actions: Vec<Action> = if suggestions.is_empty() {
                vec![Action::new("discover", "pokedex --discover")]
            } else {
                suggestions.iter()
                    .map(|(slug, _display, _score)| Action::new("did_you_mean", &format!("pokedex item show {slug}")))
                    .collect()
            };
            ErrorResponse::not_found(
                &format!("No item named '{item}'"),
                actions,
            ).print()?;
            return Ok(());
        }
    };

    let info = queries::get_item(conn, item_id, game)?;

    let mut actions = vec![];
    // Link to Pokémon that hold this item
    for holder in &info.held_by {
        let cmd = format!("pokedex pokemon show {}", holder.pokemon_slug);
        if !actions.iter().any(|a: &Action| a.cmd == cmd) {
            actions.push(Action::new("held_by", &cmd));
        }
    }
    actions.push(Action::new("discover", "pokedex --discover"));

    let mut cmd = format!("pokedex item show {}", info.name);
    if let Some(g) = game {
        cmd.push_str(&format!(" --game={g}"));
    }
    let response = Response::new(info, actions, Meta::simple(&cmd));
    response.print(format)
}
