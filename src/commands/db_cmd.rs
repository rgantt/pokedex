use anyhow::Result;
use rusqlite::Connection;
use std::path::PathBuf;

use crate::db;
use crate::db::seed;
use crate::output::*;
use serde::Serialize;

pub fn seed_cmd(
    conn: &mut Connection,
    from: Option<&str>,
    refresh: bool,
    keep_cache: bool,
    format: &OutputFormat,
) -> Result<()> {
    if db::is_seeded(conn)? && !refresh {
        #[derive(Serialize)]
        struct AlreadySeeded { seeded: bool, message: String }

        let response = Response::new(
            AlreadySeeded {
                seeded: true,
                message: "Database already seeded. Use --refresh to re-download and reseed.".to_string(),
            },
            vec![
                Action::new("refresh", "pokedex db seed --refresh"),
                Action::new("pokemon_list", "pokedex pokemon list"),
            ],
            Meta::simple("pokedex db seed"),
        );
        return response.print(format);
    }

    if refresh && db::is_seeded(conn)? {
        eprintln!("Dropping reference data for reseed...");
        seed::drop_reference_data(conn)?;
    }

    let csv_dir = if let Some(path) = from {
        PathBuf::from(path)
    } else {
        seed::download_and_extract(keep_cache)?
    };

    eprintln!("Seeding database...");
    seed::seed_from_directory(conn, &csv_dir)?;

    if !keep_cache && from.is_none() {
        eprintln!("Cleaning up cache...");
        seed::clear_cache()?;
    }

    eprintln!("Done!");

    #[derive(Serialize)]
    struct SeedResult { seeded: bool }

    let response = Response::new(
        SeedResult { seeded: true },
        vec![
            Action::new("pokemon_list", "pokedex pokemon list"),
            Action::new("dex_list", "pokedex dex list"),
            Action::new("type_list", "pokedex type list"),
            Action::new("game_list", "pokedex game list"),
        ],
        Meta::simple("pokedex db seed"),
    );
    response.print(format)
}
