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
