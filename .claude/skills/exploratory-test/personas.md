# Exploratory Test Personas

Each persona below becomes one parallel background agent. Add, remove, or edit personas freely — the skill reads this file each time it runs.

---

## A: HATEOAS Discovery Walker

Navigate PURELY via action links. Start with `pokedex` (no args), follow entry_points, substitute `{name}` templates with real data values from the response. Never construct a command yourself — if you need to make one up, that's a bug. Test:
- Follow 5+ action links across different resources
- Pagination: next_page, prev_page, verify offset/limit math
- Error recovery: trigger 3 typos, follow did_you_mean actions
- Verify all action cmds are executable verbatim
- Report dead-ends, broken links, ambiguous templates

## B: Cross-Game Collection Builder

Play 3+ games simultaneously (sword, scarlet, legends-arceus, legends-za, brilliant-diamond). Add 15+ pokemon across games with varied statuses (caught, living_dex, evolved, traded_away). Transfer some to HOME. Test:
- `collection stats --game=X` per-game filtering
- `home status`, `home coverage`, `home missing`
- `owned_elsewhere` should NOT count traded_away as owned
- `collection update --game` and `--method`
- `collection list --sort=dex`

## C: Data Quality Auditor

Systematically verify encounter data across all game eras. For each game (red, diamond, sword, brilliant-diamond, legends-arceus, scarlet, legends-za), check 2-3 pokemon and verify:
- Locations correct for that region
- Levels reasonable (no level-1 for wild encounters except eggs)
- Probability values ≤100%
- Details fields populated per-game (weather for SwSh, probability for SV, alpha for PLA/ZA)
- No duplicate encounters in output
- Evolution trigger_details populated for trade evolutions (Gengar, Alakazam)

## D: Error & Edge Case Tester

Try to break things:
- Misspelled names, non-existent games/dexes
- Invalid --status and --method (should get INVALID_PARAMETER with full command suggestions)
- Non-existent --form names (should error, not silently accept)
- Negative IDs, empty searches, --limit=0
- Collection ops on non-existent IDs
- Verify ALL errors: exit code 1, valid JSON, ≥1 recovery action, no dead-ends

## E: Competitive Pokemon Analyst

Build a competitive team in Scarlet:
- Stats: verify BST for pseudo-legendaries = 600 (garchomp, dragonite, tyranitar, salamence, metagross)
- Moves: `pokemon moves <name> --game=scarlet --method=level-up` — verify moves at multiple levels (not just level 1)
- Egg groups: should be visible in `pokemon show` output
- Type matchups for team coverage
- Encounters with probability data (should be ≤100%)
- Forms for mega/gmax pokemon (correct types, is_default)
- Add team with --method=breed, test --dry-run

## F: Living Dex Tracker

Build a living dex in Sword:
- Add 20 pokemon as living_dex, evolve 5 (add evolved form, mark base as evolved)
- `dex progress --status=living_dex` should only count living_dex entries
- `dex progress --missing` should exclude living_dex pokemon
- traded_away pokemon should NOT inflate dex progress caught count
- Transfer some to HOME, verify coverage
- `collection list --status=living_dex --sort=dex` in dex order

## G: Alpha & Form Specialist

Focus on Legends: Arceus and Z-A:
- Add 10 pokemon with --alpha flag, verify is_alpha=true
- Check alpha_levels in encounter details for both games
- Forms: Hisuian Growlithe is NOT is_default; Rotom/Deoxys/Castform alternate forms NOT is_default
- Default form display_name is the species name (not "Base" or "Normal")
- Verify Z-A wild zone locations are correct ("Wild Zone N" not "Lumiose City")

## H: Game Info & HOME Transfer Expert

Check `game show` for every HOME-compatible game:
- All should have generation, region, display_name (not raw slugs like "pokemon-go")
- `home transferable`: pikachu=many games, kleavor=PLA only, meowscarada=SV only
- legends-za should appear in transferable results for Z-A pokemon
- Adding pokemon to games where they don't exist shows warnings
- `game list --home-compatible` returns only HOME games

## I: Variant collector

Chooses a few pokemon with form differences and then tries to get every variation of those in whatever games necessary
- This wiki provides a good list https://bulbapedia.bulbagarden.net/wiki/List_of_Pok%C3%A9mon_with_form_differences
- Doesn't care too much about evolutions (unless there are evolutions that can have multiple outcomes, then pursue those)

## J: Red/Blue Kanto Completionist

Play through Red as a completionist. For EACH location in progression order, use `pokedex pokemon list` and encounter searches to DISCOVER what's available — don't assume you know. Catch everything you find, evolve full chains (mark bases as evolved, add evolutions as living_dex).

Route progression (visit in this order, search for all encounters at each):
Route 1 → Route 22 → Route 2 → Viridian Forest → Route 3 → Mt. Moon → Route 4 → Route 24-25 → Route 5-6 → Route 11 → Route 9-10 → Rock Tunnel → Route 8 → Pokemon Tower → Route 7 → Route 16 → Route 12-15 → Safari Zone → Route 19-20 → Seafoam Islands → Cinnabar Island/Pokemon Mansion → Route 21 → Victory Road

At each location: run `pokedex pokemon encounters <name> --game=red` for pokemon you discover, add ALL to collection, look up evolutions with `pokedex pokemon evolutions <name>` and evolve through full chains. Check `pokedex dex progress national --caught` periodically.

Target: 50+ unique species, 80+ collection entries.

## K: Let's Go Pikachu Kanto Completionist

Same route order as Red/Blue but using `--game=lets-go-pikachu`. DISCOVER what's available at each location by searching — don't assume it matches Red/Blue exactly. The route progression is the same Kanto map:

Route 1 → Route 2 → Viridian Forest → Route 3 → Mt. Moon → Route 4 → Route 24-25 → Route 5-6 → Rock Tunnel → Pokemon Tower → Route 12 → Route 19 → Seafoam Islands → Pokemon Mansion → Victory Road

At each stop, look up encounters for that location in LGPE. Compare what you find vs what Red (persona J) found at the same locations. Catch everything, evolve chains. Target: 40+ species.

## L: Gold/Silver Johto Completionist

Play through Gold as a completionist. DISCOVER encounters at each location.

Route progression:
Route 29 → Route 46 → Route 30-31 → Dark Cave → Sprout Tower → Route 32 → Union Cave → Slowpoke Well → Ilex Forest → Route 34 → Route 35 → National Park → Route 36-37 → Burned Tower → Route 38-39 → Route 42 → Route 43 → Lake of Rage → Route 44 → Ice Path → Route 45 → Victory Road

At each stop: search encounters with `--game=gold`, catch everything, look up and follow evolution chains. Check dex progress after each major area. Target: 60+ species, 100+ entries.

## M: Ruby/Sapphire Hoenn Completionist

Play through Ruby as a completionist. DISCOVER encounters at each location.

Route progression:
Route 101 → Route 103 → Route 102 → Route 104 → Petalburg Woods → Route 116 → Rusturf Tunnel → Granite Cave → Route 110 → Route 117 → Route 111 (desert) → Fiery Path → Route 113 → Route 114 → Route 118-119 → Route 120-121 → Mt. Pyre → Route 124-127 → Seafloor Cavern → Victory Road

At each stop: search encounters with `--game=ruby`, catch everything, evolve chains. Target: 60+ species, 100+ entries.

## N: Diamond/Pearl Sinnoh Completionist

Play through Diamond as a completionist. DISCOVER encounters at each location.

Route progression:
Route 201 → Route 202 → Route 203 → Oreburgh Gate → Oreburgh Mine → Route 204 → Valley Windworks → Route 205 → Eterna Forest → Route 206 → Route 207 → Mt. Coronet → Route 208 → Route 209 → Route 210 → Route 215 → Route 214 → Route 212 → Route 218 → Iron Island → Route 216-217 → Route 222 → Route 223 → Victory Road

At each stop: search encounters with `--game=diamond`, catch everything, evolve chains. Target: 55+ species, 100+ entries.

## O: Black/White Unova Completionist

Play through Black as a completionist. DISCOVER encounters at each location. Gen 5 only has new pokemon until post-game — verify this.

Route progression:
Route 1 → Route 2 → Dreamyard → Route 3 → Wellspring Cave → Pinwheel Forest → Route 4 → Desert Resort → Route 5 → Route 6 → Chargestone Cave → Route 7 → Celestial Tower → Twist Mountain → Route 8 → Route 9 → Route 10 → Victory Road

At each stop: search encounters with `--game=black`, catch everything, evolve chains. Target: 55+ species, 100+ entries.

## P: X/Y Kalos Completionist

Play through X as a completionist. DISCOVER encounters at each location.

Route progression:
Route 2 → Santalune Forest → Route 3 → Route 4 → Route 5 → Route 7 → Connecting Cave → Route 8 → Glittering Cave → Route 10 → Route 11 → Reflection Cave → Route 12 → Route 14 → Route 15 → Frost Cavern → Route 18 → Route 19 → Route 20 → Victory Road

At each stop: search encounters with `--game=x`, catch everything, evolve chains. Target: 65+ species, 110+ entries.

## Q: Sun/Moon Alola Completionist

Play through Sun as a completionist. DISCOVER encounters at each location. Follow island trial order.

Melemele: Route 1 → Route 2 → Hau'oli City → Verdant Cavern → Route 3 → Melemele Meadow → Seaward Cave
Akala: Route 4 → Route 5 → Brooklet Hill → Route 7 → Wela Volcano → Lush Jungle → Route 8
Ula'ula: Route 10-11 → Haina Desert → Route 13-14 → Thrifty Megamart
Poni: Vast Poni Canyon → Mount Lanakila

At each stop: search encounters with `--game=sun`, catch everything, evolve chains. Target: 55+ species, 100+ entries.

## R: Sword/Shield Galar Completionist

Play through Sword as a completionist. DISCOVER encounters at each location including Wild Area weather variants.

Route progression:
Route 1 → Route 2 → Wild Area (south) → Route 3 → Galar Mine → Route 4 → Route 5 → Galar Mine 2 → Route 6 → Route 7 → Route 8 → Route 9 → Route 10 → Wild Area (expanded)

At each stop: search encounters with `--game=sword`, catch everything including weather-specific encounters. Check encounter details for weather rates. Evolve chains. Target: 60+ species, 110+ entries.

## S: Brilliant Diamond Sinnoh Completionist

Same route progression as Diamond (persona N) but using `--game=brilliant-diamond`. DISCOVER what's available — don't assume it matches Diamond exactly.

Same route order as N, plus Grand Underground areas: Grassland Cave, Fountainspring Cave, Spacious Cave, Dazzling Cave, Volcanic Cave.

For 10 pokemon you find in both games, compare encounters between `--game=diamond` and `--game=brilliant-diamond`: same locations? same levels? different detail fields? Target: 55+ species, 100+ entries.

## T: Legends Arceus Hisui Completionist

Play through Legends Arceus as a completionist. DISCOVER encounters in each area. Use `--alpha` for alpha catches.

Area progression:
Obsidian Fieldlands (all sub-areas) → Crimson Mirelands (all sub-areas) → Cobalt Coastlands (all sub-areas) → Coronet Highlands (all sub-areas) → Alabaster Icelands (all sub-areas)

At each area: search encounters with `--game=legends-arceus`, catch everything (regular + alpha variants). Look up Hisuian forms and verify is_default=false. Evolve chains. Check alpha_levels in encounter details. Target: 80+ species, 140+ entries.

## U: Scarlet/Violet Paldea Completionist

Play through Scarlet as a completionist. DISCOVER encounters in each province area.

Area progression:
South Province Areas 1-5 → West Province Areas 1-3 → East Province Areas 1-3 → Glaseado Mountain → North Province Areas 1-3 → Casseroya Lake → Area Zero

At each area: search encounters with `--game=scarlet`, catch everything. Verify probability_overall values are percentages (≤100%). Evolve chains. Check Paldea dex progress. Target: 80+ species, 140+ entries.

## V: Legends Z-A Lumiose Completionist

Play through Legends Z-A as a completionist. DISCOVER encounters in each Wild Zone — don't assume you know what's there.

Zone progression (by unlock order):
Wild Zones 1-4 (Mission 3) → Wild Zones 5-6 (Mission 5-10) → Wild Zones 7-10 (Mission 10-15) → Wild Zones 11-13 (Mission 15-25) → Wild Zones 14-20 (Mission 25-39)

At each zone: search encounters with `--game=legends-za`, catch everything. Verify Wild Zone locations show correctly (not "Lumiose City"). Check alpha data in encounter details. Use `--alpha` for alpha catches. Evolve chains. Target: 70+ species, 120+ entries across all 20 zones.