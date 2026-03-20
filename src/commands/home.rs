use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;

pub fn status(conn: &Connection, format: &OutputFormat) -> Result<()> {
    let status = queries::get_home_status(conn)?;

    let actions = vec![
        Action::new("coverage", "pokedex home coverage"),
        Action::new("missing", "pokedex home missing"),
        Action::new("collection_in_home", "pokedex collection list --in-home"),
    ];

    let response = Response::new(status, actions, Meta::simple("pokedex home status"));
    response.print(format)
}

pub fn transferable(conn: &Connection, pokemon: &str, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, name) = match resolved {
        Some(r) => r,
        None => {
            let err = ErrorResponse::not_found(
                &format!("No pokémon named '{pokemon}'"),
                vec![Action::new("search", &format!("pokedex pokemon search {pokemon}"))],
            );
            err.print()?;
            return Ok(());
        }
    };

    let games = queries::get_home_transferable(conn, species_id)?;

    let actions = vec![
        Action::new("show_pokemon", &format!("pokedex pokemon show {name}")),
        Action::new("add_to_collection", &format!("pokedex collection add --pokemon={name} --game=<game> --in-home")),
    ];

    let response = Response::new(games, actions, Meta::simple(&format!("pokedex home transferable {pokemon}")));
    response.print(format)
}

pub fn missing(conn: &Connection, dex: &str, limit: u64, offset: u64, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokedex(conn, dex)?;
    let (pokedex_id, dex_name) = match resolved {
        Some(r) => r,
        None => {
            // Default to national if the name doesn't resolve
            let r = queries::resolve_pokedex(conn, "national")?;
            match r {
                Some(r) => r,
                None => {
                    let err = ErrorResponse::not_found(
                        &format!("No pokédex named '{dex}'"),
                        vec![Action::new("list_dexes", "pokedex dex list")],
                    );
                    err.print()?;
                    return Ok(());
                }
            }
        }
    };

    let (entries, total) = queries::get_home_missing(conn, pokedex_id, limit, offset)?;

    let cmd = format!("pokedex home missing --dex={dex_name}");

    let mut actions = vec![
        Action::new("show", "pokedex pokemon show {species_name}"),
        Action::new("coverage", "pokedex home coverage"),
    ];

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("{cmd} --limit={limit} --offset={}", offset + limit)));
    }
    if offset > 0 {
        actions.push(Action::new("prev_page", &format!("{cmd} --limit={limit} --offset={}", offset.saturating_sub(limit))));
    }

    let response = Response::new(entries, actions, Meta::paginated(&cmd, total, limit, offset));
    response.print(format)
}

pub fn coverage(conn: &Connection, format: &OutputFormat) -> Result<()> {
    let progress = queries::get_home_coverage(conn)?;

    let actions = vec![
        Action::new("missing", "pokedex home missing"),
        Action::new("status", "pokedex home status"),
        Action::new("dex_progress", "pokedex dex progress national"),
    ];

    let response = Response::new(progress, actions, Meta::simple("pokedex home coverage"));
    response.print(format)
}
