use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use std::collections::HashMap;

use super::models::*;

// ---- Regional form encounter mapping ----

#[derive(serde::Deserialize)]
struct RegionalEncounterEntry {
    species: Option<String>,
    form: Option<String>,
    games: Option<Vec<String>>,
    #[allow(dead_code)]
    comment: Option<String>,
    #[allow(dead_code)]
    wild_methods: Option<Vec<String>>,
}

/// Build a lookup: (species_name, game_slug) -> form_label
/// for annotating encounter pokemon_name with regional form info.
fn build_regional_form_map() -> HashMap<(String, String), String> {
    let json_str = include_str!("../../data/overrides/regional_encounters.json");
    let entries: Vec<RegionalEncounterEntry> = serde_json::from_str(json_str).unwrap_or_default();
    let mut map = HashMap::new();
    for entry in &entries {
        if let (Some(species), Some(form), Some(games)) = (&entry.species, &entry.form, &entry.games) {
            for game in games {
                map.insert((species.to_lowercase(), game.to_lowercase()), form.clone());
            }
        }
    }
    map
}

// ---- Pokemon queries ----

pub fn resolve_pokemon(conn: &Connection, identifier: &str) -> Result<Option<(i64, String)>> {
    // Try as ID first
    if let Ok(id) = identifier.parse::<i64>() {
        let result = conn.query_row(
            "SELECT s.id, s.name FROM species s WHERE s.id = ?1",
            params![id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        );
        if let Ok(r) = result {
            return Ok(Some(r));
        }
    }
    // Try by name (case-insensitive, with hyphens)
    let name = identifier.to_lowercase().replace(' ', "-");
    let result = conn.query_row(
        "SELECT s.id, s.name FROM species s WHERE s.name = ?1",
        params![name],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    );
    match result {
        Ok(r) => return Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => {},
        Err(e) => return Err(e.into()),
    }

    // Try pokemon table (handles form names like "growlithe-hisui")
    let result = conn.query_row(
        "SELECT p.species_id, s.name FROM pokemon p JOIN species s ON s.id = p.species_id WHERE LOWER(p.name) = LOWER(?1)",
        params![name],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    );
    match result {
        Ok(r) => return Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => {},
        Err(e) => return Err(e.into()),
    }

    // Try pokemon_forms.name for cosmetic forms (vivillon-polar, etc.)
    let form_result = conn.query_row(
        "SELECT p.species_id, s.name FROM pokemon_forms pf \
         JOIN pokemon p ON p.id = pf.pokemon_id \
         JOIN species s ON s.id = p.species_id \
         WHERE LOWER(pf.name) = LOWER(?1)",
        params![name],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
    );
    match form_result {
        Ok(r) => return Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => {},
        Err(e) => return Err(e.into()),
    }

    // Regional forms (e.g., growlithe-hisui) are already handled by the pokemon table
    // lookup above (lines 64-68), since they have their own pokemon.name entries.

    Ok(None)
}

/// Resolve a form name to its pokemon_id. Returns None if the identifier
/// is not a form name or matches the default form.
pub fn resolve_form_pokemon_id(conn: &Connection, identifier: &str) -> Result<Option<i64>> {
    let name = identifier.to_lowercase().replace(' ', "-");
    let result = conn.query_row(
        "SELECT p.id, p.is_default FROM pokemon p WHERE LOWER(p.name) = LOWER(?1)",
        params![name],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    );
    match result {
        Ok((pokemon_id, is_default)) => {
            if is_default == 1 { return Ok(None); } else { return Ok(Some(pokemon_id)); }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {},
        Err(e) => return Err(e.into()),
    }

    // Try pokemon_forms.name for cosmetic forms (vivillon-polar, etc.)
    let form_result = conn.query_row(
        "SELECT p.id, p.is_default FROM pokemon_forms pf \
         JOIN pokemon p ON p.id = pf.pokemon_id \
         WHERE LOWER(pf.name) = LOWER(?1)",
        params![name],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    );
    match form_result {
        Ok((pokemon_id, is_default)) => {
            if is_default == 1 { return Ok(None); } else { return Ok(Some(pokemon_id)); }
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => {},
        Err(e) => return Err(e.into()),
    }

    // Regional forms (e.g., growlithe-hisui) are already handled by the pokemon table
    // lookup above, since they have their own pokemon.name entries.

    Ok(None)
}

/// Get types for a specific pokemon_id (not necessarily the default form).
pub fn get_pokemon_types_by_pokemon_id(conn: &Connection, pokemon_id: i64) -> Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(tn.name, t.name) FROM pokemon_types pt \
         JOIN types t ON t.id = pt.type_id \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         WHERE pt.pokemon_id = ?1 \
         ORDER BY pt.slot"
    )?;
    let types: Vec<String> = stmt
        .query_map(params![pokemon_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(types)
}

/// Get the display name for a specific pokemon form by pokemon_id.
pub fn get_form_display_name(conn: &Connection, pokemon_id: i64) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT COALESCE(pfn.pokemon_name, pfn.name, pf.form_name) \
         FROM pokemon_forms pf \
         JOIN pokemon p ON p.id = pf.pokemon_id \
         LEFT JOIN pokemon_form_names pfn ON pfn.pokemon_form_id = pf.id \
         WHERE pf.pokemon_id = ?1 AND p.is_default = 0",
        params![pokemon_id],
        |row| row.get::<_, Option<String>>(0),
    );
    match result {
        Ok(name) => Ok(name),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_display_name(conn: &Connection, species_id: i64) -> Result<String> {
    conn.query_row(
        "SELECT COALESCE(sn.name, s.name) FROM species s LEFT JOIN species_names sn ON sn.species_id = s.id WHERE s.id = ?1",
        params![species_id],
        |row| row.get(0),
    ).map_err(Into::into)
}

pub fn get_species(conn: &Connection, species_id: i64) -> Result<Species> {
    let (name, generation, capture_rate, is_baby, is_legendary, is_mythical, evolves_from_id): (String, i64, i64, i64, i64, i64, Option<i64>) = conn.query_row(
        "SELECT s.name, s.generation_id, s.capture_rate, s.is_baby, s.is_legendary, s.is_mythical, s.evolves_from_species_id \
         FROM species s WHERE s.id = ?1",
        params![species_id],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
    )?;

    let display_name = get_display_name(conn, species_id)?;
    let types = get_species_types(conn, species_id)?;
    let genus = conn.query_row(
        "SELECT genus FROM species_names WHERE species_id = ?1",
        params![species_id],
        |row| row.get::<_, Option<String>>(0),
    ).unwrap_or(None);

    let evolves_from = evolves_from_id.map(|eid| get_display_name(conn, eid).unwrap_or_default());

    let mut egg_stmt = conn.prepare(
        "SELECT COALESCE(egn.name, eg.name) FROM pokemon_egg_groups peg \
         JOIN egg_groups eg ON eg.id = peg.egg_group_id \
         LEFT JOIN egg_group_names egn ON egn.egg_group_id = eg.id \
         WHERE peg.species_id = ?1 ORDER BY peg.egg_group_id"
    )?;
    let egg_groups: Vec<String> = egg_stmt
        .query_map(params![species_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let stats = get_pokemon_stats(conn, species_id).ok();
    let abilities = get_pokemon_abilities(conn, species_id).unwrap_or_default();

    Ok(Species {
        id: species_id,
        name,
        display_name,
        generation,
        types,
        capture_rate,
        is_baby: is_baby != 0,
        is_legendary: is_legendary != 0,
        is_mythical: is_mythical != 0,
        evolves_from,
        genus,
        egg_groups,
        stats,
        abilities,
    })
}

pub fn get_species_types(conn: &Connection, species_id: i64) -> Result<Vec<String>> {
    // Get types from the default pokemon entry for this species
    let mut stmt = conn.prepare(
        "SELECT COALESCE(tn.name, t.name) FROM pokemon_types pt \
         JOIN pokemon p ON p.id = pt.pokemon_id \
         JOIN types t ON t.id = pt.type_id \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         WHERE p.species_id = ?1 AND p.is_default = 1 \
         ORDER BY pt.slot"
    )?;
    let types: Vec<String> = stmt
        .query_map(params![species_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();
    Ok(types)
}

pub fn get_pokemon_abilities(conn: &Connection, species_id: i64) -> Result<Vec<AbilityInfo>> {
    let mut stmt = conn.prepare(
        "SELECT a.name, COALESCE(an.name, a.name), pa.is_hidden, \
         (SELECT ap.short_effect FROM ability_prose ap WHERE ap.ability_id = a.id LIMIT 1) \
         FROM pokemon_abilities pa \
         JOIN abilities a ON a.id = pa.ability_id \
         LEFT JOIN ability_names an ON an.ability_id = a.id \
         JOIN pokemon p ON p.id = pa.pokemon_id \
         WHERE p.species_id = ?1 AND p.is_default = 1 \
         ORDER BY pa.slot"
    )?;
    let abilities = stmt
        .query_map(params![species_id], |row| {
            Ok(AbilityInfo {
                name: row.get(0)?,
                display_name: row.get(1)?,
                is_hidden: row.get::<_, i64>(2)? != 0,
                short_effect: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(abilities)
}

pub fn list_species(
    conn: &Connection,
    type_filter: Option<&str>,
    generation: Option<u32>,
    category: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<(Vec<SpeciesSummary>, u64)> {
    let mut conditions = Vec::new();
    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

    if let Some(t) = type_filter {
        conditions.push(
            "EXISTS (SELECT 1 FROM pokemon_types pt JOIN pokemon p ON p.id = pt.pokemon_id \
             JOIN types ty ON ty.id = pt.type_id \
             LEFT JOIN type_names tn ON tn.type_id = ty.id \
             WHERE p.species_id = s.id AND p.is_default = 1 AND (LOWER(ty.name) = LOWER(?1) OR LOWER(tn.name) = LOWER(?1)))"
        );
        bind_values.push(Box::new(t.to_string()));
    }
    if let Some(g) = generation {
        let idx = bind_values.len() + 1;
        conditions.push(Box::leak(format!("s.generation_id = ?{idx}").into_boxed_str()));
        bind_values.push(Box::new(g as i64));
    }
    if let Some(cat) = category {
        match cat.to_lowercase().as_str() {
            "legendary" => conditions.push("s.is_legendary = 1"),
            "mythical" => conditions.push("s.is_mythical = 1"),
            "baby" => conditions.push("s.is_baby = 1"),
            _ => {}
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Count total
    let count_sql = format!("SELECT COUNT(*) FROM species s {where_clause}");
    let total: u64 = {
        let mut stmt = conn.prepare(&count_sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
        stmt.query_row(params.as_slice(), |row| row.get(0))?
    };

    let query = format!(
        "SELECT s.id, s.name, s.generation_id FROM species s {where_clause} ORDER BY s.id LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2,
    );

    let mut stmt = conn.prepare(&query)?;
    bind_values.push(Box::new(limit as i64));
    bind_values.push(Box::new(offset as i64));
    let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();

    let rows: Vec<(i64, String, i64)> = stmt
        .query_map(params.as_slice(), |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?
        .filter_map(|r| r.ok())
        .collect();

    let mut results = Vec::new();
    for (id, name, gen_id) in rows {
        let display_name = get_display_name(conn, id).unwrap_or_else(|_| name.clone());
        let types = get_species_types(conn, id).unwrap_or_default();
        results.push(SpeciesSummary {
            id,
            name,
            display_name,
            types,
            generation: gen_id,
        });
    }

    Ok((results, total))
}

pub fn search_species(conn: &Connection, query: &str, limit: u64) -> Result<Vec<SearchResult>> {
    let query_lower = query.to_lowercase();
    if query_lower.trim().is_empty() {
        return Ok(Vec::new());
    }

    // Get all species names for fuzzy matching
    let mut stmt = conn.prepare(
        "SELECT s.id, s.name, COALESCE(sn.name, s.name) as display_name \
         FROM species s LEFT JOIN species_names sn ON sn.species_id = s.id \
         ORDER BY s.id"
    )?;
    let all: Vec<(i64, String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let mut scored: Vec<(f64, i64, String, String)> = all
        .into_iter()
        .map(|(id, name, display)| {
            let name_lower = name.to_lowercase();
            let display_lower = display.to_lowercase();

            // Prioritize exact prefix matches, then use strsim
            let score = if name_lower == query_lower || display_lower == query_lower {
                1.0
            } else if name_lower.starts_with(&query_lower) || display_lower.starts_with(&query_lower) {
                0.95
            } else if name_lower.contains(&query_lower) || display_lower.contains(&query_lower) {
                0.9
            } else {
                let s1 = strsim::jaro_winkler(&query_lower, &name_lower);
                let s2 = strsim::jaro_winkler(&query_lower, &display_lower);
                f64::max(s1, s2)
            };
            (score, id, name, display)
        })
        .filter(|(score, _, _, _)| *score > 0.6)
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(limit as usize);

    let mut results = Vec::new();
    for (score, id, name, display_name) in scored {
        let types = get_species_types(conn, id).unwrap_or_default();
        let generation: i64 = conn.query_row(
            "SELECT generation_id FROM species WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )?;
        results.push(SearchResult {
            species: SpeciesSummary {
                id,
                name,
                display_name,
                types,
                generation,
            },
            score: (score * 100.0).round() / 100.0,
        });
    }

    Ok(results)
}

pub fn get_evolution_chain(conn: &Connection, species_id: i64) -> Result<EvolutionNode> {
    // Find the chain root
    let chain_id: Option<i64> = conn.query_row(
        "SELECT evolution_chain_id FROM species WHERE id = ?1",
        params![species_id],
        |row| row.get(0),
    )?;

    let chain_id = chain_id.context("Species has no evolution chain")?;

    // Find root species (one with no evolves_from in this chain)
    let root_id: i64 = conn.query_row(
        "SELECT id FROM species WHERE evolution_chain_id = ?1 AND evolves_from_species_id IS NULL \
         ORDER BY id LIMIT 1",
        params![chain_id],
        |row| row.get(0),
    )?;

    build_evolution_node(conn, root_id)
}

fn build_evolution_node(conn: &Connection, species_id: i64) -> Result<EvolutionNode> {
    let name: String = conn.query_row(
        "SELECT name FROM species WHERE id = ?1",
        params![species_id],
        |row| row.get(0),
    )?;
    let display_name = get_display_name(conn, species_id)?;

    // Get ALL evolution methods for this species (may differ by game/generation)
    let mut methods = Vec::new();
    {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT et.name, \
             COALESCE( \
               pe.trigger_detail, \
               CASE WHEN pe.minimum_level IS NOT NULL THEN 'Level ' || pe.minimum_level END, \
               CASE WHEN pe.trigger_item_id IS NOT NULL THEN 'Use ' || COALESCE((SELECT name FROM items WHERE id = pe.trigger_item_id), 'item') END, \
               CASE WHEN pe.minimum_happiness IS NOT NULL THEN 'Happiness ' || pe.minimum_happiness END, \
               CASE WHEN pe.known_move_id IS NOT NULL THEN 'Know ' || COALESCE((SELECT name FROM moves WHERE id = pe.known_move_id), 'move') END, \
               CASE WHEN pe.held_item_id IS NOT NULL THEN 'Hold ' || COALESCE((SELECT name FROM items WHERE id = pe.held_item_id), 'item') END, \
               '' \
             ) || CASE WHEN pe.time_of_day != '' THEN ' (' || pe.time_of_day || ')' ELSE '' END, \
             COALESCE((SELECT COALESCE(ln.name, l.name) FROM locations l \
               LEFT JOIN (SELECT location_id, name FROM location_names GROUP BY location_id) ln ON ln.location_id = l.id \
               WHERE l.id = pe.location_id), NULL), \
             COALESCE((SELECT name FROM items WHERE id = pe.trigger_item_id), NULL) \
             FROM pokemon_evolution pe \
             JOIN evolution_triggers et ON et.id = pe.evolution_trigger_id \
             WHERE pe.evolved_species_id = ?1 ORDER BY pe.id DESC"
        )?;
        let rows = stmt.query_map(params![species_id], |row| {
            let trigger: String = row.get(0)?;
            let detail: String = row.get::<_, String>(1).unwrap_or_default();
            let location: Option<String> = row.get(2)?;
            let item: Option<String> = row.get(3)?;
            Ok(EvolutionMethod {
                trigger,
                trigger_detail: if detail.is_empty() { None } else { Some(detail) },
                location,
                item,
            })
        })?;

        // Deduplicate methods by (trigger, trigger_detail) — keep unique methods only
        let mut seen = std::collections::HashSet::new();
        for method in rows.flatten() {
            let key = (method.trigger.clone(), method.trigger_detail.clone());
            if seen.insert(key) {
                methods.push(method);
            }
        }
    }

    // Find children
    let mut child_stmt = conn.prepare(
        "SELECT id FROM species WHERE evolves_from_species_id = ?1 ORDER BY id"
    )?;
    let child_ids: Vec<i64> = child_stmt
        .query_map(params![species_id], |row| row.get(0))?
        .filter_map(|r| r.ok())
        .collect();

    let mut children = Vec::new();
    for child_id in child_ids {
        children.push(build_evolution_node(conn, child_id)?);
    }

    Ok(EvolutionNode {
        species_id,
        species_name: name,
        display_name,
        methods,
        children,
    })
}

pub fn get_pokemon_forms(conn: &Connection, species_id: i64) -> Result<Vec<PokemonForm>> {
    let mut stmt = conn.prepare(
        "SELECT pf.id, pf.pokemon_id, pf.name, \
         CASE WHEN p.is_default = 1 AND pf.is_default = 1 AND pf.form_name IS NULL THEN COALESCE(sn.name, s.name) \
              WHEN p.is_default = 1 AND pf.is_default = 1 AND pf.form_name IS NOT NULL THEN COALESCE(pfn.pokemon_name, pfn.name, COALESCE(sn.name, s.name)) \
              ELSE COALESCE(pfn.pokemon_name, pfn.name, pf.form_name, COALESCE(sn.name, s.name)) \
         END, \
         pf.form_name, (p.is_default AND pf.is_default), pf.is_mega, pf.is_battle_only \
         FROM pokemon_forms pf \
         JOIN pokemon p ON p.id = pf.pokemon_id \
         JOIN species s ON s.id = p.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         LEFT JOIN pokemon_form_names pfn ON pfn.pokemon_form_id = pf.id \
         WHERE p.species_id = ?1 \
         ORDER BY pf.form_order"
    )?;

    let forms: Vec<PokemonForm> = stmt
        .query_map(params![species_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .map(|(id, pokemon_id, name, display_name, form_name, is_default, is_mega, is_battle_only)| {
            // Get form-specific types, or fall back to pokemon types
            let types = get_form_types(conn, id, pokemon_id);
            PokemonForm {
                id,
                pokemon_id,
                name,
                display_name,
                form_name,
                is_default: is_default != 0,
                is_mega: is_mega != 0,
                is_battle_only: is_battle_only != 0,
                types,
            }
        })
        .collect();

    Ok(forms)
}

fn get_form_types(conn: &Connection, form_id: i64, pokemon_id: i64) -> Vec<String> {
    // Try form-specific types first
    let mut stmt = conn.prepare(
        "SELECT COALESCE(tn.name, t.name) FROM pokemon_form_types pft \
         JOIN types t ON t.id = pft.type_id \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         WHERE pft.pokemon_form_id = ?1 ORDER BY pft.slot"
    ).unwrap();
    let types: Vec<String> = stmt
        .query_map(params![form_id], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    if !types.is_empty() {
        return types;
    }

    // Fall back to pokemon types
    let mut stmt = conn.prepare(
        "SELECT COALESCE(tn.name, t.name) FROM pokemon_types pt \
         JOIN types t ON t.id = pt.type_id \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         WHERE pt.pokemon_id = ?1 ORDER BY pt.slot"
    ).unwrap();
    stmt.query_map(params![pokemon_id], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn get_pokemon_stats(conn: &Connection, species_id: i64) -> Result<PokemonStats> {
    let display_name = get_display_name(conn, species_id)?;
    let pokemon_id: i64 = conn.query_row(
        "SELECT id FROM pokemon WHERE species_id = ?1 AND is_default = 1",
        params![species_id],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT s.name, ps.base_value FROM pokemon_stats ps \
         JOIN stats s ON s.id = ps.stat_id \
         WHERE ps.pokemon_id = ?1 ORDER BY ps.stat_id"
    )?;
    let stats: Vec<(String, i64)> = stmt
        .query_map(params![pokemon_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let get = |name: &str| stats.iter().find(|(n, _)| n == name).map(|(_, v)| *v).unwrap_or(0);

    let hp = get("hp");
    let attack = get("attack");
    let defense = get("defense");
    let special_attack = get("special-attack");
    let special_defense = get("special-defense");
    let speed = get("speed");

    Ok(PokemonStats {
        pokemon_name: display_name,
        hp,
        attack,
        defense,
        special_attack,
        special_defense,
        speed,
        total: hp + attack + defense + special_attack + special_defense + speed,
    })
}

/// Get stats for a specific pokemon_id (not necessarily the default form).
pub fn get_pokemon_stats_by_pokemon_id(conn: &Connection, pokemon_id: i64) -> Result<PokemonStats> {
    // Get display name from the pokemon's species
    let display_name: String = conn.query_row(
        "SELECT COALESCE(sn.name, s.name) FROM pokemon p \
         JOIN species s ON s.id = p.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         WHERE p.id = ?1",
        params![pokemon_id],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT s.name, ps.base_value FROM pokemon_stats ps \
         JOIN stats s ON s.id = ps.stat_id \
         WHERE ps.pokemon_id = ?1 ORDER BY ps.stat_id"
    )?;
    let stats: Vec<(String, i64)> = stmt
        .query_map(params![pokemon_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .filter_map(|r| r.ok())
        .collect();

    let get = |name: &str| stats.iter().find(|(n, _)| n == name).map(|(_, v)| *v).unwrap_or(0);

    let hp = get("hp");
    let attack = get("attack");
    let defense = get("defense");
    let special_attack = get("special-attack");
    let special_defense = get("special-defense");
    let speed = get("speed");

    Ok(PokemonStats {
        pokemon_name: display_name,
        hp,
        attack,
        defense,
        special_attack,
        special_defense,
        speed,
        total: hp + attack + defense + special_attack + special_defense + speed,
    })
}

/// Get abilities for a specific pokemon_id (not necessarily the default form).
pub fn get_pokemon_abilities_by_id(conn: &Connection, pokemon_id: i64) -> Result<Vec<AbilityInfo>> {
    let mut stmt = conn.prepare(
        "SELECT a.name, COALESCE(an.name, a.name), pa.is_hidden, \
         (SELECT ap.short_effect FROM ability_prose ap WHERE ap.ability_id = a.id LIMIT 1) \
         FROM pokemon_abilities pa \
         JOIN abilities a ON a.id = pa.ability_id \
         LEFT JOIN ability_names an ON an.ability_id = a.id \
         WHERE pa.pokemon_id = ?1 \
         ORDER BY pa.slot"
    )?;
    let abilities = stmt
        .query_map(params![pokemon_id], |row| {
            Ok(AbilityInfo {
                name: row.get(0)?,
                display_name: row.get(1)?,
                is_hidden: row.get::<_, i64>(2)? != 0,
                short_effect: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(abilities)
}

pub fn get_encounters(
    conn: &Connection,
    species_id: i64,
    game_filter: Option<&str>,
) -> Result<Vec<Encounter>> {
    let pokemon_id: i64 = conn.query_row(
        "SELECT id FROM pokemon WHERE species_id = ?1 AND is_default = 1",
        params![species_id],
        |row| row.get(0),
    )?;

    let display_name = get_display_name(conn, species_id)?;
    let species_name: String = conn.query_row(
        "SELECT name FROM species WHERE id = ?1", params![species_id], |row| row.get(0),
    )?;
    let regional_map = build_regional_form_map();

    let mut sql = String::from(
        "SELECT DISTINCT \
         COALESCE(ln.name, l.name) as loc_name, \
         COALESCE(la.name, '') as area_name, \
         COALESCE(vn.name, v.name) as game_name, \
         COALESCE(emn.name, em.name) as method_name, \
         e.min_level, e.max_level, es.rarity, e.id, \
         v.name as game_slug \
         FROM encounters e \
         JOIN encounter_slots es ON es.id = e.encounter_slot_id \
         JOIN encounter_methods em ON em.id = es.encounter_method_id \
         LEFT JOIN (SELECT encounter_method_id, name FROM encounter_method_names GROUP BY encounter_method_id) emn ON emn.encounter_method_id = em.id \
         JOIN location_areas la ON la.id = e.location_area_id \
         JOIN locations l ON l.id = la.location_id \
         LEFT JOIN (SELECT location_id, name FROM location_names GROUP BY location_id) ln ON ln.location_id = l.id \
         JOIN versions v ON v.id = e.version_id \
         LEFT JOIN (SELECT version_id, name FROM version_names GROUP BY version_id) vn ON vn.version_id = v.id \
         WHERE e.pokemon_id = ?1"
    );

    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(pokemon_id)];

    if let Some(game) = game_filter {
        // Match on slug first; only fall back to display name if slug doesn't match.
        // This avoids matching multiple versions with the same display name
        // (e.g., "red" matching both "red" and "red-japan" which are both named "Red").
        sql.push_str(" AND (LOWER(v.name) = LOWER(?2) OR (vn.name IS NOT NULL AND LOWER(vn.name) = LOWER(?2) AND NOT EXISTS (SELECT 1 FROM versions v2 WHERE LOWER(v2.name) = LOWER(?2))))");
        bind_values.push(Box::new(game.to_string()));
    }

    sql.push_str(" ORDER BY loc_name, area_name, method_name");

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();

    let rows: Vec<Encounter> = stmt
        .query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, String>(8)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .map(|(location, area, game, method, min_level, max_level, rarity, encounter_id, game_slug)| {
            let conditions = get_encounter_conditions(conn, encounter_id);
            let details = get_encounter_details(conn, encounter_id);
            // Filter out misleading level-1 data for static/fixed/raid encounters
            let has_uncatchable_note = details.as_ref()
                .and_then(|d| d.note.as_ref())
                .is_some_and(|n| n.contains("Can not be caught"));
            let is_bogus_level = min_level == 1 && max_level == 1
                && (method == "Static Encounter" || method == "Fixed Encounter" || method == "Max Raid Battle"
                    || method == "Special Encounter"
                    || method == "static-encounter" || method == "fixed-encounter" || method == "max-raid-battle"
                    || method == "special-encounter"
                    || has_uncatchable_note);
            let (final_min, final_max) = if is_bogus_level {
                (None, None)
            } else {
                (Some(min_level), Some(max_level))
            };
            // Annotate with regional form if applicable (e.g., "Darumaka" -> "Galarian Darumaka" in Sword)
            let annotated_name = if let Some(form_label) = regional_map.get(&(species_name.to_lowercase(), game_slug.to_lowercase())) {
                // Only annotate wild encounter methods, not NPC trades/gifts
                let is_wild = !method.to_lowercase().contains("trade") && !method.to_lowercase().contains("gift");
                if is_wild {
                    format!("{form_label} {display_name}")
                } else {
                    display_name.clone()
                }
            } else {
                display_name.clone()
            };
            // Prefer rate_overall from encounter_details over slot rarity
            // (e.g., LGPE has slot rarity=100 placeholder but PokeDB has real rates)
            let effective_rarity = details.as_ref()
                .and_then(|d| d.rate_overall.as_ref())
                .and_then(|r| r.trim_end_matches('%').parse::<i64>().ok())
                .or(rarity);

            Encounter {
                pokemon_name: annotated_name,
                species_slug: species_name.clone(),
                location,
                area,
                game,
                game_slug,
                method,
                min_level: final_min,
                max_level: final_max,
                rarity: effective_rarity,
                conditions,
                details,
            }
        })
        .filter(|enc| {
            // Filter out uncatchable encounters
            if let Some(ref det) = enc.details
                && let Some(ref note) = det.note
                    && note.contains("Can not be caught") {
                        return false;
                    }
            true
        })
        .collect();

    Ok(rows)
}

pub fn get_location_encounters(
    conn: &Connection,
    location: &str,
    game_filter: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<(Vec<Encounter>, u64)> {
    let location_lower = location.to_lowercase();

    let base_from = "FROM encounters e \
         JOIN encounter_slots es ON es.id = e.encounter_slot_id \
         JOIN encounter_methods em ON em.id = es.encounter_method_id \
         LEFT JOIN (SELECT encounter_method_id, name FROM encounter_method_names GROUP BY encounter_method_id) emn ON emn.encounter_method_id = em.id \
         JOIN location_areas la ON la.id = e.location_area_id \
         JOIN locations l ON l.id = la.location_id \
         LEFT JOIN (SELECT location_id, name FROM location_names GROUP BY location_id) ln ON ln.location_id = l.id \
         JOIN versions v ON v.id = e.version_id \
         LEFT JOIN (SELECT version_id, name FROM version_names GROUP BY version_id) vn ON vn.version_id = v.id \
         JOIN pokemon p ON p.id = e.pokemon_id \
         JOIN species s ON s.id = p.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id";

    // Try exact match first; only fall back to LIKE if exact returns nothing.
    // This prevents e.g. "wild-zone-1" from also matching "wild-zone-10", "wild-zone-11".
    let exact_where = " WHERE (LOWER(l.name) = LOWER(?1) OR LOWER(COALESCE(ln.name, '')) = LOWER(?1) OR LOWER(la.name) = LOWER(?1))";
    let like_where = " WHERE (LOWER(la.name) = LOWER(?1) OR LOWER(l.name) = LOWER(?1) OR LOWER(COALESCE(ln.name, '')) = LOWER(?1) \
                OR LOWER(la.name) LIKE '%' || LOWER(?1) || '%' OR LOWER(COALESCE(ln.name, '')) LIKE '%' || LOWER(?1) || '%')";

    let exact_count: u64 = {
        let sql = format!("SELECT COUNT(DISTINCT e.id) {base_from}{exact_where}");
        let mut stmt = conn.prepare(&sql)?;
        stmt.query_row(params![&location_lower], |row| row.get(0))?
    };

    let where_clause = if exact_count > 0 { exact_where } else { like_where };
    let mut base_sql = format!("{base_from}{where_clause}");

    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(location_lower)];

    if let Some(game) = game_filter {
        base_sql.push_str(" AND (LOWER(v.name) = LOWER(?2) OR (vn.name IS NOT NULL AND LOWER(vn.name) = LOWER(?2) AND NOT EXISTS (SELECT 1 FROM versions v2 WHERE LOWER(v2.name) = LOWER(?2))))");
        bind_values.push(Box::new(game.to_string()));
    }

    // Count query
    let count_sql = format!("SELECT COUNT(DISTINCT e.id) {base_sql}");
    let total: u64 = {
        let mut stmt = conn.prepare(&count_sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
        stmt.query_row(params.as_slice(), |row| row.get(0))?
    };

    let regional_map = build_regional_form_map();

    // Data query
    let select_sql = format!(
        "SELECT DISTINCT \
         COALESCE(sn.name, s.name) as pokemon_name, \
         COALESCE(ln.name, l.name) as loc_name, \
         COALESCE(la.name, '') as area_name, \
         COALESCE(vn.name, v.name) as game_name, \
         COALESCE(emn.name, em.name) as method_name, \
         e.min_level, e.max_level, es.rarity, e.id, \
         v.name as game_slug, \
         s.name as species_slug \
         {base_sql} \
         ORDER BY pokemon_name, method_name \
         LIMIT ?{limit_idx} OFFSET ?{offset_idx}",
        limit_idx = bind_values.len() + 1,
        offset_idx = bind_values.len() + 2,
    );
    bind_values.push(Box::new(limit as i64));
    bind_values.push(Box::new(offset as i64));

    let mut stmt = conn.prepare(&select_sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();

    let rows: Vec<Encounter> = stmt
        .query_map(params.as_slice(), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, Option<i64>>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, String>(9)?,
                row.get::<_, String>(10)?,
            ))
        })?
        .filter_map(|r| r.ok())
        .map(|(pokemon_name, location_name, area, game, method, min_level, max_level, rarity, encounter_id, game_slug, species_slug)| {
            let conditions = get_encounter_conditions(conn, encounter_id);
            let details = get_encounter_details(conn, encounter_id);
            let has_uncatchable_note = details.as_ref()
                .and_then(|d| d.note.as_ref())
                .is_some_and(|n| n.contains("Can not be caught"));
            let is_bogus_level = min_level == 1 && max_level == 1
                && (method == "Static Encounter" || method == "Fixed Encounter" || method == "Max Raid Battle"
                    || method == "Special Encounter"
                    || method == "static-encounter" || method == "fixed-encounter" || method == "max-raid-battle"
                    || method == "special-encounter"
                    || has_uncatchable_note);
            let (final_min, final_max) = if is_bogus_level {
                (None, None)
            } else {
                (Some(min_level), Some(max_level))
            };
            // Annotate with regional form if applicable (e.g., "Darumaka" -> "Galarian Darumaka" in Sword)
            let annotated_name = if let Some(form_label) = regional_map.get(&(species_slug.to_lowercase(), game_slug.to_lowercase())) {
                let is_wild = !method.to_lowercase().contains("trade") && !method.to_lowercase().contains("gift");
                if is_wild {
                    format!("{form_label} {pokemon_name}")
                } else {
                    pokemon_name
                }
            } else {
                pokemon_name
            };
            // Prefer rate_overall from encounter_details over slot rarity
            let effective_rarity = details.as_ref()
                .and_then(|d| d.rate_overall.as_ref())
                .and_then(|r| r.trim_end_matches('%').parse::<i64>().ok())
                .or(rarity);

            Encounter {
                pokemon_name: annotated_name,
                species_slug: species_slug.clone(),
                location: location_name,
                area,
                game,
                game_slug,
                method,
                min_level: final_min,
                max_level: final_max,
                rarity: effective_rarity,
                conditions,
                details,
            }
        })
        .filter(|enc| {
            if let Some(ref det) = enc.details
                && let Some(ref note) = det.note
                    && note.contains("Can not be caught") {
                        return false;
                    }
            true
        })
        .collect();

    Ok((rows, total))
}

fn get_encounter_details(conn: &Connection, encounter_id: i64) -> Option<EncounterDetails> {
    conn.query_row(
        "SELECT rate_overall, rate_morning, rate_day, rate_night, \
         during_any_time, during_morning, during_day, during_evening, during_night, \
         while_weather_overall, \
         weather_clear_rate, weather_cloudy_rate, weather_rain_rate, weather_thunderstorm_rate, \
         weather_snow_rate, weather_blizzard_rate, weather_harshsunlight_rate, \
         weather_sandstorm_rate, weather_fog_rate, \
         on_terrain_land, on_terrain_watersurface, on_terrain_underwater, \
         probability_overall, group_rate, group_pokemon, alpha_levels, \
         tera_raid_star_level, max_raid_perfect_ivs, \
         max_raid_rate_1_star, max_raid_rate_2_star, max_raid_rate_3_star, \
         max_raid_rate_4_star, max_raid_rate_5_star, \
         hidden_ability_possible, visible, note \
         FROM encounter_details WHERE encounter_id = ?1",
        params![encounter_id],
        |row| {
            Ok(EncounterDetails {
                rate_overall: row.get(0)?,
                rate_morning: row.get(1)?,
                rate_day: row.get(2)?,
                rate_night: row.get(3)?,
                during_any_time: row.get::<_, Option<i64>>(4)?.map(|v| v != 0),
                during_morning: row.get::<_, Option<i64>>(5)?.map(|v| v != 0),
                during_day: row.get::<_, Option<i64>>(6)?.map(|v| v != 0),
                during_evening: row.get::<_, Option<i64>>(7)?.map(|v| v != 0),
                during_night: row.get::<_, Option<i64>>(8)?.map(|v| v != 0),
                while_weather_overall: row.get::<_, Option<i64>>(9)?.map(|v| v != 0),
                weather_clear_rate: row.get(10)?,
                weather_cloudy_rate: row.get(11)?,
                weather_rain_rate: row.get(12)?,
                weather_thunderstorm_rate: row.get(13)?,
                weather_snow_rate: row.get(14)?,
                weather_blizzard_rate: row.get(15)?,
                weather_harshsunlight_rate: row.get(16)?,
                weather_sandstorm_rate: row.get(17)?,
                weather_fog_rate: row.get(18)?,
                on_terrain_land: row.get::<_, Option<i64>>(19)?.map(|v| v != 0),
                on_terrain_watersurface: row.get::<_, Option<i64>>(20)?.map(|v| v != 0),
                on_terrain_underwater: row.get::<_, Option<i64>>(21)?.map(|v| v != 0),
                probability_overall: row.get(22)?,
                group_rate: row.get(23)?,
                group_pokemon: row.get(24)?,
                alpha_levels: row.get(25)?,
                tera_raid_star_level: row.get(26)?,
                max_raid_perfect_ivs: row.get(27)?,
                max_raid_rate_1_star: row.get(28)?,
                max_raid_rate_2_star: row.get(29)?,
                max_raid_rate_3_star: row.get(30)?,
                max_raid_rate_4_star: row.get(31)?,
                max_raid_rate_5_star: row.get(32)?,
                hidden_ability_possible: row.get::<_, Option<i64>>(33)?.map(|v| v != 0),
                visible: row.get::<_, Option<i64>>(34)?.map(|v| v != 0),
                note: row.get(35)?,
            })
        },
    ).ok()
    // D12: Filter out encounter details where all fields are None/empty
    .filter(|d| !d.is_empty())
}

fn get_encounter_conditions(conn: &Connection, encounter_id: i64) -> Vec<String> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(ecvn.name, ecv.name) \
         FROM encounter_condition_value_map ecvm \
         JOIN encounter_condition_values ecv ON ecv.id = ecvm.encounter_condition_value_id \
         LEFT JOIN (SELECT encounter_condition_value_id, name FROM encounter_condition_value_names GROUP BY encounter_condition_value_id) ecvn ON ecvn.encounter_condition_value_id = ecv.id \
         WHERE ecvm.encounter_id = ?1"
    ).unwrap();
    stmt.query_map(params![encounter_id], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect()
}

pub fn get_pokemon_moves(
    conn: &Connection,
    species_id: i64,
    game_filter: Option<&str>,
    method_filter: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<(Vec<PokemonMove>, u64)> {
    let pokemon_id: i64 = conn.query_row(
        "SELECT id FROM pokemon WHERE species_id = ?1 AND is_default = 1",
        params![species_id],
        |row| row.get(0),
    )?;

    let mut base_sql = String::from(
        "FROM pokemon_moves pm \
         JOIN moves m ON m.id = pm.move_id \
         LEFT JOIN move_names mn ON mn.move_id = m.id \
         JOIN types t ON t.id = m.type_id \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         LEFT JOIN move_damage_classes dc ON dc.id = m.damage_class_id \
         LEFT JOIN move_damage_class_names dcn ON dcn.move_damage_class_id = dc.id \
         JOIN pokemon_move_methods pmm ON pmm.id = pm.pokemon_move_method_id \
         JOIN version_groups vg ON vg.id = pm.version_group_id \
         WHERE pm.pokemon_id = ?1"
    );

    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(pokemon_id)];

    if let Some(game) = game_filter {
        // Match version group by looking up the version
        base_sql.push_str(
            " AND pm.version_group_id = (SELECT version_group_id FROM versions WHERE LOWER(name) = LOWER(?2) LIMIT 1)"
        );
        bind_values.push(Box::new(game.to_string()));
    }

    if let Some(method) = method_filter {
        let idx = bind_values.len() + 1;
        base_sql.push_str(&format!(" AND LOWER(pmm.name) = LOWER(?{idx})"));
        bind_values.push(Box::new(method.to_string()));
    }

    // Count total
    let count_sql = format!("SELECT COUNT(DISTINCT m.name || '|' || pmm.name || '|' || vg.name || '|' || pm.level) {base_sql}");
    let total: u64 = {
        let mut stmt = conn.prepare(&count_sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
        stmt.query_row(params.as_slice(), |row| row.get(0))?
    };

    let sql = format!(
        "SELECT DISTINCT m.name, COALESCE(mn.name, m.name) as display_name, \
         COALESCE(tn.name, t.name) as type_name, m.power, m.accuracy, m.pp, \
         COALESCE(dcn.name, dc.name) as damage_class, \
         pmm.name as method_name, pm.level, vg.name as vg_name \
         {base_sql} ORDER BY pm.pokemon_move_method_id, pm.level, m.name \
         LIMIT ?{} OFFSET ?{}",
        bind_values.len() + 1,
        bind_values.len() + 2,
    );

    bind_values.push(Box::new(limit as i64));
    bind_values.push(Box::new(offset as i64));

    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();

    let results: Vec<PokemonMove> = stmt
        .query_map(params.as_slice(), |row| {
            Ok(PokemonMove {
                move_name: row.get(0)?,
                display_name: row.get(1)?,
                type_name: row.get(2)?,
                power: row.get(3)?,
                accuracy: row.get(4)?,
                pp: row.get(5)?,
                damage_class: row.get::<_, String>(6).unwrap_or_default(),
                learn_method: row.get(7)?,
                level: row.get(8)?,
                game: row.get(9)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok((results, total))
}

// ---- Type queries ----

pub fn list_types(conn: &Connection) -> Result<Vec<TypeInfo>> {
    let mut stmt = conn.prepare(
        "SELECT t.name, COALESCE(tn.name, t.name) FROM types t \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         ORDER BY t.id"
    )?;
    let results = stmt
        .query_map([], |row| {
            Ok(TypeInfo {
                name: row.get(0)?,
                display_name: row.get(1)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(results)
}

pub fn get_type_matchups(conn: &Connection, type_name: &str) -> Result<TypeMatchups> {
    let (type_id, name, display_name): (i64, String, String) = conn.query_row(
        "SELECT t.id, t.name, COALESCE(tn.name, t.name) FROM types t \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         WHERE LOWER(t.name) = LOWER(?1) OR LOWER(tn.name) = LOWER(?1)",
        params![type_name],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    )?;

    // Attacking matchups
    let mut atk_stmt = conn.prepare(
        "SELECT COALESCE(tn.name, t.name), te.damage_factor FROM type_efficacy te \
         JOIN types t ON t.id = te.defending_type_id \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         WHERE te.attacking_type_id = ?1 AND te.damage_factor != 100"
    )?;
    let mut atk_super = Vec::new();
    let mut atk_not_very = Vec::new();
    let mut atk_no_effect = Vec::new();
    for row in atk_stmt.query_map(params![type_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))? {
        let (name, factor) = row?;
        match factor {
            200 => atk_super.push(name),
            50 => atk_not_very.push(name),
            0 => atk_no_effect.push(name),
            _ => {}
        }
    }

    // Defending matchups
    let mut def_stmt = conn.prepare(
        "SELECT COALESCE(tn.name, t.name), te.damage_factor FROM type_efficacy te \
         JOIN types t ON t.id = te.attacking_type_id \
         LEFT JOIN type_names tn ON tn.type_id = t.id \
         WHERE te.defending_type_id = ?1 AND te.damage_factor != 100"
    )?;
    let mut def_super = Vec::new();
    let mut def_not_very = Vec::new();
    let mut def_no_effect = Vec::new();
    for row in def_stmt.query_map(params![type_id], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))? {
        let (name, factor) = row?;
        match factor {
            200 => def_super.push(name),
            50 => def_not_very.push(name),
            0 => def_no_effect.push(name),
            _ => {}
        }
    }

    Ok(TypeMatchups {
        type_name: name,
        display_name,
        attacking: TypeEffectiveness {
            super_effective: atk_super,
            not_very_effective: atk_not_very,
            no_effect: atk_no_effect,
        },
        defending: TypeEffectiveness {
            super_effective: def_super,
            not_very_effective: def_not_very,
            no_effect: def_no_effect,
        },
    })
}

// ---- Dex queries ----

pub fn list_pokedexes(conn: &Connection) -> Result<Vec<PokedexInfo>> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, COALESCE(pn.name, p.name), \
         COALESCE(rn.name, r.name), \
         (SELECT COUNT(*) FROM pokemon_dex_numbers pdn WHERE pdn.pokedex_id = p.id) \
         FROM pokedexes p \
         LEFT JOIN pokedex_names pn ON pn.pokedex_id = p.id \
         LEFT JOIN regions r ON r.id = p.region_id \
         LEFT JOIN region_names rn ON rn.region_id = r.id \
         WHERE p.is_main_series = 1 \
         ORDER BY p.id"
    )?;
    let results = stmt
        .query_map([], |row| {
            Ok(PokedexInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                display_name: row.get(2)?,
                region: row.get(3)?,
                species_count: row.get(4)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(results)
}

pub fn resolve_pokedex(conn: &Connection, name: &str) -> Result<Option<(i64, String)>> {
    let result = conn.query_row(
        "SELECT p.id, p.name FROM pokedexes p \
         LEFT JOIN pokedex_names pn ON pn.pokedex_id = p.id \
         WHERE LOWER(p.name) = LOWER(?1) OR LOWER(pn.name) = LOWER(?1)",
        params![name],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );
    match result {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn get_dex_entries(
    conn: &Connection,
    pokedex_id: i64,
    limit: u64,
    offset: u64,
) -> Result<(Vec<DexEntry>, u64)> {
    let total: u64 = conn.query_row(
        "SELECT COUNT(*) FROM pokemon_dex_numbers WHERE pokedex_id = ?1",
        params![pokedex_id],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT pdn.pokedex_number, pdn.species_id, s.name, COALESCE(sn.name, s.name) \
         FROM pokemon_dex_numbers pdn \
         JOIN species s ON s.id = pdn.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         WHERE pdn.pokedex_id = ?1 \
         ORDER BY pdn.pokedex_number \
         LIMIT ?2 OFFSET ?3"
    )?;
    let entries: Vec<DexEntry> = stmt
        .query_map(params![pokedex_id, limit, offset], |row| {
            Ok(DexEntry {
                pokedex_number: row.get(0)?,
                species_id: row.get(1)?,
                name: row.get(2)?,
                display_name: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok((entries, total))
}

pub fn get_dex_progress(
    conn: &Connection,
    pokedex_id: i64,
    dex_name: &str,
    show_missing: bool,
    show_caught: bool,
    game_filter: Option<&str>,
    status_filter: Option<&str>,
    limit: u64,
    offset: u64,
) -> Result<(DexProgress, u64)> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pokemon_dex_numbers WHERE pokedex_id = ?1",
        params![pokedex_id],
        |row| row.get(0),
    )?;

    // Build a subquery for "caught" species
    let mut caught_conditions = vec!["c.status != 'traded_away'".to_string()];
    if let Some(game) = game_filter {
        caught_conditions.push(format!(
            "c.game_id = (SELECT id FROM games WHERE LOWER(name) = LOWER('{game}'))"
        ));
    }
    if let Some(status) = status_filter {
        caught_conditions.push(format!("c.status = '{status}'"));
    }
    let caught_where = caught_conditions.join(" AND ");

    let caught: i64 = conn.query_row(
        &format!(
            "SELECT COUNT(DISTINCT pdn.species_id) FROM pokemon_dex_numbers pdn \
             INNER JOIN collection c ON c.species_id = pdn.species_id AND {caught_where} \
             WHERE pdn.pokedex_id = ?1"
        ),
        params![pokedex_id],
        |row| row.get(0),
    )?;

    let percentage = if total > 0 { ((caught as f64 / total as f64) * 10000.0).round() / 100.0 } else { 0.0 };

    // Count filtered entries for pagination
    let filtered_count: u64 = if show_missing {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM pokemon_dex_numbers pdn \
                 JOIN species s ON s.id = pdn.species_id \
                 WHERE pdn.pokedex_id = ?1 \
                 AND NOT EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND {caught_where})"
            ),
            params![pokedex_id],
            |row| row.get(0),
        )?
    } else if show_caught {
        conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM pokemon_dex_numbers pdn \
                 JOIN species s ON s.id = pdn.species_id \
                 WHERE pdn.pokedex_id = ?1 \
                 AND EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND {caught_where})"
            ),
            params![pokedex_id],
            |row| row.get(0),
        )?
    } else {
        total as u64
    };

    // Get individual entries based on filter
    let filter_clause = if show_missing {
        format!(
            "AND NOT EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND {caught_where})"
        )
    } else if show_caught {
        format!(
            "AND EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND {caught_where})"
        )
    } else {
        String::new()
    };

    let mut stmt = conn.prepare(&format!(
        "SELECT pdn.pokedex_number, pdn.species_id, s.name, COALESCE(sn.name, s.name), \
         EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND {caught_where}) as is_caught \
         FROM pokemon_dex_numbers pdn \
         JOIN species s ON s.id = pdn.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         WHERE pdn.pokedex_id = ?1 {filter_clause} \
         ORDER BY pdn.pokedex_number \
         LIMIT ?2 OFFSET ?3"
    ))?;

    let entries: Vec<DexProgressEntry> = stmt
        .query_map(params![pokedex_id, limit, offset], |row| {
            Ok(DexProgressEntry {
                pokedex_number: row.get(0)?,
                species_id: row.get(1)?,
                name: row.get(2)?,
                display_name: row.get(3)?,
                caught: row.get::<_, i64>(4)? != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok((DexProgress {
        dex_name: dex_name.to_string(),
        total,
        caught,
        percentage,
        entries,
    }, filtered_count))
}

// ---- Game queries ----

pub fn list_games(conn: &Connection, home_compatible: bool) -> Result<Vec<GameInfo>> {
    let where_clause = if home_compatible { " WHERE g.connects_to_home = 1" } else { "" };
    let sql = format!(
        "SELECT g.id, g.name, g.connects_to_home, g.transfer_direction, \
         gen.id as generation, \
         COALESCE(rn.name, r.name) as region, \
         COALESCE(vn.name, \
           CASE g.name \
             WHEN 'pokemon-go' THEN 'Pokémon GO' \
             WHEN 'pokemon-bank' THEN 'Pokémon Bank' \
             WHEN 'home' THEN 'Pokémon HOME' \
             WHEN 'legends-za' THEN 'Legends: Z-A' \
             ELSE g.name \
           END \
         ) as display_name \
         FROM games g \
         LEFT JOIN version_groups vg ON vg.id = g.version_group_id \
         LEFT JOIN generations gen ON gen.id = vg.generation_id \
         LEFT JOIN (SELECT version_group_id, MAX(region_id) as region_id FROM version_group_regions GROUP BY version_group_id) vgr ON vgr.version_group_id = vg.id \
         LEFT JOIN regions r ON r.id = vgr.region_id \
         LEFT JOIN region_names rn ON rn.region_id = r.id \
         LEFT JOIN versions v ON v.version_group_id = vg.id AND LOWER(v.name) = LOWER(g.name) \
         LEFT JOIN version_names vn ON vn.version_id = v.id \
         {where_clause} \
         GROUP BY g.id \
         ORDER BY g.id"
    );
    let mut stmt = conn.prepare(&sql)?;
    let results = stmt
        .query_map([], |row| {
            Ok(GameInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                connects_to_home: row.get::<_, i64>(2)? != 0,
                transfer_direction: row.get(3)?,
                generation: row.get(4)?,
                region: row.get(5)?,
                display_name: row.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(results)
}

pub fn resolve_game(conn: &Connection, name: &str) -> Result<Option<(i64, String)>> {
    let result = conn.query_row(
        "SELECT id, name FROM games WHERE LOWER(name) = LOWER(?1)",
        params![name],
        |row| Ok((row.get(0)?, row.get(1)?)),
    );
    match result {
        Ok(r) => Ok(Some(r)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

// ---- Collection queries ----

pub fn add_collection_entry(
    conn: &Connection,
    species_id: i64,
    form_id: Option<i64>,
    game_id: i64,
    shiny: bool,
    in_home: bool,
    is_alpha: bool,
    status: &str,
    method: Option<&str>,
    nickname: Option<&str>,
    notes: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO collection (species_id, form_id, game_id, shiny, in_home, is_alpha, status, method, nickname, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![species_id, form_id, game_id, shiny, in_home, is_alpha, status, method, nickname, notes],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn remove_collection_entry(conn: &Connection, id: i64) -> Result<bool> {
    let rows = conn.execute("DELETE FROM collection WHERE id = ?1", params![id])?;
    Ok(rows > 0)
}

pub fn update_collection_entry(
    conn: &Connection,
    id: i64,
    status: Option<&str>,
    in_home: Option<bool>,
    shiny: Option<bool>,
    nickname: Option<&str>,
    notes: Option<&str>,
    game_id: Option<i64>,
    method: Option<&str>,
) -> Result<bool> {
    let mut sets = Vec::new();
    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(s) = status {
        sets.push(format!("status = ?{idx}"));
        bind_values.push(Box::new(s.to_string()));
        idx += 1;
    }
    if let Some(h) = in_home {
        sets.push(format!("in_home = ?{idx}"));
        bind_values.push(Box::new(h as i64));
        idx += 1;
    }
    if let Some(s) = shiny {
        sets.push(format!("shiny = ?{idx}"));
        bind_values.push(Box::new(s as i64));
        idx += 1;
    }
    if let Some(n) = nickname {
        sets.push(format!("nickname = ?{idx}"));
        bind_values.push(Box::new(n.to_string()));
        idx += 1;
    }
    if let Some(n) = notes {
        sets.push(format!("notes = ?{idx}"));
        bind_values.push(Box::new(n.to_string()));
        idx += 1;
    }
    if let Some(g) = game_id {
        sets.push(format!("game_id = ?{idx}"));
        bind_values.push(Box::new(g));
        idx += 1;
    }
    if let Some(m) = method {
        sets.push(format!("method = ?{idx}"));
        bind_values.push(Box::new(m.to_string()));
        idx += 1;
    }

    if sets.is_empty() {
        return Ok(false);
    }

    sets.push("updated_at = datetime('now')".to_string());

    let sql = format!("UPDATE collection SET {} WHERE id = ?{idx}", sets.join(", "));
    bind_values.push(Box::new(id));

    let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
    let rows = conn.execute(&sql, params.as_slice())?;
    Ok(rows > 0)
}

pub fn get_collection_entry(conn: &Connection, id: i64) -> Result<Option<CollectionEntry>> {
    let result = conn.query_row(
        "SELECT c.id, c.species_id, s.name, \
         COALESCE( \
             CASE WHEN c.form_id IS NOT NULL THEN \
                 (SELECT COALESCE(pfn.pokemon_name, pfn.name) \
                  FROM pokemon_form_names pfn WHERE pfn.pokemon_form_id = c.form_id) \
             END, \
             COALESCE(sn.name, s.name) \
         ), \
         pf.form_name, g.name, c.shiny, c.in_home, c.is_alpha, c.status, \
         c.method, c.nickname, c.notes, c.created_at, c.updated_at \
         FROM collection c \
         JOIN species s ON s.id = c.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         LEFT JOIN pokemon_forms pf ON pf.id = c.form_id \
         LEFT JOIN games g ON g.id = c.game_id \
         WHERE c.id = ?1",
        params![id],
        |row| {
            Ok(CollectionEntry {
                id: row.get(0)?,
                species_id: row.get(1)?,
                species_name: row.get(2)?,
                display_name: row.get(3)?,
                form_name: row.get(4)?,
                game: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                shiny: row.get::<_, i64>(6)? != 0,
                in_home: row.get::<_, i64>(7)? != 0,
                is_alpha: row.get::<_, i64>(8)? != 0,
                status: row.get(9)?,
                method: row.get(10)?,
                nickname: row.get(11)?,
                notes: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        },
    );

    match result {
        Ok(e) => Ok(Some(e)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

pub fn list_collection(
    conn: &Connection,
    game_filter: Option<&str>,
    pokemon_filter: Option<&str>,
    shiny_only: bool,
    in_home_only: bool,
    status_filter: Option<&str>,
    limit: u64,
    offset: u64,
    sort: &str,
) -> Result<(Vec<CollectionEntry>, u64)> {
    let mut conditions = Vec::new();
    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(game) = game_filter {
        conditions.push(format!("LOWER(g.name) = LOWER(?{idx})"));
        bind_values.push(Box::new(game.to_string()));
        idx += 1;
    }
    if let Some(pokemon) = pokemon_filter {
        conditions.push(format!(
            "(LOWER(s.name) = LOWER(?{idx}) OR LOWER(sn.name) = LOWER(?{idx}))"
        ));
        bind_values.push(Box::new(pokemon.to_string()));
        idx += 1;
    }
    if shiny_only {
        conditions.push("c.shiny = 1".to_string());
    }
    if in_home_only {
        conditions.push("c.in_home = 1".to_string());
    }
    if let Some(status) = status_filter {
        conditions.push(format!("c.status = ?{idx}"));
        bind_values.push(Box::new(status.to_string()));
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    let order_clause = if sort == "dex" {
        "ORDER BY c.species_id ASC"
    } else {
        "ORDER BY c.id DESC"
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM collection c \
         JOIN species s ON s.id = c.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         LEFT JOIN games g ON g.id = c.game_id \
         {where_clause}"
    );
    let total: u64 = {
        let mut stmt = conn.prepare(&count_sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
        stmt.query_row(params.as_slice(), |row| row.get(0))?
    };

    let query = format!(
        "SELECT c.id, c.species_id, s.name, \
         COALESCE( \
             CASE WHEN c.form_id IS NOT NULL THEN \
                 (SELECT COALESCE(pfn.pokemon_name, pfn.name) \
                  FROM pokemon_form_names pfn WHERE pfn.pokemon_form_id = c.form_id) \
             END, \
             COALESCE(sn.name, s.name) \
         ), \
         pf.form_name, g.name, c.shiny, c.in_home, c.is_alpha, c.status, \
         c.method, c.nickname, c.notes, c.created_at, c.updated_at \
         FROM collection c \
         JOIN species s ON s.id = c.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         LEFT JOIN pokemon_forms pf ON pf.id = c.form_id \
         LEFT JOIN games g ON g.id = c.game_id \
         {where_clause} \
         {order_clause} \
         LIMIT ?{idx} OFFSET ?{}",
        idx + 1
    );

    bind_values.push(Box::new(limit as i64));
    bind_values.push(Box::new(offset as i64));

    let mut stmt = conn.prepare(&query)?;
    let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();

    let entries: Vec<CollectionEntry> = stmt
        .query_map(params.as_slice(), |row| {
            Ok(CollectionEntry {
                id: row.get(0)?,
                species_id: row.get(1)?,
                species_name: row.get(2)?,
                display_name: row.get(3)?,
                form_name: row.get(4)?,
                game: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                shiny: row.get::<_, i64>(6)? != 0,
                in_home: row.get::<_, i64>(7)? != 0,
                is_alpha: row.get::<_, i64>(8)? != 0,
                status: row.get(9)?,
                method: row.get(10)?,
                nickname: row.get(11)?,
                notes: row.get(12)?,
                created_at: row.get(13)?,
                updated_at: row.get(14)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok((entries, total))
}

pub fn get_collection_stats(conn: &Connection, game_filter: Option<&str>) -> Result<CollectionStats> {
    let (game_join, game_where) = if let Some(game) = game_filter {
        (
            " JOIN games g ON g.id = c.game_id",
            format!(" WHERE LOWER(g.name) = LOWER('{}')", game.replace('\'', "''")),
        )
    } else {
        ("", String::new())
    };

    let total_entries: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM collection c{game_join}{game_where}"), [], |row| row.get(0),
    )?;
    let unique_species: i64 = conn.query_row(
        &format!("SELECT COUNT(DISTINCT c.species_id) FROM collection c{game_join}{game_where}"), [], |row| row.get(0),
    )?;
    let shiny_count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM collection c{game_join}{game_where}{}", if game_where.is_empty() { " WHERE c.shiny = 1" } else { " AND c.shiny = 1" }), [], |row| row.get(0),
    )?;
    let in_home_count: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM collection c{game_join}{game_where}{}", if game_where.is_empty() { " WHERE c.in_home = 1" } else { " AND c.in_home = 1" }), [], |row| row.get(0),
    )?;

    let mut status_stmt = conn.prepare(
        &format!("SELECT c.status, COUNT(*) FROM collection c{game_join}{game_where} GROUP BY c.status ORDER BY c.status")
    )?;
    let by_status: Vec<StatusCount> = status_stmt
        .query_map([], |row| Ok(StatusCount { status: row.get(0)?, count: row.get(1)? }))?
        .filter_map(|r| r.ok())
        .collect();

    let by_game = if game_filter.is_some() {
        // When filtering by game, no need for by_game breakdown
        Vec::new()
    } else {
        let mut game_stmt = conn.prepare(
            "SELECT g.name, COUNT(*) FROM collection c \
             LEFT JOIN games g ON g.id = c.game_id \
             GROUP BY c.game_id ORDER BY COUNT(*) DESC"
        )?;
        game_stmt
            .query_map([], |row| Ok(GameCount {
                game: row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "unknown".to_string()),
                count: row.get(1)?,
            }))?
            .filter_map(|r| r.ok())
            .collect()
    };

    Ok(CollectionStats {
        total_entries,
        unique_species,
        shiny_count,
        in_home_count,
        by_status,
        by_game,
    })
}

// ---- HOME queries ----

pub fn get_home_status(conn: &Connection) -> Result<HomeStatus> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM collection WHERE in_home = 1", [], |row| row.get(0),
    )?;
    let unique: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT species_id) FROM collection WHERE in_home = 1", [], |row| row.get(0),
    )?;
    let shiny: i64 = conn.query_row(
        "SELECT COUNT(*) FROM collection WHERE in_home = 1 AND shiny = 1", [], |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT g.name, COUNT(*) FROM collection c \
         LEFT JOIN games g ON g.id = c.game_id \
         WHERE c.in_home = 1 \
         GROUP BY c.game_id ORDER BY COUNT(*) DESC"
    )?;
    let by_game: Vec<GameCount> = stmt
        .query_map([], |row| Ok(GameCount {
            game: row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "unknown".to_string()),
            count: row.get(1)?,
        }))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(HomeStatus {
        total_in_home: total,
        unique_species_in_home: unique,
        shiny_in_home: shiny,
        by_game_origin: by_game,
    })
}

pub fn get_home_transferable(conn: &Connection, species_id: i64) -> Result<Vec<GameInfo>> {
    // Check if the species appears in any pokedex linked to a game's version group
    let mut stmt = conn.prepare(
        "SELECT DISTINCT g.id, g.name, g.connects_to_home, g.transfer_direction, \
         gen.id as generation, \
         COALESCE(rn.name, r.name) as region, \
         COALESCE(vn.name, \
           CASE g.name \
             WHEN 'pokemon-go' THEN 'Pokémon GO' \
             WHEN 'pokemon-bank' THEN 'Pokémon Bank' \
             WHEN 'home' THEN 'Pokémon HOME' \
             WHEN 'legends-za' THEN 'Legends: Z-A' \
             ELSE g.name \
           END \
         ) as display_name \
         FROM games g \
         JOIN version_groups vg ON vg.id = g.version_group_id \
         JOIN pokedex_version_groups pvg ON pvg.version_group_id = vg.id \
         JOIN pokemon_dex_numbers pdn ON pdn.pokedex_id = pvg.pokedex_id \
         LEFT JOIN generations gen ON gen.id = vg.generation_id \
         LEFT JOIN (SELECT version_group_id, MAX(region_id) as region_id FROM version_group_regions GROUP BY version_group_id) vgr ON vgr.version_group_id = vg.id \
         LEFT JOIN regions r ON r.id = vgr.region_id \
         LEFT JOIN region_names rn ON rn.region_id = r.id \
         LEFT JOIN versions v ON v.version_group_id = vg.id AND LOWER(v.name) = LOWER(g.name) \
         LEFT JOIN version_names vn ON vn.version_id = v.id \
         WHERE g.connects_to_home = 1 AND pdn.species_id = ?1 \
         GROUP BY g.id \
         ORDER BY g.id"
    )?;
    let results: Vec<GameInfo> = stmt
        .query_map(params![species_id], |row| {
            Ok(GameInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                connects_to_home: true,
                transfer_direction: row.get(3)?,
                generation: row.get(4)?,
                region: row.get(5)?,
                display_name: row.get(6)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Fallback: if no dex mapping exists, check which HOME games have encounter data for this species
    if results.is_empty() {
        let mut stmt = conn.prepare(
            "SELECT DISTINCT g.id, g.name, g.connects_to_home, g.transfer_direction, \
             gen.id as generation, \
             COALESCE(rn.name, r.name) as region, \
             COALESCE(vn.name, \
               CASE g.name \
                 WHEN 'pokemon-go' THEN 'Pokémon GO' \
                 WHEN 'pokemon-bank' THEN 'Pokémon Bank' \
                 WHEN 'home' THEN 'Pokémon HOME' \
                 WHEN 'legends-za' THEN 'Legends: Z-A' \
                 ELSE g.name \
               END \
             ) as display_name \
             FROM games g \
             JOIN versions v ON LOWER(v.name) = LOWER(g.name) \
             JOIN encounters e ON e.version_id = v.id \
             JOIN pokemon p ON p.id = e.pokemon_id \
             LEFT JOIN version_groups vg ON vg.id = g.version_group_id \
             LEFT JOIN generations gen ON gen.id = vg.generation_id \
             LEFT JOIN (SELECT version_group_id, MAX(region_id) as region_id FROM version_group_regions GROUP BY version_group_id) vgr ON vgr.version_group_id = vg.id \
             LEFT JOIN regions r ON r.id = vgr.region_id \
             LEFT JOIN region_names rn ON rn.region_id = r.id \
             LEFT JOIN version_names vn ON vn.version_id = v.id \
             WHERE g.connects_to_home = 1 AND p.species_id = ?1 \
             GROUP BY g.id ORDER BY g.id"
        )?;
        let fallback = stmt
            .query_map(params![species_id], |row| {
                Ok(GameInfo {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    connects_to_home: true,
                    transfer_direction: row.get(3)?,
                    generation: row.get(4)?,
                    region: row.get(5)?,
                    display_name: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        return Ok(fallback);
    }

    Ok(results)
}

pub fn get_home_missing(conn: &Connection, pokedex_id: i64, limit: u64, offset: u64) -> Result<(Vec<HomeMissingEntry>, u64)> {
    let total: u64 = conn.query_row(
        "SELECT COUNT(*) FROM pokemon_dex_numbers pdn \
         WHERE pdn.pokedex_id = ?1 \
         AND NOT EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND c.in_home = 1)",
        params![pokedex_id],
        |row| row.get(0),
    )?;

    let mut stmt = conn.prepare(
        "SELECT pdn.pokedex_number, pdn.species_id, s.name, COALESCE(sn.name, s.name), \
         EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND c.in_home = 0 AND c.status != 'traded_away') as owned_elsewhere \
         FROM pokemon_dex_numbers pdn \
         JOIN species s ON s.id = pdn.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         WHERE pdn.pokedex_id = ?1 \
         AND NOT EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND c.in_home = 1) \
         ORDER BY pdn.pokedex_number \
         LIMIT ?2 OFFSET ?3"
    )?;
    let entries: Vec<HomeMissingEntry> = stmt
        .query_map(params![pokedex_id, limit, offset], |row| {
            Ok(HomeMissingEntry {
                pokedex_number: row.get(0)?,
                species_id: row.get(1)?,
                name: row.get(2)?,
                display_name: row.get(3)?,
                owned_elsewhere: row.get::<_, i64>(4)? != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok((entries, total))
}

pub fn get_home_coverage(conn: &Connection) -> Result<DexProgress> {
    // Use the national dex (id=1) as baseline for HOME coverage
    let national_id = 1i64;
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pokemon_dex_numbers WHERE pokedex_id = ?1",
        params![national_id],
        |row| row.get(0),
    )?;
    let caught: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT pdn.species_id) FROM pokemon_dex_numbers pdn \
         INNER JOIN collection c ON c.species_id = pdn.species_id AND c.in_home = 1 \
         WHERE pdn.pokedex_id = ?1",
        params![national_id],
        |row| row.get(0),
    )?;
    let percentage = if total > 0 { ((caught as f64 / total as f64) * 10000.0).round() / 100.0 } else { 0.0 };

    Ok(DexProgress {
        dex_name: "national".to_string(),
        total,
        caught,
        percentage,
        entries: Vec::new(),
    })
}
