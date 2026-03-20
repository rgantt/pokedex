use anyhow::Result;
use clap::Parser;
use pokedex::cli::*;
use pokedex::db;
use pokedex::discover;
use pokedex::commands;
use pokedex::output::OutputFormat;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.discover {
        return discover::print_discovery();
    }

    let format: OutputFormat = cli.format.parse().unwrap_or_default();

    let command = match cli.command {
        Some(c) => c,
        None => {
            // No subcommand: print discovery by default (agent-friendly)
            return discover::print_discovery();
        }
    };

    let mut conn = db::open()?;

    match command {
        Commands::Db { command: db_cmd } => match db_cmd {
            DbCommands::Seed { from, refresh, keep_cache } => {
                commands::db_cmd::seed_cmd(
                    &mut conn,
                    from.as_deref(),
                    refresh,
                    keep_cache,
                    &format,
                )?;
            }
        },
        Commands::Pokemon { command: pokemon_cmd } => {
            if !db::is_seeded(&conn)? {
                eprintln!("Database not seeded. Run: pokedex db seed");
                std::process::exit(1);
            }
            match pokemon_cmd {
                PokemonCommands::List { type_filter, generation, category, limit, offset } => {
                    commands::pokemon::list(&conn, type_filter.as_deref(), generation, category.as_deref(), limit, offset, &format)?;
                }
                PokemonCommands::Show { pokemon } => {
                    commands::pokemon::show(&conn, &pokemon, &format)?;
                }
                PokemonCommands::Search { query, limit } => {
                    commands::pokemon::search(&conn, &query, limit, &format)?;
                }
                PokemonCommands::Evolutions { pokemon } => {
                    commands::pokemon::evolutions(&conn, &pokemon, &format)?;
                }
                PokemonCommands::Forms { pokemon } => {
                    commands::pokemon::forms(&conn, &pokemon, &format)?;
                }
                PokemonCommands::Encounters { pokemon, game } => {
                    commands::pokemon::encounters(&conn, &pokemon, game.as_deref(), &format)?;
                }
                PokemonCommands::Moves { pokemon, game, method } => {
                    commands::pokemon::moves(&conn, &pokemon, game.as_deref(), method.as_deref(), &format)?;
                }
                PokemonCommands::Stats { pokemon } => {
                    commands::pokemon::stats(&conn, &pokemon, &format)?;
                }
            }
        }
        Commands::Type { command: type_cmd } => {
            if !db::is_seeded(&conn)? {
                eprintln!("Database not seeded. Run: pokedex db seed");
                std::process::exit(1);
            }
            match type_cmd {
                TypeCommands::List => {
                    commands::type_cmd::list(&conn, &format)?;
                }
                TypeCommands::Matchups { type_name } => {
                    commands::type_cmd::matchups(&conn, &type_name, &format)?;
                }
                TypeCommands::Pokemon { type_name, limit, offset } => {
                    commands::type_cmd::pokemon_of_type(&conn, &type_name, limit, offset, &format)?;
                }
            }
        }
        Commands::Dex { command: dex_cmd } => {
            if !db::is_seeded(&conn)? {
                eprintln!("Database not seeded. Run: pokedex db seed");
                std::process::exit(1);
            }
            match dex_cmd {
                DexCommands::List => {
                    commands::dex::list(&conn, &format)?;
                }
                DexCommands::Show { dex, limit, offset } => {
                    commands::dex::show(&conn, &dex, limit, offset, &format)?;
                }
                DexCommands::Progress { dex, missing, caught, game, status, limit, offset } => {
                    commands::dex::progress(&conn, &dex, missing, caught, game.as_deref(), status.as_deref(), limit, offset, &format)?;
                }
            }
        }
        Commands::Game { command: game_cmd } => {
            match game_cmd {
                GameCommands::List { home_compatible, .. } => {
                    commands::game::list(&conn, home_compatible, &format)?;
                }
                GameCommands::Show { game } => {
                    commands::game::show(&conn, &game, &format)?;
                }
            }
        }
        Commands::Collection { command: col_cmd } => {
            match col_cmd {
                CollectionCommands::Add { pokemon, game, form, shiny, in_home, alpha, status, method, nickname, notes, dry_run } => {
                    commands::collection::add(
                        &conn, &pokemon, &game, form.as_deref(), shiny, in_home, alpha,
                        &status, method.as_deref(), nickname.as_deref(), notes.as_deref(),
                        dry_run, &format,
                    )?;
                }
                CollectionCommands::Remove { id, dry_run } => {
                    commands::collection::remove(&conn, id, dry_run, &format)?;
                }
                CollectionCommands::Update { id, status, in_home, shiny, nickname, notes, game, method } => {
                    commands::collection::update(
                        &conn, id, status.as_deref(), in_home, shiny,
                        nickname.as_deref(), notes.as_deref(),
                        game.as_deref(), method.as_deref(), &format,
                    )?;
                }
                CollectionCommands::List { game, pokemon, shiny_only, in_home, status, limit, offset, sort } => {
                    commands::collection::list_entries(
                        &conn, game.as_deref(), pokemon.as_deref(), shiny_only,
                        in_home, status.as_deref(), limit, offset, &sort, &format,
                    )?;
                }
                CollectionCommands::Show { id } => {
                    commands::collection::show_entry(&conn, id, &format)?;
                }
                CollectionCommands::Stats { game } => {
                    commands::collection::stats(&conn, game.as_deref(), &format)?;
                }
            }
        }
        Commands::Home { command: home_cmd } => {
            match home_cmd {
                HomeCommands::Status => {
                    commands::home::status(&conn, &format)?;
                }
                HomeCommands::Transferable { pokemon } => {
                    commands::home::transferable(&conn, &pokemon, &format)?;
                }
                HomeCommands::Missing { dex, limit, offset } => {
                    commands::home::missing(&conn, &dex, limit, offset, &format)?;
                }
                HomeCommands::Coverage => {
                    commands::home::coverage(&conn, &format)?;
                }
            }
        }
    }

    Ok(())
}
