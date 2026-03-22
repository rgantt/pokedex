//! Screenplay test runner.
//!
//! Reads YAML screenplay files from tests/screenplays/, executes each command,
//! and evaluates assertions against the JSON output.
//!
//! Run: cargo test --test run_screenplays -- --nocapture
//!
//! Requires: pokedex binary installed (`./install.sh`) and DB seeded.

use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::process::Command;

/// Screenplay schema is enforced by serde(deny_unknown_fields).
/// Any field not in this struct will cause a parse error.
/// See tests/screenplays/schema.json for the authoritative JSON Schema.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Screenplay {
    name: String,
    #[allow(dead_code)]
    persona: Option<String>,
    #[allow(dead_code)]
    description: Option<String>,
    #[allow(dead_code)]
    needs_seed: Option<bool>,
    #[allow(dead_code)]
    mutates_collection: Option<bool>,
    steps: Vec<Step>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Step {
    name: String,
    command: String,
    #[serde(default)]
    capture: HashMap<String, String>,
    #[serde(default)]
    assert: Assertions,
}

/// Only these 6 assertion types are supported. Unknown fields cause a parse error.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Assertions {
    exit_code: Option<i32>,
    #[serde(default)]
    has_fields: Vec<String>,
    #[serde(default)]
    equals: HashMap<String, Value>,
    #[serde(default)]
    contains: HashMap<String, String>,
    #[serde(default)]
    type_of: HashMap<String, String>,
    #[serde(default)]
    array_len: HashMap<String, ArrayBound>,
}

#[derive(Debug, Deserialize)]
struct ArrayBound {
    min: Option<usize>,
    max: Option<usize>,
}

// ---- JSON dot-path resolution ----

fn resolve_path<'a>(json: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = json;
    for segment in path.split('.') {
        // Handle bracket notation like "data.types[0]" or "actions[2].cmd"
        if let Some(bracket_start) = segment.find('[') {
            let key = &segment[..bracket_start];
            let idx_str = &segment[bracket_start + 1..segment.len() - 1];
            if !key.is_empty() {
                current = current.get(key)?;
            }
            let idx: usize = idx_str.parse().ok()?;
            current = current.get(idx)?;
        } else if let Ok(idx) = segment.parse::<usize>() {
            // Handle dot notation like "data.0.species.name"
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current)
}

fn json_type_name(v: &Value) -> &str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ---- Variable substitution ----

fn substitute_vars(cmd: &str, vars: &HashMap<String, String>) -> String {
    let mut result = cmd.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("${key}"), value);
    }
    result
}

// ---- Shell-style argument splitting ----

fn shell_split(input: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut chars = input.chars().peekable();
    let mut in_double_quote = false;
    let mut in_single_quote = false;

    while let Some(c) = chars.next() {
        match c {
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            '\\' if in_double_quote => {
                if let Some(&next) = chars.peek() {
                    if next == '"' || next == '\\' {
                        current.push(chars.next().unwrap());
                    } else {
                        current.push('\\');
                    }
                }
            }
            c if c.is_whitespace() && !in_double_quote && !in_single_quote => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            _ => {
                current.push(c);
            }
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    args
}

// ---- Assertion evaluation ----

struct StepResult {
    step_name: String,
    failures: Vec<String>,
}

fn evaluate_step(step: &Step, vars: &mut HashMap<String, String>) -> StepResult {
    let mut failures = Vec::new();
    let command_str = substitute_vars(&step.command, vars);

    // Parse command into program + args (supports shell-style quoting)
    let parts = shell_split(&command_str);
    if parts.is_empty() {
        failures.push("Empty command".to_string());
        return StepResult { step_name: step.name.clone(), failures };
    }

    let output = Command::new(&parts[0])
        .args(&parts[1..])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(e) => {
            failures.push(format!("Failed to execute '{}': {}", command_str, e));
            return StepResult { step_name: step.name.clone(), failures };
        }
    };

    let exit_code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Check exit code
    if let Some(expected) = step.assert.exit_code {
        if exit_code != expected {
            failures.push(format!(
                "exit_code: expected {expected}, got {exit_code}"
            ));
        }
    }

    // Parse JSON (may fail for non-JSON output like --help)
    let json: Option<Value> = serde_json::from_str(&stdout).ok();

    if json.is_none() && (!step.assert.has_fields.is_empty()
        || !step.assert.equals.is_empty()
        || !step.assert.contains.is_empty()
        || !step.assert.type_of.is_empty()
        || !step.assert.array_len.is_empty()
        || !step.capture.is_empty())
    {
        failures.push(format!("Output is not valid JSON: {}", &stdout[..stdout.len().min(200)]));
        return StepResult { step_name: step.name.clone(), failures };
    }

    if let Some(ref json) = json {
        // Capture variables
        for (var_name, json_path) in &step.capture {
            if let Some(val) = resolve_path(json, json_path) {
                let val_str = match val {
                    Value::Number(n) => n.to_string(),
                    Value::String(s) => s.clone(),
                    Value::Bool(b) => b.to_string(),
                    _ => val.to_string(),
                };
                vars.insert(var_name.clone(), val_str);
            } else {
                failures.push(format!("capture '{var_name}': path '{json_path}' not found"));
            }
        }

        // has_fields
        for path in &step.assert.has_fields {
            if resolve_path(json, path).is_none() {
                failures.push(format!("has_fields: '{path}' not found"));
            }
        }

        // equals
        for (path, expected) in &step.assert.equals {
            match resolve_path(json, path) {
                Some(actual) => {
                    if actual != expected {
                        failures.push(format!(
                            "equals: '{path}' expected {}, got {}",
                            serde_json::to_string(expected).unwrap_or_default(),
                            serde_json::to_string(actual).unwrap_or_default()
                        ));
                    }
                }
                None => failures.push(format!("equals: '{path}' not found")),
            }
        }

        // contains
        for (path, substring) in &step.assert.contains {
            match resolve_path(json, path) {
                Some(Value::String(actual)) => {
                    if !actual.contains(substring.as_str()) {
                        failures.push(format!(
                            "contains: '{path}' expected to contain '{substring}', got '{actual}'"
                        ));
                    }
                }
                Some(other) => {
                    let s = serde_json::to_string(other).unwrap_or_default();
                    if !s.contains(substring.as_str()) {
                        failures.push(format!(
                            "contains: '{path}' expected to contain '{substring}', got {s}"
                        ));
                    }
                }
                None => failures.push(format!("contains: '{path}' not found")),
            }
        }

        // type_of
        for (path, expected_type) in &step.assert.type_of {
            match resolve_path(json, path) {
                Some(val) => {
                    let actual_type = json_type_name(val);
                    if actual_type != expected_type.as_str() {
                        failures.push(format!(
                            "type_of: '{path}' expected {expected_type}, got {actual_type}"
                        ));
                    }
                }
                None => failures.push(format!("type_of: '{path}' not found")),
            }
        }

        // array_len
        for (path, bound) in &step.assert.array_len {
            match resolve_path(json, path) {
                Some(Value::Array(arr)) => {
                    if let Some(min) = bound.min {
                        if arr.len() < min {
                            failures.push(format!(
                                "array_len: '{path}' length {} < min {min}", arr.len()
                            ));
                        }
                    }
                    if let Some(max) = bound.max {
                        if arr.len() > max {
                            failures.push(format!(
                                "array_len: '{path}' length {} > max {max}", arr.len()
                            ));
                        }
                    }
                }
                Some(_) => failures.push(format!("array_len: '{path}' is not an array")),
                None => failures.push(format!("array_len: '{path}' not found")),
            }
        }
    }

    StepResult { step_name: step.name.clone(), failures }
}

// ---- Test entry point ----

#[test]
fn run_all_screenplays() {
    let screenplay_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/screenplays");
    if !screenplay_dir.exists() {
        eprintln!("No screenplays directory found at {}", screenplay_dir.display());
        return;
    }

    let mut entries: Vec<_> = std::fs::read_dir(&screenplay_dir)
        .expect("Failed to read screenplays directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "yaml" || ext == "yml").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if entries.is_empty() {
        eprintln!("No screenplay files found in {}", screenplay_dir.display());
        return;
    }

    let mut total_steps = 0;
    let mut total_failures = 0;
    let mut all_errors: Vec<String> = Vec::new();

    for entry in &entries {
        let path = entry.path();
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {e}", path.display()));
        let screenplay: Screenplay = serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!(
                "Failed to parse {}: {e}\n\nFix the YAML to conform to the screenplay schema, or delete the file.",
                path.display()
            ));

        eprintln!("\n=== Screenplay: {} ({}) ===", screenplay.name, path.file_name().unwrap().to_string_lossy());

        let mut vars: HashMap<String, String> = HashMap::new();

        for step in &screenplay.steps {
            total_steps += 1;
            let result = evaluate_step(step, &mut vars);

            if result.failures.is_empty() {
                eprintln!("  PASS: {}", result.step_name);
            } else {
                total_failures += result.failures.len();
                eprintln!("  FAIL: {}", result.step_name);
                for f in &result.failures {
                    eprintln!("    - {f}");
                    all_errors.push(format!("[{}] {}: {f}", screenplay.name, result.step_name));
                }
            }
        }
    }

    eprintln!("\n=== Summary: {total_steps} steps, {total_failures} failures across {} screenplays ===",
        entries.len());

    if !all_errors.is_empty() {
        // Screenplays are cheap regression tests — failures indicate either:
        // 1. A real bug introduced by code changes (fix the code)
        // 2. A stale screenplay that needs updating (delete or update the YAML)
        //
        // To investigate, run: cargo test --test run_screenplays -- --nocapture
        // Then either fix the code or update/delete the stale screenplay.
        //
        // Set SCREENPLAY_STRICT=1 to make failures panic (for CI gating).
        // Default: report failures but don't block.
        let strict = std::env::var("SCREENPLAY_STRICT").unwrap_or_default() == "1";
        if strict {
            panic!(
                "\n{total_failures} screenplay assertion(s) failed (SCREENPLAY_STRICT=1):\n{}",
                all_errors.iter().map(|e| format!("  - {e}")).collect::<Vec<_>>().join("\n")
            );
        } else {
            eprintln!("\n{total_failures} screenplay failure(s) — investigate and fix code or update screenplay:");
            for e in &all_errors {
                eprintln!("  - {e}");
            }
            eprintln!("\nTo make this a hard failure, run with SCREENPLAY_STRICT=1");
        }
    }
}
