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

Play through Red, catching EVERY available pokemon at each location. Goal: near-complete Kanto dex. For each location, run `pokedex pokemon encounters <name> --game=red` for every pokemon listed, add each to collection, evolve every base form through its full chain (mark bases as evolved, add evolutions as living_dex). Check `pokedex dex progress national --caught` after each area.

Progress route by route, catching all wild pokemon at each stop:
1. Pallet Town: charmander (starter gift)
2. Route 1: pidgey, rattata
3. Route 22: nidoran-f, nidoran-m, spearow, mankey
4. Route 2: pidgey, rattata, caterpie, weedle
5. Viridian Forest: pikachu, caterpie, metapod, weedle, kakuna
6. Route 3: spearow, jigglypuff, nidoran-m, nidoran-f
7. Mt. Moon: zubat, geodude, paras, clefairy
8. Route 4: rattata, spearow, ekans, sandshrew
9. Route 24-25: oddish, bellsprout, abra, pidgey
10. Route 5-6: pidgey, meowth, oddish, bellsprout
11. Route 11: drowzee, ekans, sandshrew, spearow
12. Route 9-10: voltorb, magnemite, spearow
13. Rock Tunnel: zubat, geodude, machop, onix
14. Route 8: pidgey, meowth, vulpix, growlithe
15. Pokemon Tower: gastly, haunter, cubone
16. Celadon City: eevee (gift)
17. Route 16: snorlax, doduo
18. Route 12-15: oddish, gloom, bellsprout, ditto, pidgey
19. Safari Zone: kangaskhan, tauros, chansey, scyther, pinsir, exeggcute, rhyhorn, nidorino, nidorina
20. Route 19-20: tentacool, horsea, shellder, krabby, staryu
21. Seafoam Islands: seel, shellder, psyduck, slowpoke, articuno
22. Cinnabar/Pokemon Mansion: grimer, muk, koffing, weezing, growlithe, vulpix, magmar
23. Route 21: tangela
24. Victory Road: machoke, geodude, golbat, onix, marowak

Evolve chains: charmander→charmeleon→charizard, pidgey→pidgeotto→pidgeot, rattata→raticate, caterpie→metapod→butterfree, weedle→kakuna→beedrill, nidoran-f→nidorina→nidoqueen, nidoran-m→nidorino→nidoking, zubat→golbat, geodude→graveler→golem, abra→kadabra→alakazam, machop→machoke→machamp, gastly→haunter→gengar, eevee→(pick one: vaporeon/jolteon/flareon)

Target: 50+ unique species caught, 80+ collection entries including evolutions.

## K: Let's Go Pikachu Kanto Completionist

Same route order as Red/Blue but with `--game=lets-go-pikachu`. Catch everything at every location. This game has the same Kanto map but different encounter methods.

Progress through ALL locations (same as J) but:
1. Route 1-2: pidgey, rattata, oddish, bellsprout, caterpie
2. Viridian Forest: pikachu, caterpie, metapod, weedle, kakuna, bulbasaur (rare)
3. Route 3: spearow, jigglypuff, nidoran-m, nidoran-f, mankey
4. Mt. Moon: zubat, geodude, paras, clefairy
5. Route 24: oddish, bellsprout, abra, charmander (gift)
6. Route 5-6: pidgey, oddish, bellsprout, psyduck, jigglypuff, meowth
7. Rock Tunnel: zubat, geodude, machop, onix
8. Pokemon Tower: gastly, haunter, cubone
9. Route 12: snorlax, oddish, bellsprout, pidgey
10. Route 19: tentacool, horsea, shellder, magikarp, staryu
11. Seafoam Islands: seel, psyduck, slowpoke, shellder, articuno
12. Pokemon Mansion: grimer, koffing, growlithe, vulpix, magmar
13. Victory Road: machoke, geodude, golbat, onix, chansey

Compare encounter data with Red (persona J) — same locations should have mostly same pokemon. Catch 40+ species.

## L: Gold/Silver Johto Completionist

Play through Gold catching everything. Goal: complete Johto dex.

1. Route 29: pidgey, sentret, hoothoot, rattata
2. Route 46: geodude, spearow, rattata
3. Route 30-31: pidgey, caterpie, weedle, poliwag, metapod, bellsprout
4. Dark Cave: zubat, geodude, teddiursa
5. Sprout Tower: rattata, gastly, bellsprout
6. Route 32: ekans, bellsprout, mareep, wooper, hoppip
7. Union Cave: zubat, geodude, onix, sandshrew
8. Slowpoke Well: slowpoke, zubat
9. Ilex Forest: oddish, paras, caterpie, metapod, zubat
10. Route 34: drowzee, abra, ditto, jigglypuff
11. Route 35: nidoran-f, nidoran-m, drowzee, ditto, yanma
12. National Park: caterpie, weedle, scyther, pinsir (bug contest)
13. Route 36-37: nidoran-f, nidoran-m, growlithe, stantler, vulpix, pidgey
14. Burned Tower: rattata, koffing, zubat
15. Route 38-39: rattata, tauros, miltank, snubbull, meowth, magnemite
16. Route 42: mareep, spearow, mankey, zubat
17. Route 43: pidgeotto, flaaffy, girafarig, venonat
18. Lake of Rage: magikarp, gyarados (red gyarados)
19. Route 44: poliwag, tangela, lickitung, bellsprout, weepinbell
20. Ice Path: zubat, golbat, swinub, jynx, delibird
21. Route 45: geodude, graveler, gligar, phanpy, donphan
22. Victory Road: golbat, graveler, onix, rhyhorn, ursaring

Evolve chains: cyndaquil→quilava→typhlosion, sentret→furret, hoothoot→noctowl, mareep→flaaffy→ampharos, wooper→quagsire, bellsprout→weepinbell→victreebel, geodude→graveler→golem, gastly→haunter→gengar, abra→kadabra→alakazam, poliwag→poliwhirl→poliwrath, swinub→piloswine

Target: 60+ species, 100+ collection entries.

## M: Ruby/Sapphire Hoenn Completionist

Play through Ruby catching everything at each location.

1. Route 101: zigzagoon, wurmple, poochyena
2. Route 103: zigzagoon, wingull, poochyena
3. Route 102: zigzagoon, wurmple, lotad, ralts, poochyena
4. Route 104: zigzagoon, wurmple, marill, wingull, taillow
5. Petalburg Woods: shroomish, wurmple, silcoon, slakoth, vigoroth
6. Route 116: nincada, whismur, taillow, skitty, abra
7. Rusturf Tunnel: whismur, zubat
8. Granite Cave: zubat, makuhita, geodude, aron, sableye
9. Route 110: gulpin, oddish, electrike, minun, plusle
10. Route 117: oddish, marill, roselia, illumise, volbeat
11. Route 111: sandshrew, trapinch, cacnea, baltoy
12. Fiery Path: numel, koffing, grimer, torkoal, slugma
13. Route 113: spinda, skarmory, sandshrew
14. Route 114: swablu, zangoose, lotad, lombre
15. Route 118-119: electrike, zigzagoon, wingull, kecleon, oddish, tropius
16. Route 120-121: absol, oddish, marill, kecleon, shuppet
17. Mt. Pyre: shuppet, duskull, vulpix, chimecho, meditite
18. Route 124-127: tentacool, wingull (surf/dive)
19. Seafloor Cavern: zubat, golbat
20. Victory Road: golbat, lairon, loudred, hariyama, medicham

Evolve chains: mudkip→marshtomp→swampert, zigzagoon→linoone, wurmple→silcoon→beautifly/cascoon→dustox, ralts→kirlia→gardevoir, taillow→swellow, shroomish→breloom, whismur→loudred→exploud, makuhita→hariyama, aron→lairon→aggron, electrike→manectric, numel→camerupt, trapinch→vibrava→flygon, swablu→altaria, shuppet→banette, duskull→dusclops

Target: 60+ species, 100+ entries.

## N: Diamond/Pearl Sinnoh Completionist

Play through Diamond catching everything.

1. Route 201: starly, bidoof
2. Route 202: starly, bidoof, shinx, kricketot
3. Route 203: starly, bidoof, shinx, abra, zubat
4. Oreburgh Gate: zubat, geodude, psyduck
5. Oreburgh Mine: geodude, onix, zubat
6. Route 204: budew, zubat, shinx, starly
7. Route 204 north: budew, pachirisu, shellos
8. Valley Windworks: buizel, shellos, pachirisu, drifloon
9. Route 205: buizel, shellos, pachirisu, bidoof
10. Eterna Forest: buneary, wurmple, beautifly, dustox, gastly, budew
11. Route 206: stunky, kricketot, ponyta
12. Route 207: machop, geodude, ponyta
13. Mt. Coronet: zubat, geodude, machop, chingling
14. Route 208: bidoof, psyduck, machop, ralts
15. Route 209: starly, bibarel, mime-jr, chansey
16. Route 210: ponyta, staravia, geodude, kricketune
17. Route 215: kadabra, ponyta, geodude
18. Route 214: ponyta, geodude, graveler, girafarig, houndour
19. Route 212: roselia, budew, ralts, croagunk, shellos
20. Route 218: shellos, gastrodon, floatzel, mr-mime
21. Iron Island: onix, steelix, golbat, graveler
22. Route 216-217: snover, sneasel, zubat, machoke, medicham
23. Route 222: electabuzz, magnemite, chatot, luxio
24. Route 223: tentacruel, pelipper, mantyke
25. Victory Road: golbat, graveler, onix, steelix, medicham

Evolve chains: chimchar→monferno→infernape, starly→staravia→staraptor, bidoof→bibarel, shinx→luxio→luxray, geodude→graveler→golem, zubat→golbat→crobat, buizel→floatzel, buneary→lopunny, budew→roselia→roserade, machop→machoke→machamp, ponyta→rapidash, ralts→kirlia→gardevoir, snover→abomasnow, sneasel→weavile

Target: 55+ species, 100+ entries.

## O: Black/White Unova Completionist

Play through Black catching everything. All Gen 5 species.

1. Route 1: patrat, lillipup
2. Route 2: patrat, lillipup, purrloin
3. Dreamyard: munna, patrat, purrloin, audino
4. Route 3: pidove, blitzle, lillipup
5. Wellspring Cave: woobat, roggenrola, drilbur
6. Pinwheel Forest: sewaddle, venipede, cottonee, tympole, throh
7. Route 4: sandile, darumaka, scraggy, trubbish
8. Desert Resort: sandile, sigilyph, maractus, yamask
9. Route 5: minccino, liepard, gothita, trubbish
10. Route 6: deerling, karrablast, shelmet, tranquill
11. Chargestone Cave: joltik, klink, ferroseed, tynamo, boldore
12. Route 7: deerling, tranquill, zebstrika, watchog
13. Celestial Tower: litwick, elgyem, golbat
14. Twist Mountain: boldore, cubchoo, cryogonal, gurdurr
15. Route 8: palpitoad, stunfisk, shelmet, karrablast
16. Route 9: pawniard, garbodor, liepard
17. Route 10: bouffalant, herdier, throh, rufflet
18. Victory Road: boldore, mienfoo, heatmor, durant, deino

Evolve chains: tepig→pignite→emboar, patrat→watchog, lillipup→herdier→stoutland, pidove→tranquill→unfezant, blitzle→zebstrika, sewaddle→swadloon→leavanny, venipede→whirlipede→scolipede, sandile→krokorok→krookodile, darumaka→darmanitan, roggenrola→boldore→gigalith, joltik→galvantula, litwick→lampent→chandelure, deino→zweilous→hydreigon, cubchoo→beartic

Target: 55+ species, 100+ entries.

## P: X/Y Kalos Completionist

Play through X catching everything.

1. Route 2: caterpie, weedle, pidgey, zigzagoon, fletchling
2. Santalune Forest: pikachu, caterpie, weedle, pansage, pansear, panpour, scatterbug
3. Route 3: pidgey, fletchling, bunnelby, azurill, dunsparce
4. Route 4: flabebe, ledyba, ralts, combee, skitty
5. Route 5: pancham, furfrou, gulpin, abra, doduo
6. Route 7: smeargle, hoppip, roselia, flabebe, ducklett
7. Connecting Cave: zubat, whismur, axew, meditite
8. Route 8: absol, mienfoo, spoink, zangoose, inkay
9. Glittering Cave: machop, onix, kangaskhan, mawile, cubone
10. Route 10: eevee, emolga, golett, hawlucha, snubbull
11. Route 11: nidoran-f, nidoran-m, hariyama, chingling, stunky
12. Reflection Cave: mr-mime, sableye, roggenrola, carbink, wobbuffet
13. Route 12: tauros, miltank, pachirisu, chatot, heracross
14. Route 14: haunter, quagsire, goomy, stunfisk, skorupi
15. Route 15: klefki, foongus, pawniard, murkrow
16. Frost Cavern: jynx, piloswine, beartic, sneasel, cryogonal
17. Route 18: torkoal, heatmor, durant, lairon, pupitar
18. Route 19: sliggoo, gurdurr, drapion, weepinbell
19. Route 20: trevenant, amoonguss, zoroark, noctowl
20. Victory Road: lickitung, gurdurr, zweilous, noibat

Evolve chains: froakie→frogadier→greninja, fletchling→fletchinder→talonflame, bunnelby→diggersby, ralts→kirlia→gardevoir, pancham→pangoro, axew→fraxure→hatchet, goomy→sliggoo→goodra, inkay→malamar, noibat→noivern, honedge→doublade→aegislash

Target: 65+ species, 110+ entries.

## Q: Sun/Moon Alola Completionist

Play through Sun catching everything at each trial location.

Melemele Island:
1. Route 1: pikipek, yungoos, ledyba, caterpie, grubbin
2. Route 2: drowzee, smeargle, growlithe, spearow, makuhita
3. Hau'oli City: wingull, magnemite, grimer-alola, meowth-alola
4. Verdant Cavern: yungoos, diglett, zubat
5. Route 3: rufflet, spearow, mankey, delibird, hawlucha
6. Melemele Meadow: oricorio, caterpie, butterfree, cottonee, petilil
7. Seaward Cave: zubat, psyduck, seel, delibird

Akala Island:
8. Route 4: lillipup, yungoos, eevee, mudbray, grubbin
9. Route 5: fomantis, trumbeak, dewpider, salandit
10. Brooklet Hill: dewpider, surskit, poliwag, wishiwashi
11. Route 7: tentacool, wingull, staryu, magikarp
12. Wela Volcano: cubone, fletchling, salandit, kangaskhan
13. Lush Jungle: fomantis, morelull, paras, bounsweet
14. Route 8: stufful, fletchinder, trumbeak

Ula'ula Island:
15. Route 10-11: pancham, skarmory, komala
16. Haina Desert: sandile, dugtrio-alola, trapinch, gabite
17. Route 13-14: gumshoos, raticate-alola, gastly, haunter
18. Thrifty Megamart: gastly, haunter, mimikyu

Poni Island:
19. Vast Poni Canyon: jangmo-o, machoke, boldore, carbink
20. Mount Lanakila: snorunt, absol, vulpix-alola, vanillite, sneasel

Evolve chains: rowlet→dartrix→decidueye, pikipek→trumbeak→toucannon, yungoos→gumshoos, grubbin→charjabug→vikavolt, dewpider→araquanid, fomantis→lurantis, salandit→salazzle, bounsweet→steenee→tsareena, jangmo-o→hakamo-o→kommo-o, mudbray→mudsdale, stufful→bewear, rockruff→lycanroc

Target: 55+ species, 100+ entries.

## R: Sword/Shield Galar Completionist

Play through Sword catching everything. Include Wild Area weather encounters.

1. Route 1: skwovet, wooloo, rookidee, blipbug, nickit
2. Route 2: chewtle, yamper, rookidee, blipbug, lotad
3. Wild Area south: machop, onix, snover, stufful, pikachu, tyrogue, ralts, vulpix, swinub, mudbray, growlithe, gastly
4. Route 3: gossifleur, rookidee, sizzlipede, vulpix, mudbray
5. Galar Mine: rolycoly, roggenrola, woobat, diglett, timburr
6. Route 4: wooloo, yamper, meowth-galar, milcery, pikachu
7. Route 5: farfetchd-galar, applin, eldegoss, swoobat, drifloon
8. Galar Mine 2: binacle, noibat, shuckle, stunfisk-galar, carkol
9. Route 6: silicobra, torkoal, helioptile, axew, galvantula
10. Route 7: thievul, corviknight, perrserker, inkay, galvantula
11. Route 8: snom, falinks, duraludon, togedemaru, grapploct
12. Route 9: clobbopus, octillery, pelipper, gastrodon, pyukumuku
13. Route 10: snom, sneasel, abomasnow, beartic, duraludon
14. Wild Area (expanded): duskull, sneasel, dreepy, larvitar, deino, goomy, jangmo-o

Evolve chains: grookey→thwackey→rillaboom, rookidee→corvisquire→corviknight, wooloo→dubwool, chewtle→drednaw, yamper→boltund, applin→flapple/appletun, snom→frosmoth, dreepy→drakloak→dragapult, larvitar→pupitar→tyranitar, deino→zweilous→hydreigon, rolycoly→carkol→coalossal, sizzlipede→centiskorch

Target: 60+ species, 110+ entries.

## S: Brilliant Diamond Sinnoh Completionist

Same routes as Diamond (persona N) but using `--game=brilliant-diamond`. Catch everything including Grand Underground pokemon:

Same route progression as N, plus Grand Underground areas:
1. Grassland Cave: ralts, eevee, houndour, absol
2. Fountainspring Cave: psyduck, golduck, barboach, wooper
3. Spacious Cave: geodude, onix, zubat, golbat
4. Dazzling Cave: bronzor, chingling, clefairy
5. Volcanic Cave: slugma, magby, houndour, numel

For 10 key pokemon (starly, shinx, geodude, zubat, machop, ponyta, buizel, gastly, budew, snover), look up encounters in BOTH `--game=diamond` and `--game=brilliant-diamond` and compare: same locations? same levels? different details fields (BDSP has time-of-day)?

Target: 55+ species, 100+ entries. Focus on verifying BDSP data vs original D/P data.

## T: Legends Arceus Hisui Completionist

Catch everything in each area. Use `--alpha` for alpha pokemon. Goal: complete Hisui pokedex.

Obsidian Fieldlands (catch all):
1. starly, staravia, bidoof, bibarel, shinx, luxio, ponyta, rapidash
2. eevee, buizel, floatzel, wurmple, silcoon, beautifly, cascoon, dustox
3. machop, machoke, geodude, graveler, psyduck, golduck
4. zubat, golbat, pichu, pikachu, magikarp, gyarados
5. Alpha catches: rapidash, snorlax, alakazam, infernape

Crimson Mirelands (catch all):
6. croagunk, toxicroak, hippopotas, hippowdon, tangela, tangrowth
7. yanma, yanmega, teddiursa, ursaring, rhyhorn, rhydon
8. combee, vespiquen, roselia, roserade, petilil, lilligant-hisui
9. Alpha catches: torterra, pachirisu, hippowdon

Cobalt Coastlands (catch all):
10. tentacool, tentacruel, octillery, aipom, ambipom
11. shellos, gastrodon, qwilfish-hisui, magby, magmar
12. mantyke, mantine, drifblim, chatot, remoraid
13. Alpha catches: walrein, gyarados, empoleon

Coronet Highlands (catch all):
14. clefairy, clefable, gible, gabite, garchomp
15. misdreavus, mismagius, bronzor, bronzong, unown
16. golem, luxray, mothim, chimecho
17. Alpha catches: electivire, crobat, mothim

Alabaster Icelands (catch all):
18. snorunt, glalie, froslass, swinub, piloswine, mamoswine
19. sneasel-hisui, snover, abomasnow, bergmite, avalugg-hisui
20. Alpha catches: garchomp, lucario, mamoswine, gallade

Verify: alpha_levels in encounter details, Hisuian forms (growlithe-hisui, zorua-hisui, voltorb-hisui) have is_default=false, correct Hisui area names.

Target: 80+ species, 140+ entries including alphas and evolutions.

## U: Scarlet/Violet Paldea Completionist

Catch everything in each province. Open world — thorough sweep.

South Province:
1. Area 1: lechonk, tarountula, fletchling, hoppip, psyduck, buizel, paldean-wooper, pawmi
2. Area 2: ralts, kirlia, shinx, luxio, marill, makuhita, rockruff, yungoos
3. Area 3: drifloon, murkrow, starly, rockruff, flittle
4. Area 4: venonat, girafarig, flittle, grimer, bonsly
5. Area 5: girafarig, venonat, bonsly, flittle, hippopotas, larvitar

West Province:
6. Area 1: shinx, flabebe, maschiff, shroodle, sunkern
7. Area 2: deerling, rockruff, litleo, mudbray, psyduck
8. Area 3: braviary, skiddo, tauros-paldea, mudsdale, skiddo

East Province:
9. Area 1: phanpy, mareep, girafarig, houndour, stantler
10. Area 2: deino, dreepy, tatsugiri, veluza, dondozo
11. Area 3: dondozo, flamingo, tropius, dunsparce

Glaseado Mountain:
12. snover, abomasnow, bergmite, cryogonal, cetoddle, frigibax, beartic

North Province:
13. Area 1: palossand, grimer, drifblim, farigiraf
14. Area 2: zweilous, hydreigon, dragapult, garchomp, lucario
15. Area 3: dragonite, tyranitar, salamence, goodra

Casseroya Lake: dondozo, gyarados, dragonair, dratini, magikarp

Area Zero: great-tusk, iron-treads, brute-bonnet, iron-hands, flutter-mane, iron-jugulis

Evolve chains: sprigatito→floragato→meowscarada, lechonk→oinkologne, tarountula→spidops, pawmi→pawmo→pawmot, fletchling→fletchinder→talonflame, ralts→kirlia→gardevoir/gallade, shinx→luxio→luxray, larvitar→pupitar→tyranitar, deino→zweilous→hydreigon, dreepy→drakloak→dragapult, dratini→dragonair→dragonite, frigibax→arctibax→baxcalibur, cetoddle→cetitan

Target: 80+ species, 140+ entries. Verify all probability_overall ≤100%.

## V: Legends Z-A Lumiose Completionist

Catch everything in every Wild Zone. Progress in unlock order, sweep each zone completely.

Wild Zones 1-2 (Mission 3):
1. Zone 1: bunnelby, fletchling, mareep, pichu, weedle, scatterbug, pidgey (all with alpha variants)
2. Zone 2: litleo, skiddo, oddish, bellsprout, budew, roselia

Wild Zones 3-4 (Mission 3):
3. Zone 3: flabebe, espurr, ralts, kirlia, honedge, litwick
4. Zone 4: gastly, haunter, phantump, pumpkaboo, golett

Wild Zones 5-6 (Mission 5-10):
5. Zone 5: pidgey, pidgeotto, electrike, patrat, venipede, whirlipede
6. Zone 6: magnemite, voltorb, klink, klefki, mawile

Wild Zones 7-8 (Mission 10):
7. Zone 7: froakie, poliwag, psyduck, shellos, goomy
8. Zone 8: snover, bergmite, swinub, sneasel, amaura

Wild Zones 9-10 (Mission 15):
9. Zone 9: noibat, hawlucha, fletchinder, sigilyph, emolga
10. Zone 10: honedge, doublade, phantump, litwick, lampent, spiritomb

Wild Zones 11-14 (Mission 20-30):
11. Zone 11: ralts, kirlia, gardevoir, florges, sylveon
12. Zone 12: sableye, duskull, misdreavus, banette, chandelure
13. Zone 13: tyrantrum, aurorus, goodra, dragalge, clawitzer
14. Zone 14: lucario, aegislash, talonflame, greninja, delphox

Wild Zones 15-20 (Mission 30-39):
15-20. Endgame pokemon at higher levels, guaranteed alpha spawns

For each pokemon: `pokedex pokemon encounters <name> --game=legends-za`, verify Wild Zone N location (not "Lumiose City"), check alpha data in details, add with `--alpha` for alpha catches.

Evolve chains: bunnelby→diggersby, fletchling→fletchinder→talonflame, mareep→flaaffy→ampharos, pichu→pikachu→raichu, weedle→kakuna→beedrill, scatterbug→spewpa→vivillon, litleo→pyroar, espurr→meowstic, honedge→doublade→aegislash, gastly→haunter→gengar, noibat→noivern, goomy→sliggoo→goodra, froakie→frogadier→greninja

Target: 70+ species, 120+ entries. Check all 20 zones covered.