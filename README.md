# pokedex

[![Coverage](https://github.com/rgantt/pokedex/actions/workflows/coverage.yml/badge.svg)](https://github.com/rgantt/pokedex/actions/workflows/coverage.yml)
[![codecov](https://codecov.io/gh/rgantt/pokedex/graph/badge.svg)](https://codecov.io/gh/rgantt/pokedex)

A SQLite-backed CLI for tracking your Pokémon collection across HOME-compatible games. Designed to be used by AI agents — every command outputs structured JSON with navigable action links, so an agent can discover and traverse the entire command tree without documentation.

## Quick Start

```bash
./install.sh                        # build + install to /usr/local/bin
pokedex db seed                     # downloads PokeAPI + PokeDB data (~13MB), builds local DB (~47MB)
pokedex pokemon show charizard
```

## What's In The Database

The seed command downloads from two sources automatically:

**PokeAPI** (github.com/PokeAPI/pokeapi) — 118 CSV files covering:
- 1,025 species with types, abilities, stats, evolution chains, egg groups
- 937 moves with learnsets per game (618K move entries)
- 1,578 forms (regional, mega, gmax, gender, cosmetic)
- 65K encounters for Gen 1–5 games with rarity, conditions, level ranges
- Items, natures, type matchups, pokédex entries, flavor text

**PokeDB.org** — supplementary encounter data for modern games:
- 65K additional encounters covering Sword/Shield, BDSP, Legends: Arceus, Scarlet/Violet + all DLC
- Per-game encounter metadata: SwSh weather rates, SV probability weights and terrain, PLA alpha levels
- 73 encounter methods, 2,672 location areas

### Encounter Coverage

| Game | Encounters |
|------|-----------|
| Sword | 10,071 |
| Shield | 10,034 |
| Scarlet | 3,991 |
| Violet | 4,019 |
| Brilliant Diamond | 1,995 |
| Shining Pearl | 2,002 |
| Legends: Arceus | 1,631 |
| Let's Go Pikachu | 1,383 |
| Let's Go Eevee | 1,403 |
| Gen 1–5 games | 65,369 |

## Commands

Run `pokedex --discover` for the full machine-readable command tree. Run any command with `--help` for usage.

### Pokemon Queries

```bash
pokedex pokemon list --type-filter=fire --generation=1
pokedex pokemon show charizard           # types, egg groups, stats, abilities
pokedex pokemon show growlithe-hisui     # form-specific types (Fire/Rock), gen 8
pokedex pokemon show meowth-alola       # Alolan form shows gen 7 (not gen 1)
pokedex pokemon search bulbsaur          # fuzzy search (handles typos)
pokedex pokemon evolutions eevee         # full chain with all game-era methods
pokedex pokemon forms charizard          # base, mega-x, mega-y, gmax
pokedex pokemon moves pikachu --game=sword --method=level-up
pokedex pokemon encounters karrablast --game=sword
pokedex pokemon encounters darumaka --game=sword  # shows "Galarian Darumaka"
```

### Location Encounters

```bash
pokedex location encounters viridian-forest --game=red
pokedex location encounters wild-zone-3 --game=legends-za
pokedex location encounters rolling-fields --game=sword
```

### Type Matchups

```bash
pokedex type list
pokedex type matchups dragon             # offensive + defensive effectiveness
pokedex type pokemon fire                # all fire-type species
```

### Pokédex Progress

```bash
pokedex dex list                         # all available pokédexes
pokedex dex progress national --missing  # what you still need
pokedex dex progress paldea --status=living_dex
```

### Collection Management

```bash
pokedex collection add --pokemon=charizard --game=scarlet --shiny --in-home --method=catch
pokedex collection add --pokemon=pikachu --game=sword --status=living_dex
pokedex collection add --pokemon=charmander --game=scarlet --status=evolved --notes="evolved into charmeleon"
pokedex collection update 1 --status=transferred --in-home=true
pokedex collection update 1 --status=evolved --dry-run  # preview before updating
pokedex collection list --game=scarlet --shiny-only
pokedex collection show 1
pokedex collection stats
pokedex collection remove 1 --dry-run    # preview before deleting
```

**Collection statuses:**
| Status | Meaning |
|--------|---------|
| `caught` | You have it in a game |
| `living_dex` | Part of your living dex (one of each, held in HOME/game) |
| `evolved` | Was this species, evolved into something else |
| `traded_away` | Traded to another player |
| `transferred` | Moved to HOME or another game |

### Pokémon HOME

```bash
pokedex home status                      # what's in HOME
pokedex home coverage                    # national dex completion %
pokedex home missing                     # species not yet in HOME
pokedex home transferable pikachu        # which games can pikachu go to/from
```

### Games

```bash
pokedex game list --home-compatible
pokedex game show scarlet                # gen 9, Paldea
pokedex game show firered                # gen 3, Kanto (not Hoenn — remakes show correct region)
pokedex game show heartgold              # gen 4, Johto
```

### Database

```bash
pokedex db seed                          # auto-download and build DB
pokedex db seed --from ./csv/            # use local PokeAPI CSVs
pokedex db seed --refresh                # re-download, preserve collection
pokedex db seed --keep-cache             # keep CSVs at ~/.pokedex/cache/
```

## Output Format

Every command returns a JSON envelope:

```json
{
  "data": { ... },
  "actions": [
    { "rel": "evolutions", "cmd": "pokedex pokemon evolutions charmander" },
    { "rel": "add_to_collection", "cmd": "pokedex collection add --pokemon=charizard --game=<game>" },
    { "rel": "type_matchups", "cmd": "pokedex type matchups Fire" }
  ],
  "meta": {
    "command": "pokedex pokemon show charizard",
    "total": 1025,
    "limit": 50,
    "offset": 0
  }
}
```

**`actions`** contain literal CLI commands the agent can execute next — no URL construction or API knowledge needed. This is the HATEOAS principle applied to a CLI: the output tells you what you can do from here.

**Errors** include recovery suggestions:

```json
{
  "error": { "code": "NOT_FOUND", "message": "No pokémon named 'bulbsaur'" },
  "actions": [
    { "rel": "did_you_mean", "cmd": "pokedex pokemon show bulbasaur" },
    { "rel": "search", "cmd": "pokedex pokemon search bulbsaur" }
  ]
}
```

## Encounter Details

Modern games have richer encounter metadata than a simple rarity percentage. The `details` field on encounters contains game-specific data:

**Sword/Shield** — per-weather rates:
```json
{ "weather_snow_rate": "30%", "weather_blizzard_rate": "one", "hidden_ability_possible": true }
```

**Scarlet/Violet** — probability weights and terrain:
```json
{ "probability_overall": "50", "on_terrain_land": true, "group_rate": "100%", "group_pokemon": "snover-default" }
```

**Legends: Arceus** — alpha levels and time/weather:
```json
{ "alpha_levels": "60 - 73", "during_any_time": true, "while_weather_overall": true }
```

**BDSP** — time-of-day rates:
```json
{ "rate_night": "20%", "rate_overall": "10%" }
```

## Configuration

| Setting | Default | Override |
|---------|---------|---------|
| Database location | `~/.pokedex/db.sqlite` | `POKEDEX_DB_PATH` env var |
| Download cache | `~/.pokedex/cache/` | cleaned after seed unless `--keep-cache` |
| Output format | `json` | `--format=table` (currently same as json) |

## Testing

```bash
cargo test --test run_screenplays        # 2000+ regression steps across 26 screenplays
cargo test --test validate_encounters    # encounter data integrity checks
```

The screenplay tests replay recorded CLI interactions with assertions on exit codes, field presence, value equality, and array bounds. Each screenplay represents a different testing perspective — game playthroughs (Red through Scarlet), edge cases, competitive analysis, form variants, HOME transfers, and more. New screenplays accumulate over time with timestamped filenames.

### Screenplay Recorder

Agents use `scripts/screenplay.py` to record test steps instead of hand-writing YAML:

```bash
python3 scripts/screenplay.py --session D init "Edge Cases" D "Test error handling"
python3 scripts/screenplay.py --session D step "Show pikachu" "pokedex pokemon show pikachu" \
  --exit-code 0 --equals "data.name=pikachu" --has-fields "data.types,data.stats"
python3 scripts/screenplay.py --session D done
```

### Data Quality

Known upstream data issues are tracked in `data/known_issues.md`. The curated override system in `data/overrides/` fixes issues we can correct locally (evolution methods, form defaults, encounter version-exclusive swaps).

## Install

Requires Rust 1.85+ (edition 2024).

```bash
./install.sh                   # builds release binary, copies to /usr/local/bin
pokedex db seed                # download data and build local DB
```

The install script runs `cargo build --release` and copies the binary to `/usr/local/bin/pokedex`. The binary is fully self-contained (7MB) — no runtime dependencies, no system SQLite needed.
