use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;

fn validate_type_name(conn: &Connection, type_name: &str, command_template: &str) -> Result<Option<()>> {
    let types = queries::list_types(conn)?;
    let suggestions: Vec<Action> = types.iter()
        .filter(|t| strsim::jaro_winkler(&type_name.to_lowercase(), &t.name.to_lowercase()) > 0.6)
        .map(|t| Action::new("did_you_mean", &format!("{} {}", command_template, t.name)))
        .collect();
    let mut all_suggestions = suggestions;
    if all_suggestions.is_empty() {
        all_suggestions.push(Action::new("list", "pokedex type list"));
    }
    ErrorResponse::not_found(
        &format!("No type named '{type_name}'"),
        all_suggestions,
    ).print()?;
    Ok(None)
}

pub fn list(conn: &Connection, format: &OutputFormat) -> Result<()> {
    let types = queries::list_types(conn)?;

    let mut actions: Vec<Action> = types.iter().map(|t| {
        Action::new("matchups", &format!("pokedex type matchups {}", t.name))
    }).collect();
    actions.push(Action::new("pokemon", "pokedex type pokemon {name}"));

    let response = Response::new(types, actions, Meta::simple("pokedex type list"));
    response.print(format)
}

pub fn matchups(conn: &Connection, type_name: &str, format: &OutputFormat) -> Result<()> {
    let matchups = match queries::get_type_matchups(conn, type_name) {
        Ok(m) => m,
        Err(_) => {
            validate_type_name(conn, type_name, "pokedex type matchups")?;
            return Ok(());
        }
    };

    let mut actions = vec![
        Action::new("pokemon_of_type", &format!("pokedex type pokemon {}", matchups.type_name)),
        Action::new("all_types", "pokedex type list"),
    ];
    for t in &matchups.attacking.super_effective {
        actions.push(Action::new("matchup_detail", &format!("pokedex type matchups {}", t.to_lowercase())));
    }

    let response = Response::new(matchups, actions, Meta::simple(&format!("pokedex type matchups {type_name}")));
    response.print(format)
}

pub fn pokemon_of_type(conn: &Connection, type_name: &str, limit: u64, offset: u64, format: &OutputFormat) -> Result<()> {
    // Validate the type exists before querying
    let types = queries::list_types(conn)?;
    let type_exists = types.iter().any(|t| t.name.to_lowercase() == type_name.to_lowercase());
    if !type_exists {
        validate_type_name(conn, type_name, "pokedex type pokemon")?;
        return Ok(());
    }

    let limit = limit.max(1);
    let (species, total) = queries::list_species(conn, Some(type_name), None, None, limit, offset)?;

    let mut actions = vec![
        Action::new("show", "pokedex pokemon show {name}"),
    ];

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("pokedex type pokemon {type_name} --limit={limit} --offset={}", offset + limit)));
    }
    if offset > 0 {
        actions.push(Action::new("prev_page", &format!("pokedex type pokemon {type_name} --limit={limit} --offset={}", offset.saturating_sub(limit))));
    }
    actions.push(Action::new("matchups", &format!("pokedex type matchups {type_name}")));

    let response = Response::new(
        species,
        actions,
        Meta::paginated(&format!("pokedex type pokemon {type_name}"), total, limit, offset),
    );
    response.print(format)
}
