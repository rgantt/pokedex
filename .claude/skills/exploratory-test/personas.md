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

## J-V: Game-Specific Completionist Playthroughs

**All game personas (J through V) follow this exact workflow at EACH location. Do not skip steps.**

### The Loop (repeat for every location in route progression)

1. **DISCOVER**: `pokedex location encounters <location-slug> --game=<game>` to see what's available. If the slug doesn't work, try variants (region prefix like `johto-route-29`, or partial match).
2. **INSPECT**: For each species found, run `pokedex pokemon show <name>` to see types, egg groups, evolution chain info.
3. **CATCH**: `pokedex collection add --pokemon=<name> --game=<game> --method=catch` for every species. Verify the add succeeds (exit code 0, entry returned with correct game and species). If the add fails, REPORT IT — this is a critical bug.
4. **EVOLVE**: `pokedex pokemon evolutions <name>` for species with evolutions. Add the evolved form: `pokedex collection add --pokemon=<evolved> --game=<game> --status=living_dex`. Update the base: `pokedex collection update <id> --status=evolved`. Verify both operations succeed.
5. **VERIFY**: Every 3-4 locations, run:
   - `pokedex collection stats --game=<game>` — verify counts match what you added
   - `pokedex collection list --game=<game> --sort=dex --limit=10` — verify entries are there
   - `pokedex dex progress national --caught --limit=5` — verify dex progress reflects catches

### What to report
- Any `collection add` that fails for a valid game (CRITICAL)
- Any `location encounters` that returns wrong data or errors unexpectedly
- Any `collection stats` counts that don't match expectations
- Any `dex progress` that doesn't reflect what you caught
- Regional form annotations: verify Galarian/Hisuian/Paldean/Alolan names appear correctly on wild encounters
- Data quality: levels reasonable, probabilities ≤100%, locations correct for the region

---

## J: Red/Blue Kanto Completionist

Game: `--game=red`. Route progression:
Route 1 → Route 22 → Route 2 → Viridian Forest → Route 3 → Mt. Moon → Route 4 → Route 24-25 → Route 5-6 → Route 9-10 → Rock Tunnel → Pokemon Tower → Route 12-15 → Safari Zone → Seafoam Islands → Pokemon Mansion → Victory Road

Target: 40+ species caught and tracked in collection.

## K: Let's Go Pikachu Kanto Completionist

Game: `--game=lets-go-pikachu`. Same Kanto route order as J. Target: 30+ species.

## L: Gold/Silver Johto Completionist

Game: `--game=gold`. Route progression:
Route 29 → Route 30-31 → Dark Cave → Sprout Tower → Route 32 → Union Cave → Ilex Forest → Route 34-35 → National Park → Route 36-37 → Route 38-39 → Lake of Rage → Ice Path → Route 45 → Victory Road

Target: 40+ species caught and tracked in collection.

## M: Ruby/Sapphire Hoenn Completionist

Game: `--game=ruby`. Route progression:
Route 101 → Route 102-104 → Petalburg Woods → Route 116 → Granite Cave → Route 110 → Route 117 → Route 111 (desert) → Fiery Path → Route 113-114 → Route 118-121 → Mt. Pyre → Victory Road

Target: 40+ species.

## N: Diamond/Pearl Sinnoh Completionist

Game: `--game=diamond`. Route progression:
Route 201-203 → Oreburgh Mine → Route 204-205 → Eterna Forest → Route 206-208 → Route 209-210 → Route 214-215 → Iron Island → Route 216-217 → Victory Road

Target: 40+ species.

## O: Black/White Unova Completionist

Game: `--game=black`. Route progression:
Route 1-3 → Wellspring Cave → Pinwheel Forest → Route 4 → Desert Resort → Route 5-6 → Chargestone Cave → Celestial Tower → Twist Mountain → Route 8-10 → Victory Road

Target: 40+ species. Verify all catches are Gen 5 species (no older pokemon until post-game).

## P: X/Y Kalos Completionist

Game: `--game=x`. Route progression:
Route 2 → Santalune Forest → Route 3-5 → Route 7-8 → Glittering Cave → Route 10-12 → Route 14-15 → Frost Cavern → Route 18-20 → Victory Road

Target: 40+ species.

## Q: Sun/Moon Alola Completionist

Game: `--game=sun`. Route progression:
Route 1-3 → Verdant Cavern → Brooklet Hill → Wela Volcano → Lush Jungle → Route 10-11 → Haina Desert → Vast Poni Canyon → Mount Lanakila

Target: 40+ species. Check for Alolan form annotations on encounters.

## R: Sword/Shield Galar Completionist

Game: `--game=sword`. Route progression:
Route 1-2 → Wild Area south → Route 3 → Galar Mine → Route 4-5 → Galar Mine 2 → Route 6-10 → Wild Area expanded

Target: 40+ species. Check weather rates in encounter details. Verify Galarian form annotations.

## S: Brilliant Diamond Sinnoh Completionist

Game: `--game=brilliant-diamond`. Same route order as N. Also check Grand Underground areas. Compare encounter data between `--game=diamond` and `--game=brilliant-diamond` for 5 pokemon.

Target: 40+ species.

## T: Legends Arceus Hisui Completionist

Game: `--game=legends-arceus`. Area progression:
Obsidian Fieldlands → Crimson Mirelands → Cobalt Coastlands → Coronet Highlands → Alabaster Icelands

Use `--alpha` for alpha catches. Verify alpha_levels in details. Check Hisuian form annotations ("Hisuian Growlithe" not "Growlithe"). Verify `pokemon show growlithe-hisui` shows Fire/Rock types.

Target: 40+ species.

## U: Scarlet/Violet Paldea Completionist

Game: `--game=scarlet`. Area progression:
South Province Areas 1-5 → West Province Areas 1-3 → East Province Areas 1-3 → Glaseado Mountain → North Province Areas 1-3 → Casseroya Lake

Verify probability_overall ≤100%. Check Paldean form annotations ("Paldean Wooper"). Check Paldea dex progress.

Target: 40+ species.

## V: Legends Z-A Lumiose Completionist

Game: `--game=legends-za`. Zone progression:
Wild Zones 1-4 → 5-6 → 7-10 → 11-13 → 14-20

Use `--alpha` for alpha catches. Verify Wild Zone locations correct. Check alpha data in details.

Target: 40+ species across all 20 zones.