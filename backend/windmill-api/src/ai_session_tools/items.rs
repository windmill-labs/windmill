//! Draft writes for the item kinds that are plain records: schedules, triggers, resources, variables.

use std::sync::LazyLock;

use serde_json::{json, Map, Value};

use super::workspace::{finish_draft_write, persist_draft, write_base, Persist};
use super::{non_null, put, tool_parameters, Ctx, ToolResult, SCHEMAS};

pub(super) const TOOLS: &[&str] = &[
    "write_schedule",
    "write_trigger",
    "write_resource",
    "write_variable",
];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "write_schedule" => write_schedule(ctx, args).await,
        "write_trigger" => write_trigger(ctx, args).await,
        "write_resource" => write_resource(ctx, args).await,
        "write_variable" => write_variable(ctx, args).await,
        _ => unreachable!("{name} is not an items tool"),
    }
}

// ============================================================================
// Argument parsing. The session parses arguments with zod schemas; the same schemas
// reach this server as JSON Schema (`gen/tools.json`, `gen/schemas.json`), and `parse`
// applies them with zod's semantics rather than JSON Schema's: an undeclared key is
// dropped instead of refused, a `default` fills a missing key, a union yields the
// first branch that accepts the value.
// ============================================================================

fn tool_schema(name: &str) -> Value {
    tool_parameters(name)
        .map(|parameters| Value::Object(parameters.clone()))
        .unwrap_or_default()
}

/// A schedule's whole schema: the fields `write_schedule` lists plus the ones it takes
/// through `advanced`, which the session spreads to the top level before parsing.
static SCHEDULE_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    let mut schema = tool_schema("write_schedule");
    let advanced: Value = serde_json::from_str(&SCHEMAS.schedule).unwrap_or_default();
    if let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) {
        properties.shift_remove("advanced");
        if let Some(advanced) = advanced.get("properties").and_then(Value::as_object) {
            properties.extend(advanced.clone());
        }
    }
    schema
});

fn received(value: Option<&Value>) -> &'static str {
    match value {
        None => "undefined",
        Some(Value::Null) => "null",
        Some(Value::Bool(_)) => "boolean",
        Some(Value::Number(_)) => "number",
        Some(Value::String(_)) => "string",
        Some(Value::Array(_)) => "array",
        Some(Value::Object(_)) => "object",
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// `value` as `schema` accepts it, `None` standing for a missing one. Each violation is
/// pushed to `issues` as `<dotted path>: <message>`.
fn parse(
    schema: &Value,
    value: Option<&Value>,
    path: &str,
    issues: &mut Vec<String>,
) -> Option<Value> {
    let mut issue = |message: String| {
        let at = if path.is_empty() { "<root>" } else { path };
        issues.push(format!("{at}: {message}"));
        None
    };
    if value.is_none() {
        if let Some(default) = schema.get("default") {
            return Some(default.clone());
        }
    }
    if let Some(branches) = schema.get("anyOf").and_then(Value::as_array) {
        let is_null = |branch: &&Value| branch.get("type").and_then(Value::as_str) == Some("null");
        if value.is_some_and(Value::is_null) && branches.iter().any(|b| is_null(&b)) {
            return Some(Value::Null);
        }
        let branches: Vec<&Value> = branches.iter().filter(|b| !is_null(b)).collect();
        // A nullable field reports what its one real type refuses.
        if let [only] = branches[..] {
            return parse(only, value, path, issues);
        }
        for branch in branches {
            let mut refused = Vec::new();
            let parsed = parse(branch, value, path, &mut refused);
            if refused.is_empty() {
                return parsed;
            }
        }
        return issue("Invalid input".to_string());
    }
    if let Some(options) = schema.get("enum").and_then(Value::as_array) {
        if value.is_some_and(|v| options.contains(v)) {
            return value.cloned();
        }
        let options: Vec<String> = options.iter().map(Value::to_string).collect();
        return issue(format!(
            "Invalid option: expected one of {}",
            options.join("|")
        ));
    }
    let Some(kind) = schema.get("type").and_then(Value::as_str) else {
        return value.cloned();
    };
    match (kind, value) {
        ("boolean", Some(Value::Bool(_))) | ("null", Some(Value::Null)) => value.cloned(),
        ("string", Some(Value::String(text))) => {
            let matches = schema
                .get("pattern")
                .and_then(Value::as_str)
                .and_then(|pattern| regex::Regex::new(pattern).ok())
                .is_none_or(|pattern| pattern.is_match(text));
            if matches {
                value.cloned()
            } else if schema.get("format").and_then(Value::as_str) == Some("date-time") {
                issue("Invalid ISO datetime".to_string())
            } else {
                issue("Invalid string".to_string())
            }
        }
        ("number" | "integer", Some(Value::Number(number))) => {
            let n = number.as_f64().unwrap_or_default();
            let bound = |key: &str| schema.get(key).and_then(Value::as_f64);
            if kind == "integer" && n.fract() != 0.0 {
                issue("Invalid input: expected int, received number".to_string())
            } else if let Some(min) = bound("minimum").filter(|min| n < *min) {
                issue(format!("Too small: expected number to be >={min}"))
            } else if let Some(max) = bound("maximum").filter(|max| n > *max) {
                issue(format!("Too big: expected number to be <={max}"))
            } else {
                value.cloned()
            }
        }
        ("array", Some(Value::Array(items))) => {
            let any = json!({});
            let item_schema = schema.get("items").unwrap_or(&any);
            Some(Value::Array(
                items
                    .iter()
                    .enumerate()
                    .filter_map(|(i, item)| {
                        parse(item_schema, Some(item), &join(path, &i.to_string()), issues)
                    })
                    .collect(),
            ))
        }
        ("object", Some(Value::Object(fields))) => {
            let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
                return value.cloned();
            };
            let required = schema.get("required").and_then(Value::as_array);
            let mut out = Map::new();
            for (key, property) in properties {
                let field = fields.get(key);
                let optional = !required.is_some_and(|r| r.iter().any(|k| k == key));
                if field.is_none() && optional && property.get("default").is_none() {
                    continue;
                }
                if let Some(parsed) = parse(property, field, &join(path, key), issues) {
                    out.insert(key.clone(), parsed);
                }
            }
            if schema.get("additionalProperties") != Some(&Value::Bool(false)) {
                for (key, field) in fields {
                    if !properties.contains_key(key) {
                        out.insert(key.clone(), field.clone());
                    }
                }
            }
            Some(Value::Object(out))
        }
        _ => {
            let expected = match kind {
                "integer" => "number",
                "object" if schema.get("properties").is_none() => "record",
                other => other,
            };
            issue(format!(
                "Invalid input: expected {expected}, received {}",
                received(value)
            ))
        }
    }
}

fn parse_args(tool: &str, args: &Value) -> Result<Value, String> {
    parse_with(&tool_schema(tool), args)
}

fn parse_with(schema: &Value, args: &Value) -> Result<Value, String> {
    let mut issues = Vec::new();
    let parsed = parse(schema, Some(args), "", &mut issues);
    match parsed {
        Some(parsed) if issues.is_empty() => Ok(parsed),
        _ => Err(format!("Invalid arguments. Issues: {}", issues.join("; "))),
    }
}

/// Key paths `supplied` carries that parsing dropped. Every sub-field of a schedule's
/// `retry` is optional, so a guessed shape parses clean, loses its misspelled keys and
/// would save a policy that does nothing.
fn dropped_keys(supplied: &Value, parsed: Option<&Value>, prefix: &str, out: &mut Vec<String>) {
    let Value::Object(supplied) = supplied else {
        if parsed.is_none() && !prefix.is_empty() {
            out.push(prefix.to_string());
        }
        return;
    };
    match parsed {
        Some(parsed @ (Value::Object(_) | Value::Array(_))) => {
            for (key, value) in supplied {
                dropped_keys(value, parsed.get(key), &join(prefix, key), out);
            }
        }
        _ => {
            if !supplied.is_empty() && !prefix.is_empty() {
                out.push(prefix.to_string());
            }
        }
    }
}

fn describe_dropped_schedule_options(dropped: &[String]) -> String {
    let is_field = |path: &&String| {
        let field = path.split('.').next().unwrap_or_default();
        field != "override" && SCHEDULE_SCHEMA["properties"].get(field).is_some()
    };
    let (misshaped, unknown): (Vec<&String>, Vec<&String>) = dropped.iter().partition(is_field);
    let list = |paths: &[&String]| {
        paths
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut parts = Vec::new();
    if !misshaped.is_empty() {
        parts.push(format!("These schedule options do not match their schema and would have been dropped: {}. Call get_schedule_schema for their exact shape.", list(&misshaped)));
    }
    if !unknown.is_empty() {
        parts.push(format!(
            "These are not schedule fields: {}.",
            list(&unknown)
        ));
    }
    parts.join(" ")
}

// ============================================================================
// Draft values
// ============================================================================

fn into_object(value: Option<Value>) -> Map<String, Value> {
    match value {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

/// A schedule's or trigger's draft: the base without what the server stamps on a
/// deployed row, then the fields of this write.
fn merge_config(base: Option<Value>, config: Value, path: &str) -> Map<String, Value> {
    let mut draft = into_object(base);
    for key in ["workspace_id", "edited_by", "edited_at", "email", "error"] {
        draft.shift_remove(key);
    }
    draft.extend(into_object(Some(config)));
    draft.insert("path".into(), json!(path));
    draft
}

fn resource_to_draft_state(resource: &Value) -> Value {
    let mut state = Map::new();
    state.insert("path".into(), resource["path"].clone());
    state.insert(
        "description".into(),
        non_null(resource.get("description"))
            .cloned()
            .unwrap_or_else(|| json!("")),
    );
    state.insert(
        "args".into(),
        non_null(resource.get("value"))
            .cloned()
            .unwrap_or_else(|| json!({})),
    );
    put(
        &mut state,
        "labels",
        non_null(resource.get("labels")).cloned(),
    );
    state.insert(
        "wsSpecific".into(),
        non_null(resource.get("ws_specific"))
            .cloned()
            .unwrap_or(json!(false)),
    );
    state.insert("resource_type".into(), resource["resource_type"].clone());
    Value::Object(state)
}

fn resource_draft(args: &Value, base: Option<Value>) -> Value {
    let mut draft = into_object(base);
    let merged = |arg: &str, key: &str| {
        non_null(args.get(arg))
            .or(non_null(draft.get(key)))
            .cloned()
    };
    let description = merged("description", "description").unwrap_or_else(|| json!(""));
    let value = merged("value", "args").unwrap_or_else(|| json!({}));
    let labels = merged("labels", "labels");
    let ws_specific = merged("ws_specific", "wsSpecific").unwrap_or(json!(false));
    let resource_type = merged("resource_type", "resource_type");
    draft.insert("path".into(), args["path"].clone());
    draft.insert("description".into(), description);
    draft.insert("args".into(), value);
    put(&mut draft, "labels", labels);
    draft.insert("wsSpecific".into(), ws_specific);
    put(&mut draft, "resource_type", resource_type);
    Value::Object(draft)
}

fn variable_to_draft_state(variable: &Value) -> Value {
    let field = |key: &str| non_null(variable.get(key)).cloned();
    // An OAuth variable's value belongs to the refresh flow, and the API hands back a
    // freshly refreshed live token for an expired one even undecrypted: carried into a
    // draft, it would pin a rotating value to a static one at the next deploy.
    let oauth_managed = variable.get("is_oauth").and_then(Value::as_bool) == Some(true)
        || field("account").is_some();
    let value = if oauth_managed { None } else { field("value") };
    let mut state = Map::new();
    state.insert("path".into(), variable["path"].clone());
    state.insert(
        "variable".into(),
        json!({
            "value": value.unwrap_or_else(|| json!("")),
            "is_secret": variable["is_secret"],
            "description": field("description").unwrap_or_else(|| json!("")),
        }),
    );
    put(&mut state, "labels", field("labels"));
    state.insert(
        "wsSpecific".into(),
        field("ws_specific").unwrap_or(json!(false)),
    );
    for key in ["account", "is_oauth", "expires_at", "value_valid_until"] {
        put(&mut state, key, field(key));
    }
    Value::Object(state)
}

/// The draft a `write_variable` call leaves. A variable's value is never read back, so
/// an omitted `value` keeps whatever the base holds: `""` for "nothing staged", or the
/// `$encrypted:` marker the draft store turned a staged secret into.
fn variable_draft(args: &Value, base: Option<Value>) -> Result<Value, String> {
    let path = args["path"].as_str().unwrap_or_default();
    let arg = |key: &str| non_null(args.get(key)).cloned();
    let stored = |key: &str| {
        base.as_ref()
            .and_then(|b| b.get("variable"))
            .and_then(|v| non_null(v.get(key)).cloned())
    };
    let value = arg("value");
    if base.is_none() && (value.is_none() || arg("is_secret").is_none()) {
        return Err(format!("Variable \"{path}\" does not exist yet, so creating it requires both value and is_secret."));
    }
    let was_secret = stored("is_secret").and_then(|s| s.as_bool());
    let is_secret = arg("is_secret")
        .and_then(|s| s.as_bool())
        .or(was_secret)
        .unwrap_or(false);
    let empty = json!("");
    // "" is how a secret draft says "nothing staged", so it cannot also mean "set the
    // secret to empty".
    if is_secret && value.as_ref() == Some(&empty) {
        return Err(format!("An empty string is not a valid value for secret variable \"{path}\". Omit value to keep the stored secret, or pass the real new one."));
    }
    let next_value = value.clone().or_else(|| stored("value"));
    if is_secret && was_secret == Some(false) && next_value.as_ref() == Some(&empty) {
        return Err(format!("Cannot make variable \"{path}\" secret without a value: it currently holds an empty one, so there would be nothing to encrypt. Pass the value it should hold."));
    }
    // The stored value of a secret cannot be read, and an `$encrypted:` marker is only
    // decrypted at deploy while the target stays secret.
    if !is_secret && was_secret == Some(true) && value.is_none() {
        return Err(format!("Cannot turn secret variable \"{path}\" into a non-secret one without a value: its stored value cannot be read, so it would be replaced by an empty one. Pass the new plaintext value, or leave is_secret unset to keep it secret."));
    }
    let description = arg("description").or_else(|| stored("description"));

    let mut draft = into_object(base);
    let merged = |arg_key: &str, key: &str| {
        non_null(args.get(arg_key))
            .or(non_null(draft.get(key)))
            .cloned()
    };
    let labels = merged("labels", "labels");
    let ws_specific = merged("ws_specific", "wsSpecific").unwrap_or(json!(false));
    let extras = ["account", "is_oauth", "expires_at"].map(|key| (key, merged(key, key)));
    draft.insert("path".into(), json!(path));
    draft.insert(
        "variable".into(),
        json!({
            "value": next_value.unwrap_or(empty),
            "is_secret": is_secret,
            "description": description.unwrap_or_else(|| json!("")),
        }),
    );
    put(&mut draft, "labels", labels);
    draft.insert("wsSpecific".into(), ws_specific);
    for (key, value) in extras {
        put(&mut draft, key, value);
    }
    // Deploying a draft reads an absent expiry as cleared. The draft starts from the base, so a
    // `null` has to remove its date rather than skip it.
    if args.get("value_valid_until").is_some_and(Value::is_null) {
        draft.remove("value_valid_until");
    } else {
        put(&mut draft, "value_valid_until", arg("value_valid_until"));
    }
    Ok(Value::Object(draft))
}

// ============================================================================
// Tools
// ============================================================================

/// The shared write: build on the caller's draft, else on the deployed item (shaped
/// into a draft value by `from_deployed`), else from nothing.
async fn write_draft(
    ctx: &Ctx<'_>,
    item_type: &str,
    trigger_kind: Option<&str>,
    path: &str,
    force: bool,
    from_deployed: impl FnOnce(Value) -> Value,
    build: impl FnOnce(Option<Value>) -> Result<Value, String>,
) -> ToolResult {
    ctx.saving_draft(item_type, path);
    let base = write_base(ctx, item_type, path, trigger_kind).await?;
    let existed = base.base.is_some();
    let from_draft = base.from_draft;
    let draft = build(
        base.base
            .map(|b| if from_draft { b } else { from_deployed(b) }),
    )?;
    let persisted = persist_draft(
        ctx,
        item_type,
        path,
        trigger_kind,
        &draft,
        base.last_sync.as_deref(),
        force,
    )
    .await?;
    Ok(match persisted {
        Persist::Saved(item) => finish_draft_write(ctx, &item, existed),
        Persist::Failed(text) => text,
    })
}

fn take_override(parsed: &mut Value) -> bool {
    parsed
        .as_object_mut()
        .and_then(|o| o.shift_remove("override"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

async fn write_schedule(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    // A named argument outranks the same key inside `advanced`.
    let mut merged = into_object(args.get("advanced").cloned());
    for (key, value) in args.as_object().into_iter().flatten() {
        if key != "advanced" {
            merged.insert(key.clone(), value.clone());
        }
    }
    let merged = Value::Object(merged);
    let mut parsed = parse_with(&SCHEDULE_SCHEMA, &merged)?;
    let mut dropped = Vec::new();
    dropped_keys(&merged, Some(&parsed), "", &mut dropped);
    if !dropped.is_empty() {
        return Err(describe_dropped_schedule_options(&dropped));
    }
    let force = take_override(&mut parsed);
    let path = parsed["path"].as_str().unwrap_or_default().to_string();
    write_draft(
        ctx,
        "schedule",
        None,
        &path,
        force,
        |deployed| deployed,
        |base| Ok(Value::Object(merge_config(base, parsed, &path))),
    )
    .await
}

async fn write_trigger(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let mut parsed = parse_args("write_trigger", args)?;
    let force = take_override(&mut parsed);
    let kind = parsed["kind"].as_str().unwrap_or_default();
    let schema: Value = SCHEMAS
        .trigger
        .get(kind)
        .and_then(|schema| serde_json::from_str(schema).ok())
        .ok_or_else(|| format!("Unknown trigger kind `{kind}`"))?;
    // The draft is filed under `kind`, so a config of another kind is refused here
    // rather than stored as a draft no editor or deploy can read.
    let mut issues = Vec::new();
    let config = parse(&schema, parsed.get("config"), "", &mut issues);
    let config = match config {
        Some(config) if issues.is_empty() => config,
        _ => {
            return Err(format!(
            "Invalid config for a \"{kind}\" trigger. Call get_trigger_schema with kind \"{kind}\" for its exact fields. Issues: {}",
            issues.join("; ")
        ))
        }
    };
    let path = config["path"].as_str().unwrap_or_default().to_string();
    write_draft(
        ctx,
        "trigger",
        Some(kind),
        &path,
        force,
        |deployed| deployed,
        |base| {
            let mut draft = merge_config(base, config, &path);
            // Optional in the schema, but a NOT NULL column. Defaulted on the merged
            // draft so an omitted field keeps the existing trigger's value.
            if kind == "email" && non_null(draft.get("workspaced_local_part")).is_none() {
                draft.insert("workspaced_local_part".into(), json!(false));
            }
            Ok(Value::Object(draft))
        },
    )
    .await
}

async fn write_resource(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let mut parsed = parse_args("write_resource", args)?;
    let force = take_override(&mut parsed);
    let path = parsed["path"].as_str().unwrap_or_default().to_string();
    write_draft(
        ctx,
        "resource",
        None,
        &path,
        force,
        |deployed| resource_to_draft_state(&deployed),
        |base| Ok(resource_draft(&parsed, base)),
    )
    .await
}

async fn write_variable(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let mut parsed = parse_args("write_variable", args)?;
    let force = take_override(&mut parsed);
    let path = parsed["path"].as_str().unwrap_or_default().to_string();
    // A variable's value is never interpolated, so its own `$var:` reference is the
    // reference syntax echoed back, not a value.
    if let Some(value) = parsed.get("value").and_then(Value::as_str) {
        if value == format!("$var:{path}") {
            return Err(format!("\"{value}\" is not a valid value for variable \"{path}\" — it is a self-reference. Omit value to keep the current one."));
        }
    }
    write_draft(
        ctx,
        "variable",
        None,
        &path,
        force,
        |deployed| variable_to_draft_state(&deployed),
        |base| variable_draft(&parsed, base),
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn issues_of(schema: &Value, value: Value) -> (Option<Value>, Vec<String>) {
        let mut issues = Vec::new();
        let parsed = parse(schema, Some(&value), "", &mut issues);
        (parsed, issues)
    }

    #[test]
    fn trigger_config_parses_like_the_session() {
        let kafka: Value = serde_json::from_str(&SCHEMAS.trigger["kafka"]).unwrap();
        let (parsed, issues) = issues_of(
            &kafka,
            json!({
                "path": "p", "script_path": "s", "is_flow": false, "kafka_resource_path": "r",
                "group_id": "g", "topics": ["a"], "unknown_field": 1,
                "filters": [{"key": "k", "value": 1, "zzz": 2}, {"any_of": [{"a": 1}]}],
            }),
        );
        assert_eq!(issues, Vec::<String>::new());
        let parsed = parsed.unwrap();
        assert_eq!(parsed.get("unknown_field"), None);
        assert_eq!(
            parsed["filters"],
            json!([{"key": "k", "value": 1}, {"any_of": [{"a": 1}]}])
        );
        assert_eq!(parsed["filter_logic"], "and");
        assert_eq!(parsed["auto_commit"], true);

        let (_, issues) = issues_of(
            &kafka,
            json!({
                "path": 5, "is_flow": "x", "topics": "a", "filters": [3], "group_id": null,
                "auto_offset_reset": "x", "kafka_resource_path": "r",
                "retry": {"exponential": {"seconds": 0, "random_factor": 1.5}, "retry_if": {}},
            }),
        );
        assert_eq!(
            issues,
            [
                "path: Invalid input: expected string, received number",
                "script_path: Invalid input: expected string, received undefined",
                "is_flow: Invalid input: expected boolean, received string",
                "group_id: Invalid input: expected string, received null",
                "topics: Invalid input: expected array, received string",
                "filters.0: Invalid input",
                "auto_offset_reset: Invalid option: expected one of \"latest\"|\"earliest\"",
                "retry.exponential.seconds: Too small: expected number to be >=1",
                "retry.exponential.random_factor: Invalid input: expected int, received number",
                "retry.retry_if.expr: Invalid input: expected string, received undefined",
            ]
        );
    }

    #[test]
    fn schedule_reports_what_parsing_would_drop() {
        let merged = json!({
            "path": "p", "schedule": "s", "timezone": "UTC", "script_path": "x",
            "is_flow": false, "args": null, "bogus": 1, "override": true,
            "retry": {"constant": {"attempts": 2, "secs": 3}},
        });
        let parsed = parse_with(&SCHEDULE_SCHEMA, &merged).unwrap();
        let mut dropped = Vec::new();
        dropped_keys(&merged, Some(&parsed), "", &mut dropped);
        assert_eq!(
            describe_dropped_schedule_options(&dropped),
            "These schedule options do not match their schema and would have been dropped: retry.constant.secs. Call get_schedule_schema for their exact shape. These are not schedule fields: bogus."
        );
        assert!(parse_with(
            &SCHEDULE_SCHEMA,
            &json!({"path": "p", "paused_until": "tomorrow"})
        )
        .unwrap_err()
        .contains("paused_until: Invalid ISO datetime"));
    }

    #[test]
    fn variable_value_rules() {
        let secret = json!({
            "path": "p",
            "variable": {"value": "$encrypted:abc", "is_secret": true, "description": "d"},
            "wsSpecific": false,
        });
        let plain_empty =
            json!({"path": "p", "variable": {"value": "", "is_secret": false, "description": ""}});

        assert!(variable_draft(&json!({"path": "p", "value": "v"}), None).is_err());
        let kept = variable_draft(
            &json!({"path": "p", "description": "new"}),
            Some(secret.clone()),
        )
        .unwrap();
        assert_eq!(
            kept["variable"],
            json!({"value": "$encrypted:abc", "is_secret": true, "description": "new"})
        );
        assert!(variable_draft(&json!({"path": "p", "value": ""}), Some(secret.clone())).is_err());
        assert!(variable_draft(
            &json!({"path": "p", "is_secret": false}),
            Some(secret.clone())
        )
        .is_err());
        let revealed = variable_draft(
            &json!({"path": "p", "is_secret": false, "value": "v"}),
            Some(secret),
        )
        .unwrap();
        assert_eq!(revealed["variable"]["value"], "v");
        assert!(
            variable_draft(&json!({"path": "p", "is_secret": true}), Some(plain_empty)).is_err()
        );

        let dated = variable_to_draft_state(&json!({
            "path": "p", "value": "v", "is_secret": false, "value_valid_until": "2027-01-15T00:00:00Z",
        }));
        let edit = |args: Value| variable_draft(&args, Some(dated.clone())).unwrap();
        assert_eq!(
            edit(json!({"path": "p", "description": "new"}))["value_valid_until"],
            "2027-01-15T00:00:00Z"
        );
        assert!(edit(json!({"path": "p", "value_valid_until": null}))
            .get("value_valid_until")
            .is_none());

        let oauth = variable_to_draft_state(&json!({
            "path": "p", "value": "live-token", "is_secret": true, "account": 3,
            "description": null, "expires_at": null,
        }));
        assert_eq!(
            oauth,
            json!({
                "path": "p",
                "variable": {"value": "", "is_secret": true, "description": ""},
                "wsSpecific": false,
                "account": 3,
            })
        );
    }
}
