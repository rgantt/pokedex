use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::db::queries;
use crate::output::*;

pub fn add(
    conn: &Connection,
    pokemon: &str,
    game: &str,
    form: Option<&str>,
    shiny: bool,
    in_home: bool,
    status: &str,
    method: Option<&str>,
    nickname: Option<&str>,
    notes: Option<&str>,
    dry_run: bool,
    format: &OutputFormat,
) -> Result<()> {
    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, species_name) = match resolved {
        Some(r) => r,
        None => {
            let results = queries::search_species(conn, pokemon, 5)?;
            let suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex collection add --pokemon={} --game={game}", r.species.name))
            }).collect();
            let err = ErrorResponse::not_found(
                &format!("No pokémon named '{pokemon}'"),
                suggestions,
            );
            err.print()?;
            return Ok(());
        }
    };

    let game_resolved = queries::resolve_game(conn, game)?;
    let (game_id, game_name) = match game_resolved {
        Some(r) => r,
        None => {
            let all = queries::list_games(conn, false)?;
            let suggestions: Vec<Action> = all.iter().map(|g| {
                Action::new("did_you_mean", &format!("pokedex collection add --pokemon={species_name} --game={}", g.name))
            }).collect();
            let err = ErrorResponse::not_found(
                &format!("No game named '{game}'"),
                suggestions,
            );
            err.print()?;
            return Ok(());
        }
    };

    // Resolve form if provided
    let form_id = if let Some(form_name) = form {
        let fid: Option<i64> = conn.query_row(
            "SELECT pf.id FROM pokemon_forms pf \
             JOIN pokemon p ON p.id = pf.pokemon_id \
             WHERE p.species_id = ?1 AND LOWER(pf.form_name) = LOWER(?2)",
            rusqlite::params![species_id, form_name],
            |row| row.get(0),
        ).ok();
        fid
    } else {
        None
    };

    #[derive(Serialize)]
    struct AddPreview {
        pokemon: String,
        game: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        form: Option<String>,
        shiny: bool,
        in_home: bool,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        method: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
        dry_run: bool,
    }

    if dry_run {
        let preview = AddPreview {
            pokemon: species_name.clone(),
            game: game_name.clone(),
            form: form.map(|s| s.to_string()),
            shiny,
            in_home,
            status: status.to_string(),
            method: method.map(|s| s.to_string()),
            nickname: nickname.map(|s| s.to_string()),
            dry_run: true,
        };
        let status_flag = if status != "caught" { format!(" --status={status}") } else { String::new() };
        let method_flag = method.map(|m| format!(" --method={m}")).unwrap_or_default();
        let nickname_flag = nickname.map(|n| format!(" --nickname={n}")).unwrap_or_default();
        let shiny_flag = if shiny { " --shiny" } else { "" };
        let home_flag = if in_home { " --in-home" } else { "" };
        let actions = vec![
            Action::with_description("confirm", &format!(
                "pokedex collection add --pokemon={species_name} --game={game_name}{shiny_flag}{home_flag}{status_flag}{method_flag}{nickname_flag}",
            ), "Run without --dry-run to save"),
        ];
        let response = Response::new(preview, actions, Meta::simple("pokedex collection add --dry-run"));
        return response.print(format);
    }

    let id = queries::add_collection_entry(
        conn, species_id, form_id, game_id, shiny, in_home, status, method, nickname, notes,
    )?;

    let entry = queries::get_collection_entry(conn, id)?.unwrap();

    let actions = vec![
        Action::new("show", &format!("pokedex collection show {id}")),
        Action::new("list", "pokedex collection list"),
        Action::new("stats", "pokedex collection stats"),
        Action::new("pokemon_info", &format!("pokedex pokemon show {species_name}")),
    ];

    let response = Response::new(entry, actions, Meta::simple(&format!("pokedex collection add --pokemon={pokemon} --game={game}")));
    response.print(format)
}

pub fn remove(conn: &Connection, id: i64, dry_run: bool, format: &OutputFormat) -> Result<()> {
    let entry = queries::get_collection_entry(conn, id)?;
    if entry.is_none() {
        let err = ErrorResponse::not_found(
            &format!("No collection entry with id {id}"),
            vec![Action::new("list", "pokedex collection list")],
        );
        err.print()?;
        return Ok(());
    }

    if dry_run {
        let response = Response::new(
            entry.unwrap(),
            vec![Action::with_description("confirm", &format!("pokedex collection remove {id}"), "Run without --dry-run to delete")],
            Meta::simple(&format!("pokedex collection remove {id} --dry-run")),
        );
        return response.print(format);
    }

    queries::remove_collection_entry(conn, id)?;

    #[derive(Serialize)]
    struct Removed { id: i64, removed: bool }

    let response = Response::new(
        Removed { id, removed: true },
        vec![Action::new("list", "pokedex collection list")],
        Meta::simple(&format!("pokedex collection remove {id}")),
    );
    response.print(format)
}

pub fn update(
    conn: &Connection,
    id: i64,
    status: Option<&str>,
    in_home: Option<bool>,
    shiny: Option<bool>,
    nickname: Option<&str>,
    notes: Option<&str>,
    format: &OutputFormat,
) -> Result<()> {
    let existing = queries::get_collection_entry(conn, id)?;
    if existing.is_none() {
        let err = ErrorResponse::not_found(
            &format!("No collection entry with id {id}"),
            vec![Action::new("list", "pokedex collection list")],
        );
        err.print()?;
        return Ok(());
    }

    queries::update_collection_entry(conn, id, status, in_home, shiny, nickname, notes)?;

    let entry = queries::get_collection_entry(conn, id)?.unwrap();

    let actions = vec![
        Action::new("show", &format!("pokedex collection show {id}")),
        Action::new("list", "pokedex collection list"),
    ];

    let response = Response::new(entry, actions, Meta::simple(&format!("pokedex collection update {id}")));
    response.print(format)
}

pub fn list_entries(
    conn: &Connection,
    game: Option<&str>,
    pokemon: Option<&str>,
    shiny_only: bool,
    in_home: bool,
    status: Option<&str>,
    limit: u64,
    offset: u64,
    format: &OutputFormat,
) -> Result<()> {
    let (entries, total) = queries::list_collection(conn, game, pokemon, shiny_only, in_home, status, limit, offset)?;

    let mut actions: Vec<Action> = entries.iter().map(|e| {
        Action::new("show", &format!("pokedex collection show {}", e.id))
    }).collect();

    let mut cmd = "pokedex collection list".to_string();
    if let Some(g) = game { cmd.push_str(&format!(" --game={g}")); }
    if let Some(p) = pokemon { cmd.push_str(&format!(" --pokemon={p}")); }
    if shiny_only { cmd.push_str(" --shiny-only"); }
    if in_home { cmd.push_str(" --in-home"); }
    if let Some(s) = status { cmd.push_str(&format!(" --status={s}")); }

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("{cmd} --limit={limit} --offset={}", offset + limit)));
    }
    actions.push(Action::new("stats", "pokedex collection stats"));
    actions.push(Action::new("add", "pokedex collection add --pokemon=<name> --game=<game>"));

    let response = Response::new(entries, actions, Meta::paginated(&cmd, total, limit, offset));
    response.print(format)
}

pub fn show_entry(conn: &Connection, id: i64, format: &OutputFormat) -> Result<()> {
    let entry = queries::get_collection_entry(conn, id)?;
    match entry {
        Some(e) => {
            let actions = vec![
                Action::new("update", &format!("pokedex collection update {id} --status=<status>")),
                Action::new("remove", &format!("pokedex collection remove {id}")),
                Action::new("pokemon_info", &format!("pokedex pokemon show {}", e.species_name)),
                Action::new("list", "pokedex collection list"),
            ];
            let response = Response::new(e, actions, Meta::simple(&format!("pokedex collection show {id}")));
            response.print(format)
        }
        None => {
            let err = ErrorResponse::not_found(
                &format!("No collection entry with id {id}"),
                vec![Action::new("list", "pokedex collection list")],
            );
            err.print()
        }
    }
}

pub fn stats(conn: &Connection, format: &OutputFormat) -> Result<()> {
    let stats = queries::get_collection_stats(conn)?;

    let actions = vec![
        Action::new("list", "pokedex collection list"),
        Action::new("home_status", "pokedex home status"),
        Action::new("home_coverage", "pokedex home coverage"),
        Action::new("dex_progress_national", "pokedex dex progress national"),
    ];

    let response = Response::new(stats, actions, Meta::simple("pokedex collection stats"));
    response.print(format)
}
