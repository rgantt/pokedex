use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use tar::Archive;

const POKEAPI_TARBALL_URL: &str =
    "https://github.com/PokeAPI/pokeapi/archive/refs/heads/master.tar.gz";

const ENGLISH_LANGUAGE_ID: &str = "9";

/// Files to skip entirely (Conquest, contests, Pokeathlon, Pal Park, battle styles, etc.)
const SKIP_PREFIXES: &[&str] = &[
    "conquest_",
    "contest_",
    "super_contest_",
    "pokeathlon_",
    "pal_park",
    "nature_battle_style",
    "move_battle_style",
    "characteristic",
    "type_game_indices",
    "location_game_indices",
    "location_area_encounter_rates",
    "berry_flavors",  // contest-oriented
];

pub fn download_and_extract(keep_cache: bool) -> Result<PathBuf> {
    let cache_dir = cache_path();
    std::fs::create_dir_all(&cache_dir)?;

    let csv_dir = cache_dir.join("csv");
    if csv_dir.exists() && csv_dir.read_dir()?.next().is_some() {
        eprintln!("Using cached CSVs at {}", csv_dir.display());
        return Ok(csv_dir);
    }

    eprintln!("Downloading PokeAPI dataset...");
    let response = reqwest::blocking::get(POKEAPI_TARBALL_URL)
        .context("Failed to download PokeAPI tarball")?;
    let bytes = response.bytes().context("Failed to read response")?;
    eprintln!("Downloaded {:.1} MB", bytes.len() as f64 / 1_000_000.0);

    eprintln!("Extracting CSVs...");
    std::fs::create_dir_all(&csv_dir)?;

    let decoder = GzDecoder::new(&bytes[..]);
    let mut archive = Archive::new(decoder);

    let mut extracted = 0;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        let path_str = path.to_string_lossy();

        // We want files under pokeapi-master/data/v2/csv/
        if !path_str.contains("data/v2/csv/") || !path_str.ends_with(".csv") {
            continue;
        }

        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_default();

        if should_skip(&filename) {
            continue;
        }

        let dest = csv_dir.join(&filename);
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        std::fs::write(&dest, &contents)?;
        extracted += 1;
    }

    eprintln!("Extracted {extracted} CSV files");

    if !keep_cache {
        // We'll clean up after seeding, not here
    }

    Ok(csv_dir)
}

fn cache_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".pokedex").join("cache")
}

pub fn clear_cache() -> Result<()> {
    let cache = cache_path();
    if cache.exists() {
        std::fs::remove_dir_all(&cache)?;
    }
    Ok(())
}

fn should_skip(filename: &str) -> bool {
    let stem = filename.trim_end_matches(".csv");
    SKIP_PREFIXES.iter().any(|prefix| stem.starts_with(prefix))
}

pub fn seed_from_directory(conn: &mut Connection, csv_dir: &Path) -> Result<()> {
    // Load CSVs into a map for easy access
    let csvs = load_csvs(csv_dir)?;

    // Disable FK checks during bulk load — we control insertion order but self-referential FKs
    // (e.g. species.evolves_from_species_id) require it
    conn.execute_batch("PRAGMA foreign_keys=OFF;")?;

    let tx = conn.transaction()?;

    // Order matters due to foreign keys
    seed_table_mapped(&tx, &csvs, "types", "types.csv", &[("id", "id"), ("name", "identifier")])?;
    seed_type_efficacy(&tx, &csvs)?;
    seed_table_mapped(&tx, &csvs, "regions", "regions.csv", &[("id", "id"), ("name", "identifier")])?;
    seed_table_mapped(&tx, &csvs, "generations", "generations.csv", &[("id", "id"), ("name", "identifier"), ("region_id", "main_region_id")])?;
    seed_table_mapped(&tx, &csvs, "version_groups", "version_groups.csv", &[("id", "id"), ("name", "identifier"), ("generation_id", "generation_id")])?;
    seed_table_mapped(&tx, &csvs, "versions", "versions.csv", &[("id", "id"), ("name", "identifier"), ("version_group_id", "version_group_id")])?;
    seed_table_mapped(&tx, &csvs, "stats", "stats.csv", &[("id", "id"), ("name", "identifier"), ("is_battle_only", "is_battle_only"), ("game_index", "game_index")])?;
    seed_table_mapped(&tx, &csvs, "growth_rates", "growth_rates.csv", &[("id", "id"), ("name", "identifier")])?;
    seed_table_mapped(&tx, &csvs, "egg_groups", "egg_groups.csv", &[("id", "id"), ("name", "identifier")])?;
    seed_table_mapped(&tx, &csvs, "evolution_chains", "evolution_chains.csv", &[("id", "id"), ("baby_trigger_item_id", "baby_trigger_item_id")])?;
    seed_table_mapped(&tx, &csvs, "evolution_triggers", "evolution_triggers.csv", &[("id", "id"), ("name", "identifier")])?;
    seed_table_mapped(&tx, &csvs, "move_damage_classes", "move_damage_classes.csv", &[("id", "id"), ("name", "identifier")])?;
    seed_table_mapped(&tx, &csvs, "encounter_methods", "encounter_methods.csv", &[("id", "id"), ("name", "identifier"), ("order_col", "order")])?;
    seed_table_mapped(&tx, &csvs, "encounter_conditions", "encounter_conditions.csv", &[("id", "id"), ("name", "identifier")])?;

    seed_species(&tx, &csvs)?;
    seed_pokemon(&tx, &csvs)?;
    seed_pokemon_types(&tx, &csvs)?;
    seed_pokemon_forms(&tx, &csvs)?;
    seed_pokemon_form_types(&tx, &csvs)?;
    seed_pokemon_stats(&tx, &csvs)?;
    seed_pokemon_egg_groups(&tx, &csvs)?;
    seed_evolution(&tx, &csvs)?;

    seed_abilities(&tx, &csvs)?;
    seed_pokemon_abilities(&tx, &csvs)?;

    seed_moves(&tx, &csvs)?;
    seed_pokemon_moves(&tx, &csvs)?;
    seed_move_meta(&tx, &csvs)?;

    seed_items(&tx, &csvs)?;
    seed_item_categories(&tx, &csvs)?;
    seed_machines(&tx, &csvs)?;
    seed_pokemon_items(&tx, &csvs)?;

    seed_natures(&tx, &csvs)?;

    seed_locations(&tx, &csvs)?;
    seed_location_areas(&tx, &csvs)?;
    seed_encounters(&tx, &csvs)?;
    dedup_encounters(&tx)?;
    seed_encounter_condition_values(&tx, &csvs)?;
    seed_encounter_condition_value_map(&tx, &csvs)?;

    seed_pokedexes(&tx, &csvs)?;
    seed_pokemon_dex_numbers(&tx, &csvs)?;
    seed_experience(&tx, &csvs)?;

    seed_version_group_regions(&tx, &csvs)?;
    seed_pokedex_version_groups(&tx, &csvs)?;
    seed_berries(&tx, &csvs)?;

    // Names (English only)
    seed_names(&tx, &csvs, "type_names", "type_names.csv", "type_id", &["name"])?;
    seed_names(&tx, &csvs, "ability_names", "ability_names.csv", "ability_id", &["name"])?;
    seed_species_names(&tx, &csvs)?;
    seed_names(&tx, &csvs, "move_names", "move_names.csv", "move_id", &["name"])?;
    seed_names(&tx, &csvs, "item_names", "item_names.csv", "item_id", &["name"])?;
    seed_names(&tx, &csvs, "location_names", "location_names.csv", "location_id", &["name"])?;
    seed_names(&tx, &csvs, "version_names", "version_names.csv", "version_id", &["name"])?;
    seed_names(&tx, &csvs, "nature_names", "nature_names.csv", "nature_id", &["name"])?;
    seed_names(&tx, &csvs, "stat_names", "stat_names.csv", "stat_id", &["name"])?;
    seed_names(&tx, &csvs, "generation_names", "generation_names.csv", "generation_id", &["name"])?;
    seed_names(&tx, &csvs, "region_names", "region_names.csv", "region_id", &["name"])?;
    seed_names(&tx, &csvs, "egg_group_names", "egg_group_prose.csv", "egg_group_id", &["name"])?;
    seed_names(&tx, &csvs, "encounter_method_names", "encounter_method_prose.csv", "encounter_method_id", &["name"])?;
    seed_names(&tx, &csvs, "encounter_condition_names", "encounter_condition_prose.csv", "encounter_condition_id", &["name"])?;
    seed_names(&tx, &csvs, "encounter_condition_value_names", "encounter_condition_value_prose.csv", "encounter_condition_value_id", &["name"])?;
    seed_names(&tx, &csvs, "pokedex_names", "pokedex_prose.csv", "pokedex_id", &["name"])?;
    seed_names(&tx, &csvs, "growth_rate_names", "growth_rate_prose.csv", "growth_rate_id", &["name"])?;
    seed_names(&tx, &csvs, "move_damage_class_names", "move_damage_class_prose.csv", "move_damage_class_id", &["name"])?;
    seed_names(&tx, &csvs, "item_category_names", "item_category_prose.csv", "item_category_id", &["name"])?;
    seed_pokemon_form_names(&tx, &csvs)?;

    // Prose/flavor text (English only)
    seed_ability_prose(&tx, &csvs)?;
    seed_move_effect_prose(&tx, &csvs)?;
    seed_item_prose(&tx, &csvs)?;
    seed_flavor_text(&tx, &csvs, "pokemon_species_flavor_text", "pokemon_species_flavor_text.csv", "species_id", "version_id")?;
    seed_flavor_text(&tx, &csvs, "move_flavor_text", "move_flavor_text.csv", "move_id", "version_group_id")?;
    seed_flavor_text(&tx, &csvs, "ability_flavor_text", "ability_flavor_text.csv", "ability_id", "version_group_id")?;
    seed_flavor_text(&tx, &csvs, "item_flavor_text", "item_flavor_text.csv", "item_id", "version_group_id")?;

    // Move and item flags
    seed_move_flags(&tx, &csvs)?;
    seed_item_flags(&tx, &csvs)?;

    // Encounter slots
    seed_encounter_slots(&tx, &csvs)?;

    // Update games table with version_group_ids
    update_games_version_groups(&tx)?;

    // Create game entries for all versions that have encounter data but aren't in the games table
    populate_games_from_versions(&tx)?;

    tx.commit()?;

    // Phase 2: Supplement with PokeDB.org encounter data (Gen 6+ games)
    eprintln!("Downloading PokeDB.org supplementary data...");
    match seed_pokedb_encounters(conn) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Warning: Failed to load PokeDB data (non-fatal): {e:#}");
            eprintln!("  PokeAPI data is loaded. Gen 6+ encounters will be missing.");
        }
    }

    // Phase 3: Load bundled Legends: Z-A encounter data (scraped from Serebii)
    eprintln!("Loading Legends: Z-A encounter data...");
    match seed_za_encounters(conn) {
        Ok(count) => eprintln!("  legends-za encounters inserted: {count} rows"),
        Err(e) => {
            eprintln!("Warning: Failed to load Z-A data (non-fatal): {e:#}");
        }
    }

    // Phase 4: Apply curated overrides
    eprintln!("Applying curated data overrides...");
    super::overrides::apply_overrides(conn)?;

    // Re-enable FK checks
    conn.execute_batch("PRAGMA foreign_keys=ON;")?;

    Ok(())
}

pub fn drop_reference_data(conn: &Connection) -> Result<()> {
    // Drop all reference data but keep collection and games tables
    let tables = [
        "encounter_condition_value_map", "encounter_condition_value_names",
        "encounter_condition_values", "encounter_condition_names", "encounters",
        "encounter_slots", "encounter_methods", "location_area_prose", "location_areas",
        "location_names", "locations", "pokemon_species_flavor_text",
        "move_flavor_text", "ability_flavor_text", "item_flavor_text",
        "move_meta_stat_changes", "move_meta", "move_flags", "move_flag_types",
        "item_flags", "item_flag_types", "pokemon_form_names",
        "move_damage_class_names", "growth_rate_names",
        "item_category_names", "pokedex_names",
        "pokemon_form_types", "pokemon_forms",
        "pokemon_dex_numbers", "pokedex_version_groups", "pokedexes",
        "pokemon_moves", "pokemon_move_methods",
        "machines", "pokemon_items",
        "pokemon_abilities", "ability_prose", "ability_names", "abilities",
        "pokemon_egg_groups", "egg_group_names", "egg_groups",
        "pokemon_stats", "pokemon_types", "pokemon",
        "pokemon_evolution", "evolution_triggers", "species",
        "evolution_chains", "natures", "nature_names",
        "moves", "move_names", "move_effect_prose", "move_damage_classes",
        "items", "item_names", "item_prose", "item_categories",
        "berries", "experience", "growth_rates",
        "version_group_regions", "versions", "version_names",
        "version_groups", "generations", "generation_names",
        "regions", "region_names", "stats", "stat_names",
        "type_efficacy", "type_names", "types",
        "encounter_conditions",
        "species_names",
    ];
    for table in &tables {
        conn.execute(&format!("DELETE FROM {table}"), [])?;
    }
    Ok(())
}

// ---- CSV loading ----

type CsvData = Vec<HashMap<String, String>>;

fn load_csvs(dir: &Path) -> Result<HashMap<String, CsvData>> {
    let mut map = HashMap::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map(|e| e == "csv").unwrap_or(false) {
            let filename = path.file_name().unwrap().to_string_lossy().to_string();
            let data = read_csv(&path)?;
            map.insert(filename, data);
        }
    }
    Ok(map)
}

fn read_csv(path: &Path) -> Result<CsvData> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .from_path(path)?;
    let headers: Vec<String> = reader.headers()?.iter().map(|s| s.to_string()).collect();
    let mut rows = Vec::new();
    for result in reader.records() {
        let record = result?;
        let mut row = HashMap::new();
        for (i, value) in record.iter().enumerate() {
            if let Some(header) = headers.get(i) {
                row.insert(header.clone(), value.to_string());
            }
        }
        rows.push(row);
    }
    Ok(rows)
}

fn get_csv<'a>(csvs: &'a HashMap<String, CsvData>, name: &str) -> Result<&'a CsvData> {
    csvs.get(name)
        .with_context(|| format!("CSV file not found: {name}"))
}

fn val<'a>(row: &'a HashMap<String, String>, key: &str) -> &'a str {
    row.get(key).map(|s| s.as_str()).unwrap_or("")
}

fn int(row: &HashMap<String, String>, key: &str) -> i64 {
    row.get(key)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0)
}

fn opt_int(row: &HashMap<String, String>, key: &str) -> Option<i64> {
    row.get(key).and_then(|s| {
        if s.is_empty() { None } else { s.parse::<i64>().ok() }
    })
}

fn opt_str(row: &HashMap<String, String>, key: &str) -> Option<String> {
    row.get(key).and_then(|s| {
        if s.is_empty() { None } else { Some(s.clone()) }
    })
}

// ---- Seed helpers ----

/// Seed a table from a CSV file. `col_mappings` is a list of (db_column, csv_column) pairs.
fn seed_table_mapped(
    tx: &rusqlite::Transaction,
    csvs: &HashMap<String, CsvData>,
    table: &str,
    csv_name: &str,
    col_mappings: &[(&str, &str)],
) -> Result<()> {
    let data = get_csv(csvs, csv_name)?;
    let db_cols: Vec<&str> = col_mappings.iter().map(|(db, _)| *db).collect();
    let placeholders: Vec<&str> = db_cols.iter().map(|_| "?").collect();
    let sql = format!(
        "INSERT OR IGNORE INTO {table} ({}) VALUES ({})",
        db_cols.join(", "),
        placeholders.join(", ")
    );
    let mut stmt = tx.prepare(&sql)?;
    let mut count = 0;
    for row in data {
        let params: Vec<rusqlite::types::Value> = col_mappings
            .iter()
            .map(|(_, csv_col)| {
                let v = val(row, csv_col);
                if v.is_empty() {
                    rusqlite::types::Value::Null
                } else if let Ok(n) = v.parse::<i64>() {
                    rusqlite::types::Value::Integer(n)
                } else {
                    rusqlite::types::Value::Text(v.to_string())
                }
            })
            .collect();
        stmt.execute(rusqlite::params_from_iter(params))?;
        count += 1;
    }
    eprintln!("  {table}: {count} rows");
    Ok(())
}

fn seed_species(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_species.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO species (id, name, generation_id, evolution_chain_id, \
         evolves_from_species_id, color_id, shape_id, habitat_id, gender_rate, \
         capture_rate, base_happiness, is_baby, is_legendary, is_mythical, \
         growth_rate_id, has_gender_differences, order_num) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            val(row, "identifier"),
            int(row, "generation_id"),
            opt_int(row, "evolution_chain_id"),
            opt_int(row, "evolves_from_species_id"),
            opt_int(row, "color_id"),
            opt_int(row, "shape_id"),
            opt_int(row, "habitat_id"),
            int(row, "gender_rate"),
            int(row, "capture_rate"),
            opt_int(row, "base_happiness"),
            int(row, "is_baby"),
            int(row, "is_legendary"),
            int(row, "is_mythical"),
            opt_int(row, "growth_rate_id"),
            int(row, "has_gender_differences"),
            int(row, "order"),
        ])?;
        count += 1;
    }
    eprintln!("  species: {count} rows");
    Ok(())
}

fn seed_pokemon(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon (id, species_id, name, height, weight, \
         base_experience, is_default, order_num) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            int(row, "species_id"),
            val(row, "identifier"),
            opt_int(row, "height"),
            opt_int(row, "weight"),
            opt_int(row, "base_experience"),
            int(row, "is_default"),
            int(row, "order"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon: {count} rows");
    Ok(())
}

fn seed_pokemon_types(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_types.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon_types (pokemon_id, type_id, slot) VALUES (?1, ?2, ?3)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "pokemon_id"),
            int(row, "type_id"),
            int(row, "slot"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon_types: {count} rows");
    Ok(())
}

fn seed_pokemon_forms(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_forms.csv")?;
    // Add introduced_in_version_group_id column if missing
    match tx.execute_batch("ALTER TABLE pokemon_forms ADD COLUMN introduced_in_version_group_id INTEGER;") {
        Ok(()) => {}
        Err(e) if e.to_string().contains("duplicate column") => {}
        Err(e) => return Err(e.into()),
    }
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon_forms (id, pokemon_id, name, form_name, \
         is_default, is_battle_only, is_mega, form_order, introduced_in_version_group_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            int(row, "pokemon_id"),
            val(row, "identifier"),
            opt_str(row, "form_identifier"),
            int(row, "is_default"),
            int(row, "is_battle_only"),
            int(row, "is_mega"),
            int(row, "form_order"),
            opt_int(row, "introduced_in_version_group_id"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon_forms: {count} rows");
    Ok(())
}

fn seed_pokemon_form_types(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("pokemon_form_types.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO pokemon_form_types (pokemon_form_id, type_id, slot) \
             VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "pokemon_form_id"),
                int(row, "type_id"),
                int(row, "slot"),
            ])?;
            count += 1;
        }
        eprintln!("  pokemon_form_types: {count} rows");
    }
    Ok(())
}

fn seed_pokemon_stats(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_stats.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon_stats (pokemon_id, stat_id, base_value, effort) \
         VALUES (?1, ?2, ?3, ?4)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "pokemon_id"),
            int(row, "stat_id"),
            int(row, "base_stat"),
            int(row, "effort"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon_stats: {count} rows");
    Ok(())
}

fn seed_pokemon_egg_groups(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_egg_groups.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon_egg_groups (species_id, egg_group_id) VALUES (?1, ?2)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "species_id"),
            int(row, "egg_group_id"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon_egg_groups: {count} rows");
    Ok(())
}

fn seed_evolution(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_evolution.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon_evolution (id, evolved_species_id, evolution_trigger_id, \
         trigger_item_id, minimum_level, gender_id, location_id, held_item_id, \
         time_of_day, known_move_id, known_move_type_id, minimum_happiness, \
         minimum_beauty, minimum_affection, relative_physical_stats, \
         party_species_id, party_type_id, trade_species_id, \
         needs_overworld_rain, turn_upside_down) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            int(row, "evolved_species_id"),
            int(row, "evolution_trigger_id"),
            opt_int(row, "trigger_item_id"),
            opt_int(row, "minimum_level"),
            opt_int(row, "gender_id"),
            opt_int(row, "location_id"),
            opt_int(row, "held_item_id"),
            opt_str(row, "time_of_day"),
            opt_int(row, "known_move_id"),
            opt_int(row, "known_move_type_id"),
            opt_int(row, "minimum_happiness"),
            opt_int(row, "minimum_beauty"),
            opt_int(row, "minimum_affection"),
            opt_int(row, "relative_physical_stats"),
            opt_int(row, "party_species_id"),
            opt_int(row, "party_type_id"),
            opt_int(row, "trade_species_id"),
            int(row, "needs_overworld_rain"),
            int(row, "turn_upside_down"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon_evolution: {count} rows");
    Ok(())
}

fn seed_type_efficacy(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "type_efficacy.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO type_efficacy (attacking_type_id, defending_type_id, damage_factor) \
         VALUES (?1, ?2, ?3)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "damage_type_id"),
            int(row, "target_type_id"),
            int(row, "damage_factor"),
        ])?;
        count += 1;
    }
    eprintln!("  type_efficacy: {count} rows");
    Ok(())
}

fn seed_abilities(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "abilities.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO abilities (id, name, generation_id, is_main_series) \
         VALUES (?1, ?2, ?3, ?4)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            val(row, "identifier"),
            int(row, "generation_id"),
            int(row, "is_main_series"),
        ])?;
        count += 1;
    }
    eprintln!("  abilities: {count} rows");
    Ok(())
}

fn seed_pokemon_abilities(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_abilities.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon_abilities (pokemon_id, ability_id, is_hidden, slot) \
         VALUES (?1, ?2, ?3, ?4)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "pokemon_id"),
            int(row, "ability_id"),
            int(row, "is_hidden"),
            int(row, "slot"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon_abilities: {count} rows");
    Ok(())
}

fn seed_moves(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "moves.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO moves (id, name, generation_id, type_id, power, pp, \
         accuracy, priority, damage_class_id, effect_id, effect_chance) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            val(row, "identifier"),
            int(row, "generation_id"),
            int(row, "type_id"),
            opt_int(row, "power"),
            opt_int(row, "pp"),
            opt_int(row, "accuracy"),
            int(row, "priority"),
            opt_int(row, "damage_class_id"),
            opt_int(row, "effect_id"),
            opt_int(row, "effect_chance"),
        ])?;
        count += 1;
    }
    eprintln!("  moves: {count} rows");
    Ok(())
}

fn seed_pokemon_moves(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_moves.csv")?;
    // This is the largest table. Use a simpler insert.
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon_moves (pokemon_id, version_group_id, move_id, \
         pokemon_move_method_id, level, order_col) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )?;

    // Also seed the methods table
    if let Some(methods) = csvs.get("pokemon_move_methods.csv") {
        let mut mstmt = tx.prepare(
            "INSERT OR IGNORE INTO pokemon_move_methods (id, name) VALUES (?1, ?2)"
        )?;
        for row in methods {
            mstmt.execute(rusqlite::params![int(row, "id"), val(row, "identifier")])?;
        }
    }

    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "pokemon_id"),
            int(row, "version_group_id"),
            int(row, "move_id"),
            int(row, "pokemon_move_method_id"),
            int(row, "level"),
            opt_int(row, "order"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon_moves: {count} rows");
    Ok(())
}

fn seed_move_meta(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("move_meta.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO move_meta (move_id, meta_category_id, meta_ailment_id, \
             min_hits, max_hits, min_turns, max_turns, drain, healing, crit_rate, \
             ailment_chance, flinch_chance, stat_chance) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "move_id"),
                int(row, "meta_category_id"),
                int(row, "meta_ailment_id"),
                opt_int(row, "min_hits"),
                opt_int(row, "max_hits"),
                opt_int(row, "min_turns"),
                opt_int(row, "max_turns"),
                int(row, "drain"),
                int(row, "healing"),
                int(row, "crit_rate"),
                int(row, "ailment_chance"),
                int(row, "flinch_chance"),
                int(row, "stat_chance"),
            ])?;
            count += 1;
        }
        eprintln!("  move_meta: {count} rows");
    }
    if let Some(data) = csvs.get("move_meta_stat_changes.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO move_meta_stat_changes (move_id, stat_id, change) \
             VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "move_id"),
                int(row, "stat_id"),
                int(row, "change"),
            ])?;
            count += 1;
        }
        eprintln!("  move_meta_stat_changes: {count} rows");
    }
    Ok(())
}

fn seed_items(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "items.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO items (id, name, category_id, cost, fling_power, fling_effect_id) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            val(row, "identifier"),
            opt_int(row, "category_id"),
            opt_int(row, "cost"),
            opt_int(row, "fling_power"),
            opt_int(row, "fling_effect_id"),
        ])?;
        count += 1;
    }
    eprintln!("  items: {count} rows");
    Ok(())
}

fn seed_item_categories(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("item_categories.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO item_categories (id, name, pocket_id) VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "id"),
                val(row, "identifier"),
                opt_int(row, "pocket_id"),
            ])?;
            count += 1;
        }
        eprintln!("  item_categories: {count} rows");
    }
    Ok(())
}

fn seed_machines(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("machines.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO machines (machine_number, version_group_id, item_id, move_id) \
             VALUES (?1, ?2, ?3, ?4)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "machine_number"),
                int(row, "version_group_id"),
                int(row, "item_id"),
                int(row, "move_id"),
            ])?;
            count += 1;
        }
        eprintln!("  machines: {count} rows");
    }
    Ok(())
}

fn seed_pokemon_items(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("pokemon_items.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO pokemon_items (pokemon_id, version_id, item_id, rarity) \
             VALUES (?1, ?2, ?3, ?4)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "pokemon_id"),
                int(row, "version_id"),
                int(row, "item_id"),
                int(row, "rarity"),
            ])?;
            count += 1;
        }
        eprintln!("  pokemon_items: {count} rows");
    }
    Ok(())
}

fn seed_natures(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "natures.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO natures (id, name, decreased_stat_id, increased_stat_id, \
         hates_flavor_id, likes_flavor_id, game_index) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            val(row, "identifier"),
            opt_int(row, "decreased_stat_id"),
            opt_int(row, "increased_stat_id"),
            opt_int(row, "hates_flavor_id"),
            opt_int(row, "likes_flavor_id"),
            opt_int(row, "game_index"),
        ])?;
        count += 1;
    }
    eprintln!("  natures: {count} rows");
    Ok(())
}

fn seed_locations(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "locations.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO locations (id, region_id, name) VALUES (?1, ?2, ?3)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            opt_int(row, "region_id"),
            val(row, "identifier"),
        ])?;
        count += 1;
    }
    eprintln!("  locations: {count} rows");
    Ok(())
}

fn seed_location_areas(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "location_areas.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO location_areas (id, location_id, name, game_index) \
         VALUES (?1, ?2, ?3, ?4)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            int(row, "location_id"),
            opt_str(row, "identifier"),
            opt_int(row, "game_index"),
        ])?;
        count += 1;
    }
    eprintln!("  location_areas: {count} rows");
    Ok(())
}

fn seed_encounter_slots(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("encounter_slots.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO encounter_slots (id, version_group_id, encounter_method_id, \
             slot, rarity) VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "id"),
                int(row, "version_group_id"),
                int(row, "encounter_method_id"),
                opt_int(row, "slot"),
                opt_int(row, "rarity"),
            ])?;
            count += 1;
        }
        eprintln!("  encounter_slots: {count} rows");
    }
    Ok(())
}

fn seed_encounters(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "encounters.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO encounters (id, version_id, location_area_id, encounter_slot_id, \
         pokemon_id, min_level, max_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            int(row, "version_id"),
            int(row, "location_area_id"),
            int(row, "encounter_slot_id"),
            int(row, "pokemon_id"),
            int(row, "min_level"),
            int(row, "max_level"),
        ])?;
        count += 1;
    }
    eprintln!("  encounters: {count} rows");
    Ok(())
}

fn dedup_encounters(tx: &rusqlite::Transaction) -> Result<()> {
    let removed: usize = tx.execute(
        "DELETE FROM encounters WHERE id NOT IN ( \
         SELECT MIN(id) FROM encounters \
         GROUP BY pokemon_id, version_id, location_area_id, min_level, max_level \
         )",
        [],
    )?;
    if removed > 0 {
        eprintln!("  encounters dedup: removed {removed} duplicate rows");
    }
    Ok(())
}

fn seed_encounter_condition_values(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("encounter_condition_values.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO encounter_condition_values (id, encounter_condition_id, name, is_default) \
             VALUES (?1, ?2, ?3, ?4)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "id"),
                int(row, "encounter_condition_id"),
                val(row, "identifier"),
                int(row, "is_default"),
            ])?;
            count += 1;
        }
        eprintln!("  encounter_condition_values: {count} rows");
    }
    Ok(())
}

fn seed_encounter_condition_value_map(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("encounter_condition_value_map.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO encounter_condition_value_map (encounter_id, encounter_condition_value_id) \
             VALUES (?1, ?2)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "encounter_id"),
                int(row, "encounter_condition_value_id"),
            ])?;
            count += 1;
        }
        eprintln!("  encounter_condition_value_map: {count} rows");
    }
    Ok(())
}

fn seed_pokedexes(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokedexes.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokedexes (id, name, region_id, is_main_series) \
         VALUES (?1, ?2, ?3, ?4)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "id"),
            val(row, "identifier"),
            opt_int(row, "region_id"),
            int(row, "is_main_series"),
        ])?;
        count += 1;
    }
    eprintln!("  pokedexes: {count} rows");
    Ok(())
}

fn seed_pokemon_dex_numbers(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    let data = get_csv(csvs, "pokemon_dex_numbers.csv")?;
    let mut stmt = tx.prepare(
        "INSERT OR IGNORE INTO pokemon_dex_numbers (species_id, pokedex_id, pokedex_number) \
         VALUES (?1, ?2, ?3)"
    )?;
    let mut count = 0;
    for row in data {
        stmt.execute(rusqlite::params![
            int(row, "species_id"),
            int(row, "pokedex_id"),
            int(row, "pokedex_number"),
        ])?;
        count += 1;
    }
    eprintln!("  pokemon_dex_numbers: {count} rows");
    Ok(())
}

fn seed_experience(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("experience.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO experience (growth_rate_id, level, experience) VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "growth_rate_id"),
                int(row, "level"),
                int(row, "experience"),
            ])?;
            count += 1;
        }
        eprintln!("  experience: {count} rows");
    }
    Ok(())
}

fn seed_version_group_regions(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("version_group_regions.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO version_group_regions (version_group_id, region_id) VALUES (?1, ?2)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "version_group_id"),
                int(row, "region_id"),
            ])?;
            count += 1;
        }
        eprintln!("  version_group_regions: {count} rows");
    }
    Ok(())
}

fn seed_pokedex_version_groups(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("pokedex_version_groups.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO pokedex_version_groups (pokedex_id, version_group_id) VALUES (?1, ?2)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "pokedex_id"),
                int(row, "version_group_id"),
            ])?;
            count += 1;
        }
        eprintln!("  pokedex_version_groups: {count} rows");
    }
    Ok(())
}

fn seed_berries(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("berries.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO berries (id, item_id, natural_gift_power, natural_gift_type_id, \
             size, max_harvest, growth_time, soil_dryness, smoothness) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "id"),
                int(row, "item_id"),
                opt_int(row, "natural_gift_power"),
                opt_int(row, "natural_gift_type_id"),
                opt_int(row, "size"),
                opt_int(row, "max_harvest"),
                opt_int(row, "growth_time"),
                opt_int(row, "soil_dryness"),
                opt_int(row, "smoothness"),
            ])?;
            count += 1;
        }
        eprintln!("  berries: {count} rows");
    }
    Ok(())
}

// ---- Name/prose tables (English only) ----

fn seed_names(
    tx: &rusqlite::Transaction,
    csvs: &HashMap<String, CsvData>,
    table: &str,
    csv_name: &str,
    id_col: &str,
    extra_cols: &[&str],
) -> Result<()> {
    if let Some(data) = csvs.get(csv_name) {
        let lang_col = if data.first().map(|r| r.contains_key("local_language_id")).unwrap_or(false) {
            "local_language_id"
        } else {
            "language_id"
        };

        let all_cols: Vec<String> = std::iter::once(id_col.to_string())
            .chain(extra_cols.iter().map(|s| s.to_string()))
            .collect();
        let placeholders: Vec<&str> = all_cols.iter().map(|_| "?").collect();
        let sql = format!(
            "INSERT OR IGNORE INTO {table} ({}) VALUES ({})",
            all_cols.join(", "),
            placeholders.join(", ")
        );
        let mut stmt = tx.prepare(&sql)?;
        let mut count = 0;
        for row in data {
            if val(row, lang_col) != ENGLISH_LANGUAGE_ID {
                continue;
            }
            let params: Vec<rusqlite::types::Value> = all_cols
                .iter()
                .map(|col| {
                    let v = val(row, col);
                    if v.is_empty() {
                        rusqlite::types::Value::Null
                    } else if let Ok(n) = v.parse::<i64>() {
                        rusqlite::types::Value::Integer(n)
                    } else {
                        rusqlite::types::Value::Text(v.to_string())
                    }
                })
                .collect();
            stmt.execute(rusqlite::params_from_iter(params))?;
            count += 1;
        }
        eprintln!("  {table}: {count} rows");
    }
    Ok(())
}

fn seed_species_names(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("pokemon_species_names.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO species_names (species_id, name, genus) VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            if val(row, "local_language_id") != ENGLISH_LANGUAGE_ID {
                continue;
            }
            stmt.execute(rusqlite::params![
                int(row, "pokemon_species_id"),
                val(row, "name"),
                opt_str(row, "genus"),
            ])?;
            count += 1;
        }
        eprintln!("  species_names: {count} rows");
    }
    Ok(())
}

fn seed_pokemon_form_names(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("pokemon_form_names.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO pokemon_form_names (pokemon_form_id, name, pokemon_name) \
             VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            if val(row, "local_language_id") != ENGLISH_LANGUAGE_ID {
                continue;
            }
            let name = val(row, "form_name");
            let pokemon_name = opt_str(row, "pokemon_name");
            if name.is_empty() && pokemon_name.is_none() {
                continue;
            }
            stmt.execute(rusqlite::params![
                int(row, "pokemon_form_id"),
                if name.is_empty() { pokemon_name.clone().unwrap_or_default() } else { name.to_string() },
                pokemon_name,
            ])?;
            count += 1;
        }
        eprintln!("  pokemon_form_names: {count} rows");
    }
    Ok(())
}

fn seed_ability_prose(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("ability_prose.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO ability_prose (ability_id, short_effect, effect) VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            if val(row, "local_language_id") != ENGLISH_LANGUAGE_ID {
                continue;
            }
            stmt.execute(rusqlite::params![
                int(row, "ability_id"),
                opt_str(row, "short_effect"),
                opt_str(row, "effect"),
            ])?;
            count += 1;
        }
        eprintln!("  ability_prose: {count} rows");
    }
    Ok(())
}

fn seed_move_effect_prose(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("move_effect_prose.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO move_effect_prose (move_effect_id, short_effect, effect) \
             VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            if val(row, "local_language_id") != ENGLISH_LANGUAGE_ID {
                continue;
            }
            stmt.execute(rusqlite::params![
                int(row, "move_effect_id"),
                opt_str(row, "short_effect"),
                opt_str(row, "effect"),
            ])?;
            count += 1;
        }
        eprintln!("  move_effect_prose: {count} rows");
    }
    Ok(())
}

fn seed_item_prose(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("item_prose.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO item_prose (item_id, short_effect, effect) VALUES (?1, ?2, ?3)"
        )?;
        let mut count = 0;
        for row in data {
            if val(row, "local_language_id") != ENGLISH_LANGUAGE_ID {
                continue;
            }
            stmt.execute(rusqlite::params![
                int(row, "item_id"),
                opt_str(row, "short_effect"),
                opt_str(row, "effect"),
            ])?;
            count += 1;
        }
        eprintln!("  item_prose: {count} rows");
    }
    Ok(())
}

fn seed_flavor_text(
    tx: &rusqlite::Transaction,
    csvs: &HashMap<String, CsvData>,
    table: &str,
    csv_name: &str,
    id1_col: &str,
    id2_col: &str,
) -> Result<()> {
    if let Some(data) = csvs.get(csv_name) {
        let sql = format!(
            "INSERT OR IGNORE INTO {table} ({id1_col}, {id2_col}, flavor_text) VALUES (?1, ?2, ?3)"
        );
        let mut stmt = tx.prepare(&sql)?;
        let mut count = 0;
        for row in data {
            if val(row, "language_id") != ENGLISH_LANGUAGE_ID {
                continue;
            }
            let text = val(row, "flavor_text");
            if text.is_empty() {
                continue;
            }
            stmt.execute(rusqlite::params![
                int(row, id1_col),
                int(row, id2_col),
                text,
            ])?;
            count += 1;
        }
        eprintln!("  {table}: {count} rows");
    }
    Ok(())
}

fn seed_move_flags(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("move_flags.csv") {
        // First seed flag types
        if let Some(flag_data) = csvs.get("move_flag_map.csv") {
            // move_flags.csv has the definitions, move_flag_map.csv has the mappings
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO move_flag_types (id, name) VALUES (?1, ?2)"
            )?;
            for row in data {
                stmt.execute(rusqlite::params![
                    int(row, "id"),
                    val(row, "identifier"),
                ])?;
            }

            let mut stmt2 = tx.prepare(
                "INSERT OR IGNORE INTO move_flags (move_id, move_flag_id) VALUES (?1, ?2)"
            )?;
            let mut count = 0;
            for row in flag_data {
                stmt2.execute(rusqlite::params![
                    int(row, "move_id"),
                    int(row, "move_flag_id"),
                ])?;
                count += 1;
            }
            eprintln!("  move_flags: {count} rows");
        } else {
            // Fallback: move_flags.csv might contain the map directly
            let mut stmt = tx.prepare(
                "INSERT OR IGNORE INTO move_flags (move_id, move_flag_id) VALUES (?1, ?2)"
            )?;
            let mut count = 0;
            for row in data {
                stmt.execute(rusqlite::params![
                    int(row, "move_id"),
                    int(row, "move_flag_id"),
                ])?;
                count += 1;
            }
            eprintln!("  move_flags: {count} rows");
        }
    }
    Ok(())
}

fn seed_item_flags(tx: &rusqlite::Transaction, csvs: &HashMap<String, CsvData>) -> Result<()> {
    if let Some(data) = csvs.get("item_flags.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO item_flag_types (id, name) VALUES (?1, ?2)"
        )?;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "id"),
                val(row, "identifier"),
            ])?;
        }
    }
    if let Some(data) = csvs.get("item_flag_map.csv") {
        let mut stmt = tx.prepare(
            "INSERT OR IGNORE INTO item_flags (item_id, item_flag_id) VALUES (?1, ?2)"
        )?;
        let mut count = 0;
        for row in data {
            stmt.execute(rusqlite::params![
                int(row, "item_id"),
                int(row, "item_flag_id"),
            ])?;
            count += 1;
        }
        eprintln!("  item_flags: {count} rows");
    }
    Ok(())
}

fn update_games_version_groups(tx: &rusqlite::Transaction) -> Result<()> {
    // Map our game names to PokeAPI version identifiers to link version_group_ids
    let mappings = [
        ("lets-go-pikachu", "lets-go-pikachu"),
        ("lets-go-eevee", "lets-go-eevee"),
        ("sword", "sword"),
        ("shield", "shield"),
        ("brilliant-diamond", "brilliant-diamond"),
        ("shining-pearl", "shining-pearl"),
        ("legends-arceus", "legends-arceus"),
        ("scarlet", "scarlet"),
        ("violet", "violet"),
    ];

    for (game_name, version_name) in &mappings {
        tx.execute(
            "UPDATE games SET version_group_id = (
                SELECT version_group_id FROM versions WHERE name = ?1
            ) WHERE name = ?2",
            rusqlite::params![version_name, game_name],
        )?;
    }
    Ok(())
}

/// Create game entries for all versions that have encounter data but aren't already
/// in the games table. This allows collection tracking for pre-HOME games (Red, Gold, etc).
fn populate_games_from_versions(tx: &rusqlite::Transaction) -> Result<()> {
    tx.execute(
        "INSERT OR IGNORE INTO games (name, version_group_id, connects_to_home, transfer_direction)
         SELECT v.name, v.version_group_id, 0, NULL
         FROM versions v
         WHERE v.id IN (SELECT DISTINCT version_id FROM encounters)
         AND v.name NOT IN (SELECT name FROM games)",
        [],
    )?;
    let count: i64 = tx.query_row("SELECT changes()", [], |row| row.get(0))?;
    if count > 0 {
        eprintln!("  pre-HOME games added: {count}");
    }
    Ok(())
}

// ============================================================
// PokeDB.org supplementary encounter data
// ============================================================

const POKEDB_BASE: &str = "https://cdn.pokedb.org/data_export_";

fn fetch_pokedb_json(table: &str) -> Result<Vec<serde_json::Value>> {
    let url = format!("{POKEDB_BASE}{table}_json");
    let response = reqwest::blocking::get(&url)
        .with_context(|| format!("Failed to fetch PokeDB {table}"))?;
    let data: Vec<serde_json::Value> = response.json()
        .with_context(|| format!("Failed to parse PokeDB {table} JSON"))?;
    Ok(data)
}

fn seed_pokedb_encounters(conn: &mut Connection) -> Result<()> {
    // Download all needed PokeDB tables
    let pokedb_locations = fetch_pokedb_json("locations")?;
    eprintln!("  pokedb locations: {} entries", pokedb_locations.len());

    let pokedb_location_areas = fetch_pokedb_json("location_areas")?;
    eprintln!("  pokedb location_areas: {} entries", pokedb_location_areas.len());

    let pokedb_methods = fetch_pokedb_json("encounter_methods")?;
    eprintln!("  pokedb encounter_methods: {} entries", pokedb_methods.len());

    let pokedb_versions = fetch_pokedb_json("versions")?;
    eprintln!("  pokedb versions: {} entries", pokedb_versions.len());

    let pokedb_encounters = fetch_pokedb_json("encounters")?;
    eprintln!("  pokedb encounters: {} entries", pokedb_encounters.len());

    conn.execute_batch("PRAGMA foreign_keys=OFF;")?;
    let tx = conn.transaction()?;

    // Build region name -> region_id map from existing data
    let region_map = build_region_map(&tx)?;

    // Step 1: Insert PokeDB locations that don't already exist
    let location_id_map = seed_pokedb_locations(&tx, &pokedb_locations, &region_map)?;

    // Step 2: Insert PokeDB location areas
    let location_area_id_map = seed_pokedb_location_areas(&tx, &pokedb_location_areas, &location_id_map)?;

    // Step 3: Insert PokeDB encounter methods
    let method_id_map = seed_pokedb_methods(&tx, &pokedb_methods)?;

    // Step 3b: Generate display names for any encounter methods missing them
    tx.execute_batch(
        "INSERT OR IGNORE INTO encounter_method_names (encounter_method_id, name) \
         SELECT id, REPLACE(REPLACE(name, '-', ' '), 'npc', 'NPC') FROM encounter_methods \
         WHERE id NOT IN (SELECT encounter_method_id FROM encounter_method_names);"
    )?;

    // Step 4: Build version identifier -> version_id map
    let version_id_map = build_version_map(&tx, &pokedb_versions)?;

    // Step 5: Build pokemon form identifier -> pokemon_id map
    let pokemon_id_map = build_pokemon_map(&tx)?;

    // Step 6: Find versions already covered by PokeAPI encounters (skip PokeDB dupes)
    let pokeapi_versions = build_pokeapi_covered_versions(&tx)?;
    eprintln!("  pokeapi versions with encounters: {} (will skip PokeDB for these)", pokeapi_versions.len());

    // Step 7: Insert encounters (only for versions PokeAPI doesn't cover)
    let encounter_count = seed_pokedb_encounter_rows(
        &tx,
        &pokedb_encounters,
        &location_area_id_map,
        &method_id_map,
        &version_id_map,
        &pokemon_id_map,
        &pokeapi_versions,
    )?;

    eprintln!("  pokedb encounters inserted: {encounter_count} rows");

    // Step 8: Normalize probability_overall weights to percentages (D3)
    normalize_probability_weights(&tx)?;

    tx.commit()?;
    Ok(())
}

fn build_region_map(tx: &rusqlite::Transaction) -> Result<HashMap<String, i64>> {
    let mut stmt = tx.prepare("SELECT id, name FROM regions")?;
    let mut map = HashMap::new();
    for row in stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))? {
        let (id, name) = row?;
        map.insert(name.to_lowercase(), id);
    }
    Ok(map)
}

fn seed_pokedb_locations(
    tx: &rusqlite::Transaction,
    pokedb_locations: &[serde_json::Value],
    region_map: &HashMap<String, i64>,
) -> Result<HashMap<String, i64>> {
    // Start IDs above existing PokeAPI locations
    let max_id: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM locations", [], |row| row.get(0),
    )?;
    let mut next_id = max_id + 1;

    // Build existing location name -> id map
    let mut map: HashMap<String, i64> = HashMap::new();
    let mut stmt = tx.prepare("SELECT id, name FROM locations")?;
    for row in stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))? {
        let (id, name) = row?;
        map.insert(name.to_lowercase(), id);
    }

    let mut insert_stmt = tx.prepare(
        "INSERT OR IGNORE INTO locations (id, region_id, name) VALUES (?1, ?2, ?3)"
    )?;
    let mut name_insert = tx.prepare(
        "INSERT OR IGNORE INTO location_names (location_id, name) VALUES (?1, ?2)"
    )?;

    let mut new_count = 0;
    for loc in pokedb_locations {
        let identifier = loc["identifier"].as_str().unwrap_or("");
        let display_name = loc["name"].as_str().unwrap_or(identifier);
        let region_slug = loc["region_area_identifier"].as_str().unwrap_or("");

        if map.contains_key(&identifier.to_lowercase()) {
            continue;
        }

        let region_id = region_map.get(&region_slug.to_lowercase()).copied();
        let id = next_id;
        next_id += 1;

        insert_stmt.execute(rusqlite::params![id, region_id, identifier])?;
        name_insert.execute(rusqlite::params![id, display_name])?;
        map.insert(identifier.to_lowercase(), id);
        new_count += 1;
    }

    eprintln!("  pokedb new locations: {new_count}");
    Ok(map)
}

fn seed_pokedb_location_areas(
    tx: &rusqlite::Transaction,
    pokedb_areas: &[serde_json::Value],
    location_id_map: &HashMap<String, i64>,
) -> Result<HashMap<String, i64>> {
    let max_id: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM location_areas", [], |row| row.get(0),
    )?;
    let mut next_id = max_id + 1;

    // Build existing area name -> id map
    let mut map: HashMap<String, i64> = HashMap::new();
    let mut stmt = tx.prepare("SELECT id, name FROM location_areas WHERE name IS NOT NULL")?;
    for row in stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))? {
        let (id, name) = row?;
        map.insert(name.to_lowercase(), id);
    }

    let mut insert_stmt = tx.prepare(
        "INSERT OR IGNORE INTO location_areas (id, location_id, name) VALUES (?1, ?2, ?3)"
    )?;

    let mut new_count = 0;
    for area in pokedb_areas {
        let identifier = area["identifier"].as_str().unwrap_or("");
        let loc_identifier = area["location_identifier"].as_str().unwrap_or("");
        let display_name = area["name"].as_str().unwrap_or(identifier);

        if map.contains_key(&identifier.to_lowercase()) {
            continue;
        }

        let location_id = location_id_map.get(&loc_identifier.to_lowercase()).copied().unwrap_or(0);
        let id = next_id;
        next_id += 1;

        insert_stmt.execute(rusqlite::params![id, location_id, identifier])?;
        map.insert(identifier.to_lowercase(), id);

        // Also insert a name entry
        tx.execute(
            "INSERT OR IGNORE INTO location_names (location_id, name) VALUES (?1, ?2)",
            rusqlite::params![location_id, display_name],
        ).ok(); // Ignore if location_id=0

        new_count += 1;
    }

    eprintln!("  pokedb new location_areas: {new_count}");
    Ok(map)
}

fn seed_pokedb_methods(
    tx: &rusqlite::Transaction,
    pokedb_methods: &[serde_json::Value],
) -> Result<HashMap<String, i64>> {
    let max_id: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM encounter_methods", [], |row| row.get(0),
    )?;
    let mut next_id = max_id + 1;

    // Build existing method name -> id map
    let mut map: HashMap<String, i64> = HashMap::new();
    let mut stmt = tx.prepare("SELECT id, name FROM encounter_methods")?;
    for row in stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))? {
        let (id, name) = row?;
        map.insert(name.to_lowercase(), id);
    }

    let mut insert_stmt = tx.prepare(
        "INSERT OR IGNORE INTO encounter_methods (id, name) VALUES (?1, ?2)"
    )?;
    let mut name_stmt = tx.prepare(
        "INSERT OR IGNORE INTO encounter_method_names (encounter_method_id, name) VALUES (?1, ?2)"
    )?;

    let mut new_count = 0;
    for method in pokedb_methods {
        let identifier = method["identifier"].as_str().unwrap_or("");
        let display_name = method["name"].as_str().unwrap_or(identifier);

        if map.contains_key(&identifier.to_lowercase()) {
            continue;
        }

        let id = next_id;
        next_id += 1;

        insert_stmt.execute(rusqlite::params![id, identifier])?;
        name_stmt.execute(rusqlite::params![id, display_name])?;
        map.insert(identifier.to_lowercase(), id);
        new_count += 1;
    }

    eprintln!("  pokedb new encounter_methods: {new_count}");
    Ok(map)
}

fn build_pokeapi_covered_versions(tx: &rusqlite::Transaction) -> Result<std::collections::HashSet<i64>> {
    // Find which version IDs already have encounter data (from PokeAPI phase)
    let mut stmt = tx.prepare(
        "SELECT DISTINCT version_id FROM encounters"
    )?;
    let ids: std::collections::HashSet<i64> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(ids)
}

fn build_version_map(
    tx: &rusqlite::Transaction,
    pokedb_versions: &[serde_json::Value],
) -> Result<HashMap<String, i64>> {
    // Map PokeDB version identifiers to our version IDs
    // First, build our existing version name -> id map
    let mut our_versions: HashMap<String, i64> = HashMap::new();
    let mut stmt = tx.prepare("SELECT id, name FROM versions")?;
    for row in stmt.query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))? {
        let (id, name) = row?;
        our_versions.insert(name.to_lowercase(), id);
    }

    let mut map: HashMap<String, i64> = HashMap::new();

    // For each PokeDB version, try to find a match in our versions table
    // PokeDB uses slugs like "sword", "scarlet" — our versions use PokeAPI slugs
    let max_id: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM versions", [], |row| row.get(0),
    )?;
    let mut next_id = max_id + 1;

    // Also get max version_group_id
    let max_vg_id: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM version_groups", [], |row| row.get(0),
    )?;
    let mut next_vg_id = max_vg_id + 1;

    for ver in pokedb_versions {
        let identifier = ver["identifier"].as_str().unwrap_or("");
        let generation_id = ver["generation_id"].as_i64().unwrap_or(0);

        // Skip combo versions like "red-blue", "diamond-pearl" — encounters should
        // reference individual versions
        if identifier.contains('-') && !matches!(identifier,
            "brilliant-diamond" | "shining-pearl" | "legends-arceus" |
            "lets-go-pikachu" | "lets-go-eevee" |
            "omega-ruby" | "alpha-sapphire" | "ultra-sun" | "ultra-moon"
        ) {
            // Check if it's a combo version — skip it, encounters should use individual
            // version identifiers. But some combos might appear in encounter data,
            // so let's map them to the first individual version.
            let parts: Vec<&str> = identifier.split('-').collect();
            if parts.len() == 2 {
                // Try the first part as a version
                if let Some(&id) = our_versions.get(&parts[0].to_lowercase()) {
                    map.insert(identifier.to_lowercase(), id);
                }
            }
            continue;
        }

        // Try direct match
        if let Some(&id) = our_versions.get(&identifier.to_lowercase()) {
            map.insert(identifier.to_lowercase(), id);
            continue;
        }

        // No match — create a new version entry (for games PokeAPI doesn't have)
        if generation_id > 0 {
            // Create version group first
            let vg_id = next_vg_id;
            next_vg_id += 1;
            tx.execute(
                "INSERT OR IGNORE INTO version_groups (id, name, generation_id) VALUES (?1, ?2, ?3)",
                rusqlite::params![vg_id, identifier, generation_id],
            )?;

            let vid = next_id;
            next_id += 1;
            tx.execute(
                "INSERT OR IGNORE INTO versions (id, name, version_group_id) VALUES (?1, ?2, ?3)",
                rusqlite::params![vid, identifier, vg_id],
            )?;

            let display_name = ver["name"].as_str().unwrap_or(identifier);
            tx.execute(
                "INSERT OR IGNORE INTO version_names (version_id, name) VALUES (?1, ?2)",
                rusqlite::params![vid, display_name],
            )?;

            our_versions.insert(identifier.to_lowercase(), vid);
            map.insert(identifier.to_lowercase(), vid);
        }
    }

    // Ensure all our existing versions are also in the map
    for (name, id) in &our_versions {
        map.entry(name.clone()).or_insert(*id);
    }

    eprintln!("  pokedb version mappings: {}", map.len());
    Ok(map)
}

fn build_pokemon_map(tx: &rusqlite::Transaction) -> Result<HashMap<String, i64>> {
    // Map "species-form" identifiers to pokemon IDs
    // PokeDB uses "bulbasaur-default", "pikachu-alola", etc.
    // Our pokemon table uses "bulbasaur", "pikachu-alola", etc.
    let mut map: HashMap<String, i64> = HashMap::new();

    let mut stmt = tx.prepare(
        "SELECT p.id, p.name, p.species_id, p.is_default, s.name as species_name \
         FROM pokemon p JOIN species s ON s.id = p.species_id"
    )?;
    for row in stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
        ))
    })? {
        let (id, pokemon_name, _species_id, is_default, species_name) = row?;

        // Map the pokemon name directly (handles forms like "pikachu-alola")
        map.insert(pokemon_name.clone(), id);

        // Only the default pokemon gets the "{species}-default" mapping
        if is_default != 0 {
            map.insert(format!("{species_name}-default"), id);
            if !pokemon_name.contains('-') {
                map.insert(format!("{pokemon_name}-default"), id);
            }
        }
    }

    // PokeDB uses longer form identifiers than PokeAPI. Add aliases.
    // e.g. PokeDB "basculin-red-stripe" -> our "basculin-red-striped"
    //      PokeDB "shellos-east-sea" -> our "shellos" (default form)
    //      PokeDB "burmy-trash-cloak" -> our "burmy" (default, forms not separate pokemon)
    let aliases: &[(&str, &str)] = &[
        ("basculin-red-stripe", "basculin-red-striped"),
        ("basculin-blue-stripe", "basculin-blue-striped"),
        ("basculin-white-stripe", "basculin-white-striped"),
        ("shellos-east-sea", "shellos"),
        ("shellos-west-sea", "shellos"),
        ("gastrodon-east-sea", "gastrodon"),
        ("gastrodon-west-sea", "gastrodon"),
        ("eiscue-ice-face", "eiscue-ice"),
        ("burmy-trash-cloak", "burmy"),
        ("burmy-sandy-cloak", "burmy"),
        ("burmy-plant-cloak", "burmy"),
        ("wormadam-trash-cloak", "wormadam-trash"),
        ("wormadam-sandy-cloak", "wormadam-sandy"),
        ("wormadam-plant-cloak", "wormadam-plant"),
        ("minior-red-core", "minior-red"),
        ("minior-blue-core", "minior-blue"),
        ("minior-green-core", "minior-green"),
        ("minior-indigo-core", "minior-indigo"),
        ("minior-orange-core", "minior-orange"),
        ("minior-violet-core", "minior-violet"),
        ("minior-yellow-core", "minior-yellow"),
        ("unown-exclamation-mark", "unown"),
        ("unown-question-mark", "unown"),
        ("tauros-paldean-combat-breed", "tauros-paldea-combat-breed"),
        ("tauros-paldean-aqua-breed", "tauros-paldea-aqua-breed"),
        ("tauros-paldean-blaze-breed", "tauros-paldea-blaze-breed"),
        ("calyrex-ice-rider", "calyrex-ice"),
        ("calyrex-shadow-rider", "calyrex-shadow"),
        ("kyurem-black-activated", "kyurem-black"),
        ("kyurem-white-activated", "kyurem-white"),
    ];
    for &(alias, target) in aliases {
        if let Some(&id) = map.get(target) {
            map.insert(alias.to_string(), id);
        }
    }

    eprintln!("  pokemon form mappings: {}", map.len());
    Ok(map)
}

/// D3: Normalize raw probability_overall spawn weights to percentages.
///
/// PokeDB probability_overall values are relative weights within a
/// (location_area, version) group, not percentages. This function converts
/// them: for each group, divide each weight by the group sum and multiply
/// by 100, rounding to 1 decimal place (e.g. "7.2%").
fn normalize_probability_weights(tx: &rusqlite::Transaction) -> Result<()> {
    // Find all (location_area_id, version_id) groups that have numeric probability values
    let mut group_stmt = tx.prepare(
        "SELECT DISTINCT e.location_area_id, e.version_id \
         FROM encounter_details ed \
         JOIN encounters e ON e.id = ed.encounter_id \
         WHERE ed.probability_overall IS NOT NULL \
         AND ed.probability_overall != '' \
         AND CAST(ed.probability_overall AS REAL) > 0"
    )?;

    let groups: Vec<(i64, i64)> = group_stmt.query_map([], |row| {
        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
    })?.filter_map(|r| r.ok()).collect();

    let mut sum_stmt = tx.prepare(
        "SELECT SUM(CAST(ed.probability_overall AS REAL)) \
         FROM encounter_details ed \
         JOIN encounters e ON e.id = ed.encounter_id \
         WHERE e.location_area_id = ?1 AND e.version_id = ?2 \
         AND ed.probability_overall IS NOT NULL \
         AND CAST(ed.probability_overall AS REAL) > 0"
    )?;

    let mut update_stmt = tx.prepare(
        "UPDATE encounter_details SET probability_overall = \
         CAST(ROUND(CAST(probability_overall AS REAL) / ?1 * 100, 1) AS TEXT) || '%' \
         WHERE encounter_id IN ( \
           SELECT id FROM encounters \
           WHERE location_area_id = ?2 AND version_id = ?3 \
         ) \
         AND probability_overall IS NOT NULL \
         AND CAST(probability_overall AS REAL) > 0"
    )?;

    let mut normalized_groups = 0;
    for (area_id, version_id) in &groups {
        let total: f64 = sum_stmt.query_row(
            rusqlite::params![area_id, version_id],
            |row| row.get(0),
        ).unwrap_or(0.0);

        if total > 0.0 {
            update_stmt.execute(rusqlite::params![total, area_id, version_id])?;
            normalized_groups += 1;
        }
    }

    eprintln!("  probability_overall normalized: {normalized_groups} location/version groups");
    Ok(())
}

fn parse_level_range(levels: &str) -> (i64, i64) {
    let levels = levels.trim();
    if let Some((min, max)) = levels.split_once('-') {
        let min = min.trim().parse::<i64>().unwrap_or(1);
        let max = max.trim().parse::<i64>().unwrap_or(min);
        (min, max)
    } else if let Ok(level) = levels.parse::<i64>() {
        (level, level)
    } else {
        (1, 1)
    }
}

fn parse_rate(rate: Option<&str>) -> Option<i64> {
    let rate = rate?;
    let rate = rate.trim().trim_end_matches('%');
    if rate.eq_ignore_ascii_case("one") {
        Some(1)
    } else {
        rate.parse::<i64>().ok()
    }
}

fn seed_pokedb_encounter_rows(
    tx: &rusqlite::Transaction,
    encounters: &[serde_json::Value],
    location_area_map: &HashMap<String, i64>,
    method_map: &HashMap<String, i64>,
    version_map: &HashMap<String, i64>,
    pokemon_map: &HashMap<String, i64>,
    pokeapi_covered_versions: &std::collections::HashSet<i64>,
) -> Result<usize> {
    // We need encounter_slot_ids. Create synthetic slots for PokeDB data.
    let max_slot_id: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM encounter_slots", [], |row| row.get(0),
    )?;
    let mut next_slot_id = max_slot_id + 1;

    let max_enc_id: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM encounters", [], |row| row.get(0),
    )?;
    let mut next_enc_id = max_enc_id + 1;

    let mut slot_stmt = tx.prepare(
        "INSERT INTO encounter_slots (id, version_group_id, encounter_method_id, slot, rarity) \
         VALUES (?1, ?2, ?3, ?4, ?5)"
    )?;
    let mut enc_stmt = tx.prepare(
        "INSERT INTO encounters (id, version_id, location_area_id, encounter_slot_id, \
         pokemon_id, min_level, max_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
    )?;

    let mut detail_stmt = tx.prepare(
        "INSERT OR IGNORE INTO encounter_details ( \
         encounter_id, rate_overall, rate_morning, rate_day, rate_night, \
         during_any_time, during_morning, during_day, during_evening, during_night, \
         while_weather_overall, while_clear, while_harsh_sunlight, while_cloudy, while_blizzard, \
         weather_clear_rate, weather_cloudy_rate, weather_rain_rate, weather_thunderstorm_rate, \
         weather_snow_rate, weather_blizzard_rate, weather_harshsunlight_rate, \
         weather_sandstorm_rate, weather_fog_rate, \
         on_terrain_land, on_terrain_watersurface, on_terrain_underwater, \
         on_terrain_overland, on_terrain_sky, \
         probability_overall, probability_morning, probability_day, \
         probability_evening, probability_night, \
         group_rate, group_pokemon, alpha_levels, boulder_required, visible, \
         max_raid_perfect_ivs, max_raid_rate_1_star, max_raid_rate_2_star, \
         max_raid_rate_3_star, max_raid_rate_4_star, max_raid_rate_5_star, \
         tera_raid_star_level, hidden_ability_possible, note \
         ) VALUES ( \
         ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, \
         ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, \
         ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, \
         ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, \
         ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48 )"
    )?;

    // Cache: (version_group_id, method_id, rarity) -> slot_id
    let mut slot_cache: HashMap<(i64, i64, Option<i64>), i64> = HashMap::new();

    let mut count = 0;
    let mut skipped = 0;

    for enc in encounters {
        let form_id = enc["pokemon_form_identifier"].as_str().unwrap_or("");
        let area_id = enc["location_area_identifier"].as_str().unwrap_or("");
        let method_id_str = enc["encounter_method_identifier"].as_str().unwrap_or("");
        let levels = enc["levels"].as_str().unwrap_or("1");
        let rate_str = enc["rate_overall"].as_str();

        // Resolve pokemon
        let pokemon_id = match pokemon_map.get(&form_id.to_lowercase()) {
            Some(&id) => id,
            None => {
                // Try stripping -default or extracting species name
                let species = if let Some(base) = form_id.strip_suffix("-default") {
                    base
                } else if let Some(idx) = form_id.rfind('-') {
                    &form_id[..idx]
                } else {
                    form_id
                };
                match pokemon_map.get(&format!("{species}-default")) {
                    Some(&id) => id,
                    None => {
                        skipped += 1;
                        continue;
                    }
                }
            }
        };

        // Resolve location area
        let location_area_id = match location_area_map.get(&area_id.to_lowercase()) {
            Some(&id) => id,
            None => {
                skipped += 1;
                continue;
            }
        };

        // Resolve method
        let method_id = match method_map.get(&method_id_str.to_lowercase()) {
            Some(&id) => id,
            None => {
                skipped += 1;
                continue;
            }
        };

        // Resolve versions (array), skipping versions already covered by PokeAPI
        let version_ids: Vec<i64> = match enc["version_identifiers"].as_array() {
            Some(arr) => arr.iter()
                .filter_map(|v| v.as_str())
                .filter_map(|name| version_map.get(&name.to_lowercase()).copied())
                .filter(|id| !pokeapi_covered_versions.contains(id))
                .collect(),
            None => continue,
        };

        if version_ids.is_empty() {
            continue; // all versions for this encounter are covered by PokeAPI
        }

        let (min_level, max_level) = parse_level_range(levels);
        let rarity = parse_rate(rate_str);

        // Check if this encounter has any meaningful detail data
        let has_details = enc_has_details(enc);

        for &version_id in &version_ids {
            // Get version_group_id for slot
            let version_group_id: i64 = tx.query_row(
                "SELECT version_group_id FROM versions WHERE id = ?1",
                rusqlite::params![version_id],
                |row| row.get(0),
            ).unwrap_or(0);

            // Get or create encounter slot
            let slot_key = (version_group_id, method_id, rarity);
            let slot_id = if let Some(&sid) = slot_cache.get(&slot_key) {
                sid
            } else {
                let sid = next_slot_id;
                next_slot_id += 1;
                slot_stmt.execute(rusqlite::params![
                    sid, version_group_id, method_id, 0, rarity
                ])?;
                slot_cache.insert(slot_key, sid);
                sid
            };

            let enc_id = next_enc_id;
            next_enc_id += 1;
            enc_stmt.execute(rusqlite::params![
                enc_id, version_id, location_area_id, slot_id,
                pokemon_id, min_level, max_level
            ])?;

            // Insert encounter details if any fields are populated
            if has_details {
                // D11: Handle non-numeric probability_overall values
                let raw_prob = json_str(enc, "probability_overall");
                let (probability_overall, prob_note) = match raw_prob.as_deref() {
                    Some("one") => (None, Some("Fixed encounter".to_string())),
                    Some("choose one") => (None, Some("Starter choice".to_string())),
                    Some("two") => (None, Some("Fixed double encounter".to_string())),
                    Some("respawns") => (None, Some("Respawning encounter".to_string())),
                    Some("only one") => (None, Some("One-time encounter".to_string())),
                    Some("unlimited") => (None, Some("Unlimited encounters".to_string())),
                    _ => (raw_prob, None),
                };

                // Merge probability note with existing note_markup
                let base_note = json_str(enc, "note_markup");
                let note = match (base_note, prob_note) {
                    (Some(n), Some(p)) => Some(format!("{n}; {p}")),
                    (Some(n), None) => Some(n),
                    (None, Some(p)) => Some(p),
                    (None, None) => None,
                };

                detail_stmt.execute(rusqlite::params![
                    enc_id,
                    json_str(enc, "rate_overall"),
                    json_str(enc, "rate_morning"),
                    json_str(enc, "rate_day"),
                    json_str(enc, "rate_night"),
                    json_bool(enc, "during_any_time"),
                    json_bool(enc, "during_morning"),
                    json_bool(enc, "during_day"),
                    json_bool(enc, "during_evening"),
                    json_bool(enc, "during_night"),
                    json_bool(enc, "while_weather_overall"),
                    json_bool(enc, "while_clear"),
                    json_bool(enc, "while_harsh_sunlight"),
                    json_bool(enc, "while_cloudy"),
                    json_bool(enc, "while_blizzard"),
                    json_str(enc, "weather_clear_rate"),
                    json_str(enc, "weather_cloudy_rate"),
                    json_str(enc, "weather_rain_rate"),
                    json_str(enc, "weather_thunderstorm_rate"),
                    json_str(enc, "weather_snow_rate"),
                    json_str(enc, "weather_blizzard_rate"),
                    json_str(enc, "weather_harshsunlight_rate"),
                    json_str(enc, "weather_sandstorm_rate"),
                    json_str(enc, "weather_fog_rate"),
                    json_bool(enc, "on_terrain_land"),
                    json_bool(enc, "on_terrain_watersurface"),
                    json_bool(enc, "on_terrain_underwater"),
                    json_bool(enc, "on_terrain_overland"),
                    json_bool(enc, "on_terrain_sky"),
                    probability_overall,
                    json_str(enc, "probability_morning"),
                    json_str(enc, "probability_day"),
                    json_str(enc, "probability_evening"),
                    json_str(enc, "probability_night"),
                    json_str(enc, "group_rate"),
                    json_str(enc, "group_pokemon"),
                    json_str(enc, "alpha_levels"),
                    json_bool(enc, "boulder_required"),
                    json_bool(enc, "visible"),
                    json_str(enc, "max_raid_perfect_ivs"),
                    json_str(enc, "max_raid_rate_1_star"),
                    json_str(enc, "max_raid_rate_2_star"),
                    json_str(enc, "max_raid_rate_3_star"),
                    json_str(enc, "max_raid_rate_4_star"),
                    json_str(enc, "max_raid_rate_5_star"),
                    json_str(enc, "tera_raid_star_level"),
                    json_bool(enc, "hidden_ability_possible"),
                    note,
                ])?;
            }

            count += 1;
        }
    }

    if skipped > 0 {
        eprintln!("  pokedb encounters skipped (unmapped): {skipped}");
    }

    Ok(count)
}

fn json_str(obj: &serde_json::Value, key: &str) -> Option<String> {
    obj.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn json_bool(obj: &serde_json::Value, key: &str) -> Option<i64> {
    obj.get(key).and_then(|v| v.as_bool()).map(|b| b as i64)
}

fn enc_has_details(enc: &serde_json::Value) -> bool {
    // Check if any detail field is non-null
    let detail_fields = [
        "rate_overall", "rate_morning", "rate_day", "rate_night",
        "during_any_time", "during_morning", "during_day", "during_evening", "during_night",
        "while_weather_overall", "while_clear", "while_harsh_sunlight", "while_cloudy", "while_blizzard",
        "weather_clear_rate", "weather_cloudy_rate", "weather_rain_rate", "weather_thunderstorm_rate",
        "weather_snow_rate", "weather_blizzard_rate", "weather_harshsunlight_rate",
        "weather_sandstorm_rate", "weather_fog_rate",
        "on_terrain_land", "on_terrain_watersurface", "on_terrain_underwater",
        "on_terrain_overland", "on_terrain_sky",
        "probability_overall", "probability_morning", "probability_day",
        "probability_evening", "probability_night",
        "group_rate", "group_pokemon", "alpha_levels", "boulder_required", "visible",
        "max_raid_perfect_ivs", "max_raid_rate_1_star", "max_raid_rate_2_star",
        "max_raid_rate_3_star", "max_raid_rate_4_star", "max_raid_rate_5_star",
        "tera_raid_star_level", "hidden_ability_possible", "note_markup",
    ];
    detail_fields.iter().any(|&f| {
        enc.get(f).is_some_and(|v| !v.is_null())
    })
}

// ============================================================
// Legends: Z-A encounter data (bundled from Serebii scrape)
// ============================================================

const ZA_ENCOUNTERS_JSON: &str = include_str!("../../data/za_encounters.json");

fn seed_za_encounters(conn: &mut Connection) -> Result<usize> {
    let encounters: Vec<serde_json::Value> = serde_json::from_str(ZA_ENCOUNTERS_JSON)?;

    conn.execute_batch("PRAGMA foreign_keys=OFF;")?;
    let tx = conn.transaction()?;

    // Ensure legends-za version exists
    let version_id: i64 = match tx.query_row(
        "SELECT id FROM versions WHERE name = 'legends-za'",
        [],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            // Create version group and version for Z-A
            let max_vg: i64 = tx.query_row(
                "SELECT COALESCE(MAX(id), 0) FROM version_groups", [], |row| row.get(0),
            )?;
            let vg_id = max_vg + 1;
            // Z-A is Gen 9 per PokeDB
            tx.execute(
                "INSERT OR IGNORE INTO version_groups (id, name, generation_id) VALUES (?1, 'legends-za', 9)",
                rusqlite::params![vg_id],
            )?;
            let max_v: i64 = tx.query_row(
                "SELECT COALESCE(MAX(id), 0) FROM versions", [], |row| row.get(0),
            )?;
            let vid = max_v + 1;
            tx.execute(
                "INSERT INTO versions (id, name, version_group_id) VALUES (?1, 'legends-za', ?2)",
                rusqlite::params![vid, vg_id],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO version_names (version_id, name) VALUES (?1, 'Legends: Z-A')",
                rusqlite::params![vid],
            )?;
            vid
        }
    };

    let version_group_id: i64 = tx.query_row(
        "SELECT version_group_id FROM versions WHERE id = ?1",
        rusqlite::params![version_id],
        |row| row.get(0),
    )?;

    // Ensure legends-za game exists
    tx.execute(
        "INSERT OR IGNORE INTO games (name, connects_to_home, transfer_direction) \
         VALUES ('legends-za', 1, 'both')",
        [],
    )?;

    // Link games table to the version_group
    tx.execute(
        "UPDATE games SET version_group_id = ?1 WHERE name = 'legends-za'",
        rusqlite::params![version_group_id],
    )?;

    // Get or create the encounter method for symbol-encounter
    let method_id: i64 = match tx.query_row(
        "SELECT id FROM encounter_methods WHERE name = 'symbol-encounter'",
        [],
        |row| row.get(0),
    ) {
        Ok(id) => id,
        Err(_) => {
            let max: i64 = tx.query_row(
                "SELECT COALESCE(MAX(id), 0) FROM encounter_methods", [], |row| row.get(0),
            )?;
            let id = max + 1;
            tx.execute(
                "INSERT INTO encounter_methods (id, name) VALUES (?1, 'symbol-encounter')",
                rusqlite::params![id],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO encounter_method_names (encounter_method_id, name) \
                 VALUES (?1, 'Symbol Encounter')",
                rusqlite::params![id],
            )?;
            id
        }
    };

    // Create a single encounter slot for Z-A
    let max_slot: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM encounter_slots", [], |row| row.get(0),
    )?;
    let slot_id = max_slot + 1;
    tx.execute(
        "INSERT INTO encounter_slots (id, version_group_id, encounter_method_id, slot, rarity) \
         VALUES (?1, ?2, ?3, 0, NULL)",
        rusqlite::params![slot_id, version_group_id, method_id],
    )?;

    // Create location + location_area for each wild zone
    let max_loc: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM locations", [], |row| row.get(0),
    )?;
    let max_area: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM location_areas", [], |row| row.get(0),
    )?;

    // Find Kalos region ID
    let kalos_region_id: Option<i64> = tx.query_row(
        "SELECT id FROM regions WHERE name = 'kalos'",
        [],
        |row| row.get(0),
    ).ok();

    // Link version_group to Kalos region
    if let Some(kalos_id) = kalos_region_id {
        tx.execute(
            "INSERT OR IGNORE INTO version_group_regions (version_group_id, region_id) VALUES (?1, ?2)",
            rusqlite::params![version_group_id, kalos_id],
        )?;
    }

    let mut loc_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut area_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    let mut next_loc_id = max_loc + 1;
    let mut next_area_id = max_area + 1;

    // Pre-create all 20 wild zones
    for zone in 1..=20 {
        let area_slug = format!("wild-zone-{zone}");
        let display_name = format!("Wild Zone {zone}");

        // Check if location already exists (from PokeDB phase)
        let existing_loc: Option<i64> = tx.query_row(
            "SELECT l.id FROM locations l \
             LEFT JOIN location_names ln ON ln.location_id = l.id \
             WHERE l.name = ?1 OR ln.name = ?1",
            rusqlite::params![&area_slug],
            |row| row.get(0),
        ).ok();

        let loc_id = if let Some(id) = existing_loc {
            // Ensure the location_names entry has the correct display name
            tx.execute(
                "UPDATE location_names SET name = ?1 WHERE location_id = ?2",
                rusqlite::params![&display_name, id],
            )?;
            id
        } else {
            let id = next_loc_id;
            next_loc_id += 1;
            tx.execute(
                "INSERT OR IGNORE INTO locations (id, region_id, name) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, kalos_region_id, &area_slug],
            )?;
            tx.execute(
                "INSERT OR IGNORE INTO location_names (location_id, name) VALUES (?1, ?2)",
                rusqlite::params![id, &display_name],
            )?;
            id
        };
        loc_map.insert(area_slug.clone(), loc_id);

        // Check if area already exists
        let existing_area: Option<i64> = tx.query_row(
            "SELECT id FROM location_areas WHERE name = ?1",
            rusqlite::params![&area_slug],
            |row| row.get(0),
        ).ok();

        let area_id = if let Some(id) = existing_area {
            // Fix location_id in case it was created by PokeDB pointing to wrong location
            tx.execute(
                "UPDATE location_areas SET location_id = ?1 WHERE id = ?2",
                rusqlite::params![loc_id, id],
            )?;
            id
        } else {
            let id = next_area_id;
            next_area_id += 1;
            tx.execute(
                "INSERT OR IGNORE INTO location_areas (id, location_id, name) VALUES (?1, ?2, ?3)",
                rusqlite::params![id, loc_id, &area_slug],
            )?;
            id
        };
        area_map.insert(area_slug, area_id);
    }

    // Build pokemon name -> id map
    let mut pokemon_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    {
        let mut stmt = tx.prepare(
            "SELECT p.id, s.name FROM pokemon p \
             JOIN species s ON s.id = p.species_id \
             WHERE p.is_default = 1"
        )?;
        for row in stmt.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })? {
            let (id, name) = row?;
            pokemon_map.insert(name.to_lowercase(), id);
        }
    }

    // Insert encounters (scoped block to drop prepared statements before commit)
    let max_enc: i64 = tx.query_row(
        "SELECT COALESCE(MAX(id), 0) FROM encounters", [], |row| row.get(0),
    )?;
    let mut next_enc_id = max_enc + 1;
    let mut count = 0;
    let mut skipped = 0;

    {
        let mut enc_stmt = tx.prepare(
            "INSERT INTO encounters (id, version_id, location_area_id, encounter_slot_id, \
             pokemon_id, min_level, max_level) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;
        let mut detail_stmt = tx.prepare(
            "INSERT OR IGNORE INTO encounter_details (encounter_id, alpha_levels, during_any_time, \
             while_weather_overall, note) VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;

        for enc in &encounters {
            let pokemon_name = enc["pokemon_name"].as_str().unwrap_or("");
            let area = enc["area"].as_str().unwrap_or("");
            let min_level = enc["min_level"].as_i64().unwrap_or(1);
            let max_level = enc["max_level"].as_i64().unwrap_or(1);
            let is_alpha = enc["is_alpha_spawn"].as_bool().unwrap_or(false);
            let alpha_chance = enc.get("alpha_chance").and_then(|v| v.as_str());
            let alpha_levels = enc.get("alpha_levels").and_then(|v| v.as_str());

            let pokemon_id = match pokemon_map.get(pokemon_name) {
                Some(&id) => id,
                None => {
                    skipped += 1;
                    continue;
                }
            };

            let area_id = match area_map.get(area) {
                Some(&id) => id,
                None => {
                    skipped += 1;
                    continue;
                }
            };

            let enc_id = next_enc_id;
            next_enc_id += 1;

            enc_stmt.execute(rusqlite::params![
                enc_id, version_id, area_id, slot_id,
                pokemon_id, min_level, max_level
            ])?;

            // Build note from alpha info
            let note = if is_alpha {
                Some(format!("Alpha spawn ({})", alpha_chance.unwrap_or("100%")))
            } else {
                alpha_chance.map(|c| format!("Alpha chance: {c}"))
            };

            detail_stmt.execute(rusqlite::params![
                enc_id,
                alpha_levels,
                1i64, // during_any_time = true
                1i64, // while_weather_overall = true
                note,
            ])?;

            count += 1;
        }
    } // prepared statements dropped here

    if skipped > 0 {
        eprintln!("  legends-za encounters skipped (unmapped): {skipped}");
    }

    tx.commit()?;
    Ok(count)
}
