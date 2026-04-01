use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Species {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub generation: i64,
    pub types: Vec<String>,
    pub capture_rate: i64,
    pub is_baby: bool,
    pub is_legendary: bool,
    pub is_mythical: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evolves_from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genus: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub egg_groups: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<PokemonStats>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<AbilityInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpeciesSummary {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub types: Vec<String>,
    pub generation: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PokemonForm {
    pub id: i64,
    pub pokemon_id: i64,
    pub name: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_name: Option<String>,
    pub is_default: bool,
    pub is_mega: bool,
    pub is_battle_only: bool,
    pub types: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PokemonStats {
    pub pokemon_name: String,
    pub hp: i64,
    pub attack: i64,
    pub defense: i64,
    pub special_attack: i64,
    pub special_defense: i64,
    pub speed: i64,
    pub total: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvolutionNode {
    pub species_id: i64,
    pub species_name: String,
    pub display_name: String,
    /// All known evolution methods for this species (may vary by game)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<EvolutionMethod>,
    pub children: Vec<EvolutionNode>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EvolutionMethod {
    pub trigger: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_requirement: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeInfo {
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeMatchup {
    pub attacking: String,
    pub defending: String,
    pub multiplier: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeMatchups {
    pub type_name: String,
    pub display_name: String,
    pub attacking: TypeEffectiveness,
    pub defending: TypeEffectiveness,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeEffectiveness {
    pub super_effective: Vec<String>,    // 2x
    pub not_very_effective: Vec<String>, // 0.5x
    pub no_effect: Vec<String>,         // 0x
}

#[derive(Debug, Clone, Serialize)]
pub struct Encounter {
    pub pokemon_name: String,
    pub species_slug: String,
    pub location: String,
    pub area: String,
    pub game: String,
    pub game_slug: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_level: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_level: Option<i64>,
    pub rarity: Option<i64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<EncounterDetails>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EncounterDetails {
    // Rates
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_overall: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_morning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_day: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_night: Option<String>,

    // Time availability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub during_any_time: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub during_morning: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub during_day: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub during_evening: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub during_night: Option<bool>,

    // Weather conditions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub while_weather_overall: Option<bool>,

    // Per-weather rates (SwSh)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_clear_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_cloudy_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_rain_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_thunderstorm_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_snow_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_blizzard_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_harshsunlight_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_sandstorm_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather_fog_rate: Option<String>,

    // Terrain (SV)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_terrain_land: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_terrain_watersurface: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_terrain_underwater: Option<bool>,

    // Probability weights (SV)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probability_overall: Option<String>,

    // Group spawns (SV)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_rate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_pokemon: Option<String>,

    // PLA
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha_levels: Option<String>,

    // Raid data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tera_raid_star_level: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_raid_perfect_ivs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_raid_rate_1_star: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_raid_rate_2_star: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_raid_rate_3_star: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_raid_rate_4_star: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_raid_rate_5_star: Option<String>,

    // Misc
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden_ability_possible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl EncounterDetails {
    /// Returns true if all fields are None/false/empty, meaning this detail
    /// row carries no useful information.
    pub fn is_empty(&self) -> bool {
        self.rate_overall.is_none()
            && self.rate_morning.is_none()
            && self.rate_day.is_none()
            && self.rate_night.is_none()
            && self.during_any_time.is_none()
            && self.during_morning.is_none()
            && self.during_day.is_none()
            && self.during_evening.is_none()
            && self.during_night.is_none()
            && self.while_weather_overall.is_none()
            && self.weather_clear_rate.is_none()
            && self.weather_cloudy_rate.is_none()
            && self.weather_rain_rate.is_none()
            && self.weather_thunderstorm_rate.is_none()
            && self.weather_snow_rate.is_none()
            && self.weather_blizzard_rate.is_none()
            && self.weather_harshsunlight_rate.is_none()
            && self.weather_sandstorm_rate.is_none()
            && self.weather_fog_rate.is_none()
            && self.on_terrain_land.is_none()
            && self.on_terrain_watersurface.is_none()
            && self.on_terrain_underwater.is_none()
            && self.probability_overall.is_none()
            && self.group_rate.is_none()
            && self.group_pokemon.is_none()
            && self.alpha_levels.is_none()
            && self.tera_raid_star_level.is_none()
            && self.max_raid_perfect_ivs.is_none()
            && self.max_raid_rate_1_star.is_none()
            && self.max_raid_rate_2_star.is_none()
            && self.max_raid_rate_3_star.is_none()
            && self.max_raid_rate_4_star.is_none()
            && self.max_raid_rate_5_star.is_none()
            && self.hidden_ability_possible.is_none()
            && self.visible.is_none()
            && self.note.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct PokemonMove {
    pub move_name: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pp: Option<i64>,
    pub damage_class: String,
    pub learn_method: String,
    pub level: i64,
    pub game: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PokedexInfo {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    pub species_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DexEntry {
    pub pokedex_number: i64,
    pub species_id: i64,
    pub name: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DexProgress {
    pub dex_name: String,
    pub total: i64,
    pub caught: i64,
    pub percentage: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<DexProgressEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DexProgressEntry {
    pub pokedex_number: i64,
    pub species_id: i64,
    pub name: String,
    pub display_name: String,
    pub caught: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameInfo {
    pub id: i64,
    pub name: String,
    pub connects_to_home: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionEntry {
    pub id: i64,
    pub species_id: i64,
    pub species_name: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub form_name: Option<String>,
    pub game: String,
    pub shiny: bool,
    pub in_home: bool,
    pub is_alpha: bool,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nickname: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CollectionStats {
    pub total_entries: i64,
    pub unique_species: i64,
    pub shiny_count: i64,
    pub in_home_count: i64,
    pub by_status: Vec<StatusCount>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub by_game: Vec<GameCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusCount {
    pub status: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameCount {
    pub game: String,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HomeStatus {
    pub total_in_home: i64,
    pub unique_species_in_home: i64,
    pub shiny_in_home: i64,
    pub by_game_origin: Vec<GameCount>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NatureInfo {
    pub name: String,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub increased_stat: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decreased_stat: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AbilityInfo {
    pub name: String,
    pub display_name: String,
    pub is_hidden: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_effect: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HomeMissingEntry {
    pub pokedex_number: i64,
    pub species_id: i64,
    pub name: String,
    pub display_name: String,
    pub owned_elsewhere: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub species: SpeciesSummary,
    pub score: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemInfo {
    pub id: i64,
    pub name: String,
    pub display_name: String,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short_effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub held_by: Vec<ItemHolder>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ItemHolder {
    pub pokemon_name: String,
    pub pokemon_slug: String,
    pub rarity: i64,
    pub game: String,
    pub game_slug: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GameEncounterSummary {
    pub species_id: i64,
    pub name: String,
    pub display_name: String,
    pub encounter_count: i64,
    pub methods: Vec<String>,
}
