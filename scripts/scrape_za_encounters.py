#!/usr/bin/env python3
"""
Scrape Pokémon Legends: Z-A wild zone encounter data from Serebii.
Downloads 20 pages with respectful delays, parses HTML, outputs JSON
matching the pokedex CLI encounter schema.

Usage:
    python3 scripts/scrape_za_encounters.py > data/za_encounters.json
"""

import html
import json
import re
import sys
import time
import unicodedata
import urllib.request

BASE_URL = "https://www.serebii.net/pokearth/lumiosecity/wildzone{}.shtml"
ZONES = range(1, 21)
DELAY_SECONDS = 3  # polite delay between requests
USER_AGENT = (
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
)
VERSION = "legends-za"
GAME_DISPLAY = "Legends: Z-A"


def normalize_name(name: str) -> str:
    """Normalize a Pokémon display name to a PokeAPI-style identifier.

    'Flabébé' -> 'flabebe', 'Mr. Mime' -> 'mr-mime', 'Nidoran♀' -> 'nidoran-f'
    """
    name = html.unescape(name)
    # Handle special characters
    name = name.replace("♀", "-f").replace("♂", "-m")
    name = name.replace("'", "").replace("'", "").replace("\u2019", "")
    name = name.replace(". ", "-").replace(".", "")
    name = name.replace(" ", "-")
    # Strip diacritics (é -> e, etc.)
    nfkd = unicodedata.normalize("NFKD", name)
    name = "".join(c for c in nfkd if not unicodedata.combining(c))
    return name.lower()


def fetch_page(zone_num: int) -> str:
    url = BASE_URL.format(zone_num)
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req) as resp:
        return resp.read().decode("utf-8", errors="replace")


def parse_zone(page_html: str, zone_num: int) -> list[dict]:
    """Parse encounter data from a wild zone page's HTML."""
    encounters = []

    # Find the encounter table (class="extradextable" containing "Wild Pokémon")
    # The structure is: header table, then data table with columns per Pokémon
    # Row pattern repeats in groups:
    #   1. sprites: <img ... alt="PokemonName">
    #   2. names: <a>Name</a>, may have Alpha icon
    #   3. types: type images
    #   4. levels: <b>Level</b><br/>X - Y
    #   5. alpha: <b>Alpha Chance</b><br/>N%<br/>Level: X - Y

    # Extract the encounter table content
    # Look for the table after "Wild Pokémon"
    marker = 'Wild Pok\xe9mon'
    if marker not in page_html:
        marker = 'Wild Pokémon'
    if marker not in page_html:
        marker = 'Wild Pok&eacute;mon'

    idx = page_html.find(marker)
    if idx == -1:
        return encounters

    # Find the data table after the header
    table_start = page_html.find('<table class="extradextable"', idx + len(marker))
    if table_start == -1:
        return encounters

    table_end = page_html.find('</table>', table_start)
    if table_end == -1:
        return encounters

    table_html = page_html[table_start:table_end + len('</table>')]

    # Extract Pokémon data by parsing the columnar structure
    # Each Pokémon appears as a column across multiple rows

    # Step 1: Extract all sprite alt texts (Pokémon names) with alpha markers
    # Sprites row: <img ... class="wildsprite" alt="Name">
    sprite_pattern = re.compile(
        r'class="wildsprite"\s+alt="([^"]+)"', re.IGNORECASE
    )
    sprite_names = sprite_pattern.findall(table_html)

    # Step 2: Check for alpha markers in the name row
    # Alpha Pokémon have: <img src="...alphaza.png" alt="Alpha"...>
    # We need to figure out which columns are alpha-only spawns
    name_cells = re.findall(
        r'<td[^>]*class="name"[^>]*>(.*?)</td>', table_html, re.DOTALL
    )
    is_alpha_spawn = []
    for cell in name_cells:
        is_alpha_spawn.append('alphaza.png' in cell or 'alt="Alpha"' in cell)

    # Step 3: Extract level ranges
    # Pattern: <b>Level</b><br />\nX - Y\n or <b>Level</b><br/>X - Y
    level_pattern = re.compile(
        r'<b>Level</b>\s*<br\s*/?>\s*(\d+\s*-\s*\d+|\d+)', re.IGNORECASE
    )
    levels = level_pattern.findall(table_html)

    # Step 4: Extract alpha chance info
    # Pattern: <b>Alpha Chance</b><br />N%<br />Level: X - Y
    alpha_pattern = re.compile(
        r'<b>Alpha Chance</b>\s*<br\s*/?>\s*(\d+)%'
        r'(?:\s*<br\s*/?>\s*Level:\s*(\d+\s*-\s*\d+))?',
        re.IGNORECASE
    )
    alpha_data = alpha_pattern.findall(table_html)

    zone_name = f"Wild Zone {zone_num}"

    # The sprite_names list has one entry per column.
    # levels and alpha_data should align 1:1 with sprite_names.
    num_pokemon = len(sprite_names)

    for i in range(num_pokemon):
        name = html.unescape(sprite_names[i])
        is_alpha = is_alpha_spawn[i] if i < len(is_alpha_spawn) else False

        # Parse level range
        if i < len(levels):
            level_str = levels[i].strip()
            if '-' in level_str:
                parts = level_str.split('-')
                min_level = int(parts[0].strip())
                max_level = int(parts[1].strip())
            else:
                min_level = max_level = int(level_str)
        else:
            min_level = max_level = 0

        # Parse alpha data
        alpha_chance = None
        alpha_levels = None
        if i < len(alpha_data):
            chance_str, alpha_level_str = alpha_data[i]
            alpha_chance = f"{chance_str}%"
            if alpha_level_str:
                alpha_levels = alpha_level_str.strip()

        encounter = {
            "pokemon_name": normalize_name(name),
            "location": zone_name,
            "area": f"wild-zone-{zone_num}",
            "game": GAME_DISPLAY,
            "version": VERSION,
            "method": "symbol-encounter",
            "min_level": min_level,
            "max_level": max_level,
            "is_alpha_spawn": is_alpha,
        }

        if alpha_chance:
            encounter["alpha_chance"] = alpha_chance
        if alpha_levels:
            # D8: Normalize alpha_levels delimiter — periods and inconsistent
            # separators become ", " (e.g. "32 - 35. 56 - 59" -> "32 - 35, 56 - 59")
            alpha_levels = re.sub(r'\.\s*', ', ', alpha_levels)
            # Also normalize any other comma variants (e.g. "," without space)
            alpha_levels = re.sub(r',\s*', ', ', alpha_levels)
            encounter["alpha_levels"] = alpha_levels

        encounters.append(encounter)

    # D4: Deduplicate encounters by (pokemon_name, area, min_level, max_level)
    seen = set()
    deduped = []
    for enc in encounters:
        key = (enc["pokemon_name"], enc["area"], enc["min_level"], enc["max_level"])
        if key not in seen:
            seen.add(key)
            deduped.append(enc)
    encounters = deduped

    return encounters


def main():
    all_encounters = []

    for zone_num in ZONES:
        sys.stderr.write(f"Fetching Wild Zone {zone_num}...")
        sys.stderr.flush()

        page = fetch_page(zone_num)
        encounters = parse_zone(page, zone_num)
        all_encounters.extend(encounters)
        sys.stderr.write(f" {len(encounters)} pokémon\n")

        if zone_num < max(ZONES):
            time.sleep(DELAY_SECONDS)

    sys.stderr.write(f"\nTotal: {len(all_encounters)} encounter entries\n")

    # Output JSON
    print(json.dumps(all_encounters, indent=2))


if __name__ == "__main__":
    main()
