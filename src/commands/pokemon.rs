use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;
use super::validate_game_filter;

const VALID_CATEGORIES: &[&str] = &["legendary", "mythical", "baby"];

pub fn list(
    conn: &Connection,
    type_filter: Option<&str>,
    generation: Option<u32>,
    category: Option<&str>,
    limit: u64,
    offset: u64,
    format: &OutputFormat,
) -> Result<()> {
    if let Some(ref t) = type_filter {
        let types = queries::list_types(conn)?;
        if !types.iter().any(|ty| ty.name.eq_ignore_ascii_case(t) || ty.display_name.eq_ignore_ascii_case(t)) {
            ErrorResponse::invalid_parameter(&format!("No type named '{t}'"), vec![Action::new("types", "pokedex type list")]).print()?;
            return Ok(());
        }
    }
    if let Some(g) = generation {
        if !(1..=9).contains(&g) {
            ErrorResponse::invalid_parameter(
                &format!("Invalid generation '{g}'. Valid values: 1-9"),
                vec![Action::new("list", "pokedex pokemon list")],
            ).print()?;
            return Ok(());
        }
    }
    if let Some(ref c) = category {
        if !VALID_CATEGORIES.iter().any(|v| v.eq_ignore_ascii_case(c)) {
            ErrorResponse::invalid_parameter(
                &format!("Invalid category '{c}'. Valid values: {}", VALID_CATEGORIES.join(", ")),
                vec![Action::new("list", "pokedex pokemon list")],
            ).print()?;
        }
    }
    let limit = super::validate_limit(limit)?;
    let (species, total) = queries::list_species(conn, type_filter, generation, category, limit, offset)?;

    let mut cmd_parts = vec!["pokedex pokemon list".to_string()];
    if let Some(t) = type_filter { cmd_parts.push(format!("--type-filter={t}")); }
    if let Some(g) = generation { cmd_parts.push(format!("--generation={g}")); }
    if let Some(c) = category { cmd_parts.push(format!("--category={c}")); }
    let cmd = cmd_parts.join(" ");

    let mut actions = vec![
        Action::new("show", "pokedex pokemon show {name}"),
    ];

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("{cmd} --limit={limit} --offset={}", offset + limit)));
    }
    if offset > 0 {
        let prev_offset = if offset > total { total.saturating_sub(limit) } else { offset.saturating_sub(limit) };
        actions.push(Action::new("prev_page", &format!("{cmd} --limit={limit} --offset={prev_offset}")));
    }

    let response = Response::new(
        species,
        actions,
        Meta::paginated(&cmd, total, limit, offset),
    );
    response.print(format)
}

pub fn show(conn: &Connection, pokemon: &str, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, _name) = match resolved {
        Some(r) => r,
        None => {
            let results = queries::search_species(conn, pokemon, 5)?;
            let mut suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex pokemon show {}", r.species.name))
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

    let mut species = queries::get_species(conn, species_id)?;

    // If the user searched for a form name (e.g. "growlithe-hisui"), override
    // the display_name, types, stats, and abilities with form-specific data.
    if let Ok(Some(form_pokemon_id)) = queries::resolve_form_pokemon_id(conn, pokemon) {
        // Get the default pokemon_id for comparison
        let default_pokemon_id: Option<i64> = conn.query_row(
            "SELECT id FROM pokemon WHERE species_id = ?1 AND is_default = 1",
            rusqlite::params![species_id],
            |row| row.get(0),
        ).ok();

        // Override types
        let form_types = queries::get_pokemon_types_by_pokemon_id(conn, form_pokemon_id)?;
        if !form_types.is_empty() {
            species.types = form_types;
        }

        // Override display_name
        if let Ok(Some(form_display)) = queries::get_form_display_name(conn, form_pokemon_id) {
            species.display_name = form_display;
        }

        // Override stats
        if let Ok(stats) = queries::get_pokemon_stats_by_pokemon_id(conn, form_pokemon_id) {
            species.stats = Some(stats);
        }

        // Override abilities
        if let Ok(abilities) = queries::get_pokemon_abilities_by_id(conn, form_pokemon_id) {
            if !abilities.is_empty() {
                species.abilities = abilities;
            }
        }

        // For cosmetic forms (same pokemon_id as default), get display name from pokemon_forms
        if default_pokemon_id == Some(form_pokemon_id) {
            let cosmetic_display: Option<String> = conn.query_row(
                "SELECT COALESCE(pfn.pokemon_name, pfn.name) FROM pokemon_forms pf \
                 LEFT JOIN pokemon_form_names pfn ON pfn.pokemon_form_id = pf.id \
                 WHERE LOWER(pf.name) = LOWER(?1)",
                rusqlite::params![pokemon],
                |row| row.get(0),
            ).ok().flatten();
            if let Some(display) = cosmetic_display {
                species.display_name = display;
            }
        }
    } else {
        // resolve_form_pokemon_id returned None — could be a cosmetic form where
        // the pokemon_id is the default (is_default=1). Check pokemon_forms directly.
        let cosmetic_display: Option<String> = conn.query_row(
            "SELECT COALESCE(pfn.pokemon_name, pfn.name) FROM pokemon_forms pf \
             LEFT JOIN pokemon_form_names pfn ON pfn.pokemon_form_id = pf.id \
             WHERE LOWER(pf.name) = LOWER(?1) AND pfn.pokemon_name IS NOT NULL",
            rusqlite::params![pokemon],
            |row| row.get(0),
        ).ok().flatten();
        if let Some(display) = cosmetic_display {
            species.display_name = display;
        }
    }

    let mut actions = vec![
        Action::new("evolutions", &format!("pokedex pokemon evolutions {}", species.name)),
        Action::new("forms", &format!("pokedex pokemon forms {}", species.name)),
        Action::new("stats", &format!("pokedex pokemon stats {}", species.name)),
        Action::new("moves", &format!("pokedex pokemon moves {}", species.name)),
        Action::new("encounters", &format!("pokedex pokemon encounters {}", species.name)),
        Action::new("add_to_collection", &format!("pokedex collection add --pokemon={} --game=<game>", species.name)),
    ];
    for type_name in &species.types {
        actions.push(Action::new("type_matchups", &format!("pokedex type matchups {}", type_name.to_lowercase())));
    }
    if let Some(first_type) = species.types.first() {
        actions.push(Action::new("same_type", &format!("pokedex type pokemon {}", first_type.to_lowercase())));
    }

    let response = Response::new(
        species,
        actions,
        Meta::simple(&format!("pokedex pokemon show {pokemon}")),
    );
    response.print(format)
}

pub fn search(conn: &Connection, query: &str, limit: u64, format: &OutputFormat) -> Result<()> {
    if query.trim().is_empty() {
        ErrorResponse::invalid_parameter(
            "Search query is required",
            vec![Action::new("list", "pokedex pokemon list --limit=20")],
        ).print()?;
        return Ok(());
    }

    let results = queries::search_species(conn, query, limit)?;

    let mut actions: Vec<Action> = results.iter().map(|r| {
        Action::new("show", &format!("pokedex pokemon show {}", r.species.name))
    }).collect();
    if actions.is_empty() {
        actions.push(Action::new("list", "pokedex pokemon list --limit=20"));
    }

    let response = Response::new(
        results,
        actions,
        Meta::simple(&format!("pokedex pokemon search {query}")),
    );
    response.print(format)
}

pub fn evolutions(conn: &Connection, pokemon: &str, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, _name) = match resolved {
        Some(r) => r,
        None => {
            let results = queries::search_species(conn, pokemon, 5)?;
            let mut suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex pokemon evolutions {}", r.species.name))
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

    let chain = queries::get_evolution_chain(conn, species_id)?;

    fn collect_species_from_chain(node: &crate::db::models::EvolutionNode, names: &mut Vec<String>) {
        names.push(node.species_name.clone());
        for child in &node.children {
            collect_species_from_chain(child, names);
        }
    }

    let mut chain_species = Vec::new();
    collect_species_from_chain(&chain, &mut chain_species);
    let actions: Vec<Action> = chain_species.iter().map(|name| {
        Action::new("show", &format!("pokedex pokemon show {name}"))
    }).collect();

    let response = Response::new(
        chain,
        actions,
        Meta::simple(&format!("pokedex pokemon evolutions {pokemon}")),
    );
    response.print(format)
}

pub fn forms(conn: &Connection, pokemon: &str, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, name) = match resolved {
        Some(r) => r,
        None => {
            let results = queries::search_species(conn, pokemon, 5)?;
            let mut suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex pokemon forms {}", r.species.name))
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

    let forms = queries::get_pokemon_forms(conn, species_id)?;

    let mut actions = vec![
        Action::new("show", &format!("pokedex pokemon show {name}")),
    ];
    for form in &forms {
        if let Some(ref form_name) = form.form_name {
            actions.push(Action::new(
                "add_form_to_collection",
                &format!("pokedex collection add --pokemon={name} --form={form_name} --game=<game>"),
            ));
        }
    }

    let response = Response::new(
        forms,
        actions,
        Meta::simple(&format!("pokedex pokemon forms {pokemon}")),
    );
    response.print(format)
}

pub fn encounters(conn: &Connection, pokemon: &str, game: Option<&str>, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, name) = match resolved {
        Some(r) => r,
        None => {
            let results = queries::search_species(conn, pokemon, 5)?;
            let mut suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex pokemon encounters {}", r.species.name))
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

    if let Some(g) = game {
        validate_game_filter(conn, g, &format!("pokedex pokemon encounters {name}"))?;
    }

    let encounters = queries::get_encounters(conn, species_id, game)?;

    let mut cmd = format!("pokedex pokemon encounters {name}");
    if let Some(g) = game { cmd.push_str(&format!(" --game={g}")); }

    let mut actions = vec![
        Action::new("show", &format!("pokedex pokemon show {name}")),
        Action::new("add_to_collection", &format!("pokedex collection add --pokemon={name} --game={}", game.unwrap_or("<game>"))),
    ];

    // Suggest filtering by specific games found in encounters (use slug for --game flag)
    let game_slugs: Vec<String> = encounters.iter().map(|e| e.game_slug.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect();
    for g in &game_slugs {
        if game.is_none() {
            actions.push(Action::new("filter_game", &format!("pokedex pokemon encounters {name} --game={g}")));
        }
    }

    let response = Response::new(encounters, actions, Meta::simple(&cmd));
    response.print(format)
}

pub fn moves(conn: &Connection, pokemon: &str, game: Option<&str>, method: Option<&str>, limit: u64, offset: u64, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, name) = match resolved {
        Some(r) => r,
        None => {
            let results = queries::search_species(conn, pokemon, 5)?;
            let mut suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex pokemon moves {}", r.species.name))
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

    if let Some(g) = game {
        validate_game_filter(conn, g, &format!("pokedex pokemon moves {name}"))?;
    }

    // Validate method if provided
    if let Some(m) = method {
        let mut stmt = conn.prepare("SELECT name FROM pokemon_move_methods")?;
        let methods: Vec<String> = stmt.query_map([], |row| row.get(0))?.filter_map(|r| r.ok()).collect();
        if !methods.iter().any(|v| v.eq_ignore_ascii_case(m)) {
            let suggestions: Vec<Action> = methods.iter().map(|vm| {
                Action::new("did_you_mean", &format!("pokedex pokemon moves {name} --method={vm}"))
            }).collect();
            ErrorResponse::invalid_parameter(
                &format!("Invalid method '{m}'. Valid values: {}", methods.join(", ")),
                suggestions,
            ).print()?;
            return Ok(());
        }
    }

    let limit = super::validate_limit(limit)?;
    let (moves, total) = queries::get_pokemon_moves(conn, species_id, game, method, limit, offset)?;

    let mut cmd = format!("pokedex pokemon moves {name}");
    if let Some(g) = game { cmd.push_str(&format!(" --game={g}")); }
    if let Some(m) = method { cmd.push_str(&format!(" --method={m}")); }

    let mut actions = vec![
        Action::new("show", &format!("pokedex pokemon show {name}")),
    ];
    if method.is_none() {
        actions.push(Action::new("filter_level_up", &format!("pokedex pokemon moves {name} --method=level-up")));
        actions.push(Action::new("filter_tm", &format!("pokedex pokemon moves {name} --method=machine")));
        actions.push(Action::new("filter_egg", &format!("pokedex pokemon moves {name} --method=egg")));
        actions.push(Action::new("filter_tutor", &format!("pokedex pokemon moves {name} --method=tutor")));
    }

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("{cmd} --limit={limit} --offset={}", offset + limit)));
    }
    if offset > 0 {
        let prev_offset = if offset > total { total.saturating_sub(limit) } else { offset.saturating_sub(limit) };
        actions.push(Action::new("prev_page", &format!("{cmd} --limit={limit} --offset={prev_offset}")));
    }

    let response = Response::new(moves, actions, Meta::paginated(&cmd, total, limit, offset));
    response.print(format)
}

pub fn stats(conn: &Connection, pokemon: &str, format: &OutputFormat) -> Result<()> {
    let resolved = queries::resolve_pokemon(conn, pokemon)?;
    let (species_id, name) = match resolved {
        Some(r) => r,
        None => {
            let results = queries::search_species(conn, pokemon, 5)?;
            let mut suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex pokemon stats {}", r.species.name))
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

    let stats = queries::get_pokemon_stats(conn, species_id)?;

    let actions = vec![
        Action::new("show", &format!("pokedex pokemon show {name}")),
        Action::new("moves", &format!("pokedex pokemon moves {name}")),
        Action::new("evolutions", &format!("pokedex pokemon evolutions {name}")),
    ];

    let response = Response::new(stats, actions, Meta::simple(&format!("pokedex pokemon stats {pokemon}")));
    response.print(format)
}
