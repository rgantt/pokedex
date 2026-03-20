-- Extended encounter details from PokeDB.org
-- Stores the rich per-game metadata that varies by generation:
--   SwSh: per-weather rates
--   SV: probability weights, terrain, group spawns
--   PLA: alpha levels, time/weather booleans
CREATE TABLE IF NOT EXISTS encounter_details (
    encounter_id INTEGER NOT NULL REFERENCES encounters(id),

    -- Overall/time-of-day rates (percentage strings)
    rate_overall TEXT,
    rate_morning TEXT,
    rate_day TEXT,
    rate_night TEXT,

    -- Time availability
    during_any_time INTEGER,
    during_morning INTEGER,
    during_day INTEGER,
    during_evening INTEGER,
    during_night INTEGER,

    -- Weather conditions (booleans)
    while_weather_overall INTEGER,
    while_clear INTEGER,
    while_harsh_sunlight INTEGER,
    while_cloudy INTEGER,
    while_blizzard INTEGER,

    -- Per-weather encounter rates (SwSh) — percentage strings
    weather_clear_rate TEXT,
    weather_cloudy_rate TEXT,
    weather_rain_rate TEXT,
    weather_thunderstorm_rate TEXT,
    weather_snow_rate TEXT,
    weather_blizzard_rate TEXT,
    weather_harshsunlight_rate TEXT,
    weather_sandstorm_rate TEXT,
    weather_fog_rate TEXT,

    -- Terrain (SV)
    on_terrain_land INTEGER,
    on_terrain_watersurface INTEGER,
    on_terrain_underwater INTEGER,
    on_terrain_overland INTEGER,
    on_terrain_sky INTEGER,

    -- Probability weights (SV) — numeric strings
    probability_overall TEXT,
    probability_morning TEXT,
    probability_day TEXT,
    probability_evening TEXT,
    probability_night TEXT,

    -- Group spawns (SV)
    group_rate TEXT,
    group_pokemon TEXT,

    -- PLA specific
    alpha_levels TEXT,
    boulder_required INTEGER,

    -- Visibility flag
    visible INTEGER,

    -- Max Raid (SwSh)
    max_raid_perfect_ivs TEXT,
    max_raid_rate_1_star TEXT,
    max_raid_rate_2_star TEXT,
    max_raid_rate_3_star TEXT,
    max_raid_rate_4_star TEXT,
    max_raid_rate_5_star TEXT,

    -- Tera Raid (SV)
    tera_raid_star_level TEXT,

    -- Misc
    hidden_ability_possible INTEGER,
    note TEXT,

    PRIMARY KEY (encounter_id)
);

CREATE INDEX IF NOT EXISTS idx_encounter_details_enc ON encounter_details(encounter_id);
