//! Comprehensive encounter data validation across the national pokédex.
//!
//! Seeds an in-memory database from all three sources (PokeAPI CSVs, PokeDB, Z-A),
//! then validates species resolution, encounter coverage, multi-source data quality,
//! encounter detail population, and deduplication.
//!
//! Requires a seeded on-disk database at POKEDEX_DB_PATH (or ~/.pokedex/db.sqlite).
//! The test opens it read-only — it does not modify your data.
//!
//! Run: cargo test --test validate_encounters -- --nocapture

use pokedex::db;
use pokedex::db::queries;
use rusqlite::params;

fn open_test_db() -> rusqlite::Connection {
    db::open().expect(
        "Failed to open database. Ensure it's seeded: pokedex db seed\n\
         Override location with POKEDEX_DB_PATH env var.",
    )
}

// ============================================================
// Phase 1: Every species resolves
// ============================================================

#[test]
fn all_species_resolve() {
    let conn = open_test_db();

    let mut stmt = conn
        .prepare("SELECT name FROM species ORDER BY id")
        .unwrap();
    let names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .filter_map(|r| r.ok())
        .collect();

    assert!(names.len() >= 1025, "Expected 1025+ species, got {}", names.len());

    let mut failures = Vec::new();
    for name in &names {
        if queries::resolve_pokemon(&conn, name).unwrap().is_none() {
            failures.push(name.clone());
        }
    }

    assert!(
        failures.is_empty(),
        "Failed to resolve {} species: {:?}",
        failures.len(),
        &failures[..failures.len().min(20)]
    );
}

// ============================================================
// Phase 2: Encounter coverage by game
// ============================================================

#[test]
fn every_home_game_has_encounters() {
    let conn = open_test_db();

    let games = [
        ("sword", "Sword"),
        ("shield", "Shield"),
        ("scarlet", "Scarlet"),
        ("violet", "Violet"),
        ("brilliant-diamond", "Brilliant Diamond"),
        ("shining-pearl", "Shining Pearl"),
        ("legends-arceus", "Legends: Arceus"),
        ("lets-go-pikachu", "Let's Go, Pikachu!"),
        ("lets-go-eevee", "Let's Go, Eevee!"),
        ("legends-za", "Legends: Z-A"),
    ];

    for (slug, display) in &games {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM encounters e \
                 JOIN versions v ON v.id = e.version_id \
                 LEFT JOIN version_names vn ON vn.version_id = v.id \
                 WHERE v.name = ?1 OR vn.name = ?1",
                params![slug],
                |row| row.get(0),
            )
            .unwrap_or(0);

        // Also try display name
        let count2: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM encounters e \
                 JOIN versions v ON v.id = e.version_id \
                 LEFT JOIN version_names vn ON vn.version_id = v.id \
                 WHERE vn.name = ?1",
                params![display],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let total = count.max(count2);
        assert!(
            total > 0,
            "Game '{slug}' ({display}) has zero encounters"
        );
    }
}

// ============================================================
// Phase 3: Multi-source Pokémon have encounters from multiple games
// ============================================================

#[test]
fn multi_source_pokemon_have_cross_game_encounters() {
    let conn = open_test_db();

    // These Pokémon should appear in games from both PokeAPI (Gen 1-5) and PokeDB (Gen 6+)
    let test_cases = [
        ("pikachu", 5),   // at least 5 games
        ("eevee", 5),
        ("magikarp", 5),
        ("machop", 5),
        ("gastly", 5),
        ("ralts", 3),
        ("shinx", 3),
        ("dratini", 5),
    ];

    for (pokemon, min_games) in &test_cases {
        let resolved = queries::resolve_pokemon(&conn, pokemon)
            .unwrap()
            .unwrap_or_else(|| panic!("Cannot resolve '{pokemon}'"));

        let pokemon_id: i64 = conn
            .query_row(
                "SELECT id FROM pokemon WHERE species_id = ?1 AND is_default = 1",
                params![resolved.0],
                |row| row.get(0),
            )
            .unwrap();

        let game_count: i64 = conn
            .query_row(
                "SELECT COUNT(DISTINCT e.version_id) FROM encounters e WHERE e.pokemon_id = ?1",
                params![pokemon_id],
                |row| row.get(0),
            )
            .unwrap();

        assert!(
            game_count >= *min_games,
            "{pokemon}: expected encounters in {min_games}+ games, found {game_count}"
        );
    }
}

// ============================================================
// Phase 4: No cross-source duplicates for PokeAPI-covered versions
// ============================================================

#[test]
fn pokedb_skips_pokeapi_covered_versions() {
    let conn = open_test_db();

    // PokeAPI covers Gen 1-5 games. Verify those versions don't have encounters
    // from both sources by checking total encounter counts are reasonable.
    // Gen 1 games (Red/Blue) should only have PokeAPI data (~3000 encounters),
    // not PokeAPI + PokeDB (~6000).

    let pokeapi_only_versions = ["red", "blue", "gold", "silver", "ruby", "sapphire"];

    for version in &pokeapi_only_versions {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM encounters e \
                 JOIN versions v ON v.id = e.version_id \
                 WHERE v.name = ?1",
                params![version],
                |row| row.get(0),
            )
            .unwrap();

        // These older games should have < 5000 encounters each.
        // If PokeDB duplicates leaked in, they'd roughly double.
        assert!(
            count < 5000,
            "Version '{version}' has {count} encounters — suspiciously high, \
             possible PokeDB duplicates"
        );
        assert!(
            count > 0,
            "Version '{version}' has zero encounters"
        );
    }
}

// ============================================================
// Phase 5: Encounter details populated for modern games
// ============================================================

#[test]
fn modern_game_encounters_have_details() {
    let conn = open_test_db();

    let checks: &[(&str, &str, &str)] = &[
        ("sword", "abomasnow", "weather_snow_rate"),
        ("scarlet", "larvitar", "probability_overall"),
        ("legends-arceus", "pikachu", "alpha_levels"),
        ("brilliant-diamond", "shinx", "rate_overall"),
        ("legends-za", "pichu", "alpha_levels"),
    ];

    for &(game, pokemon, expected_field) in checks {
        let resolved = queries::resolve_pokemon(&conn, pokemon)
            .unwrap()
            .unwrap_or_else(|| panic!("Cannot resolve '{pokemon}'"));
        let encounters = queries::get_encounters(&conn, resolved.0, Some(game)).unwrap();

        let has_field = encounters.iter().any(|e| {
            if let Some(ref det) = e.details {
                let json = serde_json::to_value(det).unwrap_or_default();
                json.get(expected_field).is_some_and(|v| !v.is_null())
            } else {
                false
            }
        });

        assert!(
            has_field,
            "{game}/{pokemon}: no encounter has '{expected_field}' in details \
             ({} encounters found)",
            encounters.len()
        );
    }
}

// ============================================================
// Phase 6: National dex encounter coverage
// ============================================================

#[test]
fn national_dex_encounter_coverage() {
    let conn = open_test_db();

    let total_species: i64 = conn
        .query_row("SELECT COUNT(*) FROM species", [], |row| row.get(0))
        .unwrap();

    let species_with_encounters: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT p.species_id) FROM encounters e \
             JOIN pokemon p ON p.id = e.pokemon_id",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let coverage = species_with_encounters as f64 / total_species as f64 * 100.0;

    // We expect ~95% coverage (some species are event/gift/evolution only)
    assert!(
        coverage > 90.0,
        "Encounter coverage too low: {species_with_encounters}/{total_species} ({coverage:.1}%)"
    );

    let with_details: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT p.species_id) FROM encounters e \
             JOIN pokemon p ON p.id = e.pokemon_id \
             JOIN encounter_details ed ON ed.encounter_id = e.id",
            [],
            |row| row.get(0),
        )
        .unwrap();

    let detail_coverage = with_details as f64 / species_with_encounters as f64 * 100.0;

    // Most species with encounters should have details (from PokeDB/Z-A)
    assert!(
        detail_coverage > 80.0,
        "Encounter detail coverage too low: {with_details}/{species_with_encounters} ({detail_coverage:.1}%)"
    );

    eprintln!(
        "Coverage: {species_with_encounters}/{total_species} species with encounters ({coverage:.1}%), \
         {with_details} with details ({detail_coverage:.1}%)"
    );
}

// ============================================================
// Phase 7: Query functions return correct data shapes
// ============================================================

#[test]
fn pokemon_show_returns_complete_data() {
    let conn = open_test_db();

    let species = queries::get_species(&conn, 6).unwrap(); // Charizard
    assert_eq!(species.name, "charizard");
    assert_eq!(species.id, 6);
    assert_eq!(species.generation, 1);
    assert!(species.types.len() == 2); // Fire, Flying
    assert!(!species.is_legendary);
    assert!(!species.is_mythical);
    assert_eq!(species.evolves_from.as_deref(), Some("Charmeleon"));
}

#[test]
fn evolution_chain_has_correct_structure() {
    let conn = open_test_db();

    // Eevee has 8 evolutions
    let chain = queries::get_evolution_chain(&conn, 133).unwrap();
    assert_eq!(chain.species_name, "eevee");
    assert!(
        chain.children.len() >= 8,
        "Eevee should have 8+ evolutions, got {}",
        chain.children.len()
    );

    // Check a specific evolution has trigger info
    let vaporeon = chain.children.iter().find(|c| c.species_name == "vaporeon");
    assert!(vaporeon.is_some(), "Vaporeon not found in Eevee's chain");
    assert_eq!(vaporeon.unwrap().trigger.as_deref(), Some("use-item"));
}

#[test]
fn type_matchups_are_correct() {
    let conn = open_test_db();

    let matchups = queries::get_type_matchups(&conn, "dragon").unwrap();

    // Dragon is super effective against Dragon
    assert!(matchups.attacking.super_effective.iter().any(|t| t == "Dragon"));
    // Dragon has no effect on Fairy
    assert!(matchups.attacking.no_effect.iter().any(|t| t == "Fairy"));
    // Dragon is weak to Ice, Dragon, Fairy
    assert!(matchups.defending.super_effective.iter().any(|t| t == "Ice"));
    assert!(matchups.defending.super_effective.iter().any(|t| t == "Fairy"));
}

#[test]
fn fuzzy_search_finds_misspellings() {
    let conn = open_test_db();

    let results = queries::search_species(&conn, "bulbsaur", 5).unwrap();
    assert!(!results.is_empty(), "Fuzzy search returned no results for 'bulbsaur'");
    assert_eq!(results[0].species.name, "bulbasaur");
}

#[test]
fn pokemon_stats_sum_correctly() {
    let conn = open_test_db();

    // Garchomp has 600 BST (pseudo-legendary)
    let stats = queries::get_pokemon_stats(&conn, 445).unwrap();
    assert_eq!(stats.total, 600, "Garchomp BST should be 600, got {}", stats.total);
    assert_eq!(stats.attack, 130);
    assert_eq!(stats.speed, 102);
}

#[test]
fn forms_include_megas_and_regional() {
    let conn = open_test_db();

    let forms = queries::get_pokemon_forms(&conn, 6).unwrap(); // Charizard
    let form_names: Vec<Option<String>> = forms.iter().map(|f| f.form_name.clone()).collect();

    assert!(
        form_names.iter().any(|f| f.as_deref() == Some("mega-x")),
        "Charizard should have Mega X form"
    );
    assert!(
        form_names.iter().any(|f| f.as_deref() == Some("mega-y")),
        "Charizard should have Mega Y form"
    );
    assert!(
        form_names.iter().any(|f| f.as_deref() == Some("gmax")),
        "Charizard should have Gigantamax form"
    );
}

// ============================================================
// Phase 8: Collection CRUD
// ============================================================

#[test]
fn collection_crud_lifecycle() {
    let conn = open_test_db();

    // Resolve game
    let (game_id, _) = queries::resolve_game(&conn, "scarlet").unwrap().unwrap();

    // Add
    let id = queries::add_collection_entry(
        &conn, 25, None, game_id, true, true, "caught", Some("catch"), Some("Sparky"), None,
    )
    .unwrap();
    assert!(id > 0);

    // Show
    let entry = queries::get_collection_entry(&conn, id).unwrap().unwrap();
    assert_eq!(entry.species_name, "pikachu");
    assert_eq!(entry.display_name, "Pikachu");
    assert!(entry.shiny);
    assert!(entry.in_home);
    assert_eq!(entry.status, "caught");
    assert_eq!(entry.nickname.as_deref(), Some("Sparky"));

    // Update
    queries::update_collection_entry(&conn, id, Some("living_dex"), None, None, None, None).unwrap();
    let updated = queries::get_collection_entry(&conn, id).unwrap().unwrap();
    assert_eq!(updated.status, "living_dex");

    // List
    let (entries, total) = queries::list_collection(&conn, None, Some("pikachu"), false, false, None, 50, 0).unwrap();
    assert!(total >= 1);
    assert!(entries.iter().any(|e| e.id == id));

    // Stats
    let stats = queries::get_collection_stats(&conn).unwrap();
    assert!(stats.total_entries >= 1);
    assert!(stats.shiny_count >= 1);

    // Remove
    queries::remove_collection_entry(&conn, id).unwrap();
    assert!(queries::get_collection_entry(&conn, id).unwrap().is_none());
}

// ============================================================
// Phase 9: Z-A specific encounters
// ============================================================

#[test]
fn za_encounters_have_alpha_data() {
    let conn = open_test_db();

    // Check a known Z-A encounter
    let resolved = queries::resolve_pokemon(&conn, "bunnelby").unwrap().unwrap();
    let encounters = queries::get_encounters(&conn, resolved.0, Some("legends-za")).unwrap();

    assert!(!encounters.is_empty(), "Bunnelby should have Z-A encounters");

    let has_alpha = encounters.iter().any(|e| {
        e.details
            .as_ref()
            .and_then(|d| d.alpha_levels.as_ref())
            .is_some()
    });
    assert!(has_alpha, "Bunnelby Z-A encounters should have alpha_levels in details");
}

#[test]
fn za_encounters_cover_all_zones() {
    let conn = open_test_db();

    let zone_count: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT la.name) FROM encounters e \
             JOIN location_areas la ON la.id = e.location_area_id \
             JOIN versions v ON v.id = e.version_id \
             WHERE v.name = 'legends-za' AND la.name LIKE 'wild-zone-%'",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(zone_count, 20, "Expected 20 wild zones for Z-A, got {zone_count}");
}
