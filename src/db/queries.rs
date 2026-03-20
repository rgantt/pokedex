use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use super::models::*;

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
        Ok(r) => Ok(Some(r)),
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

    let evolves_from = if let Some(eid) = evolves_from_id {
        Some(get_display_name(conn, eid).unwrap_or_default())
    } else {
        None
    };

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

    let query_lower = query.to_lowercase();
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
            score,
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

    // Get evolution trigger info for this species
    let evo_info: Option<(String, String)> = conn.query_row(
        "SELECT et.name, \
         COALESCE( \
           CASE WHEN pe.minimum_level IS NOT NULL THEN 'Level ' || pe.minimum_level END, \
           CASE WHEN pe.trigger_item_id IS NOT NULL THEN 'Use ' || COALESCE((SELECT name FROM items WHERE id = pe.trigger_item_id), 'item') END, \
           CASE WHEN pe.minimum_happiness IS NOT NULL THEN 'Happiness ' || pe.minimum_happiness END, \
           CASE WHEN pe.known_move_id IS NOT NULL THEN 'Know ' || COALESCE((SELECT name FROM moves WHERE id = pe.known_move_id), 'move') END, \
           CASE WHEN pe.held_item_id IS NOT NULL THEN 'Hold ' || COALESCE((SELECT name FROM items WHERE id = pe.held_item_id), 'item') END, \
           '' \
         ) || CASE WHEN pe.time_of_day != '' THEN ' (' || pe.time_of_day || ')' ELSE '' END \
         FROM pokemon_evolution pe \
         JOIN evolution_triggers et ON et.id = pe.evolution_trigger_id \
         WHERE pe.evolved_species_id = ?1 LIMIT 1",
        params![species_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ).ok();

    let (trigger, trigger_detail) = match evo_info {
        Some((t, d)) => (Some(t), if d.is_empty() { None } else { Some(d) }),
        None => (None, None),
    };

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
        trigger,
        trigger_detail,
        children,
    })
}

pub fn get_pokemon_forms(conn: &Connection, species_id: i64) -> Result<Vec<PokemonForm>> {
    let mut stmt = conn.prepare(
        "SELECT pf.id, pf.pokemon_id, pf.name, \
         COALESCE(pfn.name, pf.form_name, 'Base'), \
         pf.form_name, pf.is_default, pf.is_mega, pf.is_battle_only \
         FROM pokemon_forms pf \
         JOIN pokemon p ON p.id = pf.pokemon_id \
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

    let mut sql = String::from(
        "SELECT DISTINCT \
         COALESCE(ln.name, l.name) as loc_name, \
         COALESCE(la.name, '') as area_name, \
         COALESCE(vn.name, v.name) as game_name, \
         COALESCE(emn.name, em.name) as method_name, \
         e.min_level, e.max_level, es.rarity, e.id \
         FROM encounters e \
         JOIN encounter_slots es ON es.id = e.encounter_slot_id \
         JOIN encounter_methods em ON em.id = es.encounter_method_id \
         LEFT JOIN encounter_method_names emn ON emn.encounter_method_id = em.id \
         JOIN location_areas la ON la.id = e.location_area_id \
         JOIN locations l ON l.id = la.location_id \
         LEFT JOIN location_names ln ON ln.location_id = l.id \
         JOIN versions v ON v.id = e.version_id \
         LEFT JOIN version_names vn ON vn.version_id = v.id \
         WHERE e.pokemon_id = ?1"
    );

    let mut bind_values: Vec<Box<dyn rusqlite::types::ToSql>> = vec![Box::new(pokemon_id)];

    if let Some(game) = game_filter {
        sql.push_str(" AND (LOWER(v.name) = LOWER(?2) OR LOWER(vn.name) = LOWER(?2))");
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
            ))
        })?
        .filter_map(|r| r.ok())
        .map(|(location, area, game, method, min_level, max_level, rarity, encounter_id)| {
            let conditions = get_encounter_conditions(conn, encounter_id);
            let details = get_encounter_details(conn, encounter_id);
            Encounter {
                pokemon_name: display_name.clone(),
                location,
                area,
                game,
                method,
                min_level,
                max_level,
                rarity,
                conditions,
                details,
            }
        })
        .collect();

    Ok(rows)
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
                hidden_ability_possible: row.get::<_, Option<i64>>(28)?.map(|v| v != 0),
                visible: row.get::<_, Option<i64>>(29)?.map(|v| v != 0),
                note: row.get(30)?,
            })
        },
    ).ok()
}

fn get_encounter_conditions(conn: &Connection, encounter_id: i64) -> Vec<String> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(ecvn.name, ecv.name) \
         FROM encounter_condition_value_map ecvm \
         JOIN encounter_condition_values ecv ON ecv.id = ecvm.encounter_condition_value_id \
         LEFT JOIN encounter_condition_value_names ecvn ON ecvn.encounter_condition_value_id = ecv.id \
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
) -> Result<Vec<PokemonMove>> {
    let pokemon_id: i64 = conn.query_row(
        "SELECT id FROM pokemon WHERE species_id = ?1 AND is_default = 1",
        params![species_id],
        |row| row.get(0),
    )?;

    let mut sql = String::from(
        "SELECT DISTINCT m.name, COALESCE(mn.name, m.name) as display_name, \
         COALESCE(tn.name, t.name) as type_name, m.power, m.accuracy, m.pp, \
         COALESCE(dcn.name, dc.name) as damage_class, \
         pmm.name as method_name, pm.level, vg.name as vg_name \
         FROM pokemon_moves pm \
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
        sql.push_str(
            " AND pm.version_group_id = (SELECT version_group_id FROM versions WHERE LOWER(name) = LOWER(?2) LIMIT 1)"
        );
        bind_values.push(Box::new(game.to_string()));
    }

    if let Some(method) = method_filter {
        let idx = bind_values.len() + 1;
        sql.push_str(&format!(" AND LOWER(pmm.name) = LOWER(?{idx})"));
        bind_values.push(Box::new(method.to_string()));
    }

    sql.push_str(" ORDER BY pm.pokemon_move_method_id, pm.level, m.name");

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

    Ok(results)
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
                species_name: row.get(2)?,
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
) -> Result<DexProgress> {
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM pokemon_dex_numbers WHERE pokedex_id = ?1",
        params![pokedex_id],
        |row| row.get(0),
    )?;

    // Build a subquery for "caught" species
    let mut caught_conditions = vec!["1=1".to_string()];
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

    let percentage = if total > 0 { (caught as f64 / total as f64) * 100.0 } else { 0.0 };

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
                species_name: row.get(2)?,
                display_name: row.get(3)?,
                caught: row.get::<_, i64>(4)? != 0,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok(DexProgress {
        dex_name: dex_name.to_string(),
        total,
        caught,
        percentage,
        entries,
    })
}

// ---- Game queries ----

pub fn list_games(conn: &Connection, home_only: bool) -> Result<Vec<GameInfo>> {
    let sql = if home_only {
        "SELECT id, name, connects_to_home, transfer_direction FROM games WHERE connects_to_home = 1 ORDER BY id"
    } else {
        "SELECT id, name, connects_to_home, transfer_direction FROM games ORDER BY id"
    };
    let mut stmt = conn.prepare(sql)?;
    let results = stmt
        .query_map([], |row| {
            Ok(GameInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                connects_to_home: row.get::<_, i64>(2)? != 0,
                transfer_direction: row.get(3)?,
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
    status: &str,
    method: Option<&str>,
    nickname: Option<&str>,
    notes: Option<&str>,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO collection (species_id, form_id, game_id, shiny, in_home, status, method, nickname, notes) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![species_id, form_id, game_id, shiny, in_home, status, method, nickname, notes],
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

    if sets.is_empty() {
        return Ok(false);
    }

    sets.push(format!("updated_at = datetime('now')"));

    let sql = format!("UPDATE collection SET {} WHERE id = ?{idx}", sets.join(", "));
    bind_values.push(Box::new(id));

    let params: Vec<&dyn rusqlite::types::ToSql> = bind_values.iter().map(|b| b.as_ref()).collect();
    let rows = conn.execute(&sql, params.as_slice())?;
    Ok(rows > 0)
}

pub fn get_collection_entry(conn: &Connection, id: i64) -> Result<Option<CollectionEntry>> {
    let result = conn.query_row(
        "SELECT c.id, c.species_id, s.name, COALESCE(sn.name, s.name), \
         pf.form_name, g.name, c.shiny, c.in_home, c.status, \
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
                status: row.get(8)?,
                method: row.get(9)?,
                nickname: row.get(10)?,
                notes: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
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
        "SELECT c.id, c.species_id, s.name, COALESCE(sn.name, s.name), \
         pf.form_name, g.name, c.shiny, c.in_home, c.status, \
         c.method, c.nickname, c.notes, c.created_at, c.updated_at \
         FROM collection c \
         JOIN species s ON s.id = c.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         LEFT JOIN pokemon_forms pf ON pf.id = c.form_id \
         LEFT JOIN games g ON g.id = c.game_id \
         {where_clause} \
         ORDER BY c.id DESC \
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
                status: row.get(8)?,
                method: row.get(9)?,
                nickname: row.get(10)?,
                notes: row.get(11)?,
                created_at: row.get(12)?,
                updated_at: row.get(13)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    Ok((entries, total))
}

pub fn get_collection_stats(conn: &Connection) -> Result<CollectionStats> {
    let total_entries: i64 = conn.query_row(
        "SELECT COUNT(*) FROM collection", [], |row| row.get(0),
    )?;
    let unique_species: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT species_id) FROM collection", [], |row| row.get(0),
    )?;
    let shiny_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM collection WHERE shiny = 1", [], |row| row.get(0),
    )?;
    let in_home_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM collection WHERE in_home = 1", [], |row| row.get(0),
    )?;

    let mut status_stmt = conn.prepare(
        "SELECT status, COUNT(*) FROM collection GROUP BY status ORDER BY status"
    )?;
    let by_status: Vec<StatusCount> = status_stmt
        .query_map([], |row| Ok(StatusCount { status: row.get(0)?, count: row.get(1)? }))?
        .filter_map(|r| r.ok())
        .collect();

    let mut game_stmt = conn.prepare(
        "SELECT g.name, COUNT(*) FROM collection c \
         LEFT JOIN games g ON g.id = c.game_id \
         GROUP BY c.game_id ORDER BY COUNT(*) DESC"
    )?;
    let by_game: Vec<GameCount> = game_stmt
        .query_map([], |row| Ok(GameCount {
            game: row.get::<_, Option<String>>(0)?.unwrap_or_else(|| "unknown".to_string()),
            count: row.get(1)?,
        }))?
        .filter_map(|r| r.ok())
        .collect();

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

pub fn get_home_transferable(conn: &Connection, _species_id: i64) -> Result<Vec<GameInfo>> {
    // A species can transfer to a game if it exists in that game's version group
    // For simplicity, return all HOME-compatible games
    // TODO: Filter by actual game species availability when game_species is populated
    let mut stmt = conn.prepare(
        "SELECT id, name, connects_to_home, transfer_direction FROM games \
         WHERE connects_to_home = 1 ORDER BY id"
    )?;
    let results = stmt
        .query_map([], |row| {
            Ok(GameInfo {
                id: row.get(0)?,
                name: row.get(1)?,
                connects_to_home: true,
                transfer_direction: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(results)
}

pub fn get_home_missing(conn: &Connection, pokedex_id: i64) -> Result<Vec<DexEntry>> {
    let mut stmt = conn.prepare(
        "SELECT pdn.pokedex_number, pdn.species_id, s.name, COALESCE(sn.name, s.name) \
         FROM pokemon_dex_numbers pdn \
         JOIN species s ON s.id = pdn.species_id \
         LEFT JOIN species_names sn ON sn.species_id = s.id \
         WHERE pdn.pokedex_id = ?1 \
         AND NOT EXISTS (SELECT 1 FROM collection c WHERE c.species_id = pdn.species_id AND c.in_home = 1) \
         ORDER BY pdn.pokedex_number"
    )?;
    let entries: Vec<DexEntry> = stmt
        .query_map(params![pokedex_id], |row| {
            Ok(DexEntry {
                pokedex_number: row.get(0)?,
                species_id: row.get(1)?,
                species_name: row.get(2)?,
                display_name: row.get(3)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();
    Ok(entries)
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
    let percentage = if total > 0 { (caught as f64 / total as f64) * 100.0 } else { 0.0 };

    Ok(DexProgress {
        dex_name: "national".to_string(),
        total,
        caught,
        percentage,
        entries: Vec::new(),
    })
}
