---
name: audit-screenplays
description: Launch a Pokemon expert to audit screenplay files for game-accuracy and flow quality
argument-hint: [file pattern or persona filter, e.g. "j_red" or "playthroughs" or "all"]
---

# Screenplay Audit

Launch subagents to audit screenplay regression tests for game-accuracy, route logic, and interaction flow quality.

## The Auditor

The auditor is a Pokemon expert — but they don't trust their own memory. Before surfacing ANY factual claim ("X doesn't appear on Route Y", "X is version-exclusive"), they **verify it by running the CLI themselves**. The CLI is the source of truth for what data we serve.

Their workflow for every suspicious claim:
1. Notice something that seems off in the screenplay
2. Run the relevant `pokedex` command to check the actual data
3. Only report the issue if the CLI confirms it's wrong — OR if the CLI returns data that contradicts real game knowledge (which would be a data bug)

**The auditor has access to the `pokedex` CLI** and should use it liberally. A claim without CLI verification is worthless.

## What the auditor checks

For each screenplay step, the auditor considers:
- **Route progression**: Does the game-location order match the actual game? Verify with `pokedex location encounters <slug> --game=<game>`.
- **Pokemon availability**: Is this species actually available at this location in this game? Verify with `pokedex pokemon encounters <name> --game=<game>`.
- **Version exclusivity**: If the screenplay claims a version-exclusive, verify both versions: `--game=red` vs `--game=blue`.
- **Evolution methods**: Does the evolution method match the game era? Verify with `pokedex pokemon evolutions <name>`.
- **Type/BST assertions**: Are hardcoded type or stat values correct? Verify with `pokedex pokemon show <name>` or `pokedex pokemon stats <name>`.
- **Flow quality**: Does the playthrough feel like a real game experience, or is it a random grab-bag?

## Issue Classification

- **critical**: The CLI returns factually wrong data that would mislead users. Example: a Pokemon listed as available in a game where it doesn't exist, wrong types, wrong BST. These are product bugs.
- **wrong**: The screenplay asserts something the CLI doesn't actually return, or the screenplay's flow contradicts game logic in a way that makes the test meaningless.
- **annoying**: Technically works but no real player would do this — odd ordering, premature evolutions, testing irrelevant things.
- **awkward**: Minor flow/naming issues that don't affect correctness.

**Critical and wrong items must be verified against the CLI before reporting.** Annoying and awkward items can be reported from expertise alone.

## Setup

Before launching:
1. Read `data/known_issues.md` to understand tracked data limitations
2. Ensure the `pokedex` binary is installed and a seeded DB exists

## Selection

If `$ARGUMENTS` specifies files (e.g., "just j_red and l_gold"), only audit those.

If `$ARGUMENTS` says "playthroughs", audit all game-specific screenplays (j through v).

If `$ARGUMENTS` says "core", audit the non-game screenplays (a through i).

Otherwise, audit ALL screenplay files in `tests/screenplays/`.

**Group screenplays by game era** to keep agents focused:
- Gen 1-2: j_red, k_lgp, l_gold
- Gen 3-4: m_ruby, n_diamond, s_bdsp
- Gen 5-6: o_black, p_xy
- Gen 7-8: q_sun, r_sword
- Gen 8+ (Legends/9): t_pla, u_scarlet, v_za

**Never launch more than 6 agents.**

$ARGUMENTS

## Agent Prompt Template

Each agent gets this preamble prepended to their assignment:

> You are auditing screenplay regression tests for the `pokedex` CLI. You have deep Pokemon knowledge, but you DO NOT trust your memory — you verify every factual claim by running `pokedex` commands before reporting it.
>
> **CRITICAL RULE**: Before reporting any "critical" or "wrong" issue, you MUST run the relevant `pokedex` command to verify. If the CLI confirms the screenplay is correct, do NOT report it. Your memory may be wrong; the CLI data is what we ship.
>
> For each verified issue, classify it as:
> - **critical**: The CLI serves wrong data (wrong types, impossible encounters, wrong BSTs). These are product bugs.
> - **wrong**: The screenplay asserts something the CLI doesn't return, or the flow is logically broken.
> - **annoying**: Technically works but no real player would do this.
> - **awkward**: Minor flow/naming issues.
>
> For each item: file, step name, severity, the `pokedex` command you ran to verify, and a 1-2 sentence explanation.
>
> At the end: total items by severity, and a quality grade (A/B/C/D/F) per file.
>
> Check `data/known_issues.md` before flagging data problems.

## Aggregation

After ALL agents complete:
1. Collect all findings, grouped by file
2. Deduplicate (same issue found by multiple auditors)
3. Present a prioritized list: critical first, then wrong, annoying, awkward
4. For each item, note whether it's a CLI data bug (fix the code/data) or a screenplay issue (fix the YAML)
