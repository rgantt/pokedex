---
name: exploratory-test
description: Spin up parallel agents to exploratory test the pokedex CLI from different player personas
argument-hint: [round-number or persona filter]
---

# Pokedex Exploratory Test

Run a comprehensive exploratory test of the `pokedex` CLI by launching parallel agent testers. Each tester has a different persona and focus area defined in `personas.md` (adjacent to this file).

## Setup

Before launching testers:
1. Ensure the release binary is installed: `./install.sh`
2. Seed a fresh database: `rm -f ~/.pokedex/db.sqlite && pokedex db seed`

## Execution

1. Read `.claude/skills/exploratory-test/personas.md` to get the persona definitions
2. Select which personas to launch (see Selection Rules below)
3. Launch selected personas as background agents in a SINGLE message (parallel)
4. Each agent should use the `pokedex` CLI directly (the installed binary, not cargo run)
5. Each agent should test 10-20 commands and report ALL issues found
6. **Each agent MUST produce a screenplay YAML file** (see Screenplay Recording below)

## Selection Rules

If `$ARGUMENTS` contains a persona filter (e.g., "just D and E"), only launch those.

Otherwise, launch a balanced set of ~9-12 agents covering:

**Always include (core testers):**
- A (HATEOAS), D (edge cases), G (forms/alpha) — these catch structural bugs

**Always include at least 2 game-era playthroughs spanning early and late gens:**
- Pick 1 from early gens: J (Red/Blue Gen 1), L (Gold/Silver Gen 2), M (Ruby/Sapphire Gen 3), N (Diamond/Pearl Gen 4)
- Pick 1 from late gens: R (Sword/Shield Gen 8), U (Scarlet/Violet Gen 9), T (Legends Arceus), V (Legends Z-A)
- Rotate which specific games are tested each round to maximize coverage over time

**Fill remaining slots from:** B (cross-game), C (data quality), E (competitive), F (living dex), H (HOME transfers), I (variants), and any remaining playthrough personas

**Never launch more than 12 agents** — diminishing returns on concurrent DB writes and context aggregation

$ARGUMENTS

## Agent Prompt Template

Each agent gets this preamble prepended to its persona description:

> You are testing the `pokedex` CLI tool. Run commands, check outputs carefully, and report ALL issues you find. For every error response, check: exit code (should be 1), valid JSON, at least one recovery action. For every success response, verify: data accuracy, HATEOAS actions present, no malformed JSON. Be thorough.
>
> **SCREENPLAY RECORDING**: As you test, build a YAML screenplay of every command you run and the key assertions you check. At the END of your run, write this screenplay to `tests/screenplays/<persona_letter>_<persona_name>.yaml`. Use the schema documented below. Record 2-4 assertions per step — focus on INTENT (what should be true) not exact output matching.

## Screenplay Recording Schema

Each agent writes a YAML file at `tests/screenplays/`. The authoritative schema is at `tests/screenplays/schema.json` (JSON Schema). The runner enforces strict schema validation — **any unknown field causes a parse error, not a silent ignore.** Here is the format:

```yaml
name: "Persona Name"
persona: "X"
description: "What this persona tests"
needs_seed: true
mutates_collection: true  # set true if any step does collection add/update/remove

steps:
  - name: "human-readable step description"
    command: "pokedex pokemon show pikachu"
    assert:
      exit_code: 0                                    # exact match (always include)
      has_fields: ["data.types", "data.stats"]        # dot-path must exist and be non-null
      equals: {"data.name": "pikachu"}                # exact value match at dot-path
      contains: {"data.display_name": "Pikachu"}      # substring match
      array_len: {"data.types": {"min": 1, "max": 3}} # bounds check on arrays
      type_of: {"data.id": "number"}                  # JSON type check
    capture: {"entry_id": "data.id"}                  # capture value for later $entry_id substitution

  - name: "use captured variable"
    command: "pokedex collection show $entry_id"
    assert:
      exit_code: 0
```

### Supported assertion types (ONLY these 6 are supported):

1. `exit_code: N` — exact exit code match (integer)
2. `has_fields: ["dot.path"]` — field exists at dot-path and is non-null
3. `equals: {"dot.path": value}` — exact value match at dot-path (value can be string, number, bool, null, array)
4. `contains: {"dot.path": "substring"}` — substring match on value at dot-path (works on strings; for non-strings, matches against JSON serialization)
5. `array_len: {"dot.path": {"min": N, "max": M}}` — array length bounds check (min and max are optional)
6. `type_of: {"dot.path": "string|number|boolean|array|object"}` — JSON type check

**Do NOT use any assertion types not listed above.** The runner's `_extra` catch-all will silently ignore unknown keys like `json_path_exists`, `any_element`, `all_elements`, `all_match`, `has_action`, `action_contains`, `not_contains_key`, `json_path_any`, `json_path_count`, `contains_any`, etc. These assertions will NOT be evaluated, creating false confidence that a test passes.

### Assertion guidelines:
- **Always** include `exit_code`
- Use `has_fields` for structural checks (field exists)
- Use `equals` sparingly — only for stable values (species name, type, generation number)
- Use `contains` for display names that might change format; also useful for checking if an array contains an item (e.g., `contains: {"data": "Pikachu"}` serializes the array to JSON and checks for the substring)
- Use `array_len` with ranges, not exact counts (data may grow)
- Use `capture` + `$variable` for collection IDs that are auto-generated — NEVER use hardcoded IDs for collection updates
- Do NOT assert exact JSON output — that's fragile
- Do NOT assert exact `unique_species` or `total_entries` counts — these depend on test run order since screenplays share the same database
- Do NOT use `--notes` or other flags with spaces in values — the test runner splits commands on whitespace without shell-style quoting
- 2-4 assertions per step is ideal

## Known Data Issues

Before reporting issues, check `data/known_issues.md` for already-tracked problems. Data quality issues ARE product issues — when a new one is found:
1. Determine if it's fixable (via override, seed logic, or query change) or truly upstream
2. If fixable, fix it and mark FIXED in known_issues.md
3. If upstream and unfixable, add it as WONTFIX with explanation
4. Never dismiss data issues as "not our problem" — we serve this data

## Aggregation

After ALL agents complete:
1. Categorize every issue by severity (Critical / High / Medium / Low)
2. Deduplicate — check `data/known_issues.md` before reporting
3. Note which issues are NEW vs previously seen in earlier rounds
4. List what's VERIFIED WORKING across all testers
5. For each issue: fix it, add to known_issues.md, or explain why it's unfixable
6. **Collect all screenplay YAML files from agents and commit them to `tests/screenplays/`**
7. Run `cargo test --test run_screenplays` to verify all screenplays pass

## Convergence Tracking

Compare against previous rounds. The goal is convergence toward zero bugs:
- Round 1: 24 issues
- Round 2: 13 issues
- Round 3: 14 issues
- Round 4: 17 issues
- Round 5: 22 issues
- Round 6: ~15 issues
- Round 7: 12 issues
- Round 8: 7 issues
- Round 9: 8 issues (6 fixed, 2 skipped as not actionable)
- Round 10: 0 new bugs
- Round 11: 7 issues (all fixed)
- Round 12: 10 issues (5 fixed, 5 upstream WONTFIX)
- This round: ?
