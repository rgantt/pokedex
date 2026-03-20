use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;

pub fn list(conn: &Connection, format: &OutputFormat) -> Result<()> {
    let dexes = queries::list_pokedexes(conn)?;

    let actions: Vec<Action> = dexes.iter().map(|d| {
        Action::new("show", &format!("pokedex dex show {}", d.name))
    }).collect();

    let response = Response::new(dexes, actions, Meta::simple("pokedex dex list"));
    response.print(format)
}

pub fn show(conn: &Connection, dex: &str, limit: u64, offset: u64, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokedex(conn, dex)?;
    let (pokedex_id, dex_name) = match resolved {
        Some(r) => r,
        None => {
            let err = ErrorResponse::not_found(
                &format!("No pokédex named '{dex}'"),
                vec![Action::new("list", "pokedex dex list")],
            );
            err.print()?;
            return Ok(());
        }
    };

    let (entries, total) = queries::get_dex_entries(conn, pokedex_id, limit, offset)?;

    let mut actions: Vec<Action> = entries.iter().map(|e| {
        Action::new("show_pokemon", &format!("pokedex pokemon show {}", e.species_name))
    }).collect();

    actions.push(Action::new("progress", &format!("pokedex dex progress {dex_name}")));

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("pokedex dex show {dex_name} --limit={limit} --offset={}", offset + limit)));
    }

    let response = Response::new(
        entries,
        actions,
        Meta::paginated(&format!("pokedex dex show {dex}"), total, limit, offset),
    );
    response.print(format)
}

pub fn progress(
    conn: &Connection,
    dex: &str,
    missing: bool,
    caught: bool,
    game: Option<&str>,
    status: Option<&str>,
    limit: u64,
    offset: u64,
    format: &OutputFormat,
) -> Result<()> {
    let resolved = queries::resolve_pokedex(conn, dex)?;
    let (pokedex_id, dex_name) = match resolved {
        Some(r) => r,
        None => {
            let err = ErrorResponse::not_found(
                &format!("No pokédex named '{dex}'"),
                vec![Action::new("list", "pokedex dex list")],
            );
            err.print()?;
            return Ok(());
        }
    };

    let progress = queries::get_dex_progress(conn, pokedex_id, &dex_name, missing, caught, game, status, limit, offset)?;

    let mut actions = vec![
        Action::new("show_dex", &format!("pokedex dex show {dex_name}")),
    ];
    if !missing {
        actions.push(Action::new("show_missing", &format!("pokedex dex progress {dex_name} --missing")));
    }
    if !caught {
        actions.push(Action::new("show_caught", &format!("pokedex dex progress {dex_name} --caught")));
    }

    for entry in &progress.entries {
        if !entry.caught {
            actions.push(Action::new("show_pokemon", &format!("pokedex pokemon show {}", entry.species_name)));
        }
    }

    let mut cmd = format!("pokedex dex progress {dex}");
    if missing { cmd.push_str(" --missing"); }
    if caught { cmd.push_str(" --caught"); }
    if let Some(g) = game { cmd.push_str(&format!(" --game={g}")); }
    if let Some(s) = status { cmd.push_str(&format!(" --status={s}")); }

    let response = Response::new(progress, actions, Meta::simple(&cmd));
    response.print(format)
}
