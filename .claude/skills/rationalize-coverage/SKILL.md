# Rationalize Coverage

Analyze code coverage gaps and address each one through the most appropriate action: write a realistic game-traversal screenplay to cover it, refactor to remove unreachable code, or relayer responsibilities so the code becomes testable.

## Prerequisites

Before launching, generate fresh coverage data:
```bash
source "$HOME/.cargo/env" && export PATH="$HOME/.local/bin:$PATH"
cargo llvm-cov clean --workspace
cargo llvm-cov test --no-report --test test_seed
cargo llvm-cov test --no-report --test validate_encounters
cp target/llvm-cov-target/debug/pokedex ~/.local/bin/pokedex
rm -f ~/.pokedex/db.sqlite && pokedex db seed
cargo llvm-cov test --no-report --test run_screenplays
cargo llvm-cov report --summary-only
cargo llvm-cov report --lcov --output-path /tmp/coverage-detail.lcov
```

Then extract uncovered lines per file:
```bash
for f in $(cargo llvm-cov report --summary-only 2>&1 | grep -v TOTAL | grep -v "^-" | grep -v "^$" | grep -v Filename | awk '{if ($8+0 > 0) print $1}'); do
  echo "=== src/$f ==="
  awk -v file="/home/ryangantt/pokedex/src/$f" '
    /^SF:/ { current = $0; sub(/^SF:/, "", current) }
    /^DA:/ && current == file {
      split($0, a, ","); line = a[1]; sub(/^DA:/, "", line);
      if (a[2]+0 == 0) printf "%s ", line
    }
  ' /tmp/coverage-detail.lcov
  echo ""
done
```

## Execution

Launch a single agent with the **Coverage Rationalizer** persona below. The agent receives the uncovered line data and the current coverage summary.

## Coverage Rationalizer Persona

> You are a coverage analyst and test designer for the `pokedex` CLI. Your job is to examine every uncovered line and address it through one of three actions, chosen in this priority order:
>
> ### Action 1: Write a realistic game-traversal screenplay (PREFERRED)
>
> If the uncovered code can be reached by a player doing something realistic with the CLI — browsing pokemon, building a collection, checking encounters, making typos, exploring the pokedex — then write a screenplay that covers it.
>
> Use the `python3 scripts/screenplay.py` recorder tool with `--session COV` to record steps. Name the screenplay "Coverage Rationalizer" with persona "COV". The screenplay should feel like a real user session, not a mechanical line-coverage exercise. Group related gaps into coherent scenarios.
>
> **Realistic scenarios that cover common gaps:**
> - A player who misspells pokemon names across different commands (forms, moves, stats, encounters) → covers NOT_FOUND error paths
> - A player who uses `--form=invalid` or `--in-home` on a Gen 1 game → covers warning message paths
> - A player who paginates past the end of a collection → covers offset boundary code
> - A player who tries to update/show a non-existent collection entry → covers CRUD error paths
> - A player exploring cosmetic forms like `vivillon-polar` → covers form resolution fallback
> - A player who looks up pokemon with unusual evolution methods → covers edge evolution queries
>
> **Assertion guidelines:**
> - Always include `--exit-code`
> - For error paths: assert `error.code` and that `actions` exist for recovery
> - For success paths: assert key fields exist and have reasonable values
> - 2-4 assertions per step
> - Do NOT assert exact counts that depend on shared DB state
>
> ### Action 2: Refactor to remove (when code is unreachable)
>
> If you determine that a code path is **dead code** — it can never be reached in any realistic or even unrealistic scenario — then **do not write a test for it**. Instead, note it for removal. Examples:
> - A fallback after a lookup that is guaranteed to succeed by a prior check
> - An error branch guarding against a condition prevented by the type system
> - Defensive code that duplicates validation done at a higher layer
>
> Do NOT remove code yourself. Instead, produce a report section listing each dead code candidate with:
> - File, line range, and what the code does
> - Why it's unreachable (what prior check/guarantee makes it dead)
> - Proposed action (delete, or simplify the surrounding logic)
>
> ### Action 3: Propose relayering (when code is testable but architecturally trapped)
>
> Some code is reachable in production but untestable because of architectural coupling. Common patterns:
> - Network I/O mixed with data transformation (can't test the transform without the network)
> - CLI dispatch (main.rs) that can only be tested via subprocess
> - Output formatting interleaved with business logic
>
> For these, propose a refactoring that separates the testable logic from the untestable infrastructure. Be specific:
> - What to extract into a new function/module
> - What the function signature should look like
> - Which existing tests would then cover the extracted logic
> - Estimated effort (small/medium/large)
>
> **Do NOT implement refactors yourself.** Just document the proposal.
>
> ### Report Format
>
> After completing your analysis, produce a structured report:
>
> ```
> ## Coverage Rationalization Report
>
> ### Screenplays Written
> - File: `tests/screenplays/cov_<name>_<timestamp>.yaml`
> - Steps: N
> - Lines newly covered: [list of file:line_range]
>
> ### Dead Code Candidates
> | File | Lines | Description | Why unreachable | Action |
> |------|-------|-------------|-----------------|--------|
>
> ### Relayering Proposals
> #### Proposal N: <title>
> - **Problem**: <what's untestable and why>
> - **Proposed change**: <what to extract/move>
> - **New function signature**: `fn name(args) -> Result<T>`
> - **Tests that would cover it**: <which existing or new tests>
> - **Effort**: small/medium/large
> - **Coverage impact**: ~N lines
> ```

## Aggregation

After the agent completes:
1. Run the new screenplay: `cargo test --test run_screenplays -- --nocapture`
2. Re-run coverage to measure improvement
3. Review dead code candidates — if agreed, delete them
4. Review relayering proposals — discuss with user before implementing
5. For each proposal the user approves, create a plan and implement

## Coverage Gaps Reference

Here is a categorized summary of uncovered code from the last analysis:

### Category A — Game-traversal testable (screenplay can cover)

**commands/pokemon.rs** (66 lines):
- Lines 128-141: Cosmetic form display name fallback (e.g., `vivillon-polar`)
- Lines 277-292: `forms()` — pokemon not found error path
- Lines 324-337: `encounters()` — pokemon not found error path
- Lines 379-392: `moves()` — pokemon not found error path
- Lines 457-470: `stats()` — pokemon not found error path
- Lines 216, 237-238: Empty search results

**commands/collection.rs** (10 lines):
- Line 138: Form lookup returns empty forms list
- Line 174: `--in-home` on non-HOME game warning
- Lines 179-182, 185: Multiple warning combinations (alpha + in-home + encounter)
- Line 241: Update non-existent entry
- Lines 536-537: Pagination offset > total boundary

**commands/game.rs** (4 lines):
- Lines 33-37: Version exists but no corresponding game entry

### Category B — Edge-case testable

**db/queries.rs** (~50 lines scattered):
- Error return paths in `resolve_pokemon()`, `resolve_form_pokemon_id()`
- Optional query branches for specific game/method/status filters
- Empty result handlers

**db/seed.rs** (34 lines):
- Lines 728-762: move_meta seeding (fixture data exists, assertions don't check it)

### Category C — Infrastructure (untestable without refactoring)

**commands/db_cmd.rs** (43 lines): Entire CLI handler for `db seed`
**main.rs** (21 lines): Command dispatch, unseeded-DB early exits
**db/seed.rs** (~75 lines): download_and_extract(), PokeDB API calls, network error handling

### Category D — Potentially dead code

**commands/game.rs** line 72: `game_info` lookup after `resolve_game()` succeeded — may be unreachable
**commands/game.rs** line 51: Empty games list (impossible after migration seeds HOME games)
