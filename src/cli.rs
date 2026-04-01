use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pokedex", about = "Pokédex CLI — track your Pokémon collection")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Output the full command tree as JSON for agent discovery
    #[arg(long, global = true)]
    pub discover: bool,

    /// Output format
    #[arg(long, global = true, default_value = "json")]
    pub format: String,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Query Pokémon species data
    Pokemon {
        #[command(subcommand)]
        command: PokemonCommands,
    },
    /// Query type effectiveness and matchups
    Type {
        #[command(subcommand)]
        command: TypeCommands,
    },
    /// Query and track Pokédex completion
    Dex {
        #[command(subcommand)]
        command: DexCommands,
    },
    /// Query game information and HOME compatibility
    Game {
        #[command(subcommand)]
        command: GameCommands,
    },
    /// Manage your Pokémon collection
    Collection {
        #[command(subcommand)]
        command: CollectionCommands,
    },
    /// Pokémon HOME status and transfer info
    Home {
        #[command(subcommand)]
        command: HomeCommands,
    },
    /// Query location encounter data
    Location {
        #[command(subcommand)]
        command: LocationCommands,
    },
    /// Query item data
    Item {
        #[command(subcommand)]
        command: ItemCommands,
    },
    /// Database management
    Db {
        #[command(subcommand)]
        command: DbCommands,
    },
}

// -- Item subcommands --

#[derive(Subcommand)]
pub enum ItemCommands {
    /// Show detailed info for an item
    Show {
        /// Item name or ID (e.g. thunder-stone, 83)
        item: String,
        /// Filter held-by data to a specific game
        #[arg(long)]
        game: Option<String>,
    },
}

// -- Location subcommands --

#[derive(Subcommand)]
pub enum LocationCommands {
    /// Show what Pokémon can be found at a location
    Encounters {
        /// Location name or area slug (e.g. "viridian-forest", "wild-area-station")
        location: String,
        #[arg(long)]
        game: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
}

// -- Pokemon subcommands --

#[derive(Subcommand)]
pub enum PokemonCommands {
    /// List Pokémon with optional filters
    List {
        #[arg(long, name = "type")]
        type_filter: Option<String>,
        #[arg(long)]
        generation: Option<u32>,
        #[arg(long)]
        category: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Show detailed info for a Pokémon
    Show {
        /// Pokémon name or national dex ID
        pokemon: String,
    },
    /// Fuzzy search for a Pokémon by name
    Search {
        query: String,
        #[arg(long, default_value = "10")]
        limit: u64,
    },
    /// Show evolution chain for a Pokémon
    Evolutions {
        /// Pokémon name or national dex ID
        pokemon: String,
    },
    /// List all forms of a Pokémon
    Forms {
        /// Pokémon name or national dex ID
        pokemon: String,
    },
    /// Show where a Pokémon can be encountered
    Encounters {
        /// Pokémon name or national dex ID
        pokemon: String,
        #[arg(long)]
        game: Option<String>,
    },
    /// Show moves a Pokémon can learn
    Moves {
        /// Pokémon name or national dex ID
        pokemon: String,
        #[arg(long)]
        game: Option<String>,
        /// Filter by learn method: level-up, tm, egg, tutor
        #[arg(long)]
        method: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Show base stats for a Pokémon
    Stats {
        /// Pokémon name or national dex ID
        pokemon: String,
    },
}

// -- Type subcommands --

#[derive(Subcommand)]
pub enum TypeCommands {
    /// List all types
    List,
    /// Show type matchups (offensive and defensive)
    Matchups {
        /// Type name (e.g. fire, water, dragon)
        #[arg(name = "type")]
        type_name: String,
    },
    /// List Pokémon of a given type
    Pokemon {
        /// Type name
        #[arg(name = "type")]
        type_name: String,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
}

// -- Dex subcommands --

#[derive(Subcommand)]
pub enum DexCommands {
    /// List all available Pokédexes
    List,
    /// Show species in a Pokédex
    Show {
        /// Pokédex name (e.g. national, kanto, paldea)
        dex: String,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Look up a Pokémon by its regional dex number
    Lookup {
        /// Pokédex name (e.g. paldea, kanto, hoenn)
        dex: String,
        /// Regional dex number
        number: u64,
    },
    /// Show your completion progress for a Pokédex
    Progress {
        /// Pokédex name
        dex: String,
        /// Show only missing Pokémon
        #[arg(long)]
        missing: bool,
        /// Show only caught Pokémon
        #[arg(long)]
        caught: bool,
        /// Filter by game
        #[arg(long)]
        game: Option<String>,
        /// Only count entries with this status (e.g. living_dex)
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
}

// -- Game subcommands --

#[derive(Subcommand)]
pub enum GameCommands {
    /// List supported games
    List {
        /// Only show games that connect to Pokémon HOME
        #[arg(long)]
        home_compatible: bool,
    },
    /// Show details for a game
    Show {
        /// Game name (e.g. scarlet, sword, lets-go-pikachu)
        game: String,
    },
    /// List all Pokémon encounterable in a game
    Encounters {
        /// Game name
        game: String,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Show version-exclusive Pokémon for a game
    Exclusives {
        /// Game name
        game: String,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
}

// -- Collection subcommands --

#[derive(Subcommand)]
pub enum CollectionCommands {
    /// Add a Pokémon to your collection
    Add {
        #[arg(long)]
        pokemon: String,
        #[arg(long)]
        game: String,
        #[arg(long)]
        form: Option<String>,
        #[arg(long)]
        shiny: bool,
        #[arg(long)]
        in_home: bool,
        #[arg(long)]
        alpha: bool,
        /// Status: caught, living_dex, evolved, traded_away, transferred
        #[arg(long, default_value = "caught")]
        status: String,
        /// Method: catch, breed, trade, transfer, gift, raid, research
        #[arg(long)]
        method: Option<String>,
        #[arg(long)]
        nickname: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        /// Preview the add without saving
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove a collection entry by ID
    Remove {
        id: i64,
        /// Preview the removal without saving
        #[arg(long)]
        dry_run: bool,
    },
    /// Update a collection entry
    Update {
        id: i64,
        /// Status: caught, living_dex, evolved, traded_away, transferred
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        in_home: Option<bool>,
        #[arg(long)]
        shiny: Option<bool>,
        #[arg(long)]
        nickname: Option<String>,
        #[arg(long)]
        notes: Option<String>,
        /// Change the game for this entry
        #[arg(long)]
        game: Option<String>,
        /// Change the catch method for this entry
        #[arg(long)]
        method: Option<String>,
        /// Preview the update without saving
        #[arg(long)]
        dry_run: bool,
    },
    /// List your collection entries
    List {
        #[arg(long)]
        game: Option<String>,
        #[arg(long)]
        pokemon: Option<String>,
        #[arg(long)]
        shiny_only: bool,
        #[arg(long)]
        in_home: bool,
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
        /// Sort order: "id" (default) or "dex"
        #[arg(long, default_value = "id")]
        sort: String,
    },
    /// Show a collection entry by ID
    Show { id: i64 },
    /// Show collection statistics
    Stats {
        /// Filter stats by game
        #[arg(long)]
        game: Option<String>,
    },
}

// -- Home subcommands --

#[derive(Subcommand)]
pub enum HomeCommands {
    /// Show what's currently in HOME
    Status,
    /// Show which games a Pokémon can transfer to/from
    Transferable {
        /// Pokémon name or national dex ID
        pokemon: String,
    },
    /// Show species missing from HOME
    Missing {
        /// Dex to check against: national, kanto, paldea, etc.
        #[arg(long, default_value = "national")]
        dex: String,
        #[arg(long, default_value = "50")]
        limit: u64,
        #[arg(long, default_value = "0")]
        offset: u64,
    },
    /// Show HOME dex completion percentage
    Coverage,
}

// -- Db subcommands --

#[derive(Subcommand)]
pub enum DbCommands {
    /// Seed the database with Pokémon data (downloads from PokeAPI if no --from given)
    Seed {
        /// Path to local PokeAPI CSV directory (skips download)
        #[arg(long)]
        from: Option<String>,
        /// Re-download and reseed (preserves collection data)
        #[arg(long)]
        refresh: bool,
        /// Keep downloaded CSV cache after seeding
        #[arg(long)]
        keep_cache: bool,
    },
}
