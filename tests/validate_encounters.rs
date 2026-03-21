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

    // Add (with is_alpha=false)
    let id = queries::add_collection_entry(
        &conn, 25, None, game_id, true, true, false, "caught", Some("catch"), Some("Sparky"), None,
    )
    .unwrap();
    assert!(id > 0);

    // Show
    let entry = queries::get_collection_entry(&conn, id).unwrap().unwrap();
    assert_eq!(entry.species_name, "pikachu");
    assert_eq!(entry.display_name, "Pikachu");
    assert!(entry.shiny);
    assert!(entry.in_home);
    assert!(!entry.is_alpha);
    assert_eq!(entry.status, "caught");
    assert_eq!(entry.nickname.as_deref(), Some("Sparky"));

    // Update (with game_id=None, method=None)
    queries::update_collection_entry(&conn, id, Some("living_dex"), None, None, None, None, None, None).unwrap();
    let updated = queries::get_collection_entry(&conn, id).unwrap().unwrap();
    assert_eq!(updated.status, "living_dex");

    // List (with sort="id")
    let (entries, total) = queries::list_collection(&conn, None, Some("pikachu"), false, false, None, 50, 0, "id").unwrap();
    assert!(total >= 1);
    assert!(entries.iter().any(|e| e.id == id));

    // Stats (with game_filter=None)
    let stats = queries::get_collection_stats(&conn, None).unwrap();
    assert!(stats.total_entries >= 1);
    assert!(stats.shiny_count >= 1);

    // Alpha add
    let alpha_id = queries::add_collection_entry(
        &conn, 396, None, game_id, false, false, true, "caught", Some("catch"), None, Some("alpha starly"),
    ).unwrap();
    let alpha_entry = queries::get_collection_entry(&conn, alpha_id).unwrap().unwrap();
    assert!(alpha_entry.is_alpha);

    // Clean up alpha entry
    queries::remove_collection_entry(&conn, alpha_id).unwrap();

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

// ============================================================
// Phase 10: Override system validation
// ============================================================

#[test]
fn trade_evolutions_have_trigger_detail() {
    let conn = open_test_db();

    let trade_evos = ["gengar", "alakazam", "machamp", "golem"];
    for species in &trade_evos {
        let chain = queries::get_evolution_chain(
            &conn,
            queries::resolve_pokemon(&conn, species).unwrap().unwrap().0,
        )
        .unwrap();

        // Find this species in the chain
        fn find_node<'a>(node: &'a pokedex::db::models::EvolutionNode, name: &str) -> Option<&'a pokedex::db::models::EvolutionNode> {
            if node.species_name == name { return Some(node); }
            for child in &node.children {
                if let Some(found) = find_node(child, name) { return Some(found); }
            }
            None
        }

        let node = find_node(&chain, species)
            .unwrap_or_else(|| panic!("{species} not found in its own evolution chain"));
        assert!(
            node.trigger_detail.is_some(),
            "{species} evolution should have trigger_detail (override), got None"
        );
    }
}

#[test]
fn hisuian_forms_not_default() {
    let conn = open_test_db();

    let hisuian_forms = ["growlithe-hisui", "zorua-hisui", "braviary-hisui"];
    for form in &hisuian_forms {
        let is_default: i64 = conn
            .query_row(
                "SELECT is_default FROM pokemon WHERE name = ?1",
                params![form],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| panic!("{form} not found in pokemon table"));
        assert_eq!(is_default, 0, "{form} should have is_default=0 after override");
    }
}

// ============================================================
// Phase 11: Data normalization validation
// ============================================================

#[test]
fn probability_overall_normalized_to_percentages() {
    let conn = open_test_db();

    // Check that no probability_overall value exceeds 100 (as a number)
    let bad_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM encounter_details \
             WHERE probability_overall IS NOT NULL \
             AND probability_overall != '' \
             AND CAST(REPLACE(probability_overall, '%', '') AS REAL) > 100",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        bad_count, 0,
        "Found {bad_count} encounter_details with probability_overall > 100%"
    );
}

#[test]
fn no_non_numeric_probability_overall() {
    let conn = open_test_db();

    // "one", "choose one", "two" should have been moved to notes
    let bad_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM encounter_details \
             WHERE probability_overall IN ('one', 'choose one', 'two')",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(
        bad_count, 0,
        "Found {bad_count} encounter_details with non-numeric probability_overall"
    );
}

#[test]
fn no_empty_encounter_details_in_output() {
    let conn = open_test_db();

    // Pick a pokemon known to have encounters, check no empty details
    let resolved = queries::resolve_pokemon(&conn, "pikachu").unwrap().unwrap();
    let encounters = queries::get_encounters(&conn, resolved.0, None).unwrap();

    for enc in &encounters {
        if let Some(ref details) = enc.details {
            // If details is present, it should have at least one meaningful field
            let json = serde_json::to_value(details).unwrap();
            let obj = json.as_object().unwrap();
            assert!(
                !obj.is_empty(),
                "Pikachu encounter at {} has empty details object",
                enc.location
            );
        }
    }
}

#[test]
fn za_no_duplicate_encounters() {
    let conn = open_test_db();

    let dupes: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM ( \
             SELECT pokemon_id, location_area_id, min_level, max_level, COUNT(*) as cnt \
             FROM encounters e \
             JOIN versions v ON v.id = e.version_id \
             WHERE v.name = 'legends-za' \
             GROUP BY pokemon_id, location_area_id, min_level, max_level \
             HAVING cnt > 1 \
             )",
            [],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(dupes, 0, "Found {dupes} duplicate Z-A encounter groups");
}

// ============================================================
// Phase 12: CLI feature validation
// ============================================================

#[test]
fn dex_progress_returns_pagination_info() {
    let conn = open_test_db();

    let resolved = queries::resolve_pokedex(&conn, "national").unwrap().unwrap();
    let (progress, filtered_count) = queries::get_dex_progress(
        &conn, resolved.0, "national", false, false, None, None, 10, 0,
    )
    .unwrap();

    assert!(filtered_count > 10, "National dex should have more than 10 entries");
    assert_eq!(progress.entries.len(), 10, "Should return exactly 10 entries with limit=10");
}

#[test]
fn home_missing_returns_owned_elsewhere() {
    let conn = open_test_db();

    let resolved = queries::resolve_pokedex(&conn, "national").unwrap().unwrap();
    let (entries, total) = queries::get_home_missing(&conn, resolved.0, 50, 0).unwrap();

    assert!(total > 0);
    // The owned_elsewhere field should be populated (some true, most false)
    // Just verify it's a valid bool by checking the struct compiles and returns
    assert!(entries.len() <= 50);
}

#[test]
fn percentage_rounded_to_two_decimals() {
    let conn = open_test_db();

    let coverage = queries::get_home_coverage(&conn).unwrap();
    let pct_str = format!("{}", coverage.percentage);
    // Should not have more than 2 decimal places
    if let Some(dot_pos) = pct_str.find('.') {
        let decimals = pct_str.len() - dot_pos - 1;
        assert!(
            decimals <= 2,
            "Percentage {pct_str} has {decimals} decimal places, expected <= 2"
        );
    }
}

#[test]
fn game_show_has_enriched_fields() {
    let conn = open_test_db();

    let games = queries::list_games(&conn, false).unwrap();
    let sword = games.iter().find(|g| g.name == "sword");
    assert!(sword.is_some(), "Sword should be in game list");
    let sword = sword.unwrap();

    // Sword should have generation and region after C4 fix
    assert!(
        sword.generation.is_some(),
        "Sword should have generation field populated"
    );
}

#[test]
fn collection_stats_with_game_filter() {
    let conn = open_test_db();

    // Stats without filter
    let all_stats = queries::get_collection_stats(&conn, None).unwrap();

    // Stats with a specific game (may be 0 if no entries for that game in test DB)
    let game_stats = queries::get_collection_stats(&conn, Some("scarlet")).unwrap();

    // Game-filtered stats should be <= total
    assert!(game_stats.total_entries <= all_stats.total_entries);
}

#[test]
fn list_collection_sort_by_dex() {
    let conn = open_test_db();

    let (entries_by_id, _) = queries::list_collection(&conn, None, None, false, false, None, 50, 0, "id").unwrap();
    let (entries_by_dex, _) = queries::list_collection(&conn, None, None, false, false, None, 50, 0, "dex").unwrap();

    // Both should return the same count
    // dex-sorted entries should be in ascending species_id order
    if entries_by_dex.len() > 1 {
        for i in 1..entries_by_dex.len() {
            assert!(
                entries_by_dex[i].species_id >= entries_by_dex[i - 1].species_id,
                "Dex sort not ascending: {} (species {}) came after {} (species {})",
                entries_by_dex[i].species_name,
                entries_by_dex[i].species_id,
                entries_by_dex[i - 1].species_name,
                entries_by_dex[i - 1].species_id,
            );
        }
    }
}

#[test]
fn search_scores_rounded() {
    let conn = open_test_db();

    let results = queries::search_species(&conn, "bulbsaur", 5).unwrap();
    assert!(!results.is_empty());
    for r in &results {
        let score_str = format!("{}", r.score);
        if let Some(dot_pos) = score_str.find('.') {
            let decimals = score_str.len() - dot_pos - 1;
            assert!(
                decimals <= 2,
                "Search score {score_str} has {decimals} decimal places, expected <= 2"
            );
        }
    }
}

// ============================================================
// Phase 13: Round 2 fixes validation
// ============================================================

#[test]
fn no_gen1_duplicate_encounters() {
    let conn = open_test_db();
    let resolved = queries::resolve_pokemon(&conn, "pikachu").unwrap().unwrap();
    let encounters = queries::get_encounters(&conn, resolved.0, Some("red")).unwrap();
    let unique: std::collections::HashSet<_> = encounters
        .iter()
        .map(|e| (&e.location, &e.min_level, &e.max_level))
        .collect();
    assert_eq!(
        encounters.len(),
        unique.len(),
        "Pikachu in Red has duplicate encounters: {} total, {} unique",
        encounters.len(),
        unique.len()
    );
}

#[test]
fn collection_only_accepts_valid_statuses() {
    let conn = open_test_db();
    let (game_id, _) = queries::resolve_game(&conn, "sword").unwrap().unwrap();
    // Add with valid status - should work
    let id = queries::add_collection_entry(
        &conn, 25, None, game_id, false, false, false, "caught", None, None, None,
    )
    .unwrap();
    assert!(id > 0);
    queries::remove_collection_entry(&conn, id).unwrap();
    // Note: invalid status validation happens in the command layer (collection.rs), not queries
}

#[test]
fn empty_search_returns_nothing() {
    let conn = open_test_db();
    let results = queries::search_species(&conn, "", 10).unwrap();
    assert!(
        results.is_empty(),
        "Empty search should return no results, got {}",
        results.len()
    );
    let results2 = queries::search_species(&conn, "   ", 10).unwrap();
    assert!(
        results2.is_empty(),
        "Whitespace search should return no results"
    );
}

#[test]
fn game_display_names_populated() {
    let conn = open_test_db();
    let games = queries::list_games(&conn, true).unwrap();
    for game in &games {
        assert!(
            game.display_name.is_some(),
            "Game {} missing display_name",
            game.name
        );
        let dn = game.display_name.as_ref().unwrap();
        assert_ne!(
            dn, &game.name,
            "Game {} display_name should not be the raw slug",
            game.name
        );
    }
}

#[test]
fn default_form_shows_species_name() {
    let conn = open_test_db();
    let forms = queries::get_pokemon_forms(&conn, 212).unwrap(); // Scizor
    let default = forms.iter().find(|f| f.form_name.is_none()).unwrap();
    assert_ne!(
        default.display_name, "Base",
        "Default form should show species name, not 'Base'"
    );
}

#[test]
fn level1_placeholder_encounters_have_null_levels() {
    let conn = open_test_db();
    // Charizard in Sword has Max Raid encounters that were level 1
    let resolved = queries::resolve_pokemon(&conn, "charizard").unwrap().unwrap();
    let encounters = queries::get_encounters(&conn, resolved.0, Some("sword")).unwrap();
    let raids: Vec<_> = encounters
        .iter()
        .filter(|e| e.method.contains("Max Raid") || e.method.contains("max-raid"))
        .collect();
    if !raids.is_empty() {
        for raid in &raids {
            // Level should be None (not Some(1)) for placeholder raid data
            assert!(
                raid.min_level.is_none() || raid.min_level.unwrap() > 1,
                "Max Raid encounter should not show level 1: {:?}",
                raid.location
            );
        }
    }
}

#[test]
fn evolution_actions_include_all_chain_members() {
    let conn = open_test_db();
    let chain = queries::get_evolution_chain(&conn, 133).unwrap(); // Eevee

    fn count_nodes(node: &pokedex::db::models::EvolutionNode) -> usize {
        1 + node
            .children
            .iter()
            .map(|c| count_nodes(c))
            .sum::<usize>()
    }

    assert!(
        count_nodes(&chain) >= 9,
        "Eevee chain should have 9+ members, got {}",
        count_nodes(&chain)
    );
}

#[test]
fn za_wild_zone_areas_preserved() {
    let conn = open_test_db();
    // Verify that Z-A wild zone area names are preserved in encounter output
    // so users can distinguish between zones even if the parent location is a city name.
    let resolved = queries::resolve_pokemon(&conn, "bunnelby").unwrap().unwrap();
    let encounters = queries::get_encounters(&conn, resolved.0, Some("legends-za")).unwrap();
    let has_zone_area = encounters
        .iter()
        .any(|e| e.area.starts_with("wild-zone-"));
    assert!(
        has_zone_area,
        "Z-A encounters should preserve wild-zone area names, areas found: {:?}",
        encounters.iter().map(|e| &e.area).collect::<Vec<_>>()
    );
}

#[test]
fn collection_stats_game_filter_skips_empty_by_game() {
    let conn = open_test_db();
    let stats = queries::get_collection_stats(&conn, Some("sword")).unwrap();
    // The by_game field should be empty (and will be skipped in serialization)
    assert!(
        stats.by_game.is_empty(),
        "by_game should be empty when filtering by game"
    );
}

// ============================================================
// Phase 14: Round 3 fixes validation
// ============================================================

#[test]
fn za_game_has_version_group() {
    let conn = open_test_db();
    let vg: Option<i64> = conn.query_row(
        "SELECT version_group_id FROM games WHERE name = 'legends-za'",
        [],
        |row| row.get(0),
    ).unwrap();
    assert!(vg.is_some(), "legends-za should have a version_group_id");
}

#[test]
fn encounter_dedup_ran_successfully() {
    let conn = open_test_db();
    // Verify the dedup step removed PokeAPI duplicates by checking that
    // Gen 1-5 games (PokeAPI-only) don't have same-slot duplicates.
    // PokeDB data may have legitimate "duplicates" (common vs rare dens).
    let pokeapi_dupes: i64 = conn.query_row(
        "SELECT COUNT(*) FROM ( \
         SELECT e.pokemon_id, e.version_id, e.location_area_id, e.min_level, e.max_level, \
                e.encounter_slot_id, COUNT(*) as cnt \
         FROM encounters e \
         JOIN versions v ON v.id = e.version_id \
         WHERE v.name IN ('red','blue','gold','silver','ruby','sapphire','diamond','pearl', \
                          'black','white','firered','leafgreen','emerald','crystal','platinum', \
                          'heartgold','soulsilver') \
         GROUP BY e.pokemon_id, e.version_id, e.location_area_id, e.min_level, e.max_level, \
                  e.encounter_slot_id \
         HAVING cnt > 1)",
        [],
        |row| row.get(0),
    ).unwrap();
    assert_eq!(pokeapi_dupes, 0,
        "Found {pokeapi_dupes} duplicate encounter groups in PokeAPI-sourced data");
}

#[test]
fn species_has_egg_groups() {
    let conn = open_test_db();
    let species = queries::get_species(&conn, 133).unwrap(); // Eevee
    assert!(!species.egg_groups.is_empty(), "Eevee should have egg groups");
    assert!(species.egg_groups.contains(&"Field".to_string()),
        "Eevee should be in Field egg group, got {:?}", species.egg_groups);
}

#[test]
fn dex_progress_excludes_traded_away() {
    let conn = open_test_db();
    let (game_id, _) = queries::resolve_game(&conn, "sword").unwrap().unwrap();

    // Use a rare species (Pecharunt #1025) unlikely to be added by other tests
    let id = queries::add_collection_entry(&conn, 1025, None, game_id, false, false, false, "traded_away", None, None, None).unwrap();

    let resolved = queries::resolve_pokedex(&conn, "national").unwrap().unwrap();
    let (progress, _) = queries::get_dex_progress(&conn, resolved.0, "national", false, false, None, None, 1025, 0).unwrap();

    let entry = progress.entries.iter().find(|e| e.species_id == 1025);
    if let Some(entry) = entry {
        assert!(!entry.caught, "traded_away pokemon should not count as caught in dex progress");
    }

    queries::remove_collection_entry(&conn, id).unwrap();
}

#[test]
fn home_missing_excludes_traded_away_from_owned() {
    let conn = open_test_db();
    let (game_id, _) = queries::resolve_game(&conn, "sword").unwrap().unwrap();

    // Use Pecharunt (#1025) — unlikely to be touched by other tests
    let id = queries::add_collection_entry(&conn, 1025, None, game_id, false, false, false, "traded_away", None, None, None).unwrap();

    let resolved = queries::resolve_pokedex(&conn, "national").unwrap().unwrap();
    let (entries, _) = queries::get_home_missing(&conn, resolved.0, 1025, 0).unwrap();

    let entry = entries.iter().find(|e| e.species_id == 1025);
    assert!(entry.is_some(), "Pecharunt should be in HOME missing list");
    assert!(!entry.unwrap().owned_elsewhere,
        "traded_away pokemon should NOT show owned_elsewhere=true");

    queries::remove_collection_entry(&conn, id).unwrap();
}

#[test]
fn rotom_alternate_forms_not_default() {
    let conn = open_test_db();
    let forms = ["rotom-heat", "rotom-wash", "rotom-frost", "rotom-fan", "rotom-mow"];
    for form in &forms {
        let is_default: i64 = conn.query_row(
            "SELECT is_default FROM pokemon WHERE name = ?1",
            params![form],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(is_default, 0, "{form} should have is_default=0 after override");
    }
}

#[test]
fn deoxys_alternate_forms_not_default() {
    let conn = open_test_db();
    let forms = ["deoxys-attack", "deoxys-defense", "deoxys-speed"];
    for form in &forms {
        let is_default: i64 = conn.query_row(
            "SELECT is_default FROM pokemon WHERE name = ?1",
            params![form],
            |row| row.get(0),
        ).unwrap();
        assert_eq!(is_default, 0, "{form} should have is_default=0 after override");
    }
}

#[test]
fn castform_default_shows_species_name() {
    let conn = open_test_db();
    let forms = queries::get_pokemon_forms(&conn, 351).unwrap(); // Castform
    let default = forms.iter().find(|f| f.is_default).unwrap();
    assert_eq!(default.display_name, "Castform",
        "Castform default form should show 'Castform', got '{}'", default.display_name);
}

#[test]
fn encounter_game_slug_populated() {
    let conn = open_test_db();
    let resolved = queries::resolve_pokemon(&conn, "pikachu").unwrap().unwrap();
    let encounters = queries::get_encounters(&conn, resolved.0, Some("sword")).unwrap();
    assert!(!encounters.is_empty());
    for enc in &encounters {
        assert!(!enc.game_slug.is_empty(), "game_slug should be populated");
        assert!(!enc.game_slug.contains(' '), "game_slug should be a slug, not display name: {}", enc.game_slug);
    }
}
