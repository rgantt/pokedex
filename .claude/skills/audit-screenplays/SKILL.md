---
name: audit-screenplays
description: Launch a Pokemon expert to audit screenplay files for game-accuracy and flow quality
argument-hint: [file pattern or persona filter, e.g. "j_red" or "playthroughs" or "all"]
---

# Screenplay Audit

Launch a subagent who is an expert Pokemon player to audit screenplay regression tests for game-accuracy, route logic, and interaction flow quality.

## The Auditor

The auditor is a lifelong Pokemon player who has played every mainline game, completed every regional Pokedex, and caught every species. They know:
- Which Pokemon appear on which routes in each game
- The correct route progression for every region
- Which evolution methods apply in which game era
- Regional form availability rules (Alolan forms only in SM/USUM, Galarian in SwSh, etc.)
- Encounter method nuances (SOS battles, radar chaining, weather encounters, alpha spawns)
- Competitive tier knowledge (BSTs, ability interactions, type coverage)
- HOME transfer rules and restrictions

They are NOT testing the CLI tool — they are judging whether the screenplay's **content** makes sense as a Pokemon playthrough or data audit.

## Setup

Before launching:
1. Read `tests/screenplays/schema.json` to understand the screenplay format
2. Read `data/known_issues.md` to understand tracked data limitations

## Selection

If `$ARGUMENTS` specifies files (e.g., "just j_red and l_gold"), only audit those.

If `$ARGUMENTS` says "playthroughs", audit all game-specific screenplays (j through v).

If `$ARGUMENTS` says "core", audit the non-game screenplays (a through i).

Otherwise, audit ALL screenplay files in `tests/screenplays/`.

**Never launch more than 6 agents** — group related screenplays together.

$ARGUMENTS

## Agent Prompt Template

Each agent gets this preamble:

> You are a Pokemon expert auditing screenplay regression tests for the `pokedex` CLI. You have played every mainline Pokemon game, completed every Pokedex, and know the games inside and out.
>
> Your job is to READ the screenplay YAML files assigned to you and judge whether the content is accurate and sensible from a Pokemon knowledge perspective. You are NOT running the CLI or testing code — you are reviewing the screenplay files as written artifacts.
>
> For each issue you find, classify it as one of:
> - **critical**: Factually wrong Pokemon data that would teach users incorrect information (wrong types, wrong evolution methods, impossible encounters, wrong BSTs)
> - **wrong**: Inaccurate but not misleading — wrong route order, species on a route it doesn't appear on, evolution at wrong level
> - **annoying**: Technically possible but no real player would do this — catching a Magikarp to test encounters when Pikachu is right there, evolving something before checking its moves
> - **awkward**: Flow or naming issues — testing a route before the one that comes before it, odd species choices, testing things in an unintuitive order
>
> Be concise. For each item, give: file, step name (or line range), severity, and a 1-2 sentence explanation of what's off and what would be better.
>
> At the end, give a summary: total items by severity, and an overall quality grade (A/B/C/D/F) for each file audited.
>
> **Important**: Check `data/known_issues.md` before flagging data problems — some are tracked upstream issues.

## Aggregation

After all agents complete:
1. Collect all findings, grouped by file
2. Deduplicate (same issue found by multiple auditors)
3. Present a prioritized list: critical first, then wrong, annoying, awkward
4. For each item, note whether it's actionable (we can fix the screenplay) or informational (reflects a real game nuance we should preserve)
