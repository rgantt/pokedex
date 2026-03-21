use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;
use super::validate_game_filter;

pub fn encounters(
    conn: &Connection,
    location: &str,
    game: Option<&str>,
    limit: u64,
    offset: u64,
    format: &OutputFormat,
) -> Result<()> {
    if location.trim().is_empty() {
        ErrorResponse::invalid_parameter(
            "Location name is required",
            vec![Action::new("discover", "pokedex --discover")],
        ).print()?;
        return Ok(());
    }

    if let Some(g) = game {
        validate_game_filter(conn, g, &format!("pokedex location encounters {location}"))?;
    }

    let limit = super::validate_limit(limit)?;
    let (encounters, total) = queries::get_location_encounters(conn, location, game, limit, offset)?;

    if encounters.is_empty() && total == 0 {
        // Check if the location exists but has no encounters for this specific game
        let msg = if game.is_some() {
            let (_all_encounters, all_total) = queries::get_location_encounters(conn, location, None, 1, 0)?;
            if all_total > 0 {
                format!("No encounters at '{location}' in game '{}'. This location has {all_total} encounters in other games.", game.unwrap())
            } else {
                format!("No encounters found for location '{location}'. Try a region prefix (e.g., 'hoenn-{location}', 'kanto-{location}') or a sub-area name.")
            }
        } else {
            format!("No encounters found for location '{location}'. Try a region prefix (e.g., 'hoenn-{location}', 'kanto-{location}') or a sub-area name.")
        };
        let mut actions = vec![
            Action::new("try_dex", "pokedex dex list"),
            Action::new("discover", "pokedex --discover"),
        ];
        if game.is_some() {
            actions.insert(0, Action::new("try_without_game", &format!("pokedex location encounters {location}")));
        }
        ErrorResponse::not_found(&msg, actions).print()?;
        return Ok(());
    }

    let mut cmd = format!("pokedex location encounters {location}");
    if let Some(g) = game {
        cmd.push_str(&format!(" --game={g}"));
    }

    let mut actions = vec![
        Action::new("show", "pokedex pokemon show {species_slug}"),
    ];
    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("{cmd} --limit={limit} --offset={}", offset + limit)));
    }
    if offset > 0 {
        let prev_offset = if offset > total { total.saturating_sub(limit) } else { offset.saturating_sub(limit) };
        actions.push(Action::new("prev_page", &format!("{cmd} --limit={limit} --offset={prev_offset}")));
    }

    let response = Response::new(encounters, actions, Meta::paginated(&cmd, total, limit, offset));
    response.print(format)
}
