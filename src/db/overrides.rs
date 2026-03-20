use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::Deserialize;

// Embed override data at compile time
const EVOLUTION_OVERRIDES_JSON: &str = include_str!("../../data/overrides/evolutions.json");
const POKEMON_OVERRIDES_JSON: &str = include_str!("../../data/overrides/pokemon.json");

#[derive(Debug, Deserialize)]
struct EvolutionOverride {
    species: String,
    #[allow(dead_code)]
    trigger: String,
    trigger_detail: String,
}

#[derive(Debug, Deserialize)]
struct PokemonOverride {
    pokemon_name: String,
    set: PokemonOverrideFields,
}

#[derive(Debug, Deserialize)]
struct PokemonOverrideFields {
    is_default: Option<i64>,
}

/// Apply all curated data overrides to fix known PokeAPI data issues.
pub fn apply_overrides(conn: &mut Connection) -> Result<()> {
    let evo_count = apply_evolution_overrides(conn)
        .context("Failed to apply evolution overrides")?;
    eprintln!("  evolution overrides applied: {evo_count}");

    let pokemon_count = apply_pokemon_overrides(conn)
        .context("Failed to apply pokemon overrides")?;
    eprintln!("  pokemon overrides applied: {pokemon_count}");

    Ok(())
}

/// Add a trigger_detail column to pokemon_evolution if it doesn't already exist,
/// then update rows matching the override species + trigger.
fn apply_evolution_overrides(conn: &mut Connection) -> Result<usize> {
    let overrides: Vec<EvolutionOverride> =
        serde_json::from_str(EVOLUTION_OVERRIDES_JSON).context("Failed to parse evolutions.json")?;

    // Add trigger_detail column if missing (ALTER TABLE ADD COLUMN is idempotent-safe
    // when we catch the "duplicate column" error).
    match conn.execute_batch("ALTER TABLE pokemon_evolution ADD COLUMN trigger_detail TEXT;") {
        Ok(()) => {}
        Err(e) => {
            let msg = e.to_string();
            if !msg.contains("duplicate column") {
                return Err(e).context("Failed to add trigger_detail column");
            }
            // Column already exists — that's fine.
        }
    }

    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare(
            "UPDATE pokemon_evolution SET trigger_detail = ?1 \
             WHERE evolved_species_id = (SELECT id FROM species WHERE name = ?2) \
               AND evolution_trigger_id = (SELECT id FROM evolution_triggers WHERE name = ?3)",
        )?;

        for ov in &overrides {
            let rows = stmt.execute(rusqlite::params![ov.trigger_detail, ov.species, ov.trigger])?;
            if rows > 0 {
                count += rows;
            }
        }
    }

    tx.commit()?;
    Ok(count)
}

/// Update pokemon table fields for specific pokemon names.
fn apply_pokemon_overrides(conn: &mut Connection) -> Result<usize> {
    let overrides: Vec<PokemonOverride> =
        serde_json::from_str(POKEMON_OVERRIDES_JSON).context("Failed to parse pokemon.json")?;

    let tx = conn.transaction()?;
    let mut count = 0;

    {
        let mut stmt = tx.prepare(
            "UPDATE pokemon SET is_default = ?1 WHERE name = ?2",
        )?;

        for ov in &overrides {
            if let Some(is_default) = ov.set.is_default {
                let rows = stmt.execute(rusqlite::params![is_default, ov.pokemon_name])?;
                if rows > 0 {
                    count += rows;
                }
            }
        }
    }

    tx.commit()?;
    Ok(count)
}
