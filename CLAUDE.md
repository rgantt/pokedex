# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
cargo build                    # dev build
cargo build --release          # release build (7MB binary)
cargo run -- <command>         # run with args
cargo run -- --discover        # show full command tree
```

No test suite exists yet. No linter config — use `cargo clippy` for linting.

The DB path defaults to `~/.pokedex/db.sqlite`. Override with `POKEDEX_DB_PATH` env var (useful for testing):
```bash
POKEDEX_DB_PATH=/tmp/test.db cargo run -- db seed
```

## Architecture

This is an AI-agent-first CLI. Every command outputs a HATEOAS JSON envelope (`{data, actions, meta}`) where `actions` contains literal runnable command strings so an agent can navigate without prior knowledge. The `--discover` flag returns the full command tree. Running with no subcommand also prints discovery output.

### Data Flow

Two-phase seeding in `db::seed`:
1. **PokeAPI** — downloads `master.tar.gz` from GitHub, extracts ~118 CSVs (skipping Conquest/contest/Pal Park prefixes), bulk-loads into SQLite with FK checks disabled. Covers species, types, moves, abilities, evolutions, encounters (Gen 1-5 only), items, natures, locations.
2. **PokeDB.org** — downloads JSON from `cdn.pokedb.org` for encounters, locations, location_areas, encounter_methods, versions. Maps string identifiers to existing numeric IDs, creates new entries for Gen 6+ data not in PokeAPI. Stores rich per-game encounter metadata in `encounter_details` table. Non-fatal if download fails.

User collection data (`collection` and `games` tables) is preserved across `--refresh` reseeds.

### Command Dispatch

`main.rs` → `Cli::parse()` → match on `Commands` enum → call handler in `commands/` module → handler calls `db::queries` → wraps result in `output::Response<T>` → prints JSON.

Most handlers follow this pattern:
1. `resolve_pokemon(conn, identifier)` — try as ID, then exact name match
2. On miss: `search_species()` with strsim fuzzy matching → `ErrorResponse::not_found()` with `did_you_mean` actions
3. Query with joins to `*_names` tables for English display names
4. Build `actions` vec with related navigable commands
5. `Response::new(data, actions, meta).print(format)`

### Key Design Decisions

- **`gen` is a reserved keyword in Rust 2024 edition** — use `generation` for variable/field names.
- **DB path is intentionally not exposed via CLI** — prevents agents from querying SQLite directly. All access goes through the command hierarchy.
- **PokeAPI uses `identifier` columns, our schema uses `name`** — the `seed_table_mapped` function handles column renaming during CSV ingestion.
- **PokeDB uses string identifiers, our schema uses integer IDs** — `build_pokemon_map`, `build_version_map` etc. create lookup HashMaps for the mapping.
- **Encounter details vary by game generation** — SwSh uses `weather_*_rate` fields, SV uses `probability_overall`/`on_terrain_*`, PLA uses `alpha_levels`/time booleans. All stored in `encounter_details` table, serialized with `skip_serializing_if = "Option::is_none"`.
- **Collection supports multiple entries per species** — same Pokémon can be logged in multiple games or multiple times in the same game. Status field (`caught`, `living_dex`, `evolved`, `traded_away`, `transferred`) distinguishes current holdings from historical records.
- **Blocking HTTP** — no async runtime. `reqwest::blocking` keeps the codebase simple for a CLI tool.
