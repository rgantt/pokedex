-- ============================================================
-- Reference data (seeded from PokeAPI)
-- ============================================================

CREATE TABLE IF NOT EXISTS types (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS type_efficacy (
    attacking_type_id INTEGER NOT NULL REFERENCES types(id),
    defending_type_id INTEGER NOT NULL REFERENCES types(id),
    damage_factor INTEGER NOT NULL, -- 0, 50, 100, 200
    PRIMARY KEY (attacking_type_id, defending_type_id)
);

CREATE TABLE IF NOT EXISTS generations (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    region_id INTEGER
);

CREATE TABLE IF NOT EXISTS regions (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS version_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    generation_id INTEGER NOT NULL REFERENCES generations(id)
);

CREATE TABLE IF NOT EXISTS versions (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id)
);

CREATE TABLE IF NOT EXISTS evolution_chains (
    id INTEGER PRIMARY KEY,
    baby_trigger_item_id INTEGER
);

CREATE TABLE IF NOT EXISTS species (
    id INTEGER PRIMARY KEY, -- national dex number
    name TEXT NOT NULL UNIQUE,
    generation_id INTEGER NOT NULL REFERENCES generations(id),
    evolution_chain_id INTEGER REFERENCES evolution_chains(id),
    evolves_from_species_id INTEGER REFERENCES species(id),
    color_id INTEGER,
    shape_id INTEGER,
    habitat_id INTEGER,
    gender_rate INTEGER NOT NULL DEFAULT -1, -- -1 = genderless, 0-8 = female ratio in eighths
    capture_rate INTEGER NOT NULL DEFAULT 0,
    base_happiness INTEGER,
    is_baby INTEGER NOT NULL DEFAULT 0,
    is_legendary INTEGER NOT NULL DEFAULT 0,
    is_mythical INTEGER NOT NULL DEFAULT 0,
    growth_rate_id INTEGER,
    has_gender_differences INTEGER NOT NULL DEFAULT 0,
    order_num INTEGER NOT NULL DEFAULT 0
);

-- Pokemon table: individual pokemon entries (species can have multiple, e.g. forms)
CREATE TABLE IF NOT EXISTS pokemon (
    id INTEGER PRIMARY KEY,
    species_id INTEGER NOT NULL REFERENCES species(id),
    name TEXT NOT NULL UNIQUE,
    height INTEGER, -- decimetres
    weight INTEGER, -- hectograms
    base_experience INTEGER,
    is_default INTEGER NOT NULL DEFAULT 1,
    order_num INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS pokemon_types (
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    type_id INTEGER NOT NULL REFERENCES types(id),
    slot INTEGER NOT NULL, -- 1 = primary, 2 = secondary
    PRIMARY KEY (pokemon_id, slot)
);

CREATE TABLE IF NOT EXISTS pokemon_forms (
    id INTEGER PRIMARY KEY,
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    name TEXT NOT NULL,
    form_name TEXT, -- NULL = base form; "alola", "mega", "gmax", etc.
    is_default INTEGER NOT NULL DEFAULT 0,
    is_battle_only INTEGER NOT NULL DEFAULT 0,
    is_mega INTEGER NOT NULL DEFAULT 0,
    form_order INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS pokemon_form_types (
    pokemon_form_id INTEGER NOT NULL REFERENCES pokemon_forms(id),
    type_id INTEGER NOT NULL REFERENCES types(id),
    slot INTEGER NOT NULL,
    PRIMARY KEY (pokemon_form_id, slot)
);

CREATE TABLE IF NOT EXISTS pokemon_stats (
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    stat_id INTEGER NOT NULL,
    base_value INTEGER NOT NULL,
    effort INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (pokemon_id, stat_id)
);

CREATE TABLE IF NOT EXISTS stats (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    is_battle_only INTEGER NOT NULL DEFAULT 0,
    game_index INTEGER
);

CREATE TABLE IF NOT EXISTS abilities (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    generation_id INTEGER NOT NULL REFERENCES generations(id),
    is_main_series INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS ability_prose (
    ability_id INTEGER NOT NULL REFERENCES abilities(id),
    short_effect TEXT,
    effect TEXT,
    PRIMARY KEY (ability_id)
);

CREATE TABLE IF NOT EXISTS pokemon_abilities (
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    ability_id INTEGER NOT NULL REFERENCES abilities(id),
    is_hidden INTEGER NOT NULL DEFAULT 0,
    slot INTEGER NOT NULL,
    PRIMARY KEY (pokemon_id, slot)
);

CREATE TABLE IF NOT EXISTS egg_groups (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS pokemon_egg_groups (
    species_id INTEGER NOT NULL REFERENCES species(id),
    egg_group_id INTEGER NOT NULL REFERENCES egg_groups(id),
    PRIMARY KEY (species_id, egg_group_id)
);

-- Evolution
CREATE TABLE IF NOT EXISTS evolution_triggers (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS pokemon_evolution (
    id INTEGER PRIMARY KEY,
    evolved_species_id INTEGER NOT NULL REFERENCES species(id),
    evolution_trigger_id INTEGER NOT NULL REFERENCES evolution_triggers(id),
    trigger_item_id INTEGER,
    minimum_level INTEGER,
    gender_id INTEGER,
    location_id INTEGER,
    held_item_id INTEGER,
    time_of_day TEXT,
    known_move_id INTEGER,
    known_move_type_id INTEGER,
    minimum_happiness INTEGER,
    minimum_beauty INTEGER,
    minimum_affection INTEGER,
    relative_physical_stats INTEGER,
    party_species_id INTEGER,
    party_type_id INTEGER,
    trade_species_id INTEGER,
    needs_overworld_rain INTEGER NOT NULL DEFAULT 0,
    turn_upside_down INTEGER NOT NULL DEFAULT 0
);

-- Moves
CREATE TABLE IF NOT EXISTS moves (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    generation_id INTEGER NOT NULL REFERENCES generations(id),
    type_id INTEGER NOT NULL REFERENCES types(id),
    power INTEGER,
    pp INTEGER,
    accuracy INTEGER,
    priority INTEGER NOT NULL DEFAULT 0,
    damage_class_id INTEGER, -- 1=status, 2=physical, 3=special
    effect_id INTEGER,
    effect_chance INTEGER
);

CREATE TABLE IF NOT EXISTS move_damage_classes (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS move_effect_prose (
    move_effect_id INTEGER PRIMARY KEY,
    short_effect TEXT,
    effect TEXT
);

CREATE TABLE IF NOT EXISTS pokemon_moves (
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id),
    move_id INTEGER NOT NULL REFERENCES moves(id),
    pokemon_move_method_id INTEGER NOT NULL,
    level INTEGER NOT NULL DEFAULT 0,
    order_col INTEGER
);

CREATE TABLE IF NOT EXISTS pokemon_move_methods (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

-- Items
CREATE TABLE IF NOT EXISTS items (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    category_id INTEGER,
    cost INTEGER,
    fling_power INTEGER,
    fling_effect_id INTEGER
);

CREATE TABLE IF NOT EXISTS item_prose (
    item_id INTEGER NOT NULL REFERENCES items(id),
    short_effect TEXT,
    effect TEXT,
    PRIMARY KEY (item_id)
);

CREATE TABLE IF NOT EXISTS item_categories (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    pocket_id INTEGER
);

CREATE TABLE IF NOT EXISTS machines (
    machine_number INTEGER NOT NULL,
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id),
    item_id INTEGER NOT NULL REFERENCES items(id),
    move_id INTEGER NOT NULL REFERENCES moves(id),
    PRIMARY KEY (machine_number, version_group_id)
);

CREATE TABLE IF NOT EXISTS pokemon_items (
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    version_id INTEGER NOT NULL REFERENCES versions(id),
    item_id INTEGER NOT NULL REFERENCES items(id),
    rarity INTEGER NOT NULL
);

-- Natures
CREATE TABLE IF NOT EXISTS natures (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    decreased_stat_id INTEGER REFERENCES stats(id),
    increased_stat_id INTEGER REFERENCES stats(id),
    hates_flavor_id INTEGER,
    likes_flavor_id INTEGER,
    game_index INTEGER
);

-- Locations & Encounters
CREATE TABLE IF NOT EXISTS locations (
    id INTEGER PRIMARY KEY,
    region_id INTEGER REFERENCES regions(id),
    name TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS location_areas (
    id INTEGER PRIMARY KEY,
    location_id INTEGER NOT NULL REFERENCES locations(id),
    name TEXT,
    game_index INTEGER
);

CREATE TABLE IF NOT EXISTS encounter_methods (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    order_col INTEGER
);

CREATE TABLE IF NOT EXISTS encounter_slots (
    id INTEGER PRIMARY KEY,
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id),
    encounter_method_id INTEGER NOT NULL REFERENCES encounter_methods(id),
    slot INTEGER,
    rarity INTEGER
);

CREATE TABLE IF NOT EXISTS encounters (
    id INTEGER PRIMARY KEY,
    version_id INTEGER NOT NULL REFERENCES versions(id),
    location_area_id INTEGER NOT NULL REFERENCES location_areas(id),
    encounter_slot_id INTEGER NOT NULL REFERENCES encounter_slots(id),
    pokemon_id INTEGER NOT NULL REFERENCES pokemon(id),
    min_level INTEGER NOT NULL,
    max_level INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS encounter_conditions (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS encounter_condition_values (
    id INTEGER PRIMARY KEY,
    encounter_condition_id INTEGER NOT NULL REFERENCES encounter_conditions(id),
    name TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS encounter_condition_value_map (
    encounter_id INTEGER NOT NULL REFERENCES encounters(id),
    encounter_condition_value_id INTEGER NOT NULL REFERENCES encounter_condition_values(id),
    PRIMARY KEY (encounter_id, encounter_condition_value_id)
);

-- Pokedexes
CREATE TABLE IF NOT EXISTS pokedexes (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    region_id INTEGER REFERENCES regions(id),
    is_main_series INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS pokemon_dex_numbers (
    species_id INTEGER NOT NULL REFERENCES species(id),
    pokedex_id INTEGER NOT NULL REFERENCES pokedexes(id),
    pokedex_number INTEGER NOT NULL,
    PRIMARY KEY (species_id, pokedex_id)
);

-- Growth rates & experience
CREATE TABLE IF NOT EXISTS growth_rates (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS experience (
    growth_rate_id INTEGER NOT NULL REFERENCES growth_rates(id),
    level INTEGER NOT NULL,
    experience INTEGER NOT NULL,
    PRIMARY KEY (growth_rate_id, level)
);

-- Names lookup tables (English only for now, filtered during seed)

CREATE TABLE IF NOT EXISTS type_names (
    type_id INTEGER NOT NULL REFERENCES types(id),
    name TEXT NOT NULL,
    PRIMARY KEY (type_id)
);

CREATE TABLE IF NOT EXISTS ability_names (
    ability_id INTEGER NOT NULL REFERENCES abilities(id),
    name TEXT NOT NULL,
    PRIMARY KEY (ability_id)
);

CREATE TABLE IF NOT EXISTS species_names (
    species_id INTEGER NOT NULL REFERENCES species(id),
    name TEXT NOT NULL,
    genus TEXT,
    PRIMARY KEY (species_id)
);

CREATE TABLE IF NOT EXISTS move_names (
    move_id INTEGER NOT NULL REFERENCES moves(id),
    name TEXT NOT NULL,
    PRIMARY KEY (move_id)
);

CREATE TABLE IF NOT EXISTS item_names (
    item_id INTEGER NOT NULL REFERENCES items(id),
    name TEXT NOT NULL,
    PRIMARY KEY (item_id)
);

CREATE TABLE IF NOT EXISTS location_names (
    location_id INTEGER NOT NULL REFERENCES locations(id),
    name TEXT NOT NULL,
    PRIMARY KEY (location_id)
);

CREATE TABLE IF NOT EXISTS version_names (
    version_id INTEGER NOT NULL REFERENCES versions(id),
    name TEXT NOT NULL,
    PRIMARY KEY (version_id)
);

CREATE TABLE IF NOT EXISTS nature_names (
    nature_id INTEGER NOT NULL REFERENCES natures(id),
    name TEXT NOT NULL,
    PRIMARY KEY (nature_id)
);

CREATE TABLE IF NOT EXISTS stat_names (
    stat_id INTEGER NOT NULL REFERENCES stats(id),
    name TEXT NOT NULL,
    PRIMARY KEY (stat_id)
);

CREATE TABLE IF NOT EXISTS generation_names (
    generation_id INTEGER NOT NULL REFERENCES generations(id),
    name TEXT NOT NULL,
    PRIMARY KEY (generation_id)
);

CREATE TABLE IF NOT EXISTS region_names (
    region_id INTEGER NOT NULL REFERENCES regions(id),
    name TEXT NOT NULL,
    PRIMARY KEY (region_id)
);

CREATE TABLE IF NOT EXISTS egg_group_names (
    egg_group_id INTEGER NOT NULL REFERENCES egg_groups(id),
    name TEXT NOT NULL,
    PRIMARY KEY (egg_group_id)
);

CREATE TABLE IF NOT EXISTS encounter_method_names (
    encounter_method_id INTEGER NOT NULL REFERENCES encounter_methods(id),
    name TEXT NOT NULL,
    PRIMARY KEY (encounter_method_id)
);

CREATE TABLE IF NOT EXISTS encounter_condition_names (
    encounter_condition_id INTEGER NOT NULL REFERENCES encounter_conditions(id),
    name TEXT NOT NULL,
    PRIMARY KEY (encounter_condition_id)
);

CREATE TABLE IF NOT EXISTS encounter_condition_value_names (
    encounter_condition_value_id INTEGER NOT NULL REFERENCES encounter_condition_values(id),
    name TEXT NOT NULL,
    PRIMARY KEY (encounter_condition_value_id)
);

CREATE TABLE IF NOT EXISTS pokedex_names (
    pokedex_id INTEGER NOT NULL REFERENCES pokedexes(id),
    name TEXT NOT NULL,
    PRIMARY KEY (pokedex_id)
);

CREATE TABLE IF NOT EXISTS pokemon_form_names (
    pokemon_form_id INTEGER NOT NULL REFERENCES pokemon_forms(id),
    name TEXT NOT NULL,
    pokemon_name TEXT,
    PRIMARY KEY (pokemon_form_id)
);

CREATE TABLE IF NOT EXISTS move_damage_class_names (
    move_damage_class_id INTEGER NOT NULL REFERENCES move_damage_classes(id),
    name TEXT NOT NULL,
    PRIMARY KEY (move_damage_class_id)
);

CREATE TABLE IF NOT EXISTS growth_rate_names (
    growth_rate_id INTEGER NOT NULL REFERENCES growth_rates(id),
    name TEXT NOT NULL,
    PRIMARY KEY (growth_rate_id)
);

CREATE TABLE IF NOT EXISTS item_category_names (
    item_category_id INTEGER NOT NULL REFERENCES item_categories(id),
    name TEXT NOT NULL,
    PRIMARY KEY (item_category_id)
);

-- Species flavor text (pokedex entries)
CREATE TABLE IF NOT EXISTS pokemon_species_flavor_text (
    species_id INTEGER NOT NULL REFERENCES species(id),
    version_id INTEGER NOT NULL REFERENCES versions(id),
    flavor_text TEXT NOT NULL,
    PRIMARY KEY (species_id, version_id)
);

-- Move flavor text
CREATE TABLE IF NOT EXISTS move_flavor_text (
    move_id INTEGER NOT NULL REFERENCES moves(id),
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id),
    flavor_text TEXT NOT NULL,
    PRIMARY KEY (move_id, version_group_id)
);

-- Ability flavor text
CREATE TABLE IF NOT EXISTS ability_flavor_text (
    ability_id INTEGER NOT NULL REFERENCES abilities(id),
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id),
    flavor_text TEXT NOT NULL,
    PRIMARY KEY (ability_id, version_group_id)
);

-- Item flavor text
CREATE TABLE IF NOT EXISTS item_flavor_text (
    item_id INTEGER NOT NULL REFERENCES items(id),
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id),
    flavor_text TEXT NOT NULL,
    PRIMARY KEY (item_id, version_group_id)
);

-- Move meta (detailed move info)
CREATE TABLE IF NOT EXISTS move_meta (
    move_id INTEGER PRIMARY KEY REFERENCES moves(id),
    meta_category_id INTEGER NOT NULL,
    meta_ailment_id INTEGER NOT NULL,
    min_hits INTEGER,
    max_hits INTEGER,
    min_turns INTEGER,
    max_turns INTEGER,
    drain INTEGER NOT NULL DEFAULT 0,
    healing INTEGER NOT NULL DEFAULT 0,
    crit_rate INTEGER NOT NULL DEFAULT 0,
    ailment_chance INTEGER NOT NULL DEFAULT 0,
    flinch_chance INTEGER NOT NULL DEFAULT 0,
    stat_chance INTEGER NOT NULL DEFAULT 0
);

-- Move meta stat changes
CREATE TABLE IF NOT EXISTS move_meta_stat_changes (
    move_id INTEGER NOT NULL REFERENCES moves(id),
    stat_id INTEGER NOT NULL REFERENCES stats(id),
    change INTEGER NOT NULL,
    PRIMARY KEY (move_id, stat_id)
);

-- Berries
CREATE TABLE IF NOT EXISTS berries (
    id INTEGER PRIMARY KEY,
    item_id INTEGER NOT NULL REFERENCES items(id),
    natural_gift_power INTEGER,
    natural_gift_type_id INTEGER REFERENCES types(id),
    size INTEGER,
    max_harvest INTEGER,
    growth_time INTEGER,
    soil_dryness INTEGER,
    smoothness INTEGER
);

-- Version group regions mapping
CREATE TABLE IF NOT EXISTS version_group_regions (
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id),
    region_id INTEGER NOT NULL REFERENCES regions(id),
    PRIMARY KEY (version_group_id, region_id)
);

-- Pokedex version groups mapping
CREATE TABLE IF NOT EXISTS pokedex_version_groups (
    pokedex_id INTEGER NOT NULL REFERENCES pokedexes(id),
    version_group_id INTEGER NOT NULL REFERENCES version_groups(id),
    PRIMARY KEY (pokedex_id, version_group_id)
);

-- Move flags
CREATE TABLE IF NOT EXISTS move_flags (
    move_id INTEGER NOT NULL REFERENCES moves(id),
    move_flag_id INTEGER NOT NULL,
    PRIMARY KEY (move_id, move_flag_id)
);

CREATE TABLE IF NOT EXISTS move_flag_types (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

-- Item flags
CREATE TABLE IF NOT EXISTS item_flags (
    item_id INTEGER NOT NULL REFERENCES items(id),
    item_flag_id INTEGER NOT NULL,
    PRIMARY KEY (item_id, item_flag_id)
);

CREATE TABLE IF NOT EXISTS item_flag_types (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

-- ============================================================
-- User data
-- ============================================================

CREATE TABLE IF NOT EXISTS games (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    version_group_id INTEGER REFERENCES version_groups(id),
    connects_to_home INTEGER NOT NULL DEFAULT 0,
    transfer_direction TEXT -- 'both', 'to_home_only'
);

CREATE TABLE IF NOT EXISTS collection (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    species_id INTEGER NOT NULL REFERENCES species(id),
    form_id INTEGER REFERENCES pokemon_forms(id),
    game_id INTEGER REFERENCES games(id),
    shiny INTEGER NOT NULL DEFAULT 0,
    in_home INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'caught', -- caught, living_dex, evolved, traded_away, transferred
    method TEXT, -- catch, breed, trade, transfer, gift, raid, research
    nickname TEXT,
    notes TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_collection_species ON collection(species_id);
CREATE INDEX IF NOT EXISTS idx_collection_game ON collection(game_id);
CREATE INDEX IF NOT EXISTS idx_collection_status ON collection(status);
CREATE INDEX IF NOT EXISTS idx_collection_in_home ON collection(in_home);

-- Pre-populate games table with HOME-compatible games
INSERT OR IGNORE INTO games (name, connects_to_home, transfer_direction) VALUES
    ('lets-go-pikachu', 1, 'both'),
    ('lets-go-eevee', 1, 'both'),
    ('sword', 1, 'both'),
    ('shield', 1, 'both'),
    ('brilliant-diamond', 1, 'both'),
    ('shining-pearl', 1, 'both'),
    ('legends-arceus', 1, 'both'),
    ('scarlet', 1, 'both'),
    ('violet', 1, 'both'),
    ('pokemon-go', 1, 'to_home_only'),
    ('pokemon-bank', 1, 'to_home_only'),
    ('home', 1, 'both');

-- Indexes for reference data queries
CREATE INDEX IF NOT EXISTS idx_species_gen ON species(generation_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_species ON pokemon(species_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_types_type ON pokemon_types(type_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_moves_pokemon ON pokemon_moves(pokemon_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_moves_move ON pokemon_moves(move_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_moves_vg ON pokemon_moves(version_group_id);
CREATE INDEX IF NOT EXISTS idx_encounters_pokemon ON encounters(pokemon_id);
CREATE INDEX IF NOT EXISTS idx_encounters_location ON encounters(location_area_id);
CREATE INDEX IF NOT EXISTS idx_encounters_version ON encounters(version_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_dex_numbers_dex ON pokemon_dex_numbers(pokedex_id);
CREATE INDEX IF NOT EXISTS idx_pokemon_dex_numbers_species ON pokemon_dex_numbers(species_id);
CREATE INDEX IF NOT EXISTS idx_location_areas_location ON location_areas(location_id);
CREATE INDEX IF NOT EXISTS idx_locations_region ON locations(region_id);
