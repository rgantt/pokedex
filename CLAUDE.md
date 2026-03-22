# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Run

```bash
./install.sh                   # build release + install to /usr/local/bin
cargo build                    # dev build
cargo run -- <command>         # run with args
cargo run -- --discover        # show full command tree
```

Run tests (requires a seeded DB):
```bash
POKEDEX_DB_PATH=/tmp/test.db cargo run -- db seed
POKEDEX_DB_PATH=/tmp/test.db cargo test --test validate_encounters
cargo test --test run_screenplays    # runs 2000+ screenplay regression steps
```

No linter config — use `cargo clippy` for linting.

The DB path defaults to `~/.pokedex/db.sqlite`. Override with `POKEDEX_DB_PATH` env var (useful for testing):
```bash
POKEDEX_DB_PATH=/tmp/test.db cargo run -- db seed
```

## Architecture

This is an AI-agent-first CLI. Every command outputs a HATEOAS JSON envelope (`{data, actions, meta}`) where `actions` contains literal runnable command strings so an agent can navigate without prior knowledge. The `--discover` flag returns the full command tree. Running with no subcommand also prints discovery output.

### Data Flow

Multi-phase seeding in `db::seed`:
1. **PokeAPI** — downloads `master.tar.gz` from GitHub, extracts ~118 CSVs (skipping Conquest/contest/Pal Park prefixes), bulk-loads into SQLite with FK checks disabled. Covers species, types, moves, abilities, evolutions, encounters (Gen 1-5 only), items, natures, locations. Deduplicates encounter rows after loading.
2. **PokeDB.org** — downloads JSON from `cdn.pokedb.org` for encounters, locations, location_areas, encounter_methods, versions. Maps string identifiers to existing numeric IDs, creates new entries for Gen 6+ data not in PokeAPI. Stores rich per-game encounter metadata in `encounter_details` table. Normalizes probability weights to percentages. Non-fatal if download fails.
3. **Legends: Z-A** — loads bundled encounter data scraped from Serebii (260 encounters across 20 Wild Zones), compiled into the binary via `include_str!`.
4. **Curated overrides** — applies `data/overrides/` JSON files and programmatic fixes for known upstream data issues: evolution trigger details, regional form `is_default`, Vivillon pattern defaults, encounter version-exclusive swaps (E005: Vullaby/Rufflet in BW).
5. **Pre-HOME games** — auto-populates the `games` table from `versions` that have encounter data, enabling collection tracking for classic games (Red, Gold, Ruby, etc.).

Known data issues are tracked in `data/known_issues.md` — check before investigating data bugs.

User collection data (`collection` and `games` tables) is preserved across `--refresh` reseeds.

### Command Dispatch

`main.rs` → `Cli::try_parse()` → match on `Commands` enum → call handler in `commands/` module → handler calls `db::queries` → wraps result in `output::Response<T>` → prints JSON. Clap parse errors are caught and converted to JSON error envelopes.

Most handlers follow this pattern:
1. `resolve_pokemon(conn, identifier)` — try as ID, then species name, then pokemon name (form-specific like `growlithe-hisui`), then `pokemon_forms.name` (cosmetic forms like `vivillon-polar`)
2. On miss: `search_species()` with strsim fuzzy matching → `ErrorResponse::not_found()` with `did_you_mean` actions
3. Validate filters (game, status, method, type, category, generation) before querying — invalid values return `INVALID_PARAMETER` errors
4. Query with joins to `*_names` tables for English display names
5. Build `actions` vec with related navigable commands (template actions like `{name}` for lists, concrete actions for pagination)
6. `Response::new(data, actions, meta).print(format)`

### Key Design Decisions

- **`gen` is a reserved keyword in Rust 2024 edition** — use `generation` for variable/field names.
- **DB path is intentionally not exposed via CLI** — prevents agents from querying SQLite directly. All access goes through the command hierarchy.
- **PokeAPI uses `identifier` columns, our schema uses `name`** — the `seed_table_mapped` function handles column renaming during CSV ingestion.
- **PokeDB uses string identifiers, our schema uses integer IDs** — `build_pokemon_map`, `build_version_map` etc. create lookup HashMaps for the mapping.
- **Encounter details vary by game generation** — SwSh uses `weather_*_rate` fields, SV uses `probability_overall`/`on_terrain_*`, PLA uses `alpha_levels`/time booleans. All stored in `encounter_details` table, serialized with `skip_serializing_if = "Option::is_none"`.
- **Regional form annotations** — curated overlay in `data/overrides/regional_encounters.json` maps (species, game) → form label for wild encounters. NPC trades excluded (ambiguous forms).
- **Collection supports multiple entries per species** — same Pokémon can be logged in multiple games or multiple times in the same game. Status field (`caught`, `living_dex`, `evolved`, `traded_away`, `transferred`) distinguishes current holdings from historical records.
- **Evolution methods are per-game** — `EvolutionNode.methods` is a Vec, not a single trigger. Leafeon has both `use-item` (Leaf Stone, modern) and `level-up` (Mossy Rock, older games).
- **Form resolution** — `resolve_pokemon` checks species → pokemon → pokemon_forms in order. Form-specific types, stats, abilities, display_name, and generation override species-level values in `pokemon show`. Regional forms show the generation they were introduced in (e.g., Alolan Meowth = Gen 7, not Gen 1).
- **Form context in HATEOAS actions** — when querying a form (e.g., `growlithe-hisui`), all action links preserve the form slug (stats, moves, encounters, add_to_collection, home transferable). Evolutions and forms actions link to the base species.
- **Game region from version_group_regions** — remakes show their actual region (FireRed = Kanto, HeartGold = Johto, BDSP = Sinnoh), not the generation's native region. Uses `MAX(region_id)` subquery to pick the primary region for multi-region games.
- **All JSON output goes to stdout** — success and error responses both go to stdout. Errors call `process::exit(1)`.
- **Blocking HTTP** — no async runtime. `reqwest::blocking` keeps the codebase simple for a CLI tool.
- **Dry-run support** — `collection add`, `collection update`, and `collection remove` all support `--dry-run` with preview data and confirm action links.

### Testing

- **Screenplay regression tests** — 2000+ steps across 26 YAML files in `tests/screenplays/`. Run with `cargo test --test run_screenplays`. Each file is a sequence of CLI commands with assertions (exit codes, field presence, value equality, array bounds). Strict schema enforcement via `deny_unknown_fields`.
- **Screenplay recorder** — `scripts/screenplay.py` is a companion tool that agents use to record steps instead of hand-writing YAML. Supports concurrent sessions via `--session` flag. Run `python3 scripts/screenplay.py --help` for usage.
- **Exploratory test skill** — `.claude/skills/exploratory-test/` launches parallel agent personas to test the CLI from different player perspectives. Each run produces new timestamped screenplay files that accumulate.
- **Audit skill** — `.claude/skills/audit-screenplays/` launches Pokemon expert agents to verify screenplay accuracy against the CLI. Experts verify every claim by running commands before reporting.
