use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;
use super::validate_game_filter;

pub fn list(conn: &Connection, format: &OutputFormat) -> Result<()> {
    let dexes = queries::list_pokedexes(conn)?;

    let actions = vec![
        Action::new("show", "pokedex dex show {name}"),
        Action::new("progress", "pokedex dex progress {name}"),
    ];

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

    let limit = limit.max(1);
    let (entries, total) = queries::get_dex_entries(conn, pokedex_id, limit, offset)?;

    let mut actions = vec![
        Action::new("show", "pokedex pokemon show {name}"),
        Action::new("progress", &format!("pokedex dex progress {dex_name}")),
    ];

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("pokedex dex show {dex_name} --limit={limit} --offset={}", offset + limit)));
    }
    if offset > 0 {
        actions.push(Action::new("prev_page", &format!("pokedex dex show {dex_name} --limit={limit} --offset={}", offset.saturating_sub(limit))));
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

    if let Some(g) = game {
        validate_game_filter(conn, g)?;
    }

    if let Some(s) = status {
        const VALID_STATUSES: &[&str] = &["caught", "living_dex", "evolved", "traded_away", "transferred"];
        if !VALID_STATUSES.contains(&s) {
            let suggestions: Vec<Action> = VALID_STATUSES.iter().map(|vs| {
                Action::new("did_you_mean", &format!("pokedex dex progress {dex} --status={vs}"))
            }).collect();
            ErrorResponse::invalid_parameter(
                &format!("Invalid status '{s}'. Valid values: {}", VALID_STATUSES.join(", ")),
                suggestions,
            ).print()?;
            return Ok(());
        }
    }

    let limit = limit.max(1);
    let (progress, filtered_count) = queries::get_dex_progress(conn, pokedex_id, &dex_name, missing, caught, game, status, limit, offset)?;

    let mut cmd = format!("pokedex dex progress {dex}");
    if missing { cmd.push_str(" --missing"); }
    if caught { cmd.push_str(" --caught"); }
    if let Some(g) = game { cmd.push_str(&format!(" --game={g}")); }
    if let Some(s) = status { cmd.push_str(&format!(" --status={s}")); }

    let mut actions = vec![
        Action::new("show_dex", &format!("pokedex dex show {dex_name}")),
    ];
    if !missing {
        actions.push(Action::new("show_missing", &format!("pokedex dex progress {dex_name} --missing")));
    }
    if !caught {
        actions.push(Action::new("show_caught", &format!("pokedex dex progress {dex_name} --caught")));
    }

    // Template action for entries
    actions.push(Action::new("show", "pokedex pokemon show {name}"));

    if offset + limit < filtered_count {
        actions.push(Action::new("next_page", &format!("{cmd} --limit={limit} --offset={}", offset + limit)));
    }
    if offset > 0 {
        actions.push(Action::new("prev_page", &format!("{cmd} --limit={limit} --offset={}", offset.saturating_sub(limit))));
    }

    let response = Response::new(progress, actions, Meta::paginated(&cmd, filtered_count, limit, offset));
    response.print(format)
}
