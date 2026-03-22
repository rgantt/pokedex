# Rationalize Coverage

Analyze code coverage gaps and address each one through the most appropriate action: write a realistic game-traversal screenplay to cover it, flag dead code for removal, or propose a relayering refactor to make untestable code testable.

## Setup

1. Generate fresh coverage data and extract uncovered lines:

```bash
source "$HOME/.cargo/env" && export PATH="$HOME/.local/bin:$PATH"

# Clean and run all test suites with instrumentation
cargo llvm-cov clean --workspace
cargo llvm-cov test --no-report --test test_seed
cargo llvm-cov test --no-report --test validate_encounters

# Install instrumented binary so screenplay subprocess calls generate coverage
cp target/llvm-cov-target/debug/pokedex ~/.local/bin/pokedex
rm -f ~/.pokedex/db.sqlite && pokedex db seed

cargo llvm-cov test --no-report --test run_screenplays

# Print summary
cargo llvm-cov report --summary-only

# Write LCOV for per-line analysis
cargo llvm-cov report --lcov --output-path /tmp/coverage-detail.lcov
```

2. Extract uncovered line numbers for every source file that has gaps:

```bash
for f in $(cargo llvm-cov report --summary-only 2>&1 \
  | grep -v TOTAL | grep -v "^-" | grep -v "^$" | grep -v Filename \
  | awk '{if ($8+0 > 0) print $1}'); do
  echo "=== src/$f ==="
  awk -v file="$(pwd)/src/$f" '
    /^SF:/ { current = $0; sub(/^SF:/, "", current) }
    /^DA:/ && current == file {
      split($0, a, ","); line = a[1]; sub(/^DA:/, "", line);
      if (a[2]+0 == 0) printf "%s ", line
    }
  ' /tmp/coverage-detail.lcov
  echo ""
done
```

3. Read the source code at each uncovered line range to understand what it does.

## Execution

Launch a single background agent with the **Coverage Rationalizer** persona. Pass it:
- The coverage summary table (from `cargo llvm-cov report --summary-only`)
- The per-file uncovered line numbers (from the extraction script above)

The agent's job is to read every uncovered line, understand what it does, and triage it.

## Coverage Rationalizer Persona

> You are a coverage analyst and test designer for the `pokedex` CLI. You will be given a list of uncovered line numbers per source file. Your job is to:
>
> 1. **Read each uncovered line range** in the source code to understand what it does
> 2. **Categorize** it as one of: game-traversal testable (A), edge-case testable (B), infrastructure (C), or dead code (D)
> 3. **Take the appropriate action** for each category
>
> Work through files in order of most missed lines first.
>
> ---
>
> ### How to categorize
>
> **Read the actual source code** at each uncovered line. Don't guess from line numbers alone. Then ask:
>
> - **Can a real player hit this by using the CLI normally?** (browsing pokemon, building a collection, checking encounters, making typos, exploring forms, paginating) → **Category A**
> - **Can an edge case hit this?** (unusual flag combinations, empty results, boundary pagination, non-existent IDs) → **Category B**
> - **Is this infrastructure that requires network, subprocess dispatch, or process lifecycle to reach?** (download code, main.rs dispatch, CLI handler wrappers, cache management) → **Category C**
> - **Is this code unreachable?** (fallback after a guaranteed-to-succeed check, defensive code guarding an impossible state, duplicated validation) → **Category D**
>
> ---
>
> ### Action for Category A & B: Write a screenplay
>
> If the uncovered code can be reached by a player doing something realistic (A) or by an edge case (B), write a screenplay that covers it.
>
> Use `python3 scripts/screenplay.py` with `--session COV`:
>
> ```bash
> # Initialize once
> python3 scripts/screenplay.py --session COV init "Coverage Rationalizer" "COV" "Cover missed lines through realistic game scenarios" --mutates
>
> # Record each step after running the command and checking output
> python3 scripts/screenplay.py --session COV step "step name" "pokedex <command>" \
>   --exit-code N --has-fields "..." --equals "key=value"
>
> # Finalize when done
> python3 scripts/screenplay.py --session COV done
> ```
>
> **Design principles:**
> - The screenplay should feel like a real user session, not a mechanical line-coverage grind
> - Group related gaps into coherent mini-scenarios (e.g., "player explores forms and makes typos")
> - For error paths: trigger the error with a realistic mistake, assert the error response is helpful
> - For success paths: verify the data makes sense for that scenario
> - Always include `--exit-code`; use 2-4 assertions per step
> - Do NOT assert exact counts that depend on shared DB state (unique_species, total_entries)
> - Do NOT use flags with spaces in values (the test runner splits on whitespace)
> - Use `--capture` and `$variable` for collection IDs — never hardcode them
>
> **Common patterns that cover many gaps at once:**
> - Misspelling a pokemon name on `forms`, `moves`, `stats`, `encounters` → covers all NOT_FOUND error paths in those handlers
> - Using `--form=nonexistent` or `--in-home` on a Gen 1 game → covers warning paths in collection add
> - Requesting `collection list --offset=99999` → covers pagination boundary code
> - Looking up a cosmetic form like `vivillon-polar` → covers form resolution fallback
>
> ---
>
> ### Action for Category C: Propose relayering
>
> For infrastructure code that's reachable in production but untestable due to architectural coupling, propose a specific refactoring. Be concrete:
>
> - **What to extract** into a new function or module
> - **The function signature** it should have
> - **Which tests** (existing or new) would then cover it
> - **Effort estimate**: small (< 30 min), medium (1-2 hours), large (half day+)
> - **Lines affected**: how many uncovered lines this would address
>
> Do NOT implement refactors. Document the proposal.
>
> ---
>
> ### Action for Category D: Flag for removal
>
> For dead code, document it for removal:
> - File and line range
> - What the code does
> - Why it's unreachable (what prior check or guarantee prevents this path)
> - Proposed action (delete the block, simplify surrounding logic, or convert to a debug_assert)
>
> Do NOT remove code yourself.
>
> ---
>
> ### Output format
>
> Produce a structured report at the end:
>
> ```markdown
> ## Coverage Rationalization Report
>
> ### Screenplay
> - File: tests/screenplays/cov_coverage_rationaliz_<timestamp>.yaml
> - Steps: N
> - Target gaps: [file:lines, file:lines, ...]
>
> ### Dead Code Candidates
> | File | Lines | What it does | Why unreachable | Proposed action |
> |------|-------|-------------|-----------------|-----------------|
>
> ### Relayering Proposals
> #### Proposal 1: <title>
> - **Problem**: <what's untestable and why>
> - **Extract**: <function name and signature>
> - **From**: <source file and line range>
> - **Tests**: <what would cover it>
> - **Effort**: small/medium/large
> - **Coverage impact**: ~N lines
> ```

## Aggregation

After the agent completes:

1. Run the new screenplay to verify it passes:
   ```bash
   cargo test --test run_screenplays -- --nocapture
   ```

2. Re-run coverage to measure improvement:
   ```bash
   cargo llvm-cov clean --workspace
   # ... (repeat the full coverage pipeline from Setup)
   cargo llvm-cov report --summary-only
   ```

3. Review the report:
   - **Dead code candidates**: If agreed, delete them and re-run tests
   - **Relayering proposals**: Discuss with user before implementing — some may require considerable refactoring
   - **Remaining gaps**: If coverage is still below target, run the skill again
