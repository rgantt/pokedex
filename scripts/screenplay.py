#!/usr/bin/env python3
"""Screenplay recorder for pokedex exploratory tests.

Agents call this tool to record screenplay steps instead of hand-writing YAML.
The tool structurally guarantees correct output format.

Usage:
    screenplay.py init <name> <persona> <description> [--mutates] [--session ID]
    screenplay.py step <step-name> <command> [assertion flags...] [--session ID]
    screenplay.py done [--session ID]
    screenplay.py reset [--session ID]
"""

import argparse
import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path

# PyYAML may not be installed — use a simple YAML emitter instead
SCREENPLAYS_DIR = Path(__file__).parent.parent / "tests" / "screenplays"

VALID_TYPES = {"string", "number", "boolean", "array", "object", "null"}
VALID_ASSERT_KEYS = {"exit_code", "has_fields", "equals", "contains", "array_len", "type_of"}


def wip_path(session: str) -> Path:
    return SCREENPLAYS_DIR / f".screenplay.{session}.wip"


def parse_value(s: str):
    """Auto-type a string value: int, float, bool, JSON array/object, or string."""
    if s.lower() == "true":
        return True
    if s.lower() == "false":
        return False
    if s.lower() == "null":
        return None
    try:
        return int(s)
    except ValueError:
        pass
    try:
        return float(s)
    except ValueError:
        pass
    if s.startswith("[") or s.startswith("{"):
        try:
            return json.loads(s)
        except json.JSONDecodeError:
            pass
    return s


def parse_kv_pairs(pairs: list[str]) -> dict:
    """Parse KEY=VALUE pairs into a dict with auto-typed values."""
    result = {}
    for pair in pairs:
        if "=" not in pair:
            print(f"Error: invalid KEY=VALUE pair: {pair}", file=sys.stderr)
            sys.exit(1)
        key, val = pair.split("=", 1)
        result[key] = parse_value(val)
    return result


def parse_kv_strings(pairs: list[str]) -> dict:
    """Parse KEY=VALUE pairs into a dict with string values."""
    result = {}
    for pair in pairs:
        if "=" not in pair:
            print(f"Error: invalid KEY=VALUE pair: {pair}", file=sys.stderr)
            sys.exit(1)
        key, val = pair.split("=", 1)
        result[key] = val
    return result


def parse_array_len(specs: list[str]) -> dict:
    """Parse PATH:MIN:MAX specs into array_len format."""
    result = {}
    for spec in specs:
        parts = spec.split(":")
        if len(parts) < 2:
            print(f"Error: array-len format is PATH:MIN[:MAX], got: {spec}", file=sys.stderr)
            sys.exit(1)
        path = parts[0]
        bounds = {}
        if len(parts) >= 2 and parts[1]:
            bounds["min"] = int(parts[1])
        if len(parts) >= 3 and parts[2]:
            bounds["max"] = int(parts[2])
        result[path] = bounds
    return result


def emit_yaml_value(val, indent=0):
    """Emit a YAML value as a string."""
    if val is None:
        return "null"
    if isinstance(val, bool):
        return "true" if val else "false"
    if isinstance(val, int):
        return str(val)
    if isinstance(val, float):
        return str(val)
    if isinstance(val, str):
        # Quote strings that could be misinterpreted
        if val in ("true", "false", "null", "yes", "no", "on", "off", ""):
            return f'"{val}"'
        if val.isdigit() or (val.startswith("-") and val[1:].isdigit()):
            return f'"{val}"'
        # Quote strings with special chars
        for ch in ":{}\n[]&*?|>!%@`,'\"":
            if ch in val:
                return json.dumps(val)  # JSON quoting is YAML-safe
        return val
    if isinstance(val, list):
        items = ", ".join(emit_yaml_value(v) for v in val)
        return f"[{items}]"
    if isinstance(val, dict):
        items = ", ".join(f"{emit_yaml_value(k)}: {emit_yaml_value(v)}" for k, v in val.items())
        return "{" + items + "}"
    return str(val)


def emit_screenplay(data: dict) -> str:
    """Emit a complete screenplay as YAML string."""
    lines = []
    lines.append(f'name: {emit_yaml_value(data["name"])}')
    lines.append(f'persona: {emit_yaml_value(data["persona"])}')
    lines.append(f'description: {emit_yaml_value(data["description"])}')
    lines.append(f'needs_seed: {emit_yaml_value(data.get("needs_seed", True))}')
    lines.append(f'mutates_collection: {emit_yaml_value(data.get("mutates_collection", False))}')
    lines.append("")
    lines.append("steps:")

    for step in data.get("steps", []):
        lines.append(f'  - name: {emit_yaml_value(step["name"])}')
        lines.append(f'    command: {emit_yaml_value(step["command"])}')

        if "capture" in step:
            lines.append(f"    capture: {emit_yaml_value(step['capture'])}")

        lines.append("    assert:")
        assert_block = step["assert"]
        if "exit_code" in assert_block:
            lines.append(f"      exit_code: {assert_block['exit_code']}")
        if "has_fields" in assert_block:
            lines.append(f"      has_fields: {emit_yaml_value(assert_block['has_fields'])}")
        if "equals" in assert_block:
            lines.append(f"      equals: {emit_yaml_value(assert_block['equals'])}")
        if "contains" in assert_block:
            lines.append(f"      contains: {emit_yaml_value(assert_block['contains'])}")
        if "array_len" in assert_block:
            lines.append(f"      array_len: {emit_yaml_value(assert_block['array_len'])}")
        if "type_of" in assert_block:
            lines.append(f"      type_of: {emit_yaml_value(assert_block['type_of'])}")

        lines.append("")

    return "\n".join(lines)


def load_wip(session: str) -> dict:
    path = wip_path(session)
    if not path.exists():
        print(f"Error: no active screenplay session '{session}'. Run 'init' first.", file=sys.stderr)
        sys.exit(1)
    return json.loads(path.read_text())


def save_wip(session: str, data: dict):
    wip_path(session).write_text(json.dumps(data))


def cmd_init(args):
    path = wip_path(args.session)
    if path.exists():
        print(f"Error: session '{args.session}' already active. Run 'done' or 'reset' first.", file=sys.stderr)
        sys.exit(1)

    data = {
        "name": args.name,
        "persona": args.persona,
        "description": args.description,
        "needs_seed": True,
        "mutates_collection": args.mutates,
        "steps": [],
    }
    save_wip(args.session, data)
    print(f"Initialized screenplay: {args.name} (session={args.session})")


def cmd_step(args):
    data = load_wip(args.session)

    if not args.command.startswith("pokedex ") and args.command != "pokedex":
        print(f"Error: command must start with 'pokedex', got: {args.command}", file=sys.stderr)
        sys.exit(1)

    assert_block = {}

    if args.exit_code is not None:
        assert_block["exit_code"] = args.exit_code

    if args.has_fields:
        assert_block["has_fields"] = [f.strip() for f in args.has_fields.split(",")]

    if args.equals:
        assert_block["equals"] = parse_kv_pairs(args.equals)

    if args.contains:
        assert_block["contains"] = parse_kv_strings(args.contains)

    if args.array_len:
        assert_block["array_len"] = parse_array_len(args.array_len)

    if args.type_of:
        for pair in args.type_of:
            if "=" not in pair:
                print(f"Error: --type-of format is PATH=TYPE, got: {pair}", file=sys.stderr)
                sys.exit(1)
            path, typ = pair.split("=", 1)
            if typ not in VALID_TYPES:
                print(f"Error: invalid type '{typ}'. Valid: {', '.join(sorted(VALID_TYPES))}", file=sys.stderr)
                sys.exit(1)
        assert_block["type_of"] = parse_kv_strings(args.type_of)

    step = {
        "name": args.step_name,
        "command": args.command,
        "assert": assert_block,
    }

    if args.capture:
        step["capture"] = parse_kv_strings(args.capture)

    data["steps"].append(step)
    save_wip(args.session, data)
    step_num = len(data["steps"])
    print(f"Step {step_num}: {args.step_name}")


def cmd_done(args):
    data = load_wip(args.session)

    # Validate
    errors = []
    if not data.get("steps"):
        errors.append("No steps recorded")
    for i, step in enumerate(data.get("steps", [])):
        if "exit_code" not in step.get("assert", {}):
            errors.append(f"Step {i+1} ({step.get('name', '?')}): missing exit_code")
        for key in step.get("assert", {}):
            if key not in VALID_ASSERT_KEYS:
                errors.append(f"Step {i+1}: unknown assertion key '{key}'")

    if errors:
        print("Validation errors:", file=sys.stderr)
        for e in errors:
            print(f"  - {e}", file=sys.stderr)
        sys.exit(1)

    # Generate filename
    persona = data.get("persona", "x").lower()
    slug = data.get("name", "test").lower().replace(" ", "_").replace("-", "_")[:20]
    ts = datetime.now(timezone.utc).strftime("%Y%m%d_%H%M%S")
    filename = f"{persona}_{slug}_{ts}.yaml"
    out_path = SCREENPLAYS_DIR / filename

    # Write
    yaml_content = emit_screenplay(data)
    out_path.write_text(yaml_content)

    # Cleanup
    wip_path(args.session).unlink()
    print(f"Wrote {out_path}")
    print(f"Steps: {len(data['steps'])}")


def cmd_reset(args):
    path = wip_path(args.session)
    if path.exists():
        path.unlink()
        print(f"Reset session '{args.session}'")
    else:
        print(f"No active session '{args.session}'")


def main():
    parser = argparse.ArgumentParser(description="Screenplay recorder for pokedex tests")
    parser.add_argument("--session", default="default", help="Session ID for concurrent agents")
    sub = parser.add_subparsers(dest="cmd")

    # init
    p_init = sub.add_parser("init", help="Start a new screenplay")
    p_init.add_argument("name", help="Screenplay name")
    p_init.add_argument("persona", help="Persona letter")
    p_init.add_argument("description", help="Description")
    p_init.add_argument("--mutates", action="store_true", help="Sets mutates_collection")

    # step
    p_step = sub.add_parser("step", help="Record a step")
    p_step.add_argument("step_name", help="Human-readable step name")
    p_step.add_argument("command", help="The pokedex command to record")
    p_step.add_argument("--exit-code", type=int, default=0, dest="exit_code")
    p_step.add_argument("--has-fields", dest="has_fields", help="Comma-separated dot-paths")
    p_step.add_argument("--equals", nargs="*", help="PATH=VALUE pairs")
    p_step.add_argument("--contains", nargs="*", help="PATH=SUBSTRING pairs")
    p_step.add_argument("--array-len", nargs="*", dest="array_len", help="PATH:MIN[:MAX] specs")
    p_step.add_argument("--type-of", nargs="*", dest="type_of", help="PATH=TYPE pairs")
    p_step.add_argument("--capture", nargs="*", help="KEY=PATH pairs for variable capture")

    # done
    sub.add_parser("done", help="Finalize and write the screenplay")

    # reset
    sub.add_parser("reset", help="Abandon the current screenplay")

    args = parser.parse_args()
    if args.cmd == "init":
        cmd_init(args)
    elif args.cmd == "step":
        cmd_step(args)
    elif args.cmd == "done":
        cmd_done(args)
    elif args.cmd == "reset":
        cmd_reset(args)
    else:
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
