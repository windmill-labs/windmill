//! Running scripts and flows, test runs on drafts, and reading runs back.

use std::collections::HashMap;
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Map, Value};

use super::workspace::{
    api_message, get_draft, infer_script_schema, load_flow, load_script, path_enc,
};
use super::{enc, js_head, js_len, js_tail, opt_str_arg, pretty, str_arg, truthy, Ctx, ToolResult};

pub(super) const TOOLS: &[&str] = &[
    "test_run_script",
    "run_script",
    "test_run_flow",
    "run_flow",
    "test_run_step",
    "list_runs",
    "get_run",
    "cancel_job",
];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "test_run_script" => test_run_script(ctx, args).await,
        "run_script" => run_deployed(ctx, args, false).await,
        "test_run_flow" => test_run_flow(ctx, args).await,
        "run_flow" => run_deployed(ctx, args, true).await,
        "test_run_step" => test_run_step(ctx, args).await,
        "list_runs" => list_runs(ctx, args).await,
        "get_run" => get_run(ctx, args).await,
        "cancel_job" => cancel_job(ctx, args).await,
        _ => unreachable!("{name} is not a runs tool"),
    }
}

const MAX_RESULT_LENGTH: usize = 12000;
const MAX_LOG_LENGTH: usize = 4000;
const DEFAULT_WAIT_MS: f64 = 15000.0;
const MAX_WAIT_MS: f64 = 120000.0;
const MAX_SUBMITTED_ARGS_LENGTH: usize = 4000;

/// `JSON.stringify(value, null, 1)`, the indent `get_run` renders with.
fn pretty1(value: &Value) -> String {
    let mut out = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut out, formatter);
    value
        .serialize(&mut serializer)
        .expect("a JSON value serializes");
    String::from_utf8(out).unwrap_or_default()
}

// ============================================================================
// Job helpers shared with the other groups.
// ============================================================================

fn job_id(response: Value) -> String {
    match response {
        Value::String(id) => id.trim().to_string(),
        other => other.to_string(),
    }
}

/// Queue a script preview and return its job id. `path` files the run under an item.
pub(super) async fn run_script_preview(
    ctx: &Ctx<'_>,
    path: Option<&str>,
    language: &str,
    content: &str,
    args: Value,
) -> Result<String, String> {
    let mut body = Map::new();
    if let Some(path) = path {
        body.insert("path".into(), path.into());
    }
    body.insert("content".into(), content.into());
    body.insert("language".into(), language.into());
    body.insert("args".into(), args);
    ctx.post(&ctx.ws("jobs/run/preview"), Value::Object(body))
        .await
        .map(job_id)
}

/// Queue a flow preview of `value` and return its job id. `memory_id` names the
/// conversation a chat-enabled flow's run belongs to.
pub(super) async fn run_flow_preview(
    ctx: &Ctx<'_>,
    path: Option<&str>,
    value: &Value,
    args: Value,
    memory_id: Option<&str>,
) -> Result<String, String> {
    let mut body = Map::new();
    if let Some(path) = path {
        body.insert("path".into(), path.into());
    }
    body.insert("value".into(), value.clone());
    body.insert("args".into(), args);
    let query = memory_id
        .map(|id| format!("?memory_id={}", enc(id)))
        .unwrap_or_default();
    ctx.post(
        &ctx.ws(&format!("jobs/run/preview_flow{query}")),
        Value::Object(body),
    )
    .await
    .map(job_id)
}

/// How many seconds a run tool waits before leaving the job in the background, from
/// its `background` and `wait_seconds` arguments: 15 by default, 120 at most.
pub(super) fn wait_budget(tool_args: &Value) -> u64 {
    if tool_args.get("background").and_then(Value::as_bool) == Some(true) {
        return 0;
    }
    let ms = tool_args
        .get("wait_seconds")
        .and_then(Value::as_f64)
        .map_or(DEFAULT_WAIT_MS, |seconds| seconds.max(0.0) * 1000.0)
        .min(MAX_WAIT_MS);
    (ms / 1000.0).ceil() as u64
}

/// Poll a job once a second for up to `seconds`. The completed job, fetched once more
/// with its logs when `logs` is set, or `None` when it is still running by then. A
/// failed poll is retried on the next tick.
pub(super) async fn wait_job(ctx: &Ctx<'_>, id: &str, seconds: u64, logs: bool) -> Option<Value> {
    let path = |no_logs: bool| {
        ctx.ws(&format!(
            "jobs_u/get/{}?no_logs={no_logs}&no_code=true",
            enc(id)
        ))
    };
    for _ in 0..seconds {
        tokio::time::sleep(Duration::from_secs(1)).await;
        if let Ok(job) = ctx.get(&path(true)).await {
            if job["type"] == "CompletedJob" {
                return Some(match logs {
                    true => ctx.get(&path(false)).await.unwrap_or(job),
                    false => job,
                });
            }
        }
    }
    None
}

/// What the model reads for a job that outlived its wait.
pub(super) fn backgrounded_summary(id: &str, label: &str) -> String {
    format!(
        "Job {id} for \"{label}\" is taking a while and is now running in the background. \
         To inspect it, call get_run with id=\"{id}\" (or list_runs); \
         to stop it, call cancel_job with id=\"{id}\"."
    )
}

/// A job result as text: a string as it is, anything else as indented JSON.
pub(super) fn format_result(result: Option<&Value>) -> String {
    match result {
        Some(Value::String(text)) => text.clone(),
        other => pretty(other.unwrap_or(&Value::Null)),
    }
}

/// The last 4000 chars of a job's logs, or `None` when it logged nothing.
pub(super) fn format_logs(logs: Option<&str>) -> Option<&str> {
    logs.filter(|logs| !logs.trim().is_empty())
        .map(|logs| js_tail(logs, MAX_LOG_LENGTH))
}

/// A completed job as the run tools report it: outcome, result head (12k chars) and
/// logs tail.
pub(super) fn format_result_summary(job: &Value) -> String {
    let success = truthy(job.get("success"));
    let result = format_result(job.get("result"));
    format!(
        "Result ({})\n\n{}\n\nLogs:\n\n{}",
        if success { "SUCCESS" } else { "FAILED" },
        js_head(&result, MAX_RESULT_LENGTH),
        format_logs(job.get("logs").and_then(Value::as_str)).unwrap_or("No logs available"),
    )
}

/// Settle a just-queued job (`started` is the queueing call's answer) into a run
/// tool's result: wait within the tool's budget, then the result summary or the
/// background notice. `action` is `test` or `run`, `label` names the job in the notice.
pub(super) async fn finish_run(
    ctx: &Ctx<'_>,
    started: Result<String, String>,
    label: &str,
    is_flow: bool,
    action: &str,
    tool_args: &Value,
) -> ToolResult {
    let id = started.map_err(|e| format!("Failed to execute {action}: {}", api_message(&e)))?;
    let run_page = format!("\n\nRun page: {}", ctx.page_url(&format!("/run/{id}")));
    let Some(job) = wait_job(ctx, &id, wait_budget(tool_args), true).await else {
        return Ok(format!("{}{run_page}", backgrounded_summary(&id, label)));
    };
    let mut summary = format_result_summary(&job);
    if is_flow && !truthy(job.get("success")) {
        summary.push_str(&format!(
            "\n\nFor per-step statuses and results (subflow steps included), call get_run with id=\"{id}\"."
        ));
    }
    summary.push_str(&run_page);
    Ok(summary)
}

// ============================================================================
// Arguments: conformed to the schema of what is about to run, secrets minted.
// ============================================================================

const REFERENCE_PREFIXES: [&str; 3] = ["$var:", "$res:", "$jsonvar:"];

fn is_reference(value: &Value) -> bool {
    value.as_str().is_some_and(|s| {
        REFERENCE_PREFIXES
            .iter()
            .any(|prefix| s.starts_with(prefix))
    })
}

fn is_secret_prop(prop: &Value) -> bool {
    truthy(prop.get("password"))
}

fn is_file_prop(prop: &Value) -> bool {
    prop["contentEncoding"] == "base64" || prop["items"]["contentEncoding"] == "base64"
}

/// Apply `visit` to every value whose declaration matches `is_leaf`, at any depth;
/// `None` removes it. Descends on the value's shape, through every `oneOf` branch.
fn map_leaves(
    value: &Value,
    prop: Option<&Value>,
    is_leaf: fn(&Value) -> bool,
    visit: &mut dyn FnMut(&Value, &Value, &[Value]) -> Option<Value>,
    path: &mut Vec<Value>,
) -> Value {
    let items = prop.and_then(|p| p.get("items")).filter(|v| !v.is_null());
    match value {
        Value::Array(list) => Value::Array(
            list.iter()
                .enumerate()
                .map(|(i, item)| {
                    path.push(json!(i));
                    let mapped = map_leaves(item, items.or(prop), is_leaf, visit, path);
                    path.pop();
                    mapped
                })
                .collect(),
        ),
        Value::Object(map) => {
            let mut bags: Vec<&Map<String, Value>> = Vec::new();
            if let Some(properties) = prop.and_then(|p| p["properties"].as_object()) {
                bags.push(properties);
            }
            for branch in prop
                .and_then(|p| p["oneOf"].as_array())
                .into_iter()
                .flatten()
            {
                if let Some(properties) = branch["properties"].as_object() {
                    bags.push(properties);
                }
            }
            if bags.is_empty() {
                return match items {
                    Some(items) => map_leaves(value, Some(items), is_leaf, visit, path),
                    None => value.clone(),
                };
            }
            let mut result = Map::new();
            for (key, current) in map {
                let declared: Vec<&Value> = bags.iter().filter_map(|bag| bag.get(key)).collect();
                path.push(json!(key));
                if let Some(leaf) = declared.iter().find(|d| is_leaf(d)) {
                    if let Some(mapped) = visit(current, leaf, path) {
                        result.insert(key.clone(), mapped);
                    }
                } else {
                    let mut mapped = current.clone();
                    for declaration in declared {
                        mapped = map_leaves(&mapped, Some(declaration), is_leaf, visit, path);
                    }
                    result.insert(key.clone(), mapped);
                }
                path.pop();
            }
            Value::Object(result)
        }
        other => other.clone(),
    }
}

fn map_arg_leaves(
    args: &Map<String, Value>,
    schema: &Value,
    is_leaf: fn(&Value) -> bool,
    visit: &mut dyn FnMut(&Value, &Value, &[Value]) -> Option<Value>,
) -> Map<String, Value> {
    let root = json!({ "properties": schema.get("properties").filter(|p| p.is_object()).cloned().unwrap_or_else(|| json!({})) });
    match map_leaves(
        &Value::Object(args.clone()),
        Some(&root),
        is_leaf,
        visit,
        &mut Vec::new(),
    ) {
        Value::Object(map) => map,
        _ => Map::new(),
    }
}

/// `$var:` hands the job the variable's text and `$jsonvar:` the parsed value, which a
/// field that cannot hold a string needs.
fn reference_prefix(prop: &Value, value: &Value) -> &'static str {
    if value.is_string() && prop["type"] != "object" && prop["type"] != "array" {
        "$var:"
    } else {
        "$jsonvar:"
    }
}

/// Turn every sensitive argument into a reference: a literal is minted into a secret
/// variable that expires on its own, so the job's stored arguments name a secret
/// instead of holding one.
async fn process_secret_args(
    ctx: &Ctx<'_>,
    args: Map<String, Value>,
    schema: &Value,
) -> Result<Map<String, Value>, String> {
    if !schema.get("properties").is_some_and(Value::is_object) {
        return Ok(args);
    }
    let username = ctx.auth.username.split('@').next().unwrap_or_default();
    if username.is_empty() {
        return Ok(args);
    }

    let mut pending: Vec<(String, &'static str, Value)> = Vec::new();
    map_arg_leaves(&args, schema, is_secret_prop, &mut |value, prop, path| {
        let holds_secret = !value.is_null() && value != "" && !is_reference(value);
        if holds_secret {
            pending.push((
                Value::from(path).to_string(),
                reference_prefix(prop, value),
                value.clone(),
            ));
        }
        Some(value.clone())
    });

    let mut minted: HashMap<String, String> = HashMap::new();
    for (key, prefix, value) in pending {
        let variable = format!(
            "u/{username}/secret_arg/{}",
            windmill_common::utils::rd_string(12)
        );
        let secret = match &value {
            Value::String(text) if prefix == "$var:" => text.clone(),
            other => other.to_string(),
        };
        let expires_at = (chrono::Utc::now() + chrono::Duration::days(7))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        ctx.post(
            &ctx.ws("variables/create"),
            json!({
                "value": secret,
                "is_secret": true,
                "path": variable,
                "description": "Ephemeral secret variable",
                "expires_at": expires_at,
            }),
        )
        .await?;
        minted.insert(key, format!("{prefix}{variable}"));
    }

    Ok(map_arg_leaves(
        &args,
        schema,
        is_secret_prop,
        &mut |value, prop, path| {
            if let Some(reference) = minted.get(&Value::from(path).to_string()) {
                return Some(reference.as_str().into());
            }
            // A plain variable named for a field that cannot hold a string: its contents
            // are meant, read the way the field needs them.
            match value.as_str().and_then(|s| s.strip_prefix("$var:")) {
                Some(variable) if reference_prefix(prop, value) == "$jsonvar:" => {
                    Some(format!("$jsonvar:{variable}").into())
                }
                _ => Some(value.clone()),
            }
        },
    ))
}

fn file_marker(base64: &str) -> String {
    let bytes = js_len(base64) * 3 / 4;
    if bytes < 1024 * 1024 {
        format!(
            "<file: {} KB>",
            ((bytes as f64 / 1024.0).round() as usize).max(1)
        )
    } else {
        format!("<file: {:.1} MB>", bytes as f64 / 1024.0 / 1024.0)
    }
}

/// The arguments as the model may read them back: secrets hidden, file bytes replaced
/// by their size.
fn redact_args(args: &Map<String, Value>, schema: &Value) -> Map<String, Value> {
    let secrets = map_arg_leaves(
        args,
        schema,
        is_secret_prop,
        &mut |value, _, _| match value {
            Value::Null => None,
            value if value == "" || is_reference(value) => Some(value.clone()),
            _ => Some("<hidden>".into()),
        },
    );
    let mark = |value: &Value| match value {
        Value::String(base64) => file_marker(base64).into(),
        other => other.clone(),
    };
    map_arg_leaves(
        &secrets,
        schema,
        is_file_prop,
        &mut |value, _, _| match value {
            Value::Array(files) => Some(files.iter().map(mark).collect()),
            other => Some(mark(other)),
        },
    )
}

/// Keep the arguments the schema declares, naming the rest. A schema without
/// `properties` declares nothing.
fn drop_undeclared_args(
    args: Map<String, Value>,
    schema: &Value,
) -> (Map<String, Value>, Vec<String>) {
    let properties = schema.get("properties").and_then(Value::as_object);
    let mut undeclared = Vec::new();
    let kept = args
        .into_iter()
        .filter(|(key, _)| {
            let declared = properties.is_some_and(|p| p.contains_key(key));
            if !declared {
                undeclared.push(key.clone());
            }
            declared
        })
        .collect();
    (kept, undeclared)
}

/// A field the schema disables is not the caller's to set: the run sends its default.
/// Returns the keys whose proposed value was overwritten.
fn enforce_disabled_defaults(args: &mut Map<String, Value>, schema: &Value) -> Vec<String> {
    let mut reset = Vec::new();
    for (key, prop) in schema
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let (true, Some(default)) = (truthy(prop.get("disabled")), prop.get("default")) else {
            continue;
        };
        if args.get(key).is_some_and(|proposed| proposed != default) {
            reset.push(key.clone());
        }
        args.insert(key.clone(), default.clone());
    }
    reset
}

struct Prepared {
    args: Map<String, Value>,
    /// Lines telling the model how what ran differs from what it proposed.
    notes: Vec<String>,
}

fn plural<'a>(keys: &[String], one: &'a str, many: &'a str) -> &'a str {
    if keys.len() > 1 {
        many
    } else {
        one
    }
}

/// The arguments a run carries, from the ones proposed. `schema` is the input schema
/// of the version about to run; `None` when it is unknown, which conforms nothing.
/// `Err` is the message the tool returns when a secret could not be stored.
async fn prepare_args(
    ctx: &Ctx<'_>,
    path: &str,
    proposed: Map<String, Value>,
    schema: Option<&Value>,
    schema_noun: &str,
) -> Result<Prepared, String> {
    let Some(schema) = schema else {
        return Ok(Prepared { args: proposed, notes: Vec::new() });
    };
    let (mut args, undeclared) = drop_undeclared_args(proposed.clone(), schema);
    let reset = enforce_disabled_defaults(&mut args, schema);
    let args = process_secret_args(ctx, args, schema).await.map_err(|e| {
        format!(
            "Failed to store the sensitive arguments of \"{path}\": {}",
            api_message(&e)
        )
    })?;

    let mut notes = Vec::new();
    let redacted = redact_args(&args, schema);
    if redacted != proposed {
        let json = Value::Object(redacted).to_string();
        notes.push(if js_len(&json) > MAX_SUBMITTED_ARGS_LENGTH {
            format!(
                "Ran with arguments: {}... (truncated)",
                js_head(&json, MAX_SUBMITTED_ARGS_LENGTH)
            )
        } else {
            format!("Ran with arguments: {json}")
        });
    }
    if !reset.is_empty() {
        notes.push(format!(
            "The {schema_noun} schema disables {}, so the run used {} rather than the proposed {}. Do not propose {} again.",
            reset.join(", "),
            plural(&reset, "its default", "their defaults"),
            plural(&reset, "value", "values"),
            plural(&reset, "it", "them"),
        ));
    }
    if !undeclared.is_empty() {
        notes.push(format!(
            "The {schema_noun} schema does not declare {}, so {} not sent. Re-read the input schema and use the arguments it declares.",
            undeclared.join(", "),
            plural(&undeclared, "it was", "they were"),
        ));
    }
    Ok(Prepared { args, notes })
}

fn proposed_args(tool_args: &Value) -> Map<String, Value> {
    tool_args
        .get("args")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn has_properties(schema: Option<&Value>) -> bool {
    schema
        .and_then(|s| s.get("properties"))
        .and_then(Value::as_object)
        .is_some_and(|p| !p.is_empty())
}

/// The input schema a test run conforms its arguments to: the stored one where it
/// declares fields and speaks for the entrypoint about to run, else inferred from the
/// code. `None` when this build has no parser for the language.
fn schema_for_test_run(
    language: &str,
    content: &str,
    stored: Option<Value>,
    entrypoint: Option<&str>,
) -> Option<Value> {
    if entrypoint.is_none() && has_properties(stored.as_ref()) {
        return stored;
    }
    infer_script_schema(language, content, None, entrypoint)
}

async fn conclude(
    ctx: &Ctx<'_>,
    tool_args: &Value,
    mut notes: Vec<String>,
    started: Result<String, String>,
    label: &str,
    is_flow: bool,
    action: &str,
) -> ToolResult {
    notes.push(finish_run(ctx, started, label, is_flow, action, tool_args).await?);
    Ok(notes.join("\n"))
}

// ============================================================================
// Scripts and flows
// ============================================================================

async fn test_run_script(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let script = load_script(ctx, path).await?;
    let schema = schema_for_test_run(&script.language, &script.content, script.schema, None);
    let prepared =
        match prepare_args(ctx, path, proposed_args(args), schema.as_ref(), "script").await {
            Ok(prepared) => prepared,
            Err(message) => return Ok(message),
        };
    let started = run_script_preview(
        ctx,
        Some(path),
        &script.language,
        &script.content,
        Value::Object(prepared.args),
    )
    .await;
    conclude(ctx, args, prepared.notes, started, path, false, "test").await
}

/// `run_script` and `run_flow`: the deployed item, whatever draft sits on top of it.
async fn run_deployed(ctx: &Ctx<'_>, args: &Value, is_flow: bool) -> ToolResult {
    let path = str_arg(args, "path")?;
    let (get, run) = if is_flow {
        ("flows/get", "jobs/run/f")
    } else {
        ("scripts/get/p", "jobs/run/p")
    };
    let item = ctx
        .get(&ctx.ws(&format!("{get}/{}", path_enc(path))))
        .await?;
    let schema = item
        .get("schema")
        .filter(|s| !s.is_null())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let prepared =
        match prepare_args(ctx, path, proposed_args(args), Some(&schema), "deployed").await {
            Ok(prepared) => prepared,
            Err(message) => return Ok(message),
        };
    // A chat-enabled flow is refused without a conversation, and this run is one of its own.
    let memory_id = if is_flow && truthy(item["value"].get("chat_input_enabled")) {
        format!("&memory_id={}", uuid::Uuid::new_v4())
    } else {
        String::new()
    };
    // The arguments fill the main input schema; a preprocessor would take them for a
    // trigger event.
    let started = ctx
        .post(
            &ctx.ws(&format!(
                "{run}/{}?skip_preprocessor=true{memory_id}",
                path_enc(path)
            )),
            Value::Object(prepared.args),
        )
        .await
        .map(job_id);
    conclude(ctx, args, prepared.notes, started, path, is_flow, "run").await
}

/// The flow value a preview runs, from a loaded `{value, schema, groups}`.
fn flow_preview_value(flow: &Value) -> Value {
    let mut value = flow.get("value").cloned().unwrap_or_else(|| json!({}));
    if let (Some(groups), Some(map)) = (flow.get("groups"), value.as_object_mut()) {
        if groups.is_null() {
            map.remove("groups");
        } else {
            map.insert("groups".into(), groups.clone());
        }
    }
    value
}

async fn test_run_flow(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let memory_id = opt_str_arg(args, "memory_id");
    if memory_id.is_some_and(|id| id.len() != 36 || uuid::Uuid::parse_str(id).is_err()) {
        return Err("memory_id must be a UUID".to_string());
    }
    let flow = load_flow(ctx, path).await?;
    let schema = flow
        .flow
        .get("schema")
        .filter(|s| !s.is_null())
        .cloned()
        .unwrap_or_else(|| json!({}));
    let prepared = match prepare_args(ctx, path, proposed_args(args), Some(&schema), "flow").await {
        Ok(prepared) => prepared,
        Err(message) => return Ok(message),
    };
    let value = flow_preview_value(&flow.flow);
    // The server refuses a chat-enabled flow's run without a conversation; a test run
    // is one of its own unless the caller names another.
    let memory_id = memory_id.map(String::from).or_else(|| {
        truthy(value.get("chat_input_enabled")).then(|| uuid::Uuid::new_v4().to_string())
    });
    let started = run_flow_preview(
        ctx,
        Some(path),
        &value,
        Value::Object(prepared.args),
        memory_id.as_deref(),
    )
    .await;
    conclude(ctx, args, prepared.notes, started, path, true, "test").await
}

// ============================================================================
// One step of a flow
// ============================================================================

fn modules_of<'a>(value: &'a Value, key: &str) -> impl Iterator<Item = &'a Value> {
    value[key].as_array().into_iter().flatten()
}

/// The modules nested directly under one: loop bodies, branches, and an agent's
/// flow-module tools.
fn child_modules(module: &Value, with_tools: bool) -> Vec<&Value> {
    let value = &module["value"];
    let branches = || modules_of(value, "branches").flat_map(|b| modules_of(b, "modules"));
    match value["type"].as_str() {
        Some("forloopflow" | "whileloopflow") => modules_of(value, "modules").collect(),
        Some("branchall") => branches().collect(),
        Some("branchone") => branches().chain(modules_of(value, "default")).collect(),
        Some("aiagent") if with_tools => modules_of(value, "tools")
            .filter(|tool| {
                matches!(tool["value"].get("tool_type"), None | Some(Value::Null))
                    || tool["value"]["tool_type"] == "flowmodule"
            })
            .collect(),
        _ => Vec::new(),
    }
}

fn find_in_modules<'a>(modules: Vec<&'a Value>, id: &str) -> Option<&'a Value> {
    modules.into_iter().find_map(|module| {
        if module["id"] == id {
            Some(module)
        } else {
            find_in_modules(child_modules(module, true), id)
        }
    })
}

fn find_module<'a>(flow: &'a Value, id: &str) -> Option<&'a Value> {
    ["failure_module", "preprocessor_module"]
        .iter()
        .map(|key| &flow[*key])
        .find(|module| module["id"] == id)
        .or_else(|| find_in_modules(modules_of(flow, "modules").collect(), id))
}

fn available_step_ids(flow: &Value) -> String {
    fn collect<'a>(modules: Vec<&'a Value>, ids: &mut Vec<&'a str>) {
        for module in modules {
            ids.extend(module["id"].as_str());
            collect(child_modules(module, false), ids);
        }
    }
    let mut ids = Vec::new();
    collect(modules_of(flow, "modules").collect(), &mut ids);
    ids.extend(flow["preprocessor_module"]["id"].as_str());
    ids.extend(flow["failure_module"]["id"].as_str());
    let mut seen = Vec::new();
    for id in ids {
        if !seen.contains(&id) {
            seen.push(id);
        }
    }
    seen.join(", ")
}

/// The agent form's rows in its own order: key, whether an agent always has it, and
/// the value a run cannot tell apart from an absent key.
fn agent_fields() -> [(&'static str, bool, Option<Value>); 16] {
    [
        ("provider", true, None),
        ("temperature", false, None),
        ("max_completion_tokens", false, None),
        ("user_message", true, None),
        ("system_prompt", true, None),
        ("memory", false, Some(json!({ "kind": "off" }))),
        ("memory_id", false, Some(json!(""))),
        ("previous_messages", false, Some(json!([]))),
        ("user_attachments", false, Some(json!([]))),
        ("enabled_tools", false, None),
        ("max_iterations", false, Some(json!(10))),
        ("output_type", false, Some(json!("text"))),
        ("output_schema", false, None),
        ("streaming", false, Some(json!(true))),
        ("model", false, Some(json!(""))),
        ("reasoning_effort", false, Some(json!(""))),
    ]
}

/// The inputs a step linked to a saved agent supplies itself; the rest is the agent's.
const AGENT_FLOW_LOCAL_KEYS: [&str; 5] = [
    "user_message",
    "user_attachments",
    "enabled_tools",
    "memory_id",
    "previous_messages",
];
const AGENT_HISTORY_KEYS: [&str; 2] = ["memory_id", "previous_messages"];
/// A linked step's overrides of what its agent saved; an unlinked step has no such inputs.
const AGENT_OVERRIDE_KEYS: [&str; 2] = ["model", "reasoning_effort"];

/// Older spellings of two agent inputs, read as the current ones.
fn migrate_agent_transforms(transforms: &mut Map<String, Value>) {
    if !transforms.contains_key("user_attachments") {
        if let Some(images) = transforms.remove("user_images") {
            transforms.insert("user_attachments".into(), images);
        }
    }
    if transforms.contains_key("memory") {
        return;
    }
    let Some(legacy) = transforms.get("messages_context_length").cloned() else {
        return;
    };
    if legacy.is_null() {
        return;
    }
    match legacy["type"].as_str() {
        Some("static") => {
            let length = legacy.get("value").filter(|v| !v.is_null()).cloned();
            transforms.insert(
                "memory".into(),
                json!({ "type": "static", "value": { "kind": "auto", "context_length": length.unwrap_or(json!(0)) } }),
            );
        }
        Some("javascript") => {
            let expr = legacy["expr"].as_str().unwrap_or("undefined");
            transforms.insert(
                "memory".into(),
                json!({ "type": "javascript", "expr": format!("{{ kind: 'auto', context_length: {expr} }}") }),
            );
        }
        _ => {}
    }
    transforms.remove("messages_context_length");
}

/// The inputs a test run of an agent step takes, in the agent form's order, and the
/// values the step itself configures for them. A row is one the step sets, an agent
/// always has, or the caller proposed.
fn agent_step_form(
    module_value: &Value,
    proposed: &Map<String, Value>,
) -> (Vec<String>, Map<String, Value>) {
    let transforms = module_value.get("input_transforms");
    let linked = truthy(module_value.get("agent"));
    let mut keys = Vec::new();
    let mut args = Map::new();
    for (key, core, implicit) in agent_fields() {
        let override_key = AGENT_OVERRIDE_KEYS.contains(&key);
        let applies = if linked {
            override_key || AGENT_FLOW_LOCAL_KEYS.contains(&key)
        } else {
            !override_key
        };
        if !applies {
            continue;
        }
        let transform = transforms.and_then(|t| t.get(key));
        let is_set = core
            || match transform.and_then(|t| t["type"].as_str()) {
                Some("javascript") => truthy(transform.and_then(|t| t.get("expr"))),
                Some("ai") => true,
                _ => transform
                    .filter(|t| t.is_object())
                    .and_then(|t| t.get("value"))
                    .is_some_and(|value| !value.is_null() && Some(value) != implicit.as_ref()),
            };
        if !is_set && !proposed.contains_key(key) {
            continue;
        }
        keys.push(key.to_string());
        // ponytail: only a static input opens on its value. An expression is evaluated by
        // the session with no flow input or earlier results, so all but a literal one
        // reads as unset there too; evaluate it here if literal expressions matter.
        if let Some(value) = transform
            .filter(|t| t["type"] == "static")
            .and_then(|t| t.get("value"))
        {
            args.insert(key.to_string(), value.clone());
        }
    }
    for (key, value) in proposed {
        args.insert(key.clone(), value.clone());
    }
    // An absent `enabled_tools` runs every tool and `[]` none, and the row is fed from
    // the run's arguments either way.
    if keys.iter().any(|key| key == "enabled_tools")
        && args.get("enabled_tools").is_none_or(Value::is_null)
    {
        args.insert("enabled_tools".into(), json!([]));
    }
    (keys, args)
}

/// The input transforms a one-off test of an agent step runs with: each form key fed
/// from `flow_input`, over the step's own. A blank history input is unset.
fn agent_test_transforms(
    step: Option<&Value>,
    args: &Map<String, Value>,
    form_keys: &[String],
) -> Map<String, Value> {
    let kept = |key: &str| {
        let blank = match args.get(key) {
            None | Some(Value::Null) => true,
            Some(Value::String(s)) => s.is_empty(),
            Some(Value::Array(a)) => a.is_empty(),
            Some(_) => false,
        };
        !AGENT_HISTORY_KEYS.contains(&key) || !blank
    };
    let mut out: Map<String, Value> = step
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
        .filter(|(key, _)| kept(key))
        .map(|(key, transform)| (key.clone(), transform.clone()))
        .collect();
    for key in form_keys.iter().filter(|key| kept(key)) {
        out.insert(
            key.clone(),
            json!({ "type": "javascript", "expr": format!("flow_input.{key}") }),
        );
    }
    out
}

fn normalize_agent_ref(agent: &str) -> &str {
    let agent = agent.strip_prefix("$res:").unwrap_or(agent);
    agent.strip_prefix("res://").unwrap_or(agent)
}

fn linked_agent_paths(module: &Value, paths: &mut Vec<String>) {
    let value = &module["value"];
    if value["type"] != "aiagent" {
        return;
    }
    if let Some(agent) = value["agent"].as_str().filter(|a| !a.is_empty()) {
        let path = normalize_agent_ref(agent).to_string();
        if !paths.contains(&path) {
            paths.push(path);
        }
    }
    for tool in modules_of(value, "tools") {
        linked_agent_paths(tool, paths);
    }
}

fn inline_agent_draft(module: &mut Value, drafts: &HashMap<String, Value>) {
    let value = &mut module["value"];
    if value["type"] != "aiagent" {
        return;
    }
    let draft = value["agent"]
        .as_str()
        .and_then(|agent| drafts.get(normalize_agent_ref(agent)));
    if let (Some(draft), Some(map)) = (draft, value.as_object_mut()) {
        map.remove("agent");
        let mut transforms = Map::new();
        for (key, arg) in draft.as_object().into_iter().flatten() {
            if key != "tools" && !AGENT_HISTORY_KEYS.contains(&key.as_str()) {
                transforms.insert(key.clone(), json!({ "type": "static", "value": arg }));
            }
        }
        for key in AGENT_FLOW_LOCAL_KEYS.into_iter().chain(AGENT_OVERRIDE_KEYS) {
            if let Some(own) = map
                .get("input_transforms")
                .and_then(|t| t.get(key))
                .filter(|t| truthy(Some(t)))
            {
                transforms.insert(key.to_string(), own.clone());
            }
        }
        map.insert(
            "tools".into(),
            draft
                .get("tools")
                .filter(|t| !t.is_null())
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        map.insert("input_transforms".into(), Value::Object(transforms));
    }
    if let Some(tools) = value.get_mut("tools").and_then(Value::as_array_mut) {
        for tool in tools {
            inline_agent_draft(tool, drafts);
        }
    }
}

/// Replace every linked agent that has an unsaved draft with the draft's own
/// configuration, so the test runs what the agent editor shows.
async fn with_agent_drafts(ctx: &Ctx<'_>, module: &mut Value) -> Result<(), String> {
    let mut paths = Vec::new();
    linked_agent_paths(module, &mut paths);
    let mut drafts = HashMap::new();
    for path in paths {
        let route = ctx.ws(&format!("resources/get/{}?get_draft=true", path_enc(&path)));
        match ctx.get(&route).await {
            Ok(resource) => {
                if let Some(draft) = resource.get("draft").filter(|d| !d.is_null()) {
                    drafts.insert(
                        path,
                        draft.get("args").cloned().unwrap_or_else(|| json!({})),
                    );
                }
            }
            // A deleted or unreadable agent is an ordinary state of a link: the step
            // then runs against whatever is deployed.
            Err(e) if ["401", "403", "404"].iter().any(|s| e.starts_with(s)) => {}
            Err(e) => return Err(format!("Could not load the agent {path}: {e}")),
        }
    }
    inline_agent_draft(module, &drafts);
    Ok(())
}

async fn test_run_step(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let step_id = str_arg(args, "stepId")?;
    let flow_value = flow_preview_value(&load_flow(ctx, path).await?.flow);
    let Some(module) = find_module(&flow_value, step_id) else {
        return Err(format!(
            "Step with id \"{step_id}\" not found in flow. Available steps: {}",
            available_step_ids(&flow_value)
        ));
    };
    let value = &module["value"];
    let module_type = value["type"].as_str().unwrap_or_default();
    let text = |key: &str| value[key].as_str().unwrap_or_default().to_string();
    // A preprocessor takes the arguments of its own entrypoint, which no schema declares.
    let entrypoint = (module["id"] == "preprocessor").then_some("preprocessor");
    let label = format!("step {step_id}");
    let proposed = proposed_args(args);

    let (schema, proposed) = match module_type {
        "rawscript" => (
            schema_for_test_run(&text("language"), &text("content"), None, entrypoint),
            proposed,
        ),
        "script" | "flow" | "aidecision" => (None, proposed),
        "aiagent" => {
            let (keys, args) = agent_step_form(&migrated_agent_value(value), &proposed);
            let properties: Map<String, Value> =
                keys.into_iter().map(|key| (key, json!({}))).collect();
            (Some(json!({ "properties": properties })), args)
        }
        other => {
            return Err(format!(
                "Cannot test step of type \"{other}\". Supported types: rawscript, script, flow, aiagent, aidecision"
            ));
        }
    };

    // The targets read from the workspace, each with the schema of what will run.
    let mut script = None;
    let mut subflow_preview = None;
    let schema = match module_type {
        "script" => {
            let target = text("path");
            let hash = value["hash"].as_str().filter(|h| !h.is_empty());
            // A draft of the script runs even where the step pins a hash.
            let loaded = match hash {
                Some(hash) if get_draft(ctx, "script", &target).await?.is_none() => {
                    let deployed = ctx
                        .get(&ctx.ws(&format!("scripts/get/h/{}", enc(hash))))
                        .await?;
                    (
                        deployed["content"].as_str().unwrap_or_default().to_string(),
                        deployed["language"]
                            .as_str()
                            .unwrap_or_default()
                            .to_string(),
                        deployed.get("schema").filter(|s| !s.is_null()).cloned(),
                    )
                }
                _ => {
                    let loaded = load_script(ctx, &target).await?;
                    (loaded.content, loaded.language, loaded.schema)
                }
            };
            let schema = schema_for_test_run(&loaded.1, &loaded.0, loaded.2, entrypoint);
            script = Some((target, loaded.0, loaded.1));
            schema
        }
        "flow" => {
            // Only a draft is previewed; a deployed subflow runs by path, as its parent
            // flow would run it.
            let subflow = load_flow(ctx, &text("path")).await?;
            if subflow.is_draft {
                subflow_preview = Some(flow_preview_value(&subflow.flow));
            }
            Some(
                subflow
                    .flow
                    .get("schema")
                    .filter(|s| !s.is_null())
                    .cloned()
                    .unwrap_or_else(|| json!({})),
            )
        }
        // What a run supplies is the state; the provider and questions are the step's own.
        "aidecision" => Some(json!({ "properties": { "state": {} } })),
        _ => schema,
    };

    let prepared = match prepare_args(ctx, path, proposed, schema.as_ref(), "step").await {
        Ok(prepared) => prepared,
        Err(message) => return Ok(message),
    };
    let mut run_args = prepared.args;
    let with_entrypoint = |args: Map<String, Value>| {
        let mut out = Map::new();
        if entrypoint.is_some() {
            out.insert("_ENTRYPOINT_OVERRIDE".into(), "preprocessor".into());
        }
        out.extend(args);
        Value::Object(out)
    };
    // Run alone, stripped of the settings around it: a mock would answer for the step
    // and a suspend would hold the run for an approval.
    let single_step =
        |step_value: Value| json!({ "modules": [{ "id": module["id"], "value": step_value }] });

    let (started, is_flow) = match module_type {
        "rawscript" => (
            run_script_preview(
                ctx,
                None,
                &text("language"),
                &text("content"),
                with_entrypoint(run_args),
            )
            .await,
            false,
        ),
        "script" => {
            let (target, content, language) = script.unwrap_or_default();
            (
                run_script_preview(
                    ctx,
                    Some(&target),
                    &language,
                    &content,
                    with_entrypoint(run_args),
                )
                .await,
                false,
            )
        }
        "flow" => {
            let target = text("path");
            let started = match subflow_preview {
                Some(preview) => {
                    run_flow_preview(ctx, Some(&target), &preview, Value::Object(run_args), None)
                        .await
                }
                None => ctx
                    .post(
                        &ctx.ws(&format!(
                            "jobs/run/f/{}?skip_preprocessor=true",
                            path_enc(&target)
                        )),
                        Value::Object(run_args),
                    )
                    .await
                    .map(job_id),
            };
            (started, true)
        }
        "aiagent" => {
            let mut step_value = migrated_agent_value(value);
            let form_keys: Vec<String> = schema
                .as_ref()
                .and_then(|s| s["properties"].as_object())
                .map(|p| p.keys().cloned().collect())
                .unwrap_or_default();
            step_value["input_transforms"] = Value::Object(agent_test_transforms(
                step_value.get("input_transforms"),
                &run_args,
                &form_keys,
            ));
            let mut step = json!({ "id": module["id"], "value": step_value });
            with_agent_drafts(ctx, &mut step).await?;
            let preview = json!({ "modules": [step] });
            (
                run_flow_preview(ctx, Some(path), &preview, Value::Object(run_args), None).await,
                true,
            )
        }
        _ => {
            let mut step_value = value.clone();
            let mut transforms = value["input_transforms"]
                .as_object()
                .cloned()
                .unwrap_or_default();
            for key in run_args.keys() {
                transforms.insert(
                    key.clone(),
                    json!({ "type": "javascript", "expr": format!("flow_input.{key}") }),
                );
            }
            step_value["input_transforms"] = Value::Object(transforms);
            let preview = single_step(step_value);
            (
                run_flow_preview(
                    ctx,
                    Some(path),
                    &preview,
                    Value::Object(std::mem::take(&mut run_args)),
                    None,
                )
                .await,
                true,
            )
        }
    };
    conclude(ctx, args, prepared.notes, started, &label, is_flow, "test").await
}

fn migrated_agent_value(value: &Value) -> Value {
    let mut value = value.clone();
    if let Some(transforms) = value
        .get_mut("input_transforms")
        .and_then(Value::as_object_mut)
    {
        migrate_agent_transforms(transforms);
    }
    value
}

// ============================================================================
// Reading runs back
// ============================================================================

/// Compact metadata for one run: what identifies it and how it ended.
fn summarize_run(job: &Value) -> Map<String, Value> {
    let mut out = Map::new();
    for (key, source) in [
        ("id", "id"),
        ("job_kind", "job_kind"),
        ("path", "script_path"),
        ("created_by", "created_by"),
        ("created_at", "created_at"),
        ("started_at", "started_at"),
        ("schedule_path", "schedule_path"),
        ("is_flow_step", "is_flow_step"),
        ("tag", "tag"),
        ("worker", "worker"),
    ] {
        if let Some(value) = job.get(source) {
            out.insert(key.into(), value.clone());
        }
    }
    let flag = |key: &str| truthy(job.get(key));
    let status = if job.get("success").is_some() {
        // `success` is true for a skipped job too, so it is read last.
        if flag("canceled") {
            "canceled"
        } else if flag("is_skipped") {
            "skipped"
        } else if flag("success") {
            "success"
        } else {
            "failure"
        }
    } else if !flag("running") {
        "queued"
    } else if flag("suspend") {
        // Parked on an approval or a parallelism slot, not working.
        "suspended"
    } else {
        "running"
    };
    out.insert("status".into(), status.into());
    if job.get("success").is_some() {
        if let Some(duration) = job.get("duration_ms") {
            out.insert("duration_ms".into(), duration.clone());
        }
    }
    out
}

async fn list_runs(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(30)
        .clamp(1, 100);
    let mut query = format!("per_page={limit}");
    for (param, arg) in [
        ("script_path_exact", "path"),
        ("created_by", "created_by"),
        ("label", "label"),
    ] {
        if let Some(value) = opt_str_arg(args, arg) {
            query.push_str(&format!("&{param}={}", enc(value)));
        }
    }
    for flag in ["success", "running"] {
        if let Some(value) = args.get(flag).and_then(Value::as_bool) {
            query.push_str(&format!("&{flag}={value}"));
        }
    }
    let jobs = ctx.get(&ctx.ws(&format!("jobs/list?{query}"))).await?;
    let runs: Vec<Value> = jobs
        .as_array()
        .into_iter()
        .flatten()
        .map(|job| Value::Object(summarize_run(job)))
        .collect();
    Ok(pretty(&Value::Array(runs)))
}

async fn cancel_job(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let id = str_arg(args, "id")?;
    Ok(
        match ctx
            .post(
                &ctx.ws(&format!("jobs_u/queue/cancel/{}", enc(id))),
                json!({}),
            )
            .await
        {
            Ok(_) => format!("Job {id} was canceled."),
            Err(e) => format!(
                "Failed to cancel job {id}: {}. It may have already finished.",
                api_message(&e)
            ),
        },
    )
}

/// Per-entry result budget asked of the server for the tree view.
const TREE_RESULT_HEAD_CHARS: usize = 700;
/// Cap on a full payload handed to the model: a drilled step result, and the run's own
/// args, result and logs.
const STEP_RESULT_MAX_CHARS: usize = 12000;
/// Cap on the whole rendered tree; heads shrink progressively to fit.
const TREE_TOTAL_BUDGET_CHARS: usize = 20000;
const MAX_FAILED_ITERATIONS_SHOWN: usize = 3;
/// `(head, success_head)` tried in order until the tree fits: succeeded steps lose
/// their results first so failure detail survives longest.
const SHRINK_LADDER: [(usize, usize); 4] = [
    (TREE_RESULT_HEAD_CHARS, TREE_RESULT_HEAD_CHARS),
    (250, 250),
    (250, 0),
    (100, 0),
];
/// Module types whose sibling jobs are iterations or branches; sibling jobs of any
/// other step are retry attempts of it.
const FAN_OUT_MODULE_TYPES: [&str; 5] = [
    "forloopflow",
    "whileloopflow",
    "branchall",
    "branchone",
    "aiagent",
];
const LOGS_UNREADABLE: &str = "Logs could not be read for this run.";

/// A run's jobs as the server lists them, depth-first, with each one's children.
struct RunTree<'a> {
    entries: &'a [Value],
    children: Vec<Vec<usize>>,
}

impl<'a> RunTree<'a> {
    fn new(entries: &'a [Value]) -> Self {
        let depth = |i: usize| entries[i]["depth"].as_i64().unwrap_or(0);
        let mut children = vec![Vec::new(); entries.len()];
        let mut stack: Vec<usize> = Vec::new();
        for i in 0..entries.len() {
            while stack.last().is_some_and(|&top| depth(top) >= depth(i)) {
                stack.pop();
            }
            if let Some(&parent) = stack.last() {
                children[parent].push(i);
            }
            stack.push(i);
        }
        RunTree { entries, children }
    }

    fn status(&self, i: usize) -> &str {
        self.entries[i]["status"].as_str().unwrap_or_default()
    }

    fn shape_step(
        &self,
        i: usize,
        heads: (usize, usize),
        iteration: Option<&Value>,
    ) -> Map<String, Value> {
        let entry = &self.entries[i];
        let mut out = Map::new();
        if let Some(step) = entry["step_path"].as_str().filter(|s| !s.is_empty()) {
            out.insert("step".into(), step.into());
        }
        if let Some(iteration) = iteration {
            out.insert("iteration".into(), iteration.clone());
        }
        for key in ["label", "job_id", "status"] {
            if let Some(value) = entry.get(key) {
                out.insert(key.into(), value.clone());
            }
        }
        if let Some(duration) = entry.get("duration_ms").filter(|d| !d.is_null()) {
            out.insert("duration_ms".into(), duration.clone());
        }
        if let Some(prefix) = entry["result_prefix"].as_str() {
            let budget = if truthy(entry.get("success")) {
                heads.1
            } else {
                heads.0
            };
            let length = entry["result_length"].as_u64();
            if budget == 0 {
                if let Some(length) = length {
                    out.insert("result_total_chars".into(), length.into());
                }
            } else {
                // The server counts code points, so sizes are compared in them.
                let head = js_head(prefix, budget);
                let total = length.unwrap_or(prefix.chars().count() as u64);
                out.insert("result".into(), head.into());
                if total > head.chars().count() as u64 {
                    out.insert("result_total_chars".into(), total.into());
                }
            }
        }
        let steps = self.shape_children(i, heads);
        if !steps.is_empty() {
            out.insert("steps".into(), steps.into());
        }
        out
    }

    /// Sibling jobs grouped by flow step, so the iterations, branches or attempts of
    /// one step read as one entry.
    fn shape_children(&self, i: usize, heads: (usize, usize)) -> Vec<Value> {
        let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
        for &child in &self.children[i] {
            let entry = &self.entries[child];
            let step_id = match entry["flow_step_id"].as_str() {
                Some(id) => id.to_string(),
                None => format!("job:{}", entry["job_id"].as_str().unwrap_or_default()),
            };
            match groups.iter_mut().find(|(id, _)| *id == step_id) {
                Some((_, nodes)) => nodes.push(child),
                None => groups.push((step_id, vec![child])),
            }
        }
        groups
            .iter()
            .map(|(step_id, nodes)| Value::Object(self.shape_group(step_id, nodes, heads)))
            .collect()
    }

    fn shape_group(
        &self,
        step_id: &str,
        nodes: &[usize],
        heads: (usize, usize),
    ) -> Map<String, Value> {
        let first = &self.entries[nodes[0]];
        if nodes.len() == 1 && first["sibling_count"].as_i64().is_some_and(|n| n <= 1) {
            return self.shape_step(nodes[0], heads, None);
        }
        let step = first["step_path"].as_str().unwrap_or(step_id);
        let parent_type = first["parent_module_type"].as_str().unwrap_or_default();
        let mut out = Map::new();
        out.insert("step".into(), step.into());
        if parent_type == "branchall" {
            out.insert("type".into(), "branchall".into());
            let branches: Vec<Value> = nodes
                .iter()
                .map(|&n| Value::Object(self.shape_step(n, heads, None)))
                .collect();
            out.insert("branches".into(), branches.into());
            return out;
        }

        let index = |n: usize| &self.entries[n]["sibling_index"];
        let mut by_index = nodes.to_vec();
        by_index.sort_by_key(|&n| index(n).as_i64().unwrap_or(0));
        let last = by_index[by_index.len() - 1];

        if !parent_type.is_empty() && !FAN_OUT_MODULE_TYPES.contains(&parent_type) {
            // A retried step: the last attempt is its outcome, earlier ones are
            // status-only references.
            let mut shaped = self.shape_step(last, heads, None);
            shaped.insert("attempts".into(), by_index.len().into());
            let previous: Vec<Value> = by_index[..by_index.len() - 1]
                .iter()
                .map(|&n| {
                    json!({
                        "attempt": index(n),
                        "status": self.status(n),
                        "job_id": self.entries[n]["job_id"],
                    })
                })
                .collect();
            shaped.insert("previous_attempts".into(), previous.into());
            return shaped;
        }

        let count = |status: &str| {
            by_index
                .iter()
                .filter(|&&n| self.status(n) == status)
                .count()
        };
        let ok = count("success");
        let failed: Vec<usize> = by_index
            .iter()
            .copied()
            .filter(|&n| matches!(self.status(n), "failure" | "canceled"))
            .collect();
        let skipped = count("skipped");
        // Skipped is terminal: only running, queued and suspended are unfinished.
        let unfinished = by_index.len() - ok - failed.len() - skipped;
        let mut shown: Vec<usize> = failed
            .iter()
            .copied()
            .take(MAX_FAILED_ITERATIONS_SHOWN)
            .collect();
        if !shown.contains(&last) {
            shown.push(last);
        }

        out.insert(
            "type".into(),
            if parent_type.is_empty() {
                "loop"
            } else {
                parent_type
            }
            .into(),
        );
        out.insert("iterations".into(), by_index.len().into());
        out.insert("ok".into(), ok.into());
        if !failed.is_empty() {
            let indices: Vec<Value> = failed.iter().map(|&n| index(n).clone()).collect();
            out.insert("failed_iterations".into(), indices.into());
        }
        if skipped > 0 {
            out.insert("skipped".into(), skipped.into());
        }
        if unfinished > 0 {
            out.insert("unfinished".into(), unfinished.into());
        }
        let iterations: Vec<Value> = shown
            .iter()
            .map(|&n| Value::Object(self.shape_step(n, heads, Some(index(n)))))
            .collect();
        out.insert("iterations_shown".into(), iterations.into());
        if by_index.len() > shown.len() {
            out.insert(
                "iterations_elided".into(),
                (by_index.len() - shown.len()).into(),
            );
        }
        out
    }

    fn render(
        &self,
        note: Option<&str>,
        heads: (usize, usize),
        run_overrides: &Map<String, Value>,
    ) -> Value {
        let mut run = self.shape_step(0, heads, None);
        let steps = run.remove("steps");
        let kind = self.entries[0]["kind"].as_str().unwrap_or_default();
        // The server labels every depth-0 job "Flow", a plain script run included.
        if steps.is_none() && kind != "flow" && kind != "flowpreview" {
            run.insert("label".into(), format!("Job ({kind})").into());
        }
        // A replacement result supersedes the tree entry's head. Without one the head
        // stays: it is a real prefix of a payload too big to be read whole.
        if run_overrides.contains_key("result") {
            run.remove("result");
            run.remove("result_total_chars");
        }
        for (key, value) in run_overrides {
            run.insert(key.clone(), value.clone());
        }
        let mut out = Map::new();
        if let Some(note) = note {
            out.insert("note".into(), note.into());
        }
        out.insert("run".into(), Value::Object(run));
        if let Some(steps) = steps {
            out.insert("steps".into(), steps);
            out.insert(
                "hint".into(),
                format!("Step results are truncated. Call get_run again with step=\"<step>\" (e.g. \"b/c\", or \"b[12]\" for one loop iteration) for a step's result in full (up to {STEP_RESULT_MAX_CHARS} chars), or with id set to a step's job_id for that step's own args, result and logs.").into(),
            );
        }
        Value::Object(out)
    }
}

/// Render a run's execution tree around the run's own summary and payloads
/// (`run_overrides`), shrinking result heads until the tree fits its budget.
fn shape_flow_run_tree(response: &Value, run_overrides: &Map<String, Value>) -> String {
    let entries = response["entries"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    if entries.is_empty() {
        return pretty1(&json!({ "run": run_overrides }));
    }
    let mut notes = Vec::new();
    if let Some(enclosing) = response["enclosing_job"].as_str().filter(|j| !j.is_empty()) {
        notes.push(format!("This job is a step of a larger flow run — its enclosing run is {enclosing}; pass that id to see more of the tree."));
    }
    if truthy(response.get("truncated")) {
        notes.push(format!("The run has more jobs than the server returns — this tree only covers the first {} (depth-first), so tallies may undercount.", entries.len()));
    }
    if truthy(response.get("scope_filtered")) {
        notes.push(
            "Your token is tag-scoped: steps running on other tags are omitted from this tree."
                .to_string(),
        );
    }
    let note = (!notes.is_empty()).then(|| notes.join(" "));

    // The budget caps the tree, not the run's own args, result and logs: those are
    // capped on their own.
    let override_chars = js_len(&pretty1(&Value::Object(run_overrides.clone())));
    let tree = RunTree::new(entries);
    let mut rendered = String::new();
    for heads in SHRINK_LADDER {
        rendered = pretty1(&tree.render(note.as_deref(), heads, run_overrides));
        if js_len(&rendered).saturating_sub(override_chars) <= TREE_TOTAL_BUDGET_CHARS {
            return rendered;
        }
    }
    format!(
        "{}\n… (tree truncated at {TREE_TOTAL_BUDGET_CHARS} chars — drill into specific steps with the step parameter)",
        js_head(&rendered, TREE_TOTAL_BUDGET_CHARS + override_chars)
    )
}

/// Cap a payload to `STEP_RESULT_MAX_CHARS`. `tail` keeps the end, for logs, where the
/// failure is at the bottom.
fn cap(text: &str, tail: bool) -> String {
    if js_len(text) <= STEP_RESULT_MAX_CHARS {
        return text.to_string();
    }
    let total = text.chars().count();
    if total <= STEP_RESULT_MAX_CHARS {
        return text.to_string();
    }
    let note = format!("… (truncated: {total} chars total)");
    if tail {
        format!("{note}\n{}", js_tail(text, STEP_RESULT_MAX_CHARS))
    } else {
        format!("{}\n{note}", js_head(text, STEP_RESULT_MAX_CHARS))
    }
}

fn stringify(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => pretty1(other),
    }
}

async fn get_run(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let id = str_arg(args, "id")?;
    let results_route = |max: usize| {
        ctx.ws(&format!(
            "jobs_u/get_flow_all_results/{}?max_result_len={max}",
            enc(id)
        ))
    };
    if let Some(step) = opt_str_arg(args, "step").filter(|s| !s.is_empty()) {
        let route = format!(
            "{}&step={}",
            results_route(STEP_RESULT_MAX_CHARS),
            enc(step)
        );
        return Ok(step_result(&ctx.get(&route).await?, step));
    }

    // Only the job read is load-bearing: logs and the step tree each answer part of
    // the question, so neither failing costs the rest.
    let job_route = ctx.ws(&format!("jobs_u/get/{}?no_logs=true&no_code=true", enc(id)));
    let logs_route = ctx.ws(&format!(
        "jobs_u/get_logs/{}?remove_ansi_warnings=true",
        enc(id)
    ));
    let tree_route = results_route(TREE_RESULT_HEAD_CHARS);
    let (job, logs, results) = tokio::join!(
        ctx.get(&job_route),
        ctx.request("GET", &logs_route, None),
        ctx.get(&tree_route),
    );
    let results = results.unwrap_or_else(|_| json!({ "entries": [] }));
    if let Ok(job) = &job {
        let mut details = Map::new();
        let entries = results["entries"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        details.insert("step_list".into(), card_steps(entries).into());
        if let Ok(logs) = &logs {
            if !logs.trim().is_empty() {
                let tail = js_tail(logs, CARD_LOG_CHARS);
                if tail.len() < logs.len() {
                    details.insert("logs_cut".into(), true.into());
                }
                details.insert("logs".into(), tail.into());
            }
        }
        report_run(ctx, id, Some(job), details);
    }
    Ok(format!(
        "{}\n\nRun page: {}",
        shape_run(id, &job?, logs, &results),
        ctx.page_url(&format!("/run/{}", enc(id)))
    ))
}

/// The card of one run: what `get_run` hands it, under a one-line answer, since the model
/// shows a run it already read.
pub(super) async fn show_run(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let id = str_arg(args, "id")?;
    get_run(ctx, args).await?;
    let status = match ctx.view.lock().unwrap().as_ref() {
        Some((_, run)) => run["status"].as_str().unwrap_or("running").to_string(),
        None => return Err(format!("Run {id} could not be read.")),
    };
    Ok(format!(
        "Showing run {id} ({status}) to the user as a live card."
    ))
}

/// Runs `show_runs` draws at most: each is one job read.
const MAX_RUNS_SHOWN: usize = 20;

pub(super) async fn show_runs(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let ids: Vec<&str> = args
        .get("ids")
        .and_then(Value::as_array)
        .map(|ids| ids.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if ids.is_empty() {
        return Err("ids is required: the runs to show, from list_runs.".to_string());
    }
    let shown = &ids[..ids.len().min(MAX_RUNS_SHOWN)];
    let routes: Vec<String> = shown
        .iter()
        .map(|id| ctx.ws(&format!("jobs_u/get/{}?no_logs=true&no_code=true", enc(id))))
        .collect();
    let jobs = futures::future::join_all(routes.iter().map(|route| ctx.get(route))).await;
    let mut runs = Vec::new();
    let mut missing = Vec::new();
    for (id, job) in shown.iter().zip(jobs) {
        match job {
            Ok(job) => {
                let mut run = run_card(id, Some(&job));
                run.remove("result");
                run.insert(
                    "url".into(),
                    ctx.page_url(&format!("/run/{}", enc(id))).into(),
                );
                runs.push(Value::Object(run));
            }
            Err(_) => missing.push(*id),
        }
    }
    ctx.set_view(
        "windmill/runs",
        json!({ "workspace": ctx.w_id, "runs": runs, "url": ctx.page_url("/runs") }),
    );
    let mut text = format!("Showing {} run(s) to the user as a table.", runs.len());
    if ids.len() > shown.len() {
        text.push_str(&format!(" Only the first {MAX_RUNS_SHOWN} are shown."));
    }
    if !missing.is_empty() {
        text.push_str(&format!(" Not found: {}.", missing.join(", ")));
    }
    Ok(text)
}

/// Result head the run card shows; the full result is one click away on the run page.
const CARD_RESULT_CHARS: usize = 1500;
const CARD_STEP_RESULT_CHARS: usize = 300;
const CARD_LOG_CHARS: usize = 3000;

/// Hand the run card the run `id`, read as `job` (`None` while it is still running), with
/// `extra` fields beside it.
fn report_run(ctx: &Ctx<'_>, id: &str, job: Option<&Value>, extra: Map<String, Value>) {
    let mut run = run_card(id, job);
    run.extend(extra);
    run.insert("workspace".into(), ctx.w_id.into());
    run.insert(
        "url".into(),
        ctx.page_url(&format!("/run/{}", enc(id))).into(),
    );
    ctx.set_view("windmill/run", Value::Object(run));
}

/// A flow run's top-level steps for the card's details, grouped as the model reads them:
/// a loop as its iteration counts, a parallel step as its branches, a retry as its attempts.
fn card_steps(entries: &[Value]) -> Vec<Value> {
    if entries.is_empty() {
        return Vec::new();
    }
    RunTree::new(entries).shape_children(0, (CARD_STEP_RESULT_CHARS, CARD_STEP_RESULT_CHARS))
}

/// What the run card draws of a run: its summary, the head of its result (a failure's
/// message alone), and a running flow's top-level step.
fn run_card(id: &str, job: Option<&Value>) -> Map<String, Value> {
    let mut run = job.map(summarize_run).unwrap_or_default();
    run.insert("id".into(), id.into());
    run.entry("status").or_insert("running".into());
    if let Some(job) = job {
        let result = job.get("result").filter(|r| !r.is_null());
        let error = result
            .map(|r| &r["error"])
            .filter(|e| e["message"].is_string() && job.get("success") == Some(&json!(false)));
        // An error reads as Windmill shows it: `name: message`, then its stack.
        if let Some(stack) = error
            .and_then(|e| e["stack"].as_str())
            .filter(|s| !s.is_empty())
        {
            run.insert("stack".into(), js_head(stack, CARD_RESULT_CHARS).into());
        }
        if let Some(text) = error
            .map(|e| match e["name"].as_str().filter(|n| !n.is_empty()) {
                Some(name) => format!("{name}: {}", e["message"].as_str().unwrap_or_default()),
                None => e["message"].as_str().unwrap_or_default().to_string(),
            })
            .or_else(|| result.map(|r| format_result(Some(r))))
        {
            let head = js_head(&text, CARD_RESULT_CHARS);
            if head.len() < text.len() {
                run.insert("result_cut".into(), true.into());
            }
            run.insert("result".into(), head.into());
        }
        // 1-based, of how many: `step` already points past the last module once it is done.
        let status = &job["flow_status"];
        if let Some(modules) = status["modules"]
            .as_array()
            .filter(|_| job.get("success").is_none())
        {
            let step = status["step"].as_u64().unwrap_or(0) as usize;
            run.insert("steps".into(), modules.len().into());
            run.insert("step".into(), (step + 1).min(modules.len()).into());
            if let Some(step_id) = modules.get(step).and_then(|m| m.get("id")) {
                run.insert("step_id".into(), step_id.clone());
            }
        }
    }
    run
}

/// A run as `get_run` reports it: its summary, args, result and logs around the step
/// tree. `logs` is the log read's own outcome, so a failed read is told apart from an
/// empty log.
fn shape_run(id: &str, job: &Value, logs: Result<String, String>, results: &Value) -> String {
    let mut run = summarize_run(job);
    run.remove("id");
    // Why a run ended badly.
    for (key, source) in [
        ("canceled_by", "canceled_by"),
        ("canceled_reason", "canceled_reason"),
        ("mem_peak_kb", "mem_peak"),
    ] {
        if let Some(value) = job.get(source).filter(|v| truthy(Some(v))) {
            run.insert(key.into(), value.clone());
        }
    }
    // A successful read always carries the job itself as the root entry, so no entries
    // means the read failed, and nothing else would name the run.
    if results["entries"].as_array().is_none_or(Vec::is_empty) {
        run.insert("job_id".into(), id.into());
        run.insert("steps_unavailable".into(), true.into());
    }
    // The server swaps a payload too big to send for a marker, each field in its own
    // form. That is reported, never fetched around: the result keeps the head the step
    // tree carries.
    if let Some(job_args) = job.get("args").filter(|a| truthy(Some(a))) {
        let too_big = job_args
            .as_object()
            .is_some_and(|o| o.len() == 1 && o.get("reason") == Some(&json!("WINDMILL_TOO_BIG")));
        if too_big {
            run.insert("args_truncated".into(), true.into());
        } else {
            run.insert("args".into(), cap(&stringify(job_args), false).into());
        }
    }
    if let Some(result) = job.get("result") {
        if result == "WINDMILL_TOO_BIG" {
            run.insert("result_truncated".into(), true.into());
        } else {
            run.insert("result".into(), cap(&stringify(result), false).into());
        }
    }
    let logs = match logs {
        Err(_) => LOGS_UNREADABLE.to_string(),
        Ok(logs) if logs.trim().is_empty() => "No logs for this run.".to_string(),
        Ok(logs) => cap(&logs, true),
    };
    run.insert("logs".into(), logs.into());
    shape_flow_run_tree(results, &run)
}

/// One step's result in full, from the single entry the server resolved its address to.
fn step_result(response: &Value, step: &str) -> String {
    if let Some(error) = response["step_error"].as_str().filter(|e| !e.is_empty()) {
        let hidden = if truthy(response.get("scope_filtered")) {
            " (Steps running on tags outside your token’s scope are hidden.)"
        } else {
            ""
        };
        return format!("{error}{hidden}");
    }
    let Some(entry) = response["entries"].get(0) else {
        return "No jobs found for this run.".to_string();
    };
    let job_id = entry["job_id"].as_str().unwrap_or_default();
    let status = entry["status"].as_str().unwrap_or_default();
    if matches!(status, "running" | "queued" | "suspended") {
        return format!("Step \"{step}\" (job {job_id}) has not completed yet — status: {status}.");
    }
    let Some(prefix) = entry["result_prefix"].as_str() else {
        return format!("Step \"{step}\" (job {job_id}, {status}) has no recorded result.");
    };
    let received = prefix.chars().count() as u64;
    let total = entry["result_length"].as_u64().unwrap_or(received);
    let truncated = if total > received {
        format!("\n… (result truncated: {total} chars total)")
    } else {
        String::new()
    };
    format!("Step \"{step}\" (job {job_id}, {status}) result:\n{prefix}{truncated}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(overrides: Value) -> Value {
        let mut entry = json!({
            "job_id": "j",
            "label": "Flow",
            "kind": "script",
            "depth": 1,
            "sibling_index": 1,
            "sibling_count": 1,
            "status": "success",
            "success": true,
            "result_prefix": "\"ok\"",
            "result_length": 4,
        });
        for (key, value) in overrides.as_object().unwrap() {
            entry[key] = value.clone();
        }
        entry
    }

    fn root() -> Value {
        entry(json!({ "kind": "flow", "depth": 0, "step_path": "" }))
    }

    fn shape(entries: Vec<Value>) -> Value {
        serde_json::from_str(&shape_flow_run_tree(
            &json!({ "entries": entries }),
            &Map::new(),
        ))
        .unwrap()
    }

    fn loop_entries(n: i64, failed: &[i64]) -> Vec<Value> {
        let mut entries = vec![root()];
        for i in 1..=n {
            entries.push(entry(json!({
                "step_path": "l", "flow_step_id": "l", "kind": "flow",
                "parent_module_type": "forloopflow", "sibling_index": i, "sibling_count": n,
                "status": if failed.contains(&i) { "failure" } else { "success" },
                "success": !failed.contains(&i),
            })));
        }
        entries
    }

    #[test]
    fn subflow_steps_nest_under_their_parent_step() {
        let rendered = shape(vec![
            root(),
            entry(json!({ "step_path": "a", "flow_step_id": "a" })),
            entry(json!({ "step_path": "b", "flow_step_id": "b", "kind": "flow" })),
            entry(json!({
                "step_path": "b/c", "flow_step_id": "c", "depth": 2, "status": "failure",
                "success": false, "result_prefix": "{\"error\":\"boom\"}", "result_length": 16,
            })),
        ]);
        assert_eq!(rendered["run"]["status"], "success");
        assert!(rendered["run"].get("step").is_none());
        let step_b = &rendered["steps"][1];
        assert_eq!(step_b["step"], "b");
        assert_eq!(step_b["steps"][0]["step"], "b/c");
        assert_eq!(step_b["steps"][0]["result"], "{\"error\":\"boom\"}");
        assert!(rendered["hint"].is_string());
    }

    #[test]
    fn loop_iterations_collapse_to_a_tally() {
        let looped = &shape(loop_entries(50, &[3, 7, 21, 30]))["steps"][0];
        assert_eq!(looped["type"], "forloopflow");
        assert_eq!(looped["iterations"], 50);
        assert_eq!(looped["ok"], 46);
        assert_eq!(looped["failed_iterations"], json!([3, 7, 21, 30]));
        let shown: Vec<&Value> = looped["iterations_shown"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| &i["iteration"])
            .collect();
        assert_eq!(shown, [&json!(3), &json!(7), &json!(21), &json!(50)]);
        assert_eq!(looped["iterations_elided"], 46);

        let mut entries = loop_entries(4, &[]);
        entries[2] = entry(json!({
            "step_path": "l", "flow_step_id": "l", "parent_module_type": "forloopflow",
            "sibling_index": 2, "sibling_count": 4, "status": "running",
            "success": null, "result_prefix": null, "result_length": null,
        }));
        entries[3]["status"] = json!("skipped");
        let looped = &shape(entries)["steps"][0];
        assert_eq!(looped["ok"], 2);
        assert_eq!(looped["unfinished"], 1);
        assert_eq!(looped["skipped"], 1);
        assert!(looped.get("failed_iterations").is_none());
    }

    #[test]
    fn retried_steps_are_attempts_of_one_step() {
        let mut entries = vec![root()];
        for i in 1..=3 {
            entries.push(entry(json!({
                "step_path": "a", "flow_step_id": "a", "label": format!("Step a (attempt {i}/3)"),
                "parent_module_type": "rawscript", "sibling_index": i, "sibling_count": 3,
                "status": if i < 3 { "failure" } else { "success" }, "success": i == 3,
            })));
        }
        let step = &shape(entries)["steps"][0];
        assert!(step.get("iterations").is_none());
        assert_eq!(step["attempts"], 3);
        assert_eq!(step["label"], "Step a (attempt 3/3)");
        assert_eq!(step["previous_attempts"][1]["status"], "failure");
    }

    #[test]
    fn result_heads_shrink_to_fit_and_run_payloads_do_not_count() {
        let mut entries = vec![root()];
        for i in 0..60 {
            entries.push(entry(json!({
                "step_path": format!("s{i}"), "flow_step_id": format!("s{i}"),
                "result_prefix": "x".repeat(700), "result_length": 5000,
            })));
        }
        let mut overrides = Map::new();
        overrides.insert("logs".into(), "y".repeat(12000).into());
        let rendered = shape_flow_run_tree(&json!({ "entries": entries }), &overrides);
        assert!(rendered.len() - 12000 <= TREE_TOTAL_BUDGET_CHARS + 100);
        let parsed: Value = serde_json::from_str(&rendered).unwrap();
        assert_eq!(parsed["run"]["logs"].as_str().unwrap().len(), 12000);
        let step = &parsed["steps"][0];
        assert!(step["result"].as_str().is_none_or(|r| r.len() <= 250));
        assert_eq!(step["result_total_chars"], 5000);
    }

    #[test]
    fn truncation_counts_utf16_units_without_splitting_a_character() {
        let text = format!("{}😀b", "a".repeat(3));
        assert_eq!(js_head(&text, 4), "aaa");
        assert_eq!(js_head(&text, 5), "aaa😀");
        assert_eq!(js_tail(&text, 2), "b");
        let long = "é".repeat(STEP_RESULT_MAX_CHARS + 5);
        assert!(cap(&long, true).starts_with("… (truncated: 12005 chars total)\né"));
        assert_eq!(cap("short", false), "short");
    }

    #[test]
    fn args_conform_to_the_schema_and_read_back_redacted() {
        let schema = json!({ "properties": {
            "name": { "type": "string" },
            "locked": { "type": "number", "disabled": true, "default": 1 },
            "creds": { "type": "object", "properties": { "token": { "type": "string", "password": true } } },
            "file": { "type": "string", "contentEncoding": "base64" },
        }});
        let proposed = json!({
            "name": "x", "locked": 2, "extra": true,
            "creds": { "token": "s3cret", "user": "u" }, "file": "A".repeat(4096),
        });
        let (mut args, undeclared) =
            drop_undeclared_args(proposed.as_object().unwrap().clone(), &schema);
        assert_eq!(undeclared, ["extra"]);
        assert_eq!(enforce_disabled_defaults(&mut args, &schema), ["locked"]);
        assert_eq!(args["locked"], 1);
        let redacted = redact_args(&args, &schema);
        assert_eq!(
            redacted["creds"],
            json!({ "token": "<hidden>", "user": "u" })
        );
        assert_eq!(redacted["file"], "<file: 3 KB>");
        args["creds"]["token"] = json!("$var:u/admin/secret_arg/abc");
        assert_eq!(
            redact_args(&args, &schema)["creds"]["token"],
            "$var:u/admin/secret_arg/abc"
        );
    }

    #[test]
    fn agent_step_form_opens_on_what_the_step_sets() {
        let value = json!({ "type": "aiagent", "input_transforms": {
            "provider": { "type": "static", "value": { "kind": "openai" } },
            "user_message": { "type": "javascript", "expr": "flow_input.q" },
            "max_iterations": { "type": "static", "value": 10 },
            "temperature": { "type": "static", "value": 0.2 },
            "memory_id": { "type": "static", "value": "stale" },
        }});
        let proposed = json!({ "user_message": "hi", "enabled_tools": null });
        let (keys, args) = agent_step_form(&value, proposed.as_object().unwrap());
        assert_eq!(
            keys,
            [
                "provider",
                "temperature",
                "user_message",
                "system_prompt",
                "memory_id",
                "enabled_tools"
            ]
        );
        assert_eq!(args["user_message"], "hi");
        assert_eq!(args["enabled_tools"], json!([]));
        let mut run_args = args.clone();
        run_args.insert("memory_id".into(), json!(""));
        let transforms = agent_test_transforms(value.get("input_transforms"), &run_args, &keys);
        assert_eq!(
            transforms["user_message"]["expr"],
            "flow_input.user_message"
        );
        assert_eq!(transforms["max_iterations"]["value"], 10);
        assert!(!transforms.contains_key("memory_id"));
    }

    #[test]
    fn run_card_shows_a_failure_message_and_a_running_flow_step() {
        let failed = json!({
            "type": "CompletedJob", "success": false, "duration_ms": 3600,
            "result": { "error": { "name": "Error", "message": "duplicate key" } },
        });
        let card = run_card("j", Some(&failed));
        assert_eq!(card["status"], "failure");
        assert_eq!(card["result"], "Error: duplicate key");

        let running = json!({
            "type": "QueuedJob", "running": true,
            "flow_status": { "step": 1, "modules": [{ "id": "a" }, { "id": "b" }] },
        });
        let card = run_card("j", Some(&running));
        assert_eq!(
            (card["step"].clone(), card["steps"].clone()),
            (json!(2), json!(2))
        );
        assert_eq!(card["step_id"], "b");

        // Not read yet: still running, by id alone.
        assert_eq!(run_card("j", None)["status"], "running");

        // Cut by UTF-16 units: 800 emoji are 1,600 of them, past the head's 1,500.
        let emoji = json!({ "type": "CompletedJob", "success": true, "result": "😀".repeat(800) });
        assert_eq!(run_card("j", Some(&emoji))["result_cut"], true);
    }
}
