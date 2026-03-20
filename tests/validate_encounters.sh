#!/bin/bash
#
# Comprehensive encounter data validation across the national pokédex.
#
# Tests every species in the database, checking that:
# 1. `pokemon show` returns valid data for every species
# 2. `pokemon encounters` returns data where expected (multi-game coverage)
# 3. Multi-source encounters (PokeAPI + PokeDB + Z-A) don't produce duplicates or gaps
# 4. Encounter details are populated for modern games (SwSh, SV, BDSP, PLA, Z-A)
#
# Usage:
#   POKEDEX_DB_PATH=/path/to/db ./tests/validate_encounters.sh
#   # or just: ./tests/validate_encounters.sh (uses default ~/.pokedex/db.sqlite)

set -euo pipefail

POKEDEX=${POKEDEX:-pokedex}
DB_PATH=${POKEDEX_DB_PATH:-$HOME/.pokedex/db.sqlite}
export POKEDEX_DB_PATH="$DB_PATH"

PASS=0
FAIL=0
WARN=0
ERRORS=""

pass() { PASS=$((PASS + 1)); }
fail() { FAIL=$((FAIL + 1)); ERRORS="$ERRORS\nFAIL: $1"; echo "  FAIL: $1"; }
warn() { WARN=$((WARN + 1)); echo "  WARN: $1"; }

echo "=== Pokédex Encounter Validation ==="
echo "Database: $DB_PATH"
echo ""

# ------------------------------------------------------------------
# Phase 1: Validate every species resolves via `pokemon show`
# ------------------------------------------------------------------
echo "--- Phase 1: Species resolution (national dex) ---"

SPECIES_LIST=$($POKEDEX pokemon list --limit=2000 2>/dev/null | python3 -c "
import sys, json
data = json.load(sys.stdin)['data']
for s in data:
    print(s['name'])
")
TOTAL_SPECIES=$(echo "$SPECIES_LIST" | wc -l | tr -d ' ')
echo "Testing $TOTAL_SPECIES species..."

SHOW_FAILURES=0
for name in $SPECIES_LIST; do
    result=$($POKEDEX pokemon show "$name" 2>/dev/null || echo '{"error":true}')
    has_error=$(echo "$result" | python3 -c "import sys,json; d=json.load(sys.stdin); print('yes' if 'error' in d else 'no')" 2>/dev/null || echo "yes")
    if [ "$has_error" = "yes" ]; then
        fail "pokemon show $name — returned error"
        SHOW_FAILURES=$((SHOW_FAILURES + 1))
    else
        pass
    fi
done
echo "  Species resolution: $((TOTAL_SPECIES - SHOW_FAILURES))/$TOTAL_SPECIES passed"
echo ""

# ------------------------------------------------------------------
# Phase 2: Encounter coverage by game
# ------------------------------------------------------------------
echo "--- Phase 2: Encounter coverage by game ---"

GAMES="sword shield scarlet violet brilliant-diamond shining-pearl legends-arceus lets-go-pikachu lets-go-eevee legends-za"

for game in $GAMES; do
    count=$($POKEDEX pokemon encounters pikachu --game="$game" 2>/dev/null | python3 -c "
import sys, json
try:
    d = json.load(sys.stdin)
    print(len(d.get('data', [])))
except:
    print(0)
" 2>/dev/null || echo "0")

    if [ "$count" -gt 0 ]; then
        echo "  $game: pikachu has $count encounter(s) ✓"
        pass
    else
        # Pikachu might not be in every game — check if any pokemon has encounters
        any_count=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM encounters e JOIN versions v ON v.id = e.version_id LEFT JOIN version_names vn ON vn.version_id = v.id WHERE v.name = '$game' OR LOWER(vn.name) = LOWER('$game');" 2>/dev/null || echo "0")
        if [ "$any_count" -gt 0 ]; then
            echo "  $game: $any_count total encounters (pikachu not in this game) ✓"
            pass
        else
            fail "$game: no encounters at all"
        fi
    fi
done
echo ""

# ------------------------------------------------------------------
# Phase 3: Multi-source Pokémon (appear in PokeAPI + PokeDB + Z-A games)
# ------------------------------------------------------------------
echo "--- Phase 3: Multi-source encounter validation ---"

# These Pokémon should have encounters from multiple data sources
MULTI_SOURCE=(
    "pikachu"      # PokeAPI (Gen 1-5), PokeDB (SwSh/SV), Z-A
    "eevee"        # PokeAPI (Gen 1-5), PokeDB (SwSh/SV/LGPE), Z-A
    "gastly"       # PokeAPI (Gen 1-5), PokeDB (SwSh/BDSP), Z-A
    "machop"       # PokeAPI (Gen 1-5), PokeDB (BDSP/SV), Z-A
    "magikarp"     # PokeAPI (Gen 1-5), PokeDB (SwSh/BDSP/SV)
    "ralts"        # PokeAPI (Gen 3-5), PokeDB (SwSh/BDSP/SV), Z-A
    "shinx"        # PokeAPI (Gen 4-5), PokeDB (BDSP/SV), Z-A
    "dratini"      # PokeAPI (Gen 1-5), PokeDB (SwSh/SV), Z-A
    "larvitar"     # PokeAPI (Gen 2-5), PokeDB (SwSh/SV)
    "riolu"        # PokeAPI (Gen 4-5), PokeDB (SwSh/SV), Z-A
)

for pokemon in "${MULTI_SOURCE[@]}"; do
    # Get all encounters (no game filter)
    result=$($POKEDEX pokemon encounters "$pokemon" 2>/dev/null)

    total=$(echo "$result" | python3 -c "import sys,json; print(len(json.load(sys.stdin).get('data',[])))" 2>/dev/null || echo "0")
    games=$(echo "$result" | python3 -c "
import sys, json
d = json.load(sys.stdin).get('data', [])
games = sorted(set(e['game'] for e in d))
print(', '.join(games))
" 2>/dev/null || echo "none")

    details_count=$(echo "$result" | python3 -c "
import sys, json
d = json.load(sys.stdin).get('data', [])
print(sum(1 for e in d if e.get('details')))
" 2>/dev/null || echo "0")

    if [ "$total" -gt 0 ]; then
        echo "  $pokemon: $total encounters across [$games] ($details_count with details) ✓"
        pass
    else
        fail "$pokemon: no encounters found (expected multi-source data)"
    fi

    # Check for exact duplicate encounters (same pokemon + location + game + level)
    dupes=$(echo "$result" | python3 -c "
import sys, json
d = json.load(sys.stdin).get('data', [])
seen = set()
dupes = 0
for e in d:
    key = (e['location'], e['area'], e['game'], e['method'], e['min_level'], e['max_level'])
    if key in seen:
        dupes += 1
    seen.add(key)
print(dupes)
" 2>/dev/null || echo "0")

    if [ "$dupes" -gt 0 ]; then
        warn "$pokemon: $dupes exact duplicate encounter(s)"
    fi
done
echo ""

# ------------------------------------------------------------------
# Phase 4: Encounter details populated for modern games
# ------------------------------------------------------------------
echo "--- Phase 4: Encounter details validation ---"

check_detail() {
    local game="$1" pokemon="$2" expected_field="$3"

    has_field=$($POKEDEX pokemon encounters "$pokemon" --game="$game" 2>/dev/null | python3 -c "
import sys, json
d = json.load(sys.stdin).get('data', [])
for e in d:
    det = e.get('details') or {}
    if det.get('$expected_field'):
        print('yes')
        break
else:
    print('no')
" 2>/dev/null || echo "no")

    if [ "$has_field" = "yes" ]; then
        echo "  $game/$pokemon: has '$expected_field' in details ✓"
        pass
    else
        fail "$game/$pokemon: missing '$expected_field' in encounter details"
    fi
}

check_detail sword abomasnow weather_snow_rate
check_detail scarlet larvitar probability_overall
check_detail legends-arceus pikachu alpha_levels
check_detail brilliant-diamond shinx rate_overall
check_detail legends-za pichu alpha_levels
echo ""

# ------------------------------------------------------------------
# Phase 5: Full national dex encounter sweep
# ------------------------------------------------------------------
echo "--- Phase 5: National dex encounter sweep ---"

WITH_ENCOUNTERS=0
WITHOUT_ENCOUNTERS=0
DETAIL_COVERAGE=0

for name in $SPECIES_LIST; do
    result=$($POKEDEX pokemon encounters "$name" 2>/dev/null || echo '{"data":[]}')
    enc_count=$(echo "$result" | python3 -c "import sys,json; print(len(json.load(sys.stdin).get('data',[])))" 2>/dev/null || echo "0")

    if [ "$enc_count" -gt 0 ]; then
        WITH_ENCOUNTERS=$((WITH_ENCOUNTERS + 1))

        has_details=$(echo "$result" | python3 -c "
import sys, json
d = json.load(sys.stdin).get('data', [])
print('yes' if any(e.get('details') for e in d) else 'no')
" 2>/dev/null || echo "no")
        if [ "$has_details" = "yes" ]; then
            DETAIL_COVERAGE=$((DETAIL_COVERAGE + 1))
        fi
    else
        WITHOUT_ENCOUNTERS=$((WITHOUT_ENCOUNTERS + 1))
    fi
done

echo "  Species with encounters: $WITH_ENCOUNTERS / $TOTAL_SPECIES"
echo "  Species without encounters: $WITHOUT_ENCOUNTERS / $TOTAL_SPECIES"
echo "  Species with encounter details: $DETAIL_COVERAGE / $WITH_ENCOUNTERS"
echo ""

# ------------------------------------------------------------------
# Summary
# ------------------------------------------------------------------
echo "=== Summary ==="
echo "  Passed: $PASS"
echo "  Failed: $FAIL"
echo "  Warnings: $WARN"

if [ $FAIL -gt 0 ]; then
    echo ""
    echo "Failures:"
    echo -e "$ERRORS"
    exit 1
fi

echo ""
echo "All checks passed."
