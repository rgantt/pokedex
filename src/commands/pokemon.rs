use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;

pub fn list(
    conn: &Connection,
    type_filter: Option<&str>,
    generation: Option<u32>,
    category: Option<&str>,
    limit: u64,
    offset: u64,
    format: &OutputFormat,
) -> Result<()> {
    let (species, total) = queries::list_species(conn, type_filter, generation, category, limit, offset)?;

    let mut cmd_parts = vec!["pokedex pokemon list".to_string()];
    if let Some(t) = type_filter { cmd_parts.push(format!("--type={t}")); }
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
        actions.push(Action::new("prev_page", &format!("{cmd} --limit={limit} --offset={}", offset.saturating_sub(limit))));
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
            let suggestions: Vec<Action> = results.iter().map(|r| {
                Action::new("did_you_mean", &format!("pokedex pokemon show {}", r.species.name))
            }).collect();
            let err = ErrorResponse::not_found(
                &format!("No pokémon named '{pokemon}'"),
                suggestions,
            );
            err.print()?;
            return Ok(());
        }
    };

    let species = queries::get_species(conn, species_id)?;

    let actions = vec![
        Action::new("evolutions", &format!("pokedex pokemon evolutions {}", species.name)),
        Action::new("forms", &format!("pokedex pokemon forms {}", species.name)),
        Action::new("stats", &format!("pokedex pokemon stats {}", species.name)),
        Action::new("moves", &format!("pokedex pokemon moves {}", species.name)),
        Action::new("encounters", &format!("pokedex pokemon encounters {}", species.name)),
        Action::new("add_to_collection", &format!("pokedex collection add --pokemon={} --game=<game>", species.name)),
        Action::new("type_matchups", &format!("pokedex type matchups {}", species.types.first().map(|s| s.as_str()).unwrap_or("normal"))),
        Action::new("same_type", &format!("pokedex type pokemon {}", species.types.first().map(|s| s.as_str()).unwrap_or("normal"))),
    ];

    let response = Response::new(
        species,
        actions,
        Meta::simple(&format!("pokedex pokemon show {pokemon}")),
    );
    response.print(format)
}

pub fn search(conn: &Connection, query: &str, limit: u64, format: &OutputFormat) -> Result<()> {
    let results = queries::search_species(conn, query, limit)?;

    let actions: Vec<Action> = results.iter().map(|r| {
        Action::new("show", &format!("pokedex pokemon show {}", r.species.name))
    }).collect();

    let response = Response::new(
        results,
        actions,
        Meta::simple(&format!("pokedex pokemon search {query}")),
    );
    response.print(format)
}

pub fn evolutions(conn: &Connection, pokemon: &str, format: &OutputFormat) -> Result<()> {
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

    let chain = queries::get_evolution_chain(conn, species_id)?;

    let actions = vec![
        Action::new("show", &format!("pokedex pokemon show {name}")),
    ];

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
            let err = ErrorResponse::not_found(
                &format!("No pokémon named '{pokemon}'"),
                vec![Action::new("search", &format!("pokedex pokemon search {pokemon}"))],
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
            let err = ErrorResponse::not_found(
                &format!("No pokémon named '{pokemon}'"),
                vec![Action::new("search", &format!("pokedex pokemon search {pokemon}"))],
            );
            err.print()?;
            return Ok(());
        }
    };

    let encounters = queries::get_encounters(conn, species_id, game)?;

    let mut cmd = format!("pokedex pokemon encounters {name}");
    if let Some(g) = game { cmd.push_str(&format!(" --game={g}")); }

    let mut actions = vec![
        Action::new("show", &format!("pokedex pokemon show {name}")),
        Action::new("add_to_collection", &format!("pokedex collection add --pokemon={name} --game=<game>")),
    ];

    // Suggest filtering by specific games found in encounters
    let games: Vec<String> = encounters.iter().map(|e| e.game.clone()).collect::<std::collections::HashSet<_>>().into_iter().collect();
    for g in &games {
        if game.is_none() {
            actions.push(Action::new("filter_game", &format!("pokedex pokemon encounters {name} --game={g}")));
        }
    }

    let response = Response::new(encounters, actions, Meta::simple(&cmd));
    response.print(format)
}

pub fn moves(conn: &Connection, pokemon: &str, game: Option<&str>, method: Option<&str>, format: &OutputFormat) -> Result<()> {
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

    let moves = queries::get_pokemon_moves(conn, species_id, game, method)?;

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

    let response = Response::new(moves, actions, Meta::simple(&cmd));
    response.print(format)
}

pub fn stats(conn: &Connection, pokemon: &str, format: &OutputFormat) -> Result<()> {
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

    let stats = queries::get_pokemon_stats(conn, species_id)?;

    let actions = vec![
        Action::new("show", &format!("pokedex pokemon show {name}")),
        Action::new("moves", &format!("pokedex pokemon moves {name}")),
        Action::new("evolutions", &format!("pokedex pokemon evolutions {name}")),
    ];

    let response = Response::new(stats, actions, Meta::simple(&format!("pokedex pokemon stats {pokemon}")));
    response.print(format)
}
