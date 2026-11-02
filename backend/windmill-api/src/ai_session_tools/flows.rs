//! Flow drafts: the compact view, structural patches and inline module code.
//!
//! The model never sees a rawscript body inside flow JSON: the compact view shows
//! `inline_script.<moduleId>` in its place, and a write resolves each such placeholder
//! back to a body (an existing module's, or an empty one for the module's own id).

use std::collections::{HashMap, HashSet};

use serde_json::{json, Map, Value};
use windmill_types::flows::FlowModule;

use super::ai_providers::{self, CatalogFor};
use super::workspace::{
    compact_flow, empty_schema, find_and_replace, finish_draft_write, load_flow, persist_draft,
    write_base, Persist, FLOW_VALUE_SETTINGS_KEYS,
};
use super::{js_len, non_null, opt_str_arg, pretty, str_arg, truthy, Ctx, ToolResult};

pub(super) const TOOLS: &[&str] = &[
    "write_flow",
    "patch_flow_json",
    "read_flow_module_code",
    "set_flow_module_code",
];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "write_flow" => write_flow(ctx, args).await,
        "patch_flow_json" => patch_flow_json(ctx, args).await,
        "read_flow_module_code" => read_flow_module_code(ctx, args).await,
        "set_flow_module_code" => set_flow_module_code(ctx, args).await,
        _ => unreachable!("{name} is not a flows tool"),
    }
}

const STRUCTURAL_KEYS: &[&str] = &[
    "modules",
    "schema",
    "preprocessor_module",
    "failure_module",
    "groups",
    "notes",
];
const SPECIAL_MODULES: [(&str, &str); 2] = [
    ("preprocessor_module", "preprocessor"),
    ("failure_module", "failure"),
];
const NOTE_COLORS: &[&str] = &[
    "yellow", "blue", "green", "purple", "pink", "orange", "red", "cyan", "lime", "gray",
];
/// Names the flow editor reserves, which an agent tool cannot answer to.
const RESERVED_TOOL_NAMES: &[&str] = &[
    "do",
    "bg",
    "ctx",
    "state",
    "if",
    "else",
    "for",
    "delete",
    "while",
    "new",
    "in",
    "failure",
    "preprocessor",
    "__wm_agent_root",
    "as",
    "Input",
    "Result",
    "Trigger",
    "__wm_web_search",
];

fn module_id(module: &Value) -> &str {
    module.get("id").and_then(Value::as_str).unwrap_or_default()
}

fn value_type(value: &Map<String, Value>) -> &str {
    value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn module_type(module: &Value) -> &str {
    module
        .get("value")
        .and_then(Value::as_object)
        .map(value_type)
        .unwrap_or_default()
}

/// The module id an `inline_script.<id>` placeholder names.
fn placeholder_ref(content: &str) -> Option<&str> {
    content
        .strip_prefix("inline_script.")
        .filter(|id| !id.is_empty() && !id.contains(['\n', '\r', '\u{2028}', '\u{2029}']))
}

fn content_of(value: &Map<String, Value>) -> Option<&str> {
    value.get("content").and_then(Value::as_str)
}

/// Every rawscript the compact view shows as a placeholder, with its owner's id: the
/// modules of loops and branches, and an AI agent's own rawscript tools (not what those
/// tools nest).
fn visit_scripts(modules: Option<&mut Value>, f: &mut dyn FnMut(&str, &mut Map<String, Value>)) {
    let Some(Value::Array(modules)) = modules else {
        return;
    };
    for module in modules {
        let id = module_id(module).to_string();
        let Some(value) = module.get_mut("value").and_then(Value::as_object_mut) else {
            continue;
        };
        let kind = value_type(value).to_string();
        match kind.as_str() {
            "rawscript" => f(&id, value),
            "forloopflow" | "whileloopflow" => visit_scripts(value.get_mut("modules"), f),
            "branchone" | "branchall" => {
                if let Some(Value::Array(branches)) = value.get_mut("branches") {
                    for branch in branches {
                        visit_scripts(branch.get_mut("modules"), f);
                    }
                }
                if kind == "branchone" {
                    visit_scripts(value.get_mut("default"), f);
                }
            }
            "aiagent" => {
                let Some(Value::Array(tools)) = value.get_mut("tools") else {
                    continue;
                };
                for tool in tools {
                    let tool_id = module_id(tool).to_string();
                    let Some(tool_value) = tool.get_mut("value").and_then(Value::as_object_mut)
                    else {
                        continue;
                    };
                    let is_flow_module =
                        tool_value.get("tool_type").and_then(Value::as_str) == Some("flowmodule");
                    if is_flow_module && value_type(tool_value) == "rawscript" {
                        f(&tool_id, tool_value);
                    }
                }
            }
            _ => {}
        }
    }
}

/// `visit_scripts` over a flow value: its modules, then its preprocessor and failure
/// modules when they are rawscripts.
fn visit_flow_scripts(
    flow: &mut Map<String, Value>,
    f: &mut dyn FnMut(&str, &mut Map<String, Value>),
) {
    visit_scripts(flow.get_mut("modules"), f);
    for (key, _) in SPECIAL_MODULES {
        let Some(module) = flow.get_mut(key) else {
            continue;
        };
        let id = module_id(module).to_string();
        if let Some(value) = module.get_mut("value").and_then(Value::as_object_mut) {
            if value_type(value) == "rawscript" {
                f(&id, value);
            }
        }
    }
}

/// Every module of a tree, parents first, an AI agent's tools included as modules
/// (`in_tool` is set from there down).
fn walk(modules: Option<&mut Value>, in_tool: bool, f: &mut dyn FnMut(&mut Value, bool)) {
    let Some(Value::Array(modules)) = modules else {
        return;
    };
    for module in modules {
        f(module, in_tool);
        let Some(value) = module.get_mut("value").and_then(Value::as_object_mut) else {
            continue;
        };
        let kind = value_type(value).to_string();
        match kind.as_str() {
            "forloopflow" | "whileloopflow" => walk(value.get_mut("modules"), in_tool, f),
            "branchone" | "branchall" => {
                if kind == "branchone" {
                    walk(value.get_mut("default"), in_tool, f);
                }
                if let Some(Value::Array(branches)) = value.get_mut("branches") {
                    for branch in branches {
                        walk(branch.get_mut("modules"), in_tool, f);
                    }
                }
            }
            "aiagent" => walk(value.get_mut("tools"), true, f),
            _ => {}
        }
    }
}

/// `walk` over a flow value: its modules, then its preprocessor and failure modules.
fn walk_flow(flow: &mut Map<String, Value>, f: &mut dyn FnMut(&mut Value, bool)) {
    walk(flow.get_mut("modules"), false, f);
    for (key, _) in SPECIAL_MODULES {
        if let Some(module) = flow.get_mut(key).filter(|m| !m.is_null()) {
            let mut single = Value::Array(vec![module.take()]);
            walk(Some(&mut single), false, f);
            *module = single[0].take();
        }
    }
}

/// Drops the lock of every step whose code differs from the same step in `base`: the worker
/// reuses any non-empty step lock, so a changed step keeping its lock runs against the old
/// dependencies. A step id `base` lacks (renamed or duplicated) keeps its lock only when some
/// `base` step has the same code. Without a base nothing is dropped: a copied flow's locks match.
pub(super) fn drop_stale_step_locks(value: &mut Map<String, Value>, base: Option<&Value>) {
    let Some(mut base) = base.and_then(Value::as_object).cloned() else {
        return;
    };
    let code = |module: &Value| module.get("value").and_then(|v| v.get("content")).cloned();
    let mut base_code_by_id = HashMap::new();
    let mut base_codes = Vec::new();
    walk_flow(&mut base, &mut |module, _| {
        base_code_by_id.insert(module_id(module).to_string(), code(module));
        base_codes.push(code(module));
    });
    walk_flow(value, &mut |module, _| {
        let (id, content) = (module_id(module).to_string(), code(module));
        let Some(step) = module.get_mut("value").and_then(Value::as_object_mut) else {
            return;
        };
        if step.get("lock").is_none_or(Value::is_null) {
            return;
        }
        let unchanged = match base_code_by_id.get(&id) {
            Some(base_code) => *base_code == content,
            None => base_codes.contains(&content),
        };
        if !unchanged {
            step.shift_remove("lock");
        }
    });
}

/// Put the stored bodies back in place of the placeholders that name them.
fn restore_inline_scripts(flow: &mut Map<String, Value>, scripts: &HashMap<String, String>) {
    visit_flow_scripts(flow, &mut |_, value| {
        let body = content_of(value)
            .and_then(placeholder_ref)
            .and_then(|id| scripts.get(id));
        if let Some(body) = body {
            value.insert("content".into(), json!(body));
        }
    });
}

fn unresolved_inline_scripts(flow: &mut Map<String, Value>) -> Vec<String> {
    let mut refs = Vec::new();
    visit_flow_scripts(flow, &mut |_, value| {
        if let Some(id) = content_of(value).and_then(placeholder_ref) {
            refs.push(id.to_string());
        }
    });
    refs
}

/// What is left of the placeholders once the stored bodies are back: one naming its own
/// module is a new body, saved empty; one naming anything else must not be saved as code.
fn finalize_inline_scripts(flow: &mut Map<String, Value>) -> Result<(), String> {
    let mut blank = |module: &mut Value, _: bool| {
        let id = module_id(module).to_string();
        let Some(value) = module.get_mut("value").and_then(Value::as_object_mut) else {
            return;
        };
        if value_type(value) == "rawscript"
            && content_of(value).and_then(placeholder_ref) == Some(id.as_str())
        {
            value.insert("content".into(), json!(""));
        }
    };
    walk_flow(flow, &mut blank);
    let unresolved = unresolved_inline_scripts(flow);
    if unresolved.is_empty() {
        return Ok(());
    }
    let refs = unresolved
        .iter()
        .map(|id| format!("\"inline_script.{id}\""))
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!("Unresolved inline script reference(s): {refs}. A rawscript \"content\" placeholder must be \"inline_script.<its own module id>\" (a new body, filled afterwards with the set-module-code tool) or reference an existing rawscript module to keep its current body. Nothing was saved."))
}

/// The inline scripts of a flow value that still have no body, in flow order.
fn empty_inline_script_ids(flow: &Map<String, Value>) -> Vec<String> {
    let mut scripts: Vec<(String, String)> = Vec::new();
    visit_flow_scripts(&mut flow.clone(), &mut |id, value| {
        let Some(content) = content_of(value) else {
            return;
        };
        match scripts.iter_mut().find(|(known, _)| known == id) {
            Some(script) => script.1 = content.to_string(),
            None => scripts.push((id.to_string(), content.to_string())),
        }
    });
    scripts
        .into_iter()
        .filter(|(_, content)| content.trim().is_empty() || content.starts_with("inline_script."))
        .map(|(id, _)| id)
        .collect()
}

/// A patched compact view applied onto the flow value it was built from. The view is
/// the whole state of what it shows, so a key it no longer has is removed; fields of
/// the value it never showed are kept.
fn apply_editable(
    original: Option<&Value>,
    editable: &Map<String, Value>,
    scripts: &HashMap<String, String>,
) -> Map<String, Value> {
    let mut value = original
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    value.insert(
        "modules".into(),
        editable
            .get("modules")
            .cloned()
            .unwrap_or_else(|| json!([])),
    );
    let optional = ["preprocessor_module", "failure_module", "groups", "notes"];
    for key in optional.iter().chain(FLOW_VALUE_SETTINGS_KEYS) {
        match non_null(editable.get(*key)).cloned() {
            Some(field) => value.insert((*key).into(), field),
            None => value.shift_remove(*key),
        };
    }
    restore_inline_scripts(&mut value, scripts);
    value
}

fn json_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn validate_settings(flow: &Map<String, Value>) -> Result<(), String> {
    let invalid = |path: &str, expected: &str, got: &Value| {
        format!(
            "Invalid flow setting {path}: Invalid input: expected {expected}, received {}",
            json_type(got)
        )
    };
    for key in FLOW_VALUE_SETTINGS_KEYS {
        let Some(value) = flow.get(*key) else {
            continue;
        };
        let (expected, ok) = match *key {
            "same_worker"
            | "preserve_step_tags"
            | "cache_ignore_s3_path"
            | "chat_input_enabled" => ("boolean", value.is_boolean()),
            "concurrency_key" | "debounce_key" | "skip_expr" | "early_return" => {
                ("string", value.is_string())
            }
            "debounce_args_to_accumulate" => ("array", value.is_array()),
            "flow_env" => ("record", value.is_object()),
            _ => ("number", value.is_number()),
        };
        if !ok {
            return Err(invalid(key, expected, value));
        }
        for (index, entry) in value.as_array().into_iter().flatten().enumerate() {
            if !entry.is_string() {
                return Err(invalid(&format!("{key}.{index}"), "string", entry));
            }
        }
    }
    Ok(())
}

/// What the server itself can read as a flow module: the shape it would refuse to run.
fn check_module_shape(module: &Value) -> Result<(), String> {
    serde_json::from_str::<FlowModule>(&module.to_string())
        .map_err(|e| e.to_string())?
        .get_value()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn tool_name_error(name: &str, siblings: &[&str]) -> Option<String> {
    if name.is_empty() {
        return Some("Tool name must not be empty".into());
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Some("Tool name must only contain letters, numbers and underscores".into());
    }
    if RESERVED_TOOL_NAMES.contains(&name) {
        return Some(format!("'{name}' is a reserved name"));
    }
    (siblings.iter().filter(|n| **n == name).count() > 1).then(|| "Duplicate tool name".into())
}

fn quoted_list(ids: &[String]) -> String {
    ids.iter()
        .map(|id| format!("\"{id}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Checks the modules array and returns every id it holds, agent tools included.
fn validate_modules(modules: &mut Value) -> Result<HashSet<String>, String> {
    if !modules.is_array() {
        return Err("Flow modules must be an array".into());
    }
    let mut shape_errors = Vec::new();
    let mut ids = Vec::new();
    let mut providerless = Vec::new();
    let mut tool_names = Vec::new();
    walk(Some(modules), false, &mut |module, in_tool| {
        let id = module_id(module);
        // A tool is not a flow module: the agent that owns it checks its shape.
        if !in_tool && shape_errors.len() < 5 {
            if let Err(e) = check_module_shape(module) {
                shape_errors.push(format!("Module \"{id}\": {e}"));
            }
        }
        if !id.is_empty() {
            ids.push(id.to_string());
        }
        let kind = module_type(module);
        if kind != "aiagent" && kind != "aidecision" {
            return;
        }
        let value = &module["value"];
        let linked = truthy(value.get("agent"));
        let provider = value
            .get("input_transforms")
            .and_then(|transforms| transforms.get("provider"))
            .filter(|provider| truthy(Some(provider)));
        if kind == "aiagent" && !linked && provider.is_none() {
            providerless.push(id.to_string());
        }

        // Only a flow module tool's summary is a name the agent calls.
        let named: Vec<&Value> = value
            .get("tools")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|tool| {
                let tool_type = tool.get("value").and_then(|v| v.get("tool_type"));
                !matches!(tool_type.and_then(Value::as_str), Some("mcp" | "websearch"))
            })
            .collect();
        fn name_of(tool: &Value) -> &str {
            tool.get("summary")
                .and_then(Value::as_str)
                .unwrap_or_default()
        }
        let siblings: Vec<&str> = named.iter().map(|tool| name_of(tool)).collect();
        for tool in &named {
            if let Some(error) = tool_name_error(name_of(tool), &siblings) {
                tool_names.push(format!(
                    "agent \"{id}\" tool \"{}\" is named {} - {error}",
                    module_id(tool),
                    json!(name_of(tool))
                ));
            }
        }
    });

    if !shape_errors.is_empty() {
        return Err(format!(
            "Invalid flow modules:\n{}",
            shape_errors.join("\n")
        ));
    }
    let unique: HashSet<String> = ids.iter().cloned().collect();
    if unique.len() != ids.len() {
        return Err("Duplicate module IDs found in flow".into());
    }
    if SPECIAL_MODULES.iter().any(|(_, id)| unique.contains(*id)) {
        return Err("Special modules must be provided via preprocessor_module and failure_module, not inside modules".into());
    }
    if !providerless.is_empty() {
        return Err(format!(
            "AI agent modules {} need a provider input transform, or an \"agent\" path linking them to a saved agent",
            quoted_list(&providerless)
        ));
    }
    if !tool_names.is_empty() {
        return Err(format!(
            "Invalid AI agent tool name(s): {}. The tool's \"summary\" is the name the agent calls it by: use underscores instead of spaces (e.g. \"search_docs\").",
            tool_names.join("; ")
        ));
    }
    Ok(unique)
}

fn check_color(color: Option<&Value>) -> Result<(), String> {
    match color {
        None | Some(Value::Null) => Ok(()),
        Some(Value::String(color)) if NOTE_COLORS.contains(&color.as_str()) => Ok(()),
        Some(_) => Err(format!("color must be one of {}", NOTE_COLORS.join(", "))),
    }
}

fn validate_groups(raw: Option<&Value>, module_ids: &HashSet<String>) -> Result<Value, String> {
    let groups = match raw {
        None | Some(Value::Null) => return Ok(Value::Null),
        Some(Value::Array(groups)) => groups,
        Some(_) => return Err("Flow groups must be an array".into()),
    };
    for (index, group) in groups.iter().enumerate() {
        let fail = |what: String| format!("Invalid group at index {index}: {what}");
        let Value::Object(group) = group else {
            return Err(fail("must be an object".into()));
        };
        for key in ["start_id", "end_id"] {
            let id = group.get(key).and_then(Value::as_str).unwrap_or_default();
            if id.is_empty() {
                return Err(fail(format!("{key} must be a non-empty string")));
            }
        }
        for key in ["start_id", "end_id"] {
            let id = group[key].as_str().unwrap_or_default();
            if !module_ids.contains(id) {
                return Err(fail(format!(
                    "{key} \"{id}\" does not match any flow module"
                )));
            }
        }
        check_color(group.get("color")).map_err(fail)?;
    }
    Ok(Value::Array(groups.clone()))
}

fn number(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9e15 {
        json!(n as i64)
    } else {
        json!(n)
    }
}

const NOTE_WIDTH: f64 = 275.0;
const NOTE_MIN_HEIGHT: f64 = 60.0;
/// Where a free note without a position goes: a column left of the flow.
const NOTE_COLUMN_X: f64 = -(NOTE_WIDTH + 100.0);
const NOTE_GAP: f64 = 24.0;

/// The editor shows a free note at a fixed size, so one written without a size needs a
/// box tall enough for its text (12px text, 1.4 line height, 16px padding).
fn free_note_height(text: &str) -> f64 {
    let chars_per_line = ((NOTE_WIDTH - 32.0) / 6.2).floor().max(1.0);
    let lines: f64 = text
        .split('\n')
        .map(|line| (js_len(line) as f64 / chars_per_line).ceil().max(1.0))
        .sum();
    (lines * 17.0 + 32.0).clamp(NOTE_MIN_HEIGHT, 600.0)
}

/// Checks the notes and fills in what the editor needs of each: a type, a color and,
/// for a free note, the size and position without which it cannot be moved.
fn validate_notes(raw: Option<&Value>, module_ids: &HashSet<String>) -> Result<Value, String> {
    let notes = match raw {
        None | Some(Value::Null) => return Ok(Value::Null),
        Some(Value::Array(notes)) => notes,
        Some(_) => return Err("Flow notes must be an array".into()),
    };
    let mut seen = HashSet::new();
    let mut stack_y = 0.0_f64;
    let mut out = Vec::new();
    for (index, note) in notes.iter().enumerate() {
        let fail = |what: String| format!("Invalid note at index {index}: {what}");
        let Value::Object(note) = note else {
            return Err(fail("must be an object".into()));
        };
        let id = note.get("id").and_then(Value::as_str).unwrap_or_default();
        if id.is_empty() {
            return Err(fail("id must be a non-empty string".into()));
        }
        if !seen.insert(id) {
            return Err(fail(format!("duplicate note id \"{id}\"")));
        }
        let Some(text) = note.get("text").and_then(Value::as_str) else {
            return Err(fail("text must be a string".into()));
        };
        let kind = match note.get("type") {
            None | Some(Value::Null) => "free",
            Some(Value::String(kind)) if kind == "free" || kind == "group" => kind.as_str(),
            Some(_) => return Err(fail("type must be \"free\" or \"group\"".into())),
        };
        check_color(note.get("color")).map_err(fail)?;
        let pair = |key: &str, a: &str, b: &str| -> Result<Option<(f64, f64)>, String> {
            let Some(value) = note.get(key).filter(|v| !v.is_null()) else {
                return Ok(None);
            };
            let field = |name: &str| value.get(name).and_then(Value::as_f64);
            match (value.is_object(), field(a), field(b)) {
                (true, Some(a), Some(b)) => Ok(Some((a, b))),
                _ => Err(fail(format!(
                    "{key} must be an object with numeric {a} and {b}"
                ))),
            }
        };
        let position = pair("position", "x", "y")?;
        let size = pair("size", "width", "height")?;
        if kind == "group" {
            if let Some(contained) = note.get("contained_node_ids") {
                let ids: Option<Vec<&str>> = contained
                    .as_array()
                    .and_then(|ids| ids.iter().map(Value::as_str).collect());
                let Some(ids) = ids else {
                    return Err(fail(
                        "contained_node_ids must be an array of strings".into(),
                    ));
                };
                if let Some(id) = ids.iter().find(|id| !module_ids.contains(**id)) {
                    return Err(fail(format!(
                        "contained_node_ids \"{id}\" does not match any flow module"
                    )));
                }
            }
        }

        let mut normalized = note.clone();
        normalized.insert("type".into(), json!(kind));
        if !note.get("color").is_some_and(Value::is_string) {
            normalized.insert("color".into(), json!("green"));
        }
        if kind == "free" {
            let (width, height) = size.unwrap_or_else(|| (NOTE_WIDTH, free_note_height(text)));
            if size.is_none() {
                normalized.insert(
                    "size".into(),
                    json!({ "width": number(width), "height": number(height) }),
                );
            }
            match position {
                None => {
                    normalized.insert(
                        "position".into(),
                        json!({ "x": number(NOTE_COLUMN_X), "y": number(stack_y) }),
                    );
                    stack_y += height + NOTE_GAP;
                }
                // A placed note standing in the column pushes the next unplaced one below it.
                Some((x, y)) if x < NOTE_COLUMN_X + NOTE_WIDTH && x + width > NOTE_COLUMN_X => {
                    stack_y = stack_y.max(y + height + NOTE_GAP);
                }
                Some(_) => {}
            }
        }
        out.push(Value::Object(normalized));
    }
    Ok(Value::Array(out))
}

/// Checks a compact flow view (`modules, schema, preprocessor_module, failure_module,
/// groups, notes` and the top-level settings) and returns it with every structural key
/// present, `null` when unset. Provider findings that do not block go to `warnings`.
fn validate_editable(
    raw: Value,
    catalog: Option<&ai_providers::Catalog>,
    warnings: &mut Vec<String>,
) -> Result<Map<String, Value>, String> {
    let Value::Object(mut flow) = raw else {
        return Err("Flow JSON must be an object".into());
    };
    let allowed = || STRUCTURAL_KEYS.iter().chain(FLOW_VALUE_SETTINGS_KEYS);
    // An unknown key is refused rather than dropped: the patch that added it would
    // otherwise report success for an edit that never lands on the flow.
    let unknown: Vec<&str> = flow
        .keys()
        .map(String::as_str)
        .filter(|key| !allowed().any(|known| known == key))
        .collect();
    if !unknown.is_empty() {
        return Err(format!(
            "Unknown top-level flow key(s): {}. Allowed keys: {}",
            unknown.join(", "),
            allowed().copied().collect::<Vec<_>>().join(", ")
        ));
    }
    validate_settings(&flow)?;

    let mut modules = flow.shift_remove("modules").unwrap_or(Value::Null);
    let module_ids = validate_modules(&mut modules)?;
    ai_providers::validate(&modules, catalog, warnings)?;
    let schema = match flow.get("schema") {
        None | Some(Value::Null) => Value::Null,
        Some(schema @ Value::Object(_)) => schema.clone(),
        Some(_) => return Err("Flow schema must be an object or null".into()),
    };
    for (key, _) in SPECIAL_MODULES {
        if let Some(module) = non_null(flow.get(key)) {
            check_module_shape(&module).map_err(|e| format!("Invalid {key}: {e}"))?;
        }
    }
    let groups = validate_groups(flow.get("groups"), &module_ids)?;
    let notes = validate_notes(flow.get("notes"), &module_ids)?;
    for (key, id) in SPECIAL_MODULES {
        let Some(module) = non_null(flow.get(key)) else {
            continue;
        };
        if module_id(&module) != id {
            return Err(format!("Invalid {key}: id must be \"{id}\""));
        }
        if !["rawscript", "script"].contains(&module_type(&module)) {
            return Err(format!(
                "Invalid {key}: only \"rawscript\" and \"script\" modules are supported"
            ));
        }
    }

    let mut editable = Map::new();
    editable.insert("modules".into(), modules);
    editable.insert("schema".into(), schema);
    for (key, _) in SPECIAL_MODULES {
        editable.insert(key.into(), flow.shift_remove(key).unwrap_or(Value::Null));
    }
    editable.insert("groups".into(), groups);
    editable.insert("notes".into(), notes);
    for key in FLOW_VALUE_SETTINGS_KEYS {
        if let Some(setting) = flow.shift_remove(*key) {
            editable.insert((*key).into(), setting);
        }
    }
    Ok(editable)
}

struct FlowWrite<'a> {
    path: &'a str,
    summary: Option<&'a str>,
    description: Option<&'a str>,
    value: Map<String, Value>,
    schema: Option<Value>,
    groups: Option<Value>,
    /// Keep the fields of the current flow value that `value` cannot express.
    preserve_base_settings: bool,
    /// Name the inline scripts left without a body in the result.
    warn_empty_scripts: bool,
    force: bool,
}

async fn write_flow_draft(ctx: &Ctx<'_>, write: FlowWrite<'_>) -> ToolResult {
    let FlowWrite { path, mut value, .. } = write;
    ctx.saving_draft("flow", path);
    let base = write_base(ctx, "flow", path, None).await?;
    let existed = base.base.is_some();
    if let Some(groups) = write.groups {
        value.insert("groups".into(), groups);
    }
    let base_flow = match base.base {
        Some(Value::Object(flow)) => Some(flow),
        _ => None,
    };
    if write.preserve_base_settings {
        let base_value = base_flow.as_ref().and_then(|flow| flow.get("value"));
        for (key, field) in base_value.and_then(Value::as_object).into_iter().flatten() {
            if !STRUCTURAL_KEYS.contains(&key.as_str()) && !value.contains_key(key) {
                value.insert(key.clone(), field.clone());
            }
        }
    }
    // Also done at deploy; here too because a test run of the draft reads the locks.
    drop_stale_step_locks(
        &mut value,
        base_flow.as_ref().and_then(|flow| flow.get("value")),
    );
    let empty_scripts = if write.warn_empty_scripts {
        empty_inline_script_ids(&value)
    } else {
        Vec::new()
    };
    let draft = match base_flow {
        Some(mut flow) => {
            flow.insert("path".into(), json!(path));
            if let Some(summary) = write.summary {
                flow.insert("summary".into(), json!(summary));
            }
            if let Some(description) = write.description {
                flow.insert("description".into(), json!(description));
            }
            flow.insert("value".into(), Value::Object(value));
            if let Some(schema) = write.schema {
                flow.insert("schema".into(), schema);
            }
            Value::Object(flow)
        }
        None => json!({
            "path": path,
            "summary": write.summary.unwrap_or_default(),
            "description": write.description.unwrap_or_default(),
            "value": value,
            "schema": write.schema.unwrap_or_else(empty_schema),
            "edited_by": "",
            "edited_at": "",
            "archived": false,
            "extra_perms": {},
        }),
    };
    let persisted = persist_draft(
        ctx,
        "flow",
        path,
        None,
        &draft,
        base.last_sync.as_deref(),
        write.force,
    )
    .await?;
    let item = match persisted {
        Persist::Saved(item) => item,
        Persist::Failed(text) => return Ok(text),
    };
    let result = finish_draft_write(ctx, &item, existed);
    if empty_scripts.is_empty() {
        return Ok(result);
    }
    let Ok(Value::Object(mut parsed)) = serde_json::from_str::<Value>(&result) else {
        return Ok(result);
    };
    let message = parsed
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let message = format!("{message}\n\nWarning: inline scripts {} have no code yet. Fill each one with set_flow_module_code(path, module_id, code) — do not re-send the whole flow.", quoted_list(&empty_scripts));
    parsed.insert("message".into(), json!(message));
    Ok(pretty(&Value::Object(parsed)))
}

/// A flow-structure argument: a JSON string, or the value itself from a client that
/// did not stringify it.
fn json_arg(args: &Value, field: &str) -> Result<Value, String> {
    let Some(Value::String(text)) = args.get(field) else {
        return Ok(args.get(field).cloned().unwrap_or(Value::Null));
    };
    serde_json::from_str(text).map_err(|e| {
        let hint = if ["modules", "preprocessor_module", "failure_module"].contains(&field) {
            " A rawscript \"content\" string with multi-line code or quotes is the usual cause. Instead of inlining large code, create the rawscript module with empty content (\"\") and fill its body with set_flow_module_code afterwards."
        } else {
            ""
        };
        format!("Invalid JSON for {field}: {e}{hint}")
    })
}

async fn write_flow(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let mut raw = Map::new();
    for key in STRUCTURAL_KEYS {
        raw.insert((*key).into(), json_arg(args, key)?);
    }
    let catalog = match ai_providers::catalog_for(ctx, &raw["modules"]).await {
        CatalogFor::Requested => return Ok(String::new()),
        CatalogFor::Ready(catalog) => Some(catalog),
        CatalogFor::NotNeeded => None,
    };
    let mut warnings = Vec::new();
    let mut editable = validate_editable(Value::Object(raw), catalog.as_ref(), &mut warnings)?;

    // A placeholder keeps the body the current flow has for that module, so a rewrite
    // does not have to send code it is not changing.
    if !unresolved_inline_scripts(&mut editable).is_empty() {
        let current = write_base(ctx, "flow", path, None).await?.base;
        if let Some(value) = current.as_ref().and_then(|flow| flow.get("value")) {
            restore_inline_scripts(&mut editable, &compact_flow(value, None).1);
        }
        finalize_inline_scripts(&mut editable)?;
    }

    let schema = non_null(editable.get("schema")).cloned();
    let groups = non_null(editable.get("groups")).cloned();
    editable.shift_remove("schema");
    editable.retain(|_, field| !field.is_null());
    let written = write_flow_draft(
        ctx,
        FlowWrite {
            path,
            summary: opt_str_arg(args, "summary"),
            description: opt_str_arg(args, "description"),
            value: editable,
            schema,
            groups,
            preserve_base_settings: true,
            warn_empty_scripts: true,
            force: args.get("override").and_then(Value::as_bool) == Some(true),
        },
    )
    .await?;
    Ok(with_provider_warnings(ctx, written, &warnings))
}

/// The provider findings that did not block the write, after the result. The chat appends
/// them itself when its own save fails and replaces this text.
fn with_provider_warnings(ctx: &Ctx<'_>, written: String, warnings: &[String]) -> String {
    let suffix = ai_providers::format_warnings(warnings);
    if !suffix.is_empty() {
        ctx.facts(json!({ "text_suffix": suffix }));
    }
    format!("{written}{suffix}")
}

async fn patch_flow_json(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let base = load_flow(ctx, path).await?;
    let (compact, scripts) = compact_flow(
        base.flow.get("value").unwrap_or(&Value::Null),
        base.flow.get("schema"),
    );
    let updated = find_and_replace(
        &compact.to_string(),
        str_arg(args, "old_string")?,
        str_arg(args, "new_string")?,
        args.get("replace_all").and_then(Value::as_bool) == Some(true),
        "compact flow JSON",
    )?;
    let parsed: Value = serde_json::from_str(&updated)
        .map_err(|e| format!("Invalid JSON after replacement: {e}"))?;
    let catalog = match ai_providers::catalog_for(ctx, &parsed["modules"]).await {
        CatalogFor::Requested => return Ok(String::new()),
        CatalogFor::Ready(catalog) => Some(catalog),
        CatalogFor::NotNeeded => None,
    };
    let mut warnings = Vec::new();
    let editable = validate_editable(parsed, catalog.as_ref(), &mut warnings)?;
    let mut value = apply_editable(base.flow.get("value"), &editable, &scripts);
    finalize_inline_scripts(&mut value)?;
    let written = write_flow_draft(
        ctx,
        FlowWrite {
            path,
            summary: base.summary.as_deref(),
            description: None,
            value,
            schema: non_null(editable.get("schema")).cloned(),
            groups: non_null(editable.get("groups")).cloned(),
            preserve_base_settings: false,
            warn_empty_scripts: true,
            force: false,
        },
    )
    .await?;
    Ok(with_provider_warnings(ctx, written, &warnings))
}

/// A step as the chat's card names it: its id, and its summary when it has one.
fn step_name(flow: Option<&Value>, module_id: &str) -> String {
    let mut flow = flow.and_then(Value::as_object).cloned().unwrap_or_default();
    let mut summary = None;
    walk_flow(&mut flow, &mut |module, _| {
        if summary.is_none() && self::module_id(module) == module_id {
            summary = module
                .get("summary")
                .and_then(Value::as_str)
                .map(String::from);
        }
    });
    match summary.filter(|s| !s.is_empty()) {
        Some(summary) => format!("{module_id} \"{summary}\""),
        None => module_id.to_string(),
    }
}

async fn read_flow_module_code(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let (path, module_id) = (str_arg(args, "path")?, str_arg(args, "module_id")?);
    let base = load_flow(ctx, path).await?;
    let (_, mut scripts) = compact_flow(base.flow.get("value").unwrap_or(&Value::Null), None);
    let code = scripts.remove(module_id).ok_or_else(|| format!("Module \"{module_id}\" is not an inline rawscript in flow \"{path}\". (Path runnables, branches, and loops have no inline script content; use patch_flow_json to inspect them.)"))?;
    ctx.status(format!(
        "Read code of step {}",
        step_name(base.flow.get("value"), module_id)
    ));
    Ok(code)
}

async fn set_flow_module_code(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let (path, module_id) = (str_arg(args, "path")?, str_arg(args, "module_id")?);
    let code = str_arg(args, "code")?;
    let base = load_flow(ctx, path).await?;
    let (compact, mut scripts) = compact_flow(
        base.flow.get("value").unwrap_or(&Value::Null),
        base.flow.get("schema"),
    );
    if !scripts.contains_key(module_id) {
        return Err(format!("Module \"{module_id}\" is not an inline rawscript in flow \"{path}\". Use patch_flow_json or write_flow for structural changes."));
    }
    scripts.insert(module_id.to_string(), code.to_string());
    let editable = compact.as_object().cloned().unwrap_or_default();
    // Several code edits of one flow read as identical rows under the generic flow label.
    ctx.status_after_save(
        format!(
            "Updated code of step {}",
            step_name(base.flow.get("value"), module_id)
        ),
        None,
    );
    write_flow_draft(
        ctx,
        FlowWrite {
            path,
            summary: base.summary.as_deref(),
            description: None,
            value: apply_editable(base.flow.get("value"), &editable, &scripts),
            schema: non_null(base.flow.get("schema")).cloned(),
            groups: non_null(base.flow.get("groups")).cloned(),
            preserve_base_settings: false,
            warn_empty_scripts: false,
            force: false,
        },
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(id: &str, content: &str) -> Value {
        json!({ "id": id, "value": { "type": "rawscript", "content": content, "language": "bun", "input_transforms": {} } })
    }

    fn content<'a>(flow: &'a Map<String, Value>, pointer: &str) -> &'a str {
        Value::pointer(&flow["modules"], pointer)
            .and_then(|module| module["value"]["content"].as_str())
            .unwrap()
    }

    #[test]
    fn placeholders_resolve_to_an_existing_body_or_an_empty_new_one() {
        let scripts = HashMap::from([("a".to_string(), "A".to_string())]);
        let mut flow = json!({
            "modules": [
                raw("a", "inline_script.a"),
                raw("copy", "inline_script.a"),
                { "id": "loop", "value": { "type": "forloopflow", "modules": [raw("new", "inline_script.new")] } },
                raw("multiline", "inline_script.a\nmore"),
            ],
            "failure_module": raw("failure", "inline_script.failure"),
        })
        .as_object()
        .cloned()
        .unwrap();
        restore_inline_scripts(&mut flow, &scripts);
        finalize_inline_scripts(&mut flow).unwrap();
        assert_eq!(content(&flow, "/0"), "A");
        assert_eq!(content(&flow, "/1"), "A");
        assert_eq!(content(&flow, "/2/value/modules/0"), "");
        assert_eq!(content(&flow, "/3"), "inline_script.a\nmore");
        assert_eq!(flow["failure_module"]["value"]["content"], "");
        assert_eq!(
            empty_inline_script_ids(&flow),
            ["new", "multiline", "failure"]
        );

        let mut dangling = json!({ "modules": [raw("b", "inline_script.gone")] })
            .as_object()
            .cloned()
            .unwrap();
        restore_inline_scripts(&mut dangling, &scripts);
        let error = finalize_inline_scripts(&mut dangling).unwrap_err();
        assert!(
            error.starts_with("Unresolved inline script reference(s): \"inline_script.gone\". ")
        );
    }

    #[test]
    fn only_steps_whose_code_changed_lose_their_lock() {
        let locked = |id: &str, content: &str| {
            let mut module = raw(id, content);
            module["value"]["lock"] = json!("lock");
            module
        };
        let base = json!({ "modules": [raw("edited", "A"), raw("kept", "B")] });
        let mut value = json!({
            "modules": [locked("edited", "B"), locked("kept", "B"), locked("renamed", "B"), locked("new", "C")],
            "failure_module": locked("failure", "F"),
        })
        .as_object()
        .cloned()
        .unwrap();
        let unlocked = value.clone();
        drop_stale_step_locks(&mut value, None);
        assert_eq!(value, unlocked);

        drop_stale_step_locks(&mut value, Some(&base));
        let has_lock = |module: &Value| module["value"].get("lock").is_some();
        let kept: Vec<bool> = value["modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(has_lock)
            .collect();
        assert_eq!(kept, [false, true, true, false]);
        assert!(!has_lock(&value["failure_module"]));
    }

    #[test]
    fn a_patch_of_the_compact_view_keeps_bodies_and_unsets_removed_settings() {
        let value = json!({
            "modules": [raw("a", "A"), raw("b", "B")],
            "same_worker": true,
            "untouched": 1,
        });
        let (compact, scripts) = compact_flow(&value, None);
        let patched = compact
            .to_string()
            .replace(",\"same_worker\":true", "")
            .replace("\"id\":\"b\"", "\"id\":\"c\"");
        let editable =
            validate_editable(serde_json::from_str(&patched).unwrap(), None, &mut vec![]).unwrap();
        let mut applied = apply_editable(Some(&value), &editable, &scripts);
        finalize_inline_scripts(&mut applied).unwrap();
        assert_eq!(content(&applied, "/0"), "A");
        // Renamed, so its placeholder still names the body it had.
        assert_eq!(content(&applied, "/1"), "B");
        assert_eq!(applied.get("same_worker"), None);
        assert_eq!(applied["untouched"], 1);
        assert_eq!(applied.get("groups"), None);
    }

    #[test]
    fn validation_reports_what_the_editor_would_refuse() {
        let check = |flow: Value| validate_editable(flow, None, &mut vec![]).unwrap_err();
        assert_eq!(
            check(json!({ "modules": [raw("a", ""), raw("a", "")] })),
            "Duplicate module IDs found in flow"
        );
        assert!(check(json!({ "modules": [], "bogus": 1 }))
            .starts_with("Unknown top-level flow key(s): bogus. Allowed keys: modules, schema,"));
        assert_eq!(
            check(json!({ "modules": [], "same_worker": "yes" })),
            "Invalid flow setting same_worker: Invalid input: expected boolean, received string"
        );
        assert!(
            check(json!({ "modules": [{ "id": "a", "value": { "type": "rawscript" } }] }))
                .starts_with("Invalid flow modules:\nModule \"a\": ")
        );
        assert_eq!(
            check(json!({ "modules": [], "failure_module": raw("oops", "") })),
            "Invalid failure_module: id must be \"failure\""
        );
        let agent = |tools: Value| {
            json!({ "modules": [{ "id": "agent", "value": {
                "type": "aiagent",
                "tools": tools,
                "input_transforms": { "provider": { "type": "static", "value": { "kind": "openai", "resource": "$res:u/a/b", "model": "m" } } },
            } }] })
        };
        let tool = |summary: &str| {
            let mut tool = raw("t", "");
            tool["summary"] = json!(summary);
            tool["value"]["tool_type"] = json!("flowmodule");
            tool
        };
        assert!(check(agent(json!([tool("search docs")]))).starts_with(
            "Invalid AI agent tool name(s): agent \"agent\" tool \"t\" is named \"search docs\" - Tool name must only contain letters, numbers and underscores. "
        ));
        validate_editable(agent(json!([tool("search_docs")])), None, &mut vec![]).unwrap();
    }

    #[test]
    fn notes_get_the_geometry_the_editor_needs() {
        let notes = validate_notes(
            Some(&json!([
                { "id": "n1", "text": "hello\nworld" },
                { "id": "n2", "text": "", "color": "red", "position": { "x": 5, "y": 6 } },
                { "id": "n3", "text": "x" },
            ])),
            &HashSet::new(),
        )
        .unwrap();
        assert_eq!(
            notes[0],
            json!({ "id": "n1", "text": "hello\nworld", "type": "free", "color": "green", "size": { "width": 275, "height": 66 }, "position": { "x": -375, "y": 0 } })
        );
        assert_eq!(notes[1]["color"], "red");
        assert_eq!(notes[1]["position"], json!({ "x": 5, "y": 6 }));
        assert_eq!(notes[2]["position"], json!({ "x": -375, "y": 90 }));
    }
}
