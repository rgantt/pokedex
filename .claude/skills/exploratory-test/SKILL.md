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
2. Launch ALL personas as background agents in a SINGLE message (parallel)
3. Each agent should use the `pokedex` CLI directly (the installed binary, not cargo run)
4. Each agent should test 10-20 commands and report ALL issues found — errors, wrong data, missing fields, confusing output, broken HATEOAS links, exit code problems
5. If `$ARGUMENTS` contains a persona filter (e.g., "just D and E"), only launch those personas

$ARGUMENTS

## Agent Prompt Template

Each agent gets this preamble prepended to its persona description:

> You are testing the `pokedex` CLI tool. Run commands, check outputs carefully, and report ALL issues you find. For every error response, check: exit code (should be 1), valid JSON, at least one recovery action. For every success response, verify: data accuracy, HATEOAS actions present, no malformed JSON. Be thorough.

## Aggregation

After ALL agents complete:
1. Categorize every issue by severity (Critical / High / Medium / Low)
2. Deduplicate — same issue found by multiple testers counts once
3. Note which issues are NEW vs previously seen in earlier rounds
4. List what's VERIFIED WORKING across all testers
5. Provide a concrete fix plan for each issue

## Convergence Tracking

Compare against previous rounds. The goal is convergence toward zero bugs:
- Round 1: 24 issues
- Round 2: 13 issues
- Round 3: 14 issues
- This round: ?
