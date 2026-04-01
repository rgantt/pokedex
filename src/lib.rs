// Command handlers and query functions pass through CLI flags directly —
// parameter counts mirror clap struct fields, so splitting would add indirection.
#![allow(clippy::too_many_arguments)]

pub mod cli;
pub mod commands;
pub mod db;
pub mod discover;
pub mod output;

use anyhow::Result;
use output::OutputFormat;

/// Run the CLI with pre-parsed arguments. Testable without subprocess.
pub fn dispatch(command: Option<cli::Commands>, format: &OutputFormat, conn: &mut rusqlite::Connection) -> Result<()> {
    let command = match command {
        Some(c) => c,
        None => return discover::print_discovery(),
    };

    match command {
        cli::Commands::Db { command: db_cmd } => match db_cmd {
            cli::DbCommands::Seed { from, refresh, keep_cache } => {
                commands::db_cmd::seed_cmd(conn, from.as_deref(), refresh, keep_cache, format)?;
            }
        },
        cli::Commands::Pokemon { command: pokemon_cmd } => {
            require_seeded(conn)?;
            dispatch_pokemon(pokemon_cmd, conn, format)?;
        }
        cli::Commands::Type { command: type_cmd } => {
            require_seeded(conn)?;
            dispatch_type(type_cmd, conn, format)?;
        }
        cli::Commands::Dex { command: dex_cmd } => {
            require_seeded(conn)?;
            dispatch_dex(dex_cmd, conn, format)?;
        }
        cli::Commands::Game { command: game_cmd } => {
            match game_cmd {
                cli::GameCommands::List { home_compatible, .. } => {
                    commands::game::list(conn, home_compatible, format)?;
                }
                cli::GameCommands::Show { game } => {
                    commands::game::show(conn, &game, format)?;
                }
                cli::GameCommands::Encounters { game, limit, offset } => {
                    commands::game::encounters(conn, &game, limit, offset, format)?;
                }
                cli::GameCommands::Exclusives { game, limit, offset } => {
                    commands::game::exclusives(conn, &game, limit, offset, format)?;
                }
            }
        }
        cli::Commands::Collection { command: col_cmd } => {
            dispatch_collection(col_cmd, conn, format)?;
        }
        cli::Commands::Location { command: loc_cmd } => {
            require_seeded(conn)?;
            match loc_cmd {
                cli::LocationCommands::Encounters { location, game, limit, offset } => {
                    commands::location::encounters(conn, &location, game.as_deref(), limit, offset, format)?;
                }
            }
        }
        cli::Commands::Item { command: item_cmd } => {
            require_seeded(conn)?;
            match item_cmd {
                cli::ItemCommands::Show { item, game } => {
                    commands::item::show(conn, &item, game.as_deref(), format)?;
                }
            }
        }
        cli::Commands::Home { command: home_cmd } => {
            match home_cmd {
                cli::HomeCommands::Status => commands::home::status(conn, format)?,
                cli::HomeCommands::Transferable { pokemon } => commands::home::transferable(conn, &pokemon, format)?,
                cli::HomeCommands::Missing { dex, limit, offset } => commands::home::missing(conn, &dex, limit, offset, format)?,
                cli::HomeCommands::Coverage => commands::home::coverage(conn, format)?,
            }
        }
    }

    Ok(())
}

fn require_seeded(conn: &rusqlite::Connection) -> Result<()> {
    if !db::is_seeded(conn)? {
        eprintln!("Database not seeded. Run: pokedex db seed");
        std::process::exit(1);
    }
    Ok(())
}

fn dispatch_pokemon(cmd: cli::PokemonCommands, conn: &rusqlite::Connection, format: &OutputFormat) -> Result<()> {
    match cmd {
        cli::PokemonCommands::List { type_filter, generation, category, limit, offset } => {
            commands::pokemon::list(conn, type_filter.as_deref(), generation, category.as_deref(), limit, offset, format)?;
        }
        cli::PokemonCommands::Show { pokemon } => commands::pokemon::show(conn, &pokemon, format)?,
        cli::PokemonCommands::Search { query, limit } => commands::pokemon::search(conn, &query, limit, format)?,
        cli::PokemonCommands::Evolutions { pokemon } => commands::pokemon::evolutions(conn, &pokemon, format)?,
        cli::PokemonCommands::Forms { pokemon } => commands::pokemon::forms(conn, &pokemon, format)?,
        cli::PokemonCommands::Encounters { pokemon, game } => {
            commands::pokemon::encounters(conn, &pokemon, game.as_deref(), format)?;
        }
        cli::PokemonCommands::Moves { pokemon, game, method, limit, offset } => {
            commands::pokemon::moves(conn, &pokemon, game.as_deref(), method.as_deref(), limit, offset, format)?;
        }
        cli::PokemonCommands::Stats { pokemon } => commands::pokemon::stats(conn, &pokemon, format)?,
    }
    Ok(())
}

fn dispatch_type(cmd: cli::TypeCommands, conn: &rusqlite::Connection, format: &OutputFormat) -> Result<()> {
    match cmd {
        cli::TypeCommands::List => commands::type_cmd::list(conn, format)?,
        cli::TypeCommands::Matchups { type_name } => commands::type_cmd::matchups(conn, &type_name, format)?,
        cli::TypeCommands::Pokemon { type_name, limit, offset } => {
            commands::type_cmd::pokemon_of_type(conn, &type_name, limit, offset, format)?;
        }
    }
    Ok(())
}

fn dispatch_dex(cmd: cli::DexCommands, conn: &rusqlite::Connection, format: &OutputFormat) -> Result<()> {
    match cmd {
        cli::DexCommands::List => commands::dex::list(conn, format)?,
        cli::DexCommands::Show { dex, limit, offset } => commands::dex::show(conn, &dex, limit, offset, format)?,
        cli::DexCommands::Lookup { dex, number } => commands::dex::lookup(conn, &dex, number, format)?,
        cli::DexCommands::Progress { dex, missing, caught, game, status, limit, offset } => {
            commands::dex::progress(conn, &dex, missing, caught, game.as_deref(), status.as_deref(), limit, offset, format)?;
        }
    }
    Ok(())
}

fn dispatch_collection(cmd: cli::CollectionCommands, conn: &rusqlite::Connection, format: &OutputFormat) -> Result<()> {
    match cmd {
        cli::CollectionCommands::Add { pokemon, game, form, shiny, in_home, alpha, status, method, nickname, notes, dry_run } => {
            commands::collection::add(
                conn, &pokemon, &game, form.as_deref(), shiny, in_home, alpha,
                &status, method.as_deref(), nickname.as_deref(), notes.as_deref(),
                dry_run, format,
            )?;
        }
        cli::CollectionCommands::Remove { id, dry_run } => commands::collection::remove(conn, id, dry_run, format)?,
        cli::CollectionCommands::Update { id, status, in_home, shiny, nickname, notes, game, method, dry_run } => {
            commands::collection::update(
                conn, id, status.as_deref(), in_home, shiny,
                nickname.as_deref(), notes.as_deref(),
                game.as_deref(), method.as_deref(), dry_run, format,
            )?;
        }
        cli::CollectionCommands::List { game, pokemon, shiny_only, in_home, status, limit, offset, sort } => {
            commands::collection::list_entries(
                conn, game.as_deref(), pokemon.as_deref(), shiny_only,
                in_home, status.as_deref(), limit, offset, &sort, format,
            )?;
        }
        cli::CollectionCommands::Show { id } => commands::collection::show_entry(conn, id, format)?,
        cli::CollectionCommands::Stats { game } => commands::collection::stats(conn, game.as_deref(), format)?,
    }
    Ok(())
}
