use anyhow::Result;
use rusqlite::Connection;

use crate::db::queries;
use crate::output::*;

pub fn list(conn: &Connection, format: &OutputFormat) -> Result<()> {
    let types = queries::list_types(conn)?;

    let actions: Vec<Action> = types.iter().map(|t| {
        Action::new("matchups", &format!("pokedex type matchups {}", t.name))
    }).collect();

    let response = Response::new(types, actions, Meta::simple("pokedex type list"));
    response.print(format)
}

pub fn matchups(conn: &Connection, type_name: &str, format: &OutputFormat) -> Result<()> {
    let matchups = queries::get_type_matchups(conn, type_name)?;

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
    let (species, total) = queries::list_species(conn, Some(type_name), None, None, limit, offset)?;

    let mut actions: Vec<Action> = species.iter().map(|s| {
        Action::new("show", &format!("pokedex pokemon show {}", s.name))
    }).collect();

    if offset + limit < total {
        actions.push(Action::new("next_page", &format!("pokedex type pokemon {type_name} --limit={limit} --offset={}", offset + limit)));
    }
    actions.push(Action::new("matchups", &format!("pokedex type matchups {type_name}")));

    let response = Response::new(
        species,
        actions,
        Meta::paginated(&format!("pokedex type pokemon {type_name}"), total, limit, offset),
    );
    response.print(format)
}
