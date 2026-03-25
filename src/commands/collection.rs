use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;

use crate::db::queries;
use crate::output::*;
use super::validate_game_filter;

const VALID_STATUSES: &[&str] = &["caught", "living_dex", "evolved", "traded_away", "transferred"];
const VALID_METHODS: &[&str] = &["catch", "breed", "trade", "transfer", "gift", "raid", "research", "evolve"];

pub fn add(
    conn: &Connection,
    pokemon: &str,
    game: &str,
    form: Option<&str>,
    shiny: bool,
    in_home: bool,
    is_alpha: bool,
    status: &str,
    method: Option<&str>,
    nickname: Option<&str>,
    notes: Option<&str>,
    dry_run: bool,
    format: &OutputFormat,
) -> Result<()> {
    let form_flag = form.map(|f| format!(" --form={f}")).unwrap_or_default();

    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, species_name) = match resolved {
        Some(r) => r,
        None => {
            let results = queries::search_species(conn, pokemon, 5)?;
            let mut suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex collection add --pokemon={} --game={game}{form_flag}", r.species.name))
            }).collect();
            if suggestions.is_empty() {
                suggestions.push(Action::new("search", &format!("pokedex pokemon search {pokemon}")));
                suggestions.push(Action::new("list", "pokedex pokemon list --limit=20"));
            }
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
            let mut scored: Vec<_> = all.iter()
                .map(|g| (strsim::jaro_winkler(&game.to_lowercase(), &g.name.to_lowercase()), g))
                .filter(|(score, _)| *score > 0.6)
                .collect();
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(5);
            let suggestions: Vec<Action> = if scored.is_empty() {
                vec![Action::new("list", "pokedex game list")]
            } else {
                scored.iter().map(|(_, g)| {
                    Action::new("did_you_mean", &format!("pokedex collection add --pokemon={species_name} --game={}{form_flag}", g.name))
                }).collect()
            };
            let err = ErrorResponse::not_found(
                &format!("No game named '{game}'"),
                suggestions,
            );
            err.print()?;
            return Ok(());
        }
    };

    // Validate status with full command context
    if !VALID_STATUSES.contains(&status) {
        let suggestions: Vec<Action> = VALID_STATUSES.iter().map(|s| {
            Action::new("did_you_mean", &format!(
                "pokedex collection add --pokemon={} --game={} --status={s}", species_name, game_name
            ))
        }).collect();
        ErrorResponse::invalid_parameter(
            &format!("Invalid status '{status}'. Valid values: {}", VALID_STATUSES.join(", ")),
            suggestions,
        ).print()?;
        return Ok(());
    }

    // Validate method with full command context
    if let Some(m) = method
        && !VALID_METHODS.contains(&m) {
            let suggestions: Vec<Action> = VALID_METHODS.iter().map(|vm| {
                Action::new("did_you_mean", &format!(
                    "pokedex collection add --pokemon={} --game={} --method={vm}", species_name, game_name
                ))
            }).collect();
            ErrorResponse::invalid_parameter(
                &format!("Invalid method '{m}'. Valid values: {}", VALID_METHODS.join(", ")),
                suggestions,
            ).print()?;
            return Ok(());
        }

    // Auto-detect form from pokemon name when no explicit --form is given
    let auto_form = if form.is_none() && pokemon.to_lowercase() != species_name.to_lowercase() {
        pokemon.to_lowercase()
            .strip_prefix(&species_name.to_lowercase())
            .map(|s| s.trim_start_matches('-').to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    };
    let effective_form: Option<String> = form.map(|s| s.to_string()).or(auto_form);

    // Resolve form if provided or auto-detected
    let form_id = if let Some(ref form_name) = effective_form {
        let fid: Option<i64> = conn.query_row(
            "SELECT pf.id FROM pokemon_forms pf \
             JOIN pokemon p ON p.id = pf.pokemon_id \
             WHERE p.species_id = ?1 AND (LOWER(pf.form_name) = LOWER(?2) OR LOWER(pf.name) = LOWER(?3))",
            rusqlite::params![species_id, form_name, format!("{}-{}", species_name, form_name)],
            |row| row.get(0),
        ).ok();
        fid
    } else {
        None
    };

    if effective_form.is_some() && form_id.is_none() {
        let forms = queries::get_pokemon_forms(conn, species_id)?;
        let form_names: Vec<String> = forms.iter()
            .filter_map(|f| f.form_name.clone())
            .collect();
        let forms_msg = if form_names.is_empty() {
            format!("{species_name} has no alternate forms")
        } else {
            format!("Available forms: {}", form_names.join(", "))
        };
        let err = ErrorResponse::not_found(
            &format!("No form '{}' for {species_name}. {forms_msg}", effective_form.as_deref().unwrap()),
            vec![Action::new("forms", &format!("pokedex pokemon forms {species_name}"))],
        );
        err.print()?;
        return Ok(());
    }

    // Validate --alpha flag: only valid for Legends games
    let alpha_warning = if is_alpha && !["legends-arceus", "legends-za"].contains(&game_name.as_str()) {
        Some(format!("Alpha Pokémon only exist in Legends: Arceus and Legends: Z-A, not in {game_name}. The --alpha flag will be saved but may be incorrect."))
    } else {
        None
    };

    // C9: Check if species has encounters in this game's versions
    // Suppress encounter warning for non-wild acquisition methods
    let encounter_warning = {
        let w = if method.map(|m| ["evolve", "breed", "trade", "transfer", "gift"].contains(&m)).unwrap_or(false) {
            None  // Don't warn for non-wild acquisition methods
        } else {
            check_species_in_game(conn, species_id, game_id)
        };

        // Warn if --in-home is set but the game doesn't connect to HOME
        let home_warning = if in_home {
            let connects: bool = conn.query_row(
                "SELECT connects_to_home FROM games WHERE id = ?1",
                rusqlite::params![game_id],
                |row| row.get::<_, i64>(0).map(|v| v != 0),
            ).unwrap_or(false);
            if !connects {
                Some(format!("{game_name} does not connect to Pokémon HOME. The --in-home flag may not be accurate."))
            } else { None }
        } else { None };

        match (w, alpha_warning, home_warning) {
            (Some(ew), Some(aw), Some(hw)) => Some(format!("{ew} {aw} {hw}")),
            (Some(ew), Some(aw), None) => Some(format!("{ew} {aw}")),
            (Some(ew), None, Some(hw)) => Some(format!("{ew} {hw}")),
            (None, Some(aw), Some(hw)) => Some(format!("{aw} {hw}")),
            (Some(ew), None, None) => Some(ew),
            (None, Some(aw), None) => Some(aw),
            (None, None, Some(hw)) => Some(hw),
            (None, None, None) => None,
        }
    };

    #[derive(Serialize)]
    struct AddPreview {
        pokemon: String,
        game: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        form: Option<String>,
        shiny: bool,
        in_home: bool,
        is_alpha: bool,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        method: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        nickname: Option<String>,
        dry_run: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        warning: Option<String>,
    }

    if dry_run {
        let preview = AddPreview {
            pokemon: species_name.clone(),
            game: game_name.clone(),
            form: effective_form.clone(),
            shiny,
            in_home,
            is_alpha,
            status: status.to_string(),
            method: method.map(|s| s.to_string()),
            nickname: nickname.map(|s| s.to_string()),
            dry_run: true,
            warning: encounter_warning.clone(),
        };
        let form_flag_str = effective_form.as_ref().map(|f| format!(" --form={f}")).unwrap_or_default();
        let status_flag = if status != "caught" { format!(" --status={status}") } else { String::new() };
        let method_flag = method.map(|m| format!(" --method={m}")).unwrap_or_default();
        let nickname_flag = nickname.map(|n| format!(" --nickname={n}")).unwrap_or_default();
        let shiny_flag = if shiny { " --shiny" } else { "" };
        let home_flag = if in_home { " --in-home" } else { "" };
        let alpha_flag = if is_alpha { " --alpha" } else { "" };
        let actions = vec![
            Action::with_description("confirm", &format!(
                "pokedex collection add --pokemon={species_name} --game={game_name}{form_flag_str}{shiny_flag}{home_flag}{alpha_flag}{status_flag}{method_flag}{nickname_flag}",
            ), "Run without --dry-run to save"),
        ];
        let response = Response::new(preview, actions, Meta::simple("pokedex collection add --dry-run"));
        return response.print(format);
    }

    let id = queries::add_collection_entry(
        conn, species_id, form_id, game_id, shiny, in_home, is_alpha, status, method, nickname, notes,
    )?;

    let entry = queries::get_collection_entry(conn, id)?.unwrap();

    #[derive(Serialize)]
    struct AddResult {
        #[serde(flatten)]
        entry: crate::db::models::CollectionEntry,
        #[serde(skip_serializing_if = "Option::is_none")]
        warning: Option<String>,
    }

    let result = AddResult {
        entry,
        warning: encounter_warning,
    };

    let pokemon_slug = if let Some(ref f) = effective_form {
        format!("{species_name}-{f}")
    } else {
        species_name.to_string()
    };
    let actions = vec![
        Action::new("show", &format!("pokedex collection show {id}")),
        Action::new("list", "pokedex collection list"),
        Action::new("stats", "pokedex collection stats"),
        Action::new("pokemon_info", &format!("pokedex pokemon show {pokemon_slug}")),
    ];

    let mut meta_cmd = format!("pokedex collection add --pokemon={pokemon} --game={game}");
    if let Some(ref f) = effective_form { meta_cmd.push_str(&format!(" --form={f}")); }
    let response = Response::new(result, actions, Meta::simple(&meta_cmd));
    response.print(format)
}

/// C9: Check if a species has any encounters in the versions belonging to a game
fn check_species_in_game(conn: &Connection, species_id: i64, game_id: i64) -> Option<String> {
    // Get the default pokemon_id for encounters check
    let pokemon_id: Option<i64> = conn.query_row(
        "SELECT id FROM pokemon WHERE species_id = ?1 AND is_default = 1",
        rusqlite::params![species_id],
        |row| row.get(0),
    ).ok();

    let pokemon_id = pokemon_id?;

    // Check if the game has a version_group_id and if any encounters exist
    let has_encounter: bool = conn.query_row(
        "SELECT EXISTS( \
         SELECT 1 FROM encounters e \
         JOIN versions v ON v.id = e.version_id \
         JOIN games g ON g.version_group_id = v.version_group_id \
         WHERE e.pokemon_id = ?1 AND g.id = ?2 \
         )",
        rusqlite::params![pokemon_id, game_id],
        |row| row.get::<_, i64>(0).map(|v| v != 0),
    ).unwrap_or(true); // default to true (no warning) if query fails

    if has_encounter {
        None
    } else {
        let game_name: String = conn.query_row(
            "SELECT name FROM games WHERE id = ?1",
            rusqlite::params![game_id],
            |row| row.get(0),
        ).unwrap_or_else(|_| "unknown".to_string());
        let species_name: String = conn.query_row(
            "SELECT name FROM species WHERE id = ?1",
            rusqlite::params![species_id],
            |row| row.get(0),
        ).unwrap_or_else(|_| "unknown".to_string());
        Some(format!("No encounter data found for {species_name} in {game_name}. The Pokémon may not be obtainable in this game."))
    }
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
        #[derive(Serialize)]
        struct DryRunRemove {
            dry_run: bool,
            #[serde(flatten)]
            entry: crate::db::models::CollectionEntry,
        }
        let preview = DryRunRemove { dry_run: true, entry: entry.unwrap() };
        let response = Response::new(
            preview,
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
    game: Option<&str>,
    method: Option<&str>,
    dry_run: bool,
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

    // Validate status if provided
    if let Some(s) = status
        && !VALID_STATUSES.contains(&s) {
            let suggestions: Vec<Action> = VALID_STATUSES.iter().map(|vs| {
                Action::new("did_you_mean", &format!("pokedex collection update {id} --status={vs}"))
            }).collect();
            ErrorResponse::invalid_parameter(
                &format!("Invalid status '{s}'. Valid values: {}", VALID_STATUSES.join(", ")),
                suggestions,
            ).print()?;
            return Ok(());
        }

    // Validate method if provided
    if let Some(m) = method
        && !VALID_METHODS.contains(&m) {
            let suggestions: Vec<Action> = VALID_METHODS.iter().map(|vm| {
                Action::new("did_you_mean", &format!("pokedex collection update {id} --method={vm}"))
            }).collect();
            ErrorResponse::invalid_parameter(
                &format!("Invalid method '{m}'. Valid values: {}", VALID_METHODS.join(", ")),
                suggestions,
            ).print()?;
            return Ok(());
        }

    // Resolve game name to game_id if provided
    let game_id = if let Some(game_name) = game {
        let resolved = queries::resolve_game(conn, game_name)?;
        match resolved {
            Some((gid, _)) => Some(gid),
            None => {
                let all = queries::list_games(conn, false)?;
                let suggestions: Vec<Action> = all.iter().map(|g| {
                    Action::new("did_you_mean", &format!("pokedex collection update {id} --game={}", g.name))
                }).collect();
                let err = ErrorResponse::not_found(
                    &format!("No game named '{game_name}'"),
                    suggestions,
                );
                err.print()?;
                return Ok(());
            }
        }
    } else {
        None
    };

    if dry_run {
        let entry = queries::get_collection_entry(conn, id)?.unwrap();
        #[derive(Serialize)]
        struct DryRunPreview {
            dry_run: bool,
            id: i64,
            current: crate::db::models::CollectionEntry,
            #[serde(skip_serializing_if = "Option::is_none")]
            new_status: Option<String>,
            #[serde(skip_serializing_if = "Option::is_none")]
            new_in_home: Option<bool>,
            #[serde(skip_serializing_if = "Option::is_none")]
            new_shiny: Option<bool>,
        }
        let preview = DryRunPreview {
            dry_run: true,
            id,
            current: entry,
            new_status: status.map(|s| s.to_string()),
            new_in_home: in_home,
            new_shiny: shiny,
        };
        let mut cmd = format!("pokedex collection update {id}");
        if let Some(s) = status { cmd.push_str(&format!(" --status={s}")); }
        if let Some(ih) = in_home { cmd.push_str(&format!(" --in-home={ih}")); }
        if let Some(sh) = shiny { cmd.push_str(&format!(" --shiny={sh}")); }
        if let Some(n) = nickname { cmd.push_str(&format!(" --nickname={n}")); }
        if let Some(g) = game { cmd.push_str(&format!(" --game={g}")); }
        if let Some(m) = method { cmd.push_str(&format!(" --method={m}")); }
        let actions = vec![
            Action::new("confirm", &cmd),
            Action::new("show", &format!("pokedex collection show {id}")),
        ];
        let response = Response::new(preview, actions, Meta::simple(&format!("pokedex collection update {id} --dry-run")));
        return response.print(format);
    }

    queries::update_collection_entry(conn, id, status, in_home, shiny, nickname, notes, game_id, method)?;

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
    sort: &str,
    format: &OutputFormat,
) -> Result<()> {
    if let Some(g) = game {
        validate_game_filter(conn, g, "pokedex collection list")?;
    }

    if let Some(s) = status
        && !VALID_STATUSES.contains(&s) {
            let suggestions: Vec<Action> = VALID_STATUSES.iter().map(|vs| {
                Action::new("did_you_mean", &format!("pokedex collection list --status={vs}"))
            }).collect();
            ErrorResponse::invalid_parameter(
                &format!("Invalid status '{s}'. Valid values: {}", VALID_STATUSES.join(", ")),
                suggestions,
            ).print()?;
            return Ok(());
        }

    let valid_sorts = ["id", "dex"];
    if !valid_sorts.contains(&sort) {
        ErrorResponse::invalid_parameter(
            &format!("Invalid sort '{sort}'. Valid values: {}", valid_sorts.join(", ")),
            vec![Action::new("list", "pokedex collection list")],
        ).print()?;
        return Ok(());
    }

    let limit = super::validate_limit(limit)?;
    let (entries, total) = queries::list_collection(conn, game, pokemon, shiny_only, in_home, status, limit, offset, sort)?;

    let mut actions = vec![
        Action::new("show", "pokedex collection show {id}"),
    ];

    let mut cmd = "pokedex collection list".to_string();
    if let Some(g) = game { cmd.push_str(&format!(" --game={g}")); }
    if let Some(p) = pokemon { cmd.push_str(&format!(" --pokemon={p}")); }
    if shiny_only { cmd.push_str(" --shiny-only"); }
    if in_home { cmd.push_str(" --in-home"); }
    if let Some(s) = status { cmd.push_str(&format!(" --status={s}")); }
    if sort != "id" { cmd.push_str(&format!(" --sort={sort}")); }

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("{cmd} --limit={limit} --offset={}", offset + limit)));
    }
    if offset > 0 {
        let prev_offset = if offset > total { total.saturating_sub(limit) } else { offset.saturating_sub(limit) };
        actions.push(Action::new("prev_page", &format!("{cmd} --limit={limit} --offset={prev_offset}")));
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
            let pokemon_slug = if let Some(ref f) = e.form_name {
                format!("{}-{f}", e.species_name)
            } else {
                e.species_name.clone()
            };
            let actions = vec![
                Action::new("update", &format!("pokedex collection update {id} --status=<status>")),
                Action::new("remove", &format!("pokedex collection remove {id}")),
                Action::new("pokemon_info", &format!("pokedex pokemon show {pokemon_slug}")),
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

pub fn stats(conn: &Connection, game: Option<&str>, format: &OutputFormat) -> Result<()> {
    if let Some(g) = game {
        validate_game_filter(conn, g, "pokedex collection stats")?;
    }

    let stats = queries::get_collection_stats(conn, game)?;

    let mut actions = vec![
        Action::new("list", "pokedex collection list"),
        Action::new("home_status", "pokedex home status"),
        Action::new("home_coverage", "pokedex home coverage"),
        Action::new("dex_progress_national", "pokedex dex progress national"),
    ];

    if let Some(g) = game {
        actions.push(Action::new("collection_for_game", &format!("pokedex collection list --game={g}")));
    }

    let cmd = if let Some(g) = game {
        format!("pokedex collection stats --game={g}")
    } else {
        "pokedex collection stats".to_string()
    };

    let response = Response::new(stats, actions, Meta::simple(&cmd));
    response.print(format)
}
