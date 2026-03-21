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

## J: Red/Blue Kanto Playthrough

Simulate playing through Red. Progress route by route, catch pokemon at each location, evolve them, build toward Elite Four. Route order:
1. Pallet Town (starter: charmander) → Route 1 (pidgey, rattata) → Route 2 (caterpie, weedle)
2. Viridian Forest (pikachu, caterpie, metapod) → Route 3 (spearow, jigglypuff, nidoran-m)
3. Mt. Moon (zubat, geodude, paras, clefairy) → Route 24 (oddish, abra)
4. Rock Tunnel (machop, onix) → Pokemon Tower (gastly, cubone)
5. Safari Zone (kangaskhan, scyther, tauros) → Seafoam Islands (seel, articuno)
6. Victory Road (machoke, golbat, onix)

At each step: check encounters with `--game=red`, add catches to collection, evolve base forms (mark as evolved, add evolution as living_dex), check dex progress. Use 10-15 pokemon total. Verify Kanto locations are correct.

## K: Let's Go Pikachu Kanto Playthrough

Same route order as Red/Blue but playing lets-go-pikachu. Route order:
1. Pallet Town (partner pikachu) → Route 1 → Viridian Forest (caterpie, weedle, pikachu, bulbasaur rare)
2. Route 3 → Mt. Moon (zubat, geodude, clefairy) → Route 24 (charmander gift)
3. Rock Tunnel → Pokemon Tower (gastly, cubone) → Route 12 (snorlax)
4. Seafoam Islands (seel, articuno) → Pokemon Mansion (grimer, koffing)
5. Victory Road (machoke, golbat, chansey)

Check encounters with `--game=lets-go-pikachu`. Verify LGPE-specific encounter data exists. Add catches, evolve, check dex progress.

## L: Gold/Silver Johto Playthrough

Progress through Johto in route order:
1. New Bark (starter: cyndaquil) → Route 29 (sentret, pidgey) → Route 30 (caterpie, weedle, poliwag)
2. Sprout Tower (gastly, bellsprout) → Route 32 (mareep, wooper, ekans)
3. Union Cave (onix, zubat) → Ilex Forest (oddish, paras)
4. Route 34 (drowzee, abra, ditto) → National Park (scyther, pinsir)
5. Burned Tower (raikou/entei/suicune) → Route 38 (tauros, miltank, snubbull)
6. Lake of Rage (gyarados) → Ice Path (swinub, jynx) → Route 45 (gligar, phanpy)
7. Victory Road (golbat, ursaring)

Check encounters with `--game=gold`. Add catches, evolve (mareep→flaaffy→ampharos, cyndaquil→quilava→typhlosion), track dex progress through Johto.

## M: Ruby/Sapphire Hoenn Playthrough

Progress through Hoenn:
1. Littleroot (starter: mudkip) → Route 101 (zigzagoon, wurmple, poochyena) → Route 102 (ralts, lotad)
2. Petalburg Woods (shroomish, slakoth) → Rustboro (nincada, whismur)
3. Granite Cave (makuhita, aron, sableye) → Route 110 (electrike, gulpin, plusle, minun)
4. Route 111 desert (trapinch, cacnea, baltoy) → Fiery Path (numel, torkoal)
5. Route 119 (tropius, kecleon) → Route 120 (absol) → Mt. Pyre (shuppet, duskull)
6. Seafloor Cavern → Victory Road (lairon, hariyama, medicham)

Check encounters with `--game=ruby`. Verify Hoenn locations. Catch and evolve (ralts→kirlia→gardevoir, mudkip→marshtomp→swampert).

## N: Diamond/Pearl Sinnoh Playthrough

Progress through Sinnoh:
1. Twinleaf (starter: chimchar) → Route 201-202 (starly, bidoof, shinx, kricketot)
2. Oreburgh Mine (geodude, onix) → Route 204 (budew, pachirisu)
3. Valley Windworks (buizel, shellos, drifloon) → Eterna Forest (buneary, gastly)
4. Route 206-207 (ponyta, machop) → Route 209 (mime-jr, chansey)
5. Route 210 fog (psyduck, scyther) → Iron Island (riolu egg)
6. Route 216-217 (snover, sneasel) → Mt. Coronet summit (dialga/palkia)
7. Victory Road (golbat, steelix, medicham)

Check encounters with `--game=diamond`. Also check brilliant-diamond for same pokemon. Compare data between original and remake.

## O: Black/White Unova Playthrough

Progress through Unova:
1. Nuvema (starter: tepig) → Route 1 (patrat, lillipup) → Dreamyard (munna)
2. Pinwheel Forest (sewaddle, venipede, cottonee, throh/sawk) → Route 4 (sandile, darumaka, scraggy)
3. Desert Resort (sigilyph, yamask) → Chargestone Cave (joltik, klink, ferroseed, tynamo)
4. Celestial Tower (litwick, elgyem) → Twist Mountain (cubchoo, cryogonal)
5. Dragonspiral Tower (golett, mienfoo, druddigon) → Route 9 (pawniard)
6. Victory Road (deino, heatmor, durant)

Check encounters with `--game=black`. Verify Unova locations. Gen 5 had no old pokemon until post-game, so all catches should be Gen 5 species.

## P: X/Y Kalos Playthrough

Progress through Kalos:
1. Vaniville (starter: froakie) → Route 2 (caterpie, pidgey, fletchling) → Santalune Forest (pikachu, scatterbug)
2. Route 4 (flabebe, ralts, combee) → Route 5 (pancham, furfrou, gulpin)
3. Route 7 (smeargle, roselia, ducklett) → Connecting Cave (axew, meditite)
4. Glittering Cave (kangaskhan, mawile) → Route 10 (eevee, emolga, hawlucha)
5. Reflection Cave (mr-mime, sableye, carbink) → Route 12 (tauros, heracross)
6. Route 14 (haunter, goomy) → Frost Cavern (jynx, sneasel, beartic)
7. Route 19 (sliggoo, gurdurr) → Victory Road (zweilous, noibat)

Check encounters with `--game=x`. Verify Kalos locations match X/Y.

## Q: Sun/Moon Alola Playthrough

Progress through Alola's island trials:
1. Route 1 (pikipek, yungoos, grubbin) → Verdant Cavern trial (gumshoos/raticate-alola)
2. Route 4-5 (eevee, mudbray, fomantis) → Brooklet Hill trial (wishiwashi)
3. Wela Volcano trial (salandit) → Lush Jungle trial (fomantis, morelull, lurantis)
4. Hokulani Observatory trial (charjabug, togedemaru) → Thrifty Megamart trial (mimikyu)
5. Vast Poni Canyon trial (jangmo-o, kommo-o)
6. Mount Lanakila (snorunt, absol, vanillite)

Check encounters with `--game=sun`. Verify Alola locations and Alolan form encounters.

## R: Sword/Shield Galar Playthrough

Progress through Galar:
1. Route 1 (wooloo, rookidee, skwovet, nickit) → Route 2 (chewtle, yamper, lotad)
2. Wild Area south (machop, ralts, pikachu, snover — varies by weather)
3. Galar Mine (rolycoly, roggenrola, woobat) → Route 4 (milcery, meowth-galar)
4. Route 5 (farfetch'd-galar, applin, drifloon) → Galar Mine 2 (binacle, noibat)
5. Route 6 (silicobra, torkoal, axew) → Route 7 (corviknight, perrserker, inkay)
6. Route 8 (snom, falinks, duraludon) → Route 9 (clobbopus, octillery)
7. Route 10 (sneasel, abomasnow, beartic)

Check encounters with `--game=sword`. Verify weather-specific rates in encounter details. Test Max Raid encounters.

## S: Brilliant Diamond Sinnoh Playthrough

Same route order as Diamond/Pearl (persona N) but playing brilliant-diamond:
1. Same progression through Sinnoh routes and cities
2. Additionally check Grand Underground encounters
3. Compare encounter data between `--game=diamond` and `--game=brilliant-diamond` for the same pokemon/location
4. Verify BDSP-specific details (time-of-day rates)
5. Focus on 10 pokemon that appear in both games and compare their data

## T: Legends Arceus Hisui Playthrough

Progress through Hisui's five areas in order:
1. Obsidian Fieldlands: starly, bidoof, shinx, ponyta, eevee, buizel, wurmple, machop (alpha: rapidash, snorlax)
2. Crimson Mirelands: croagunk, hippopotas, tangela, yanma, teddiursa, rhyhorn (alpha: torterra)
3. Cobalt Coastlands: octillery, aipom, shellos, qwilfish-hisui, magby (alpha: walrein, gyarados)
4. Coronet Highlands: clefairy, gible, misdreavus, bronzor, unown (alpha: electivire)
5. Alabaster Icelands: snorunt, swinub, sneasel-hisui, bergmite, abomasnow (alpha: garchomp, lucario)

All catches with `--game=legends-arceus --alpha` for alpha encounters. Verify alpha_levels in details. Check Hisuian form data (growlithe-hisui, zorua-hisui). Verify forms have correct is_default.

## U: Scarlet/Violet Paldea Playthrough

Open world with typical gym progression:
1. Cabo Poco (starter: sprigatito) → South Province Area 1 (lechonk, tarountula, fletchling, hoppip)
2. South Province Area 2 (ralts, shinx, marill) → Cortondo gym
3. West Province Area 1 (flabebe, shroodle, maschiff) → Cascarrafa gym
4. East Province Area 1 (mareep, phanpy, houndour) → Levincia gym
5. Glaseado Mountain (snover, cetoddle, frigibax) → Medali gym
6. North Province (zweilous, dragapult, lucario) → Casseroya Lake (dondozo, dratini)
7. Area Zero (great tusk/iron treads, flutter mane/iron jugulis)

Check encounters with `--game=scarlet`. Verify probability_overall values are percentages (≤100%). Check Paldea dex progress.

## V: Legends Z-A Lumiose Playthrough

Progress through Wild Zones in unlock order:
1. Wild Zones 1-4 (Mission 3): bunnelby, fletchling, mareep, pichu, weedle, scatterbug, litleo, honedge
2. Wild Zones 5-6 (Mission 5-10): pidgey, electrike, patrat, magnemite, voltorb
3. Wild Zones 7-10 (Mission 10-15): espurr, noibat, skiddo, eevee
4. Wild Zones 11-13 (Mission 15-25): more evolved pokemon, higher levels
5. Wild Zones 14-20 (Mission 25-39): endgame pokemon, high-level alphas

Check encounters with `--game=legends-za`. Verify Wild Zone locations (not "Lumiose City"). Check alpha data in details. Add with `--alpha` flag for alpha catches.