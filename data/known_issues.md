# Known Data Quality Issues

This file tracks data quality problems in the seeded database. Each issue has a status:
- **OPEN**: Known, not yet fixed
- **FIXED**: Fixed via override or seed logic
- **WONTFIX**: Investigated, determined to be correct or unfixable without upstream changes

Issues are referenced by the exploratory test skill so testers don't re-report known problems.

---

## Encounter Data

### E001: Sinnoh Route 201 rarity sums to 95%, not 100%
- **Status**: WONTFIX (upstream gap)
- **Source**: PokeAPI encounters.csv
- **Games**: Diamond, Pearl, Platinum
- **Detail**: Walking encounters on Route 201 sum to 95 instead of 100. Investigation: PokeAPI defines 12 encounter slots for Diamond walking (summing to 100), but slots 7, 8, 10, 12 (rarity 5+5+4+1=15... wait, that's 15 not 5). Actually only slots with rarity 5 are missing (2 slots × 5 = 10... still not 5). The actual data has 8 rows summing to 95 out of 12 possible slots summing to 100 — 4 slots with no pokemon assigned. This is upstream PokeAPI data completeness, possibly GBA-slot-dependent encounters that only trigger with specific cartridge combinations.
- **Found**: Round 11 (Tester C)

### E002: Pre-Gen-5 species appear in Black/White encounter data
- **Status**: WONTFIX
- **Source**: PokeAPI encounters.csv
- **Games**: Black, White, Black 2, White 2
- **Detail**: Fishing/surfing encounters include pre-Gen-5 species (Feebas, Goldeen, Krabby, etc.) that are only available post-National Dex. PokeAPI doesn't model pre/post-game availability.
- **Found**: Round 11 (Tester O)

### E003: Seafoam Islands meta.total disagrees with data length
- **Status**: WONTFIX (not reproducible)
- **Source**: Location encounters query
- **Games**: lets-go-pikachu, lets-go-eevee
- **Detail**: Tester K reported meta.total=51 but data length=50. Investigation shows both values are 51 — the tester saw 50 on page 1 (limit=50) and assumed that was the total. Pagination is correct: page 1 has 50, page 2 has 1.
- **Found**: Round 12 (Tester K)

### E004: Duplicate encounter rows in some Gen 4 locations
- **Status**: OPEN (partially fixed by dedup)
- **Source**: PokeAPI encounters.csv
- **Games**: Diamond, Pearl, Platinum, HeartGold, SoulSilver
- **Detail**: Some locations have duplicate encounter rows with identical data but different encounter_condition_value_map entries. The broadened dedup (grouping by pokemon/version/area/levels without slot_id) removes most but not all.
- **Found**: Round 7 (Tester M), Round 11 (Tester C)

## Evolution Data

### V001: Dipplin and Hydrapple missing evolution methods
- **Status**: FIXED (override)
- **Source**: PokeAPI pokemon_evolution.csv
- **Detail**: Gen 9 DLC evolutions (Syrupy Apple item) didn't have rows in pokemon_evolution table. Fixed via curated override: Dipplin → "Use Syrupy Apple", Hydrapple → "Level up knowing Dragon Cheer". The override system now INSERTs new rows when no matching row exists to UPDATE.
- **Found**: Round 11 (Tester I), Fixed: Round 12

### V002: Meltan→Melmetal evolution has no trigger data
- **Status**: FIXED (override)
- **Source**: PokeAPI pokemon_evolution.csv
- **Detail**: Meltan's unique GO-only evolution (400 candies) isn't in PokeAPI. Fixed via curated override: "400 Meltan Candy in Pokémon GO"
- **Found**: Round 3 (Tester 4)

## Form Data

### F001: pokemon encounters for form names returns base species data
- **Status**: WONTFIX
- **Source**: Design limitation
- **Detail**: `pokemon encounters geodude-alola` resolves to species geodude and returns all Geodude encounters (base form). The encounter table stores base pokemon_id for most games. The regional annotation overlay handles display names but can't filter encounters to form-specific ones.
- **Found**: Round 12 (Tester K)
