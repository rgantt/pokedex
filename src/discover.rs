use crate::output::*;
use serde::Serialize;

#[derive(Serialize)]
struct DiscoverOutput {
    name: String,
    description: String,
    resources: Vec<Resource>,
    entry_points: Vec<String>,
}

#[derive(Serialize)]
struct Resource {
    name: String,
    description: String,
    commands: Vec<CommandInfo>,
}

#[derive(Serialize)]
struct CommandInfo {
    name: String,
    usage: String,
    description: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    flags: Vec<FlagInfo>,
}

#[derive(Serialize)]
struct FlagInfo {
    flag: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<String>,
}

pub fn print_discovery() -> anyhow::Result<()> {
    let output = DiscoverOutput {
        name: "pokedex".to_string(),
        description: "Pokédex CLI — track your Pokémon collection with HATEOAS-style navigation".to_string(),
        resources: vec![
            Resource {
                name: "pokemon".to_string(),
                description: "Query Pokémon species data, stats, moves, encounters, and evolutions".to_string(),
                commands: vec![
                    CommandInfo {
                        name: "list".to_string(),
                        usage: "pokedex pokemon list [--type-filter=<type>] [--generation=<1-9>] [--category=<cat>] [--limit=N] [--offset=N]".to_string(),
                        description: "List Pokémon with optional filters".to_string(),
                        flags: vec![
                            FlagInfo { flag: "--type-filter".to_string(), description: "Filter by type (fire, water, etc.)".to_string(), default: None },
                            FlagInfo { flag: "--generation".to_string(), description: "Filter by generation (1-9)".to_string(), default: None },
                            FlagInfo { flag: "--category".to_string(), description: "Filter by category (legendary, mythical, baby)".to_string(), default: None },
                            FlagInfo { flag: "--limit".to_string(), description: "Results per page".to_string(), default: Some("50".to_string()) },
                            FlagInfo { flag: "--offset".to_string(), description: "Skip N results".to_string(), default: Some("0".to_string()) },
                        ],
                    },
                    CommandInfo {
                        name: "show".to_string(),
                        usage: "pokedex pokemon show <name-or-id>".to_string(),
                        description: "Show detailed info for a Pokémon".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "search".to_string(),
                        usage: "pokedex pokemon search <query> [--limit=N]".to_string(),
                        description: "Fuzzy search for a Pokémon by name".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "evolutions".to_string(),
                        usage: "pokedex pokemon evolutions <name-or-id>".to_string(),
                        description: "Show evolution chain".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "forms".to_string(),
                        usage: "pokedex pokemon forms <name-or-id>".to_string(),
                        description: "List all forms of a Pokémon".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "encounters".to_string(),
                        usage: "pokedex pokemon encounters <name-or-id> [--game=<game>]".to_string(),
                        description: "Show where a Pokémon can be encountered".to_string(),
                        flags: vec![
                            FlagInfo { flag: "--game".to_string(), description: "Filter by game".to_string(), default: None },
                        ],
                    },
                    CommandInfo {
                        name: "moves".to_string(),
                        usage: "pokedex pokemon moves <name-or-id> [--game=<game>] [--method=<method>]".to_string(),
                        description: "Show moves a Pokémon can learn".to_string(),
                        flags: vec![
                            FlagInfo { flag: "--game".to_string(), description: "Filter by game".to_string(), default: None },
                            FlagInfo { flag: "--method".to_string(), description: "Filter by learn method: level-up, machine, egg, tutor".to_string(), default: None },
                        ],
                    },
                    CommandInfo {
                        name: "stats".to_string(),
                        usage: "pokedex pokemon stats <name-or-id>".to_string(),
                        description: "Show base stats".to_string(),
                        flags: vec![],
                    },
                ],
            },
            Resource {
                name: "type".to_string(),
                description: "Query type effectiveness and matchups".to_string(),
                commands: vec![
                    CommandInfo {
                        name: "list".to_string(),
                        usage: "pokedex type list".to_string(),
                        description: "List all 18 types".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "matchups".to_string(),
                        usage: "pokedex type matchups <type>".to_string(),
                        description: "Show offensive and defensive type matchups".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "pokemon".to_string(),
                        usage: "pokedex type pokemon <type> [--limit=N] [--offset=N]".to_string(),
                        description: "List Pokémon of a given type".to_string(),
                        flags: vec![],
                    },
                ],
            },
            Resource {
                name: "dex".to_string(),
                description: "Query and track Pokédex completion".to_string(),
                commands: vec![
                    CommandInfo {
                        name: "list".to_string(),
                        usage: "pokedex dex list".to_string(),
                        description: "List all available Pokédexes".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "show".to_string(),
                        usage: "pokedex dex show <dex-name> [--limit=N] [--offset=N]".to_string(),
                        description: "Show species in a Pokédex".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "progress".to_string(),
                        usage: "pokedex dex progress <dex-name> [--missing] [--caught] [--game=<game>] [--status=<status>] [--limit=N] [--offset=N]".to_string(),
                        description: "Show your completion progress for a Pokédex".to_string(),
                        flags: vec![
                            FlagInfo { flag: "--missing".to_string(), description: "Show only missing Pokémon".to_string(), default: None },
                            FlagInfo { flag: "--caught".to_string(), description: "Show only caught Pokémon".to_string(), default: None },
                            FlagInfo { flag: "--game".to_string(), description: "Filter by game".to_string(), default: None },
                            FlagInfo { flag: "--status".to_string(), description: "Only count entries with this status (e.g. living_dex)".to_string(), default: None },
                        ],
                    },
                ],
            },
            Resource {
                name: "game".to_string(),
                description: "Query game information and HOME compatibility".to_string(),
                commands: vec![
                    CommandInfo {
                        name: "list".to_string(),
                        usage: "pokedex game list [--home-compatible]".to_string(),
                        description: "List supported games".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "show".to_string(),
                        usage: "pokedex game show <game>".to_string(),
                        description: "Show details for a game".to_string(),
                        flags: vec![],
                    },
                ],
            },
            Resource {
                name: "collection".to_string(),
                description: "Manage your Pokémon collection".to_string(),
                commands: vec![
                    CommandInfo {
                        name: "add".to_string(),
                        usage: "pokedex collection add --pokemon=<name> --game=<game> [--form=<form>] [--shiny] [--in-home] [--alpha] [--status=<status>] [--method=<method>] [--nickname=<name>] [--notes=<text>] [--dry-run]".to_string(),
                        description: "Add a Pokémon to your collection".to_string(),
                        flags: vec![
                            FlagInfo { flag: "--pokemon".to_string(), description: "Pokémon name (required)".to_string(), default: None },
                            FlagInfo { flag: "--game".to_string(), description: "Game caught in (required)".to_string(), default: None },
                            FlagInfo { flag: "--status".to_string(), description: "caught, living_dex, evolved, traded_away, transferred".to_string(), default: Some("caught".to_string()) },
                            FlagInfo { flag: "--method".to_string(), description: "catch, breed, trade, transfer, gift, raid, research".to_string(), default: None },
                            FlagInfo { flag: "--dry-run".to_string(), description: "Preview without saving".to_string(), default: None },
                        ],
                    },
                    CommandInfo {
                        name: "remove".to_string(),
                        usage: "pokedex collection remove <id> [--dry-run]".to_string(),
                        description: "Remove a collection entry by ID".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "update".to_string(),
                        usage: "pokedex collection update <id> [--status=<s>] [--in-home=<bool>] [--shiny=<bool>] [--nickname=<n>] [--notes=<n>] [--game=<g>] [--method=<m>]".to_string(),
                        description: "Update a collection entry".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "list".to_string(),
                        usage: "pokedex collection list [--game=<g>] [--pokemon=<p>] [--shiny-only] [--in-home] [--status=<s>] [--limit=N] [--offset=N] [--sort=<id|dex>]".to_string(),
                        description: "List your collection entries".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "show".to_string(),
                        usage: "pokedex collection show <id>".to_string(),
                        description: "Show a collection entry by ID".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "stats".to_string(),
                        usage: "pokedex collection stats [--game=<g>]".to_string(),
                        description: "Show collection statistics".to_string(),
                        flags: vec![],
                    },
                ],
            },
            Resource {
                name: "home".to_string(),
                description: "Pokémon HOME status and transfer info".to_string(),
                commands: vec![
                    CommandInfo {
                        name: "status".to_string(),
                        usage: "pokedex home status".to_string(),
                        description: "Show what's currently in HOME".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "transferable".to_string(),
                        usage: "pokedex home transferable <pokemon>".to_string(),
                        description: "Show which games a Pokémon can transfer to/from".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "missing".to_string(),
                        usage: "pokedex home missing [--dex=home|national] [--limit=N] [--offset=N]".to_string(),
                        description: "Show species missing from HOME".to_string(),
                        flags: vec![],
                    },
                    CommandInfo {
                        name: "coverage".to_string(),
                        usage: "pokedex home coverage".to_string(),
                        description: "Show HOME dex completion percentage".to_string(),
                        flags: vec![],
                    },
                ],
            },
            Resource {
                name: "location".to_string(),
                description: "Query location encounter data".to_string(),
                commands: vec![
                    CommandInfo {
                        name: "encounters".to_string(),
                        usage: "pokedex location encounters <location> [--game=<game>] [--limit=N] [--offset=N]".to_string(),
                        description: "Show what Pokémon can be found at a location".to_string(),
                        flags: vec![
                            FlagInfo { flag: "--game".to_string(), description: "Filter by game".to_string(), default: None },
                            FlagInfo { flag: "--limit".to_string(), description: "Results per page".to_string(), default: Some("50".to_string()) },
                            FlagInfo { flag: "--offset".to_string(), description: "Skip N results".to_string(), default: Some("0".to_string()) },
                        ],
                    },
                ],
            },
            Resource {
                name: "db".to_string(),
                description: "Database management".to_string(),
                commands: vec![
                    CommandInfo {
                        name: "seed".to_string(),
                        usage: "pokedex db seed [--from=<path>] [--refresh] [--keep-cache]".to_string(),
                        description: "Seed the database with Pokémon data (auto-downloads from PokeAPI)".to_string(),
                        flags: vec![
                            FlagInfo { flag: "--from".to_string(), description: "Path to local PokeAPI CSV directory".to_string(), default: None },
                            FlagInfo { flag: "--refresh".to_string(), description: "Re-download and reseed (preserves collection)".to_string(), default: None },
                            FlagInfo { flag: "--keep-cache".to_string(), description: "Keep downloaded CSVs after seeding".to_string(), default: None },
                        ],
                    },
                ],
            },
        ],
        entry_points: vec![
            "pokedex db seed".to_string(),
            "pokedex pokemon list".to_string(),
            "pokedex pokemon search <query>".to_string(),
            "pokedex collection list".to_string(),
            "pokedex dex list".to_string(),
            "pokedex type list".to_string(),
            "pokedex game list".to_string(),
            "pokedex home status".to_string(),
            "pokedex location encounters <location>".to_string(),
        ],
    };

    let response = Response::new(
        output,
        vec![Action::new("seed_database", "pokedex db seed")],
        Meta::simple("pokedex --discover"),
    );
    println!("{}", serde_json::to_string_pretty(&response)?);
    Ok(())
}
