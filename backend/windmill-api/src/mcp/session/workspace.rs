//! Listing and reading workspace items, and the draft store the write tools share.

use std::collections::HashMap;

use serde_json::{json, Map, Value};

use super::{
    array, bool_arg, enc, is_not_found, js_len, js_str, non_null, opt_str_arg, pretty, put,
    str_arg, truthy, Ctx, ToolResult,
};

pub(super) const TOOLS: &[&str] = &[
    "list_workspace_items",
    "read_workspace_item",
    "write_script",
    "edit_script",
    "discard_local_draft",
    "rebase_draft",
    "list_pipelines",
];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "list_workspace_items" => list_workspace_items(ctx, args).await,
        "read_workspace_item" => read_workspace_item(ctx, args).await,
        "write_script" => {
            write_script_draft(
                ctx,
                str_arg(args, "path")?,
                opt_str_arg(args, "summary"),
                str_arg(args, "language")?,
                str_arg(args, "content")?,
                bool_arg(args, "override"),
            )
            .await
        }
        "edit_script" => edit_script(ctx, args).await,
        "discard_local_draft" => discard_local_draft(ctx, args).await,
        "rebase_draft" => rebase_draft(ctx, args).await,
        "list_pipelines" => {
            let pipelines = ctx.get(&ctx.ws("assets/pipelines")).await?;
            Ok(pipelines.to_string())
        }
        _ => unreachable!("{name} is not a workspace tool"),
    }
}

// ============================================================================
// Shared helpers: the per-user draft store and the item shapes built on it.
//
// Vocabulary. An item *type* is what a tool takes (`script`, `flow`, `schedule`,
// `trigger`, `resource`, `variable`, `app`); a draft *kind* is what the drafts API
// stores under (`script`, `flow`, `raw_app`, `trigger_schedule`, `trigger_<kind>`,
// `resource`, `variable`). An *item* is the JSON object the tools show the model
// (`{type, path, summary, value, isDraft, ...}`). A draft's *value* is stored exactly
// as the Windmill editors store it, so either side opens the other's drafts:
//   script            the `NewScript` body (`path, summary, description, content, schema,
//                     is_template, language, kind`, + `parent_hash` once forked from a deploy)
//   flow              the `Flow` object (`path, summary, description, value, schema`, +
//                     `version_id` when forked from a deploy)
//   raw_app           `{summary, files, runnables, data, policy?, custom_path?,
//                     parent_version?, draft_path?}` (`normalize_app_draft`)
//   trigger_schedule  the schedule request body; `trigger_<kind>` that trigger's body
//   resource          `{path, description, args, labels, wsSpecific, resource_type}`
//   variable          `{path, variable: {value, is_secret, description}, labels,
//                     wsSpecific, account?, is_oauth?, expires_at?}`
//
// Paths and routes
//   path_enc(path)                      percent-encode a workspace path for a URL, keeping `/`.
//   draft_kind(type, trigger_kind)      the draft kind of an item type; None for a trigger
//                                       without a (known) kind.
//   exists_route / get_route(ctx, type, trigger_kind, path)
//                                       the API path that probes / fetches the DEPLOYED item.
//                                       Err when type is `trigger` and no kind was given.
//   item_exists(..) / get_deployed(..)  call them. `get_deployed` returns the raw API object
//                                       (variables are fetched without their secret).
//   deployed_item_exists(..)            `item_exists`, counting an archived script as deployed.
//   service(type, trigger_kind)         the API root an item type's routes hang off. Err for an
//                                       unknown type, or a trigger without a known kind.
//   item_type_arg(args)                 a tool's `type` argument, validated.
//
// Draft store (kind-addressed, the caller's own drafts only)
//   get_draft(ctx, kind, path)          Ok(None) when there is no draft. `Draft.created_at` is
//                                       the baseline to hand back as `last_sync`.
//   save_draft(ctx, kind, path, value, last_sync, force)
//                                       upsert, or delete with `value: None`. Ok(true) saved,
//                                       Ok(false) the server refused because the row moved past
//                                       `last_sync`. `force` skips that check.
//   delete_draft(ctx, kind, path, last_sync)
//                                       delete, with the session's error messages on failure
//                                       or conflict.
//   list_drafts(ctx)                    every draft a tool can address, as value-less items.
//
// Items
//   draft_item(kind, path, value)       a stored draft value as an item (`isDraft: true`).
//   get_draft_item(ctx, type, path, trigger_kind)
//                                       the caller's draft as `(item, Draft)`, or None.
//   serialize_for_read(item, execution_mode)
//                                       what `read_workspace_item` prints for an item: flows in
//                                       the compact view, apps summarized, variables redacted.
//   load_script(ctx, path) / load_flow(ctx, path)
//                                       the draft when one exists, else the deployed item
//                                       (Err when neither). `LoadedFlow.is_draft` says which.
//
// Writing a draft (what every `write_*` tool does)
//   write_base(ctx, type, path, trigger_kind)
//                                       the value to build on: the caller's draft, else the
//                                       deployed item's raw API object, else None (a create).
//                                       `from_draft` tells the first two apart: a deployed base
//                                       still needs shaping into the stored draft value.
//   persist_draft(ctx, type, path, trigger_kind, value, last_sync, force)
//                                       save and report: `Persist::Saved(item)`, or
//                                       `Persist::Failed(text)` where `text` is the JSON the tool
//                                       returns as its (Ok) result for a conflict or a failed save.
//   finish_draft_write(item, existed)   the success JSON of a write tool.
//
// Flows and apps
//   compact_flow(value, schema)         the compact flow view and the inline script bodies it
//                                       replaced with `inline_script.<id>`, by module id.
//   normalize_app_draft(value)          the stored raw-app draft shape of an app value.
//   app_source_to_draft_value(app, fallback)
//                                       a deployed app (or app version) as a draft value.
//   summarize_app_value(value)          file and runnable metadata of an app draft value.
//   describe_app_exposure(ctx, mode)    an app's execution mode, flagged when `guest` is inert.
//
// Text
//   find_and_replace(content, old, new, replace_all, label)
//                                       exact replace; Err when `old` is absent or ambiguous.
//   unified_patch(old_name, new_name, old, new, context)
//                                       a unified diff in the format of jsdiff's
//                                       `createTwoFilesPatch` (which defaults to 4 context lines).
//   empty_schema() / infer_script_schema(language, content, previous, entrypoint)
//                                       a script's input schema from its code. None when this
//                                       build has no parser for the language or the code does
//                                       not parse.
//   api_message(error)                  a `Ctx` error without its leading status code.
//
// The JSON and JS-semantics helpers every group shares (`truthy`, `non_null`, `put`,
// `js_len`, ...) are in `mod.rs`; the job helpers are in `runs.rs`.
// ============================================================================

pub(super) const TRIGGER_KINDS: &[&str] = &[
    "http",
    "websocket",
    "kafka",
    "nats",
    "postgres",
    "mqtt",
    "amqp",
    "sqs",
    "gcp",
    "azure",
    "email",
];

/// The item types in the order a listing reports them.
pub(super) const ITEM_TYPES: &[&str] = &[
    "script", "flow", "schedule", "trigger", "resource", "variable", "app",
];

pub(super) const FLOW_VALUE_SETTINGS_KEYS: &[&str] = &[
    "same_worker",
    "preserve_step_tags",
    "concurrent_limit",
    "concurrency_key",
    "concurrency_time_window_s",
    "debounce_delay_s",
    "debounce_key",
    "debounce_args_to_accumulate",
    "max_total_debouncing_time",
    "max_total_debounces_amount",
    "skip_expr",
    "cache_ttl",
    "cache_ignore_s3_path",
    "delete_after_secs",
    "flow_env",
    "priority",
    "early_return",
    "chat_input_enabled",
];

const MAX_LIST_LIMIT: u64 = 100;

pub(super) fn item_type_arg(args: &Value) -> Result<&str, String> {
    let item_type = str_arg(args, "type")?;
    if ITEM_TYPES.contains(&item_type) {
        Ok(item_type)
    } else {
        Err(format!("Invalid type \"{item_type}\""))
    }
}

fn object_or_empty(value: Option<&Value>) -> Value {
    match value {
        Some(Value::Object(o)) => Value::Object(o.clone()),
        _ => json!({}),
    }
}

pub(super) fn api_message(error: &str) -> &str {
    match error.split_once(": ") {
        Some((status, rest)) if status.len() == 3 && status.bytes().all(|b| b.is_ascii_digit()) => {
            rest
        }
        _ => error,
    }
}

pub(super) fn path_enc(path: &str) -> String {
    path.split('/').map(enc).collect::<Vec<_>>().join("/")
}

pub(super) fn draft_kind(item_type: &str, trigger_kind: Option<&str>) -> Option<String> {
    match item_type {
        "script" | "flow" | "resource" | "variable" => Some(item_type.to_string()),
        "app" => Some("raw_app".to_string()),
        "schedule" => Some("trigger_schedule".to_string()),
        "trigger" => trigger_kind
            .filter(|kind| TRIGGER_KINDS.contains(kind))
            .map(|kind| format!("trigger_{kind}")),
        _ => None,
    }
}

/// The item type (and trigger kind) a draft kind surfaces as. None for the kinds no
/// tool addresses: low-code apps, webhook and poll triggers, pipelines.
fn item_type_of_kind(kind: &str) -> Option<(&'static str, Option<&str>)> {
    Some(match kind {
        "script" => ("script", None),
        "flow" => ("flow", None),
        "resource" => ("resource", None),
        "variable" => ("variable", None),
        "raw_app" => ("app", None),
        "trigger_schedule" => ("schedule", None),
        _ => {
            let trigger_kind = kind.strip_prefix("trigger_")?;
            if !TRIGGER_KINDS.contains(&trigger_kind) {
                return None;
            }
            ("trigger", Some(trigger_kind))
        }
    })
}

fn item_key(item_type: &str, path: &str, trigger_kind: Option<&str>) -> String {
    if item_type == "trigger" {
        format!("trigger:{}:{path}", trigger_kind.unwrap_or_default())
    } else {
        format!("{item_type}:{path}")
    }
}

pub(super) fn service(item_type: &str, trigger_kind: Option<&str>) -> Result<String, String> {
    Ok(match item_type {
        "trigger" => {
            let kind = trigger_kind
                .filter(|kind| TRIGGER_KINDS.contains(kind))
                .ok_or("trigger_kind is required when type is trigger.")?;
            format!("{kind}_triggers")
        }
        "script" | "flow" | "schedule" | "resource" | "variable" | "app" => {
            format!("{item_type}s")
        }
        _ => return Err(format!("Invalid type \"{item_type}\"")),
    })
}

pub(super) fn exists_route(
    ctx: &Ctx<'_>,
    item_type: &str,
    trigger_kind: Option<&str>,
    path: &str,
) -> Result<String, String> {
    let by_path = if item_type == "script" { "p/" } else { "" };
    Ok(ctx.ws(&format!(
        "{}/exists/{by_path}{}",
        service(item_type, trigger_kind)?,
        path_enc(path)
    )))
}

pub(super) fn get_route(
    ctx: &Ctx<'_>,
    item_type: &str,
    trigger_kind: Option<&str>,
    path: &str,
) -> Result<String, String> {
    let by_path = if matches!(item_type, "script" | "app") {
        "p/"
    } else {
        ""
    };
    // A variable's value never reaches a tool, so it is not decrypted to begin with.
    let query = if item_type == "variable" {
        "?decrypt_secret=false"
    } else {
        ""
    };
    Ok(ctx.ws(&format!(
        "{}/get/{by_path}{}{query}",
        service(item_type, trigger_kind)?,
        path_enc(path)
    )))
}

pub(super) async fn item_exists(
    ctx: &Ctx<'_>,
    item_type: &str,
    trigger_kind: Option<&str>,
    path: &str,
) -> Result<bool, String> {
    let exists = ctx
        .get(&exists_route(ctx, item_type, trigger_kind, path)?)
        .await?;
    Ok(exists.as_bool().unwrap_or(false))
}

/// `exists` hides an archived script, which its path still resolves to and a delete by
/// path still removes, so an archived script counts as deployed.
pub(super) async fn deployed_item_exists(
    ctx: &Ctx<'_>,
    item_type: &str,
    trigger_kind: Option<&str>,
    path: &str,
) -> Result<bool, String> {
    if item_exists(ctx, item_type, trigger_kind, path).await? {
        return Ok(true);
    }
    if item_type != "script" {
        return Ok(false);
    }
    match get_deployed(ctx, "script", None, path).await {
        Ok(_) => Ok(true),
        Err(e) if is_not_found(&e) => Ok(false),
        Err(e) => Err(e),
    }
}

pub(super) async fn get_deployed(
    ctx: &Ctx<'_>,
    item_type: &str,
    trigger_kind: Option<&str>,
    path: &str,
) -> Result<Value, String> {
    ctx.get(&get_route(ctx, item_type, trigger_kind, path)?)
        .await
}

pub(super) struct Draft {
    pub value: Value,
    pub created_at: Option<String>,
}

pub(super) async fn get_draft(
    ctx: &Ctx<'_>,
    kind: &str,
    path: &str,
) -> Result<Option<Draft>, String> {
    let row = ctx
        .get(&ctx.ws(&format!("drafts/get_own/{kind}/{}", path_enc(path))))
        .await?;
    let Some(value) = non_null(row.get("value")).cloned() else {
        return Ok(None);
    };
    let created_at = row
        .get("created_at")
        .and_then(Value::as_str)
        .map(String::from);
    Ok(Some(Draft { value, created_at }))
}

pub(super) async fn save_draft(
    ctx: &Ctx<'_>,
    kind: &str,
    path: &str,
    value: Option<&Value>,
    last_sync: Option<&str>,
    force: bool,
) -> Result<bool, String> {
    let mut body = Map::new();
    body.insert("value".into(), value.cloned().unwrap_or(Value::Null));
    if !force {
        put(&mut body, "last_sync", last_sync.map(Value::from));
    }
    body.insert("force".into(), json!(force));
    let response = ctx
        .post(
            &ctx.ws(&format!("drafts/update/{kind}/{}", path_enc(path))),
            Value::Object(body),
        )
        .await?;
    Ok(response.get("status").and_then(Value::as_str) != Some("conflict"))
}

pub(super) async fn delete_draft(
    ctx: &Ctx<'_>,
    kind: &str,
    path: &str,
    last_sync: Option<&str>,
) -> Result<(), String> {
    match save_draft(ctx, kind, path, None, last_sync, false).await {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!(
            "Draft \"{path}\" changed externally since you last read it; it was not removed. Re-read and retry."
        )),
        Err(e) => Err(api_message(&e).to_string()),
    }
}

pub(super) async fn list_drafts(ctx: &Ctx<'_>) -> Result<Vec<Value>, String> {
    let rows = ctx.get(&ctx.ws("drafts/list")).await?;
    let rows = array(Some(&rows));
    let mut drafts = Map::new();
    for row in rows {
        let (Some(kind), Some(path)) = (
            row.get("kind").and_then(Value::as_str),
            row.get("path").and_then(Value::as_str),
        ) else {
            continue;
        };
        let Some((item_type, trigger_kind)) = item_type_of_kind(kind) else {
            continue;
        };
        let mut item = Map::new();
        item.insert("type".into(), json!(item_type));
        item.insert("path".into(), json!(path));
        put(
            &mut item,
            "draftPath",
            row.get("draft_path")
                .filter(|p| p.as_str() != Some(path))
                .cloned(),
        );
        put(&mut item, "summary", row.get("summary").cloned());
        item.insert("isDraft".into(), json!(true));
        put(&mut item, "triggerKind", trigger_kind.map(Value::from));
        if kind == "raw_app" {
            item.insert("rawApp".into(), json!(true));
        }
        drafts.insert(item_key(item_type, path, trigger_kind), Value::Object(item));
    }
    Ok(drafts.into_iter().map(|(_, item)| item).collect())
}

pub(super) fn normalize_app_draft(value: &Value) -> Value {
    let mut out = Map::new();
    put(&mut out, "summary", value.get("summary").cloned());
    out.insert("files".into(), object_or_empty(value.get("files")));
    out.insert("runnables".into(), object_or_empty(value.get("runnables")));
    out.insert(
        "data".into(),
        non_null(value.get("data"))
            .cloned()
            .unwrap_or_else(|| json!({ "tables": [] })),
    );
    for key in ["policy", "custom_path", "parent_version", "draft_path"] {
        put(&mut out, key, value.get(key).cloned());
    }
    Value::Object(out)
}

pub(super) fn draft_item(kind: &str, path: &str, draft: &Value) -> Option<Value> {
    let (item_type, trigger_kind) = item_type_of_kind(kind)?;
    let field = |key: &str| draft.get(key).cloned();
    let mut item = Map::new();
    item.insert("type".into(), json!(item_type));
    put(&mut item, "triggerKind", trigger_kind.map(Value::from));
    item.insert("path".into(), json!(path));
    // A staged name that repeats the storage path tells the reader nothing.
    let draft_path = |value: &Value| {
        value
            .get("draft_path")
            .filter(|p| p.as_str() != Some(path))
            .cloned()
    };
    match item_type {
        "script" => {
            put(&mut item, "draftPath", draft_path(draft));
            put(&mut item, "summary", field("summary"));
            put(&mut item, "language", field("language"));
            put(&mut item, "value", field("content"));
            put(&mut item, "schema", field("schema"));
            put(&mut item, "parentHash", field("parent_hash"));
        }
        "flow" => {
            put(&mut item, "draftPath", draft_path(draft));
            put(&mut item, "summary", field("summary"));
            put(&mut item, "parentVersionId", field("version_id"));
            let or_null = |v: Option<&Value>| v.cloned().unwrap_or(Value::Null);
            item.insert(
                "value".into(),
                json!({
                    "value": or_null(draft.get("value")),
                    "schema": or_null(draft.get("schema")),
                    "groups": or_null(draft.get("value").and_then(|v| v.get("groups"))),
                    "description": or_null(draft.get("description")),
                }),
            );
        }
        "app" => {
            let value = normalize_app_draft(draft);
            put(&mut item, "draftPath", draft_path(&value));
            put(&mut item, "summary", value.get("summary").cloned());
            put(
                &mut item,
                "parentVersionId",
                value.get("parent_version").cloned(),
            );
            item.insert("value".into(), value);
            item.insert("rawApp".into(), json!(true));
        }
        "schedule" => {
            put(
                &mut item,
                "summary",
                non_null(draft.get("summary")).cloned(),
            );
            item.insert("value".into(), draft.clone());
        }
        "trigger" => {
            put(
                &mut item,
                "summary",
                field("summary").filter(|s| truthy(Some(s))),
            );
            item.insert("value".into(), draft.clone());
        }
        "resource" => {
            put(
                &mut item,
                "summary",
                field("description").filter(|d| truthy(Some(d))),
            );
            let mut value = Map::new();
            value.insert("path".into(), json!(path));
            put(&mut value, "value", field("args"));
            put(&mut value, "description", field("description"));
            value.insert(
                "resource_type".into(),
                non_null(draft.get("resource_type"))
                    .cloned()
                    .unwrap_or_else(|| json!("")),
            );
            put(&mut value, "labels", field("labels"));
            put(&mut value, "ws_specific", field("wsSpecific"));
            item.insert("value".into(), Value::Object(value));
        }
        "variable" => {
            let variable = draft.get("variable").cloned().unwrap_or(Value::Null);
            put(
                &mut item,
                "summary",
                variable
                    .get("description")
                    .filter(|d| truthy(Some(d)))
                    .cloned(),
            );
            put(&mut item, "isSecret", variable.get("is_secret").cloned());
            let mut value = Map::new();
            value.insert("path".into(), json!(path));
            put(&mut value, "value", variable.get("value").cloned());
            put(&mut value, "is_secret", variable.get("is_secret").cloned());
            put(
                &mut value,
                "description",
                variable.get("description").cloned(),
            );
            put(&mut value, "account", field("account"));
            put(&mut value, "is_oauth", field("is_oauth"));
            put(&mut value, "expires_at", field("expires_at"));
            put(&mut value, "labels", field("labels"));
            put(&mut value, "ws_specific", field("wsSpecific"));
            item.insert("value".into(), Value::Object(value));
        }
        _ => return None,
    }
    item.insert("isDraft".into(), json!(true));
    Some(Value::Object(item))
}

pub(super) async fn get_draft_item(
    ctx: &Ctx<'_>,
    item_type: &str,
    path: &str,
    trigger_kind: Option<&str>,
) -> Result<Option<(Value, Draft)>, String> {
    let Some(kind) = draft_kind(item_type, trigger_kind) else {
        return Ok(None);
    };
    let Some(draft) = get_draft(ctx, &kind, path).await? else {
        return Ok(None);
    };
    Ok(draft_item(&kind, path, &draft.value).map(|item| (item, draft)))
}

fn stash_inline_script(
    value: &mut Map<String, Value>,
    id: &str,
    scripts: &mut HashMap<String, String>,
) {
    if let Some(Value::String(content)) = value.get("content") {
        scripts.insert(id.to_string(), content.clone());
        value.insert("content".into(), json!(format!("inline_script.{id}")));
    }
}

fn compact_modules(modules: Option<&Value>, scripts: &mut HashMap<String, String>) -> Value {
    let Some(modules) = modules.and_then(Value::as_array) else {
        return json!([]);
    };
    Value::Array(
        modules
            .iter()
            .map(|module| compact_module(module, scripts))
            .collect(),
    )
}

fn compact_branches(value: &mut Map<String, Value>, scripts: &mut HashMap<String, String>) {
    let Some(Value::Array(branches)) = value.get_mut("branches") else {
        return;
    };
    for branch in branches {
        if let Value::Object(branch) = branch {
            let modules = compact_modules(branch.get("modules"), scripts);
            branch.insert("modules".into(), modules);
        }
    }
}

fn compact_module(module: &Value, scripts: &mut HashMap<String, String>) -> Value {
    let mut module = module.clone();
    let id = module
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let Some(value) = module.get_mut("value").and_then(Value::as_object_mut) else {
        return module;
    };
    match value.get("type").and_then(Value::as_str) {
        Some("rawscript") => stash_inline_script(value, &id, scripts),
        Some("forloopflow" | "whileloopflow") if truthy(value.get("modules")) => {
            let modules = compact_modules(value.get("modules"), scripts);
            value.insert("modules".into(), modules);
        }
        Some("branchone") => {
            compact_branches(value, scripts);
            if truthy(value.get("default")) {
                let default = compact_modules(value.get("default"), scripts);
                value.insert("default".into(), default);
            }
        }
        Some("branchall") => compact_branches(value, scripts),
        Some("aiagent") => {
            if let Some(Value::Array(tools)) = value.get_mut("tools") {
                for tool in tools {
                    let tool_id = tool
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let Some(tool_value) = tool.get_mut("value").and_then(Value::as_object_mut)
                    else {
                        continue;
                    };
                    let is = |key: &str, expected: &str| {
                        tool_value.get(key).and_then(Value::as_str) == Some(expected)
                    };
                    if is("tool_type", "flowmodule") && is("type", "rawscript") {
                        stash_inline_script(tool_value, &tool_id, scripts);
                    }
                }
            }
        }
        _ => {}
    }
    module
}

pub(super) fn compact_flow(
    value: &Value,
    schema: Option<&Value>,
) -> (Value, HashMap<String, String>) {
    let mut scripts = HashMap::new();
    let mut out = Map::new();
    out.insert(
        "modules".into(),
        compact_modules(value.get("modules"), &mut scripts),
    );
    out.insert("schema".into(), schema.cloned().unwrap_or(Value::Null));
    for key in ["preprocessor_module", "failure_module"] {
        let mut module = value.get(key).cloned().unwrap_or(Value::Null);
        let id = module
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if let Some(module_value) = module.get_mut("value").and_then(Value::as_object_mut) {
            if module_value.get("type").and_then(Value::as_str) == Some("rawscript") {
                stash_inline_script(module_value, &id, &mut scripts);
            }
        }
        out.insert(key.into(), module);
    }
    for key in ["groups", "notes"] {
        out.insert(key.into(), value.get(key).cloned().unwrap_or(Value::Null));
    }
    for key in FLOW_VALUE_SETTINGS_KEYS {
        put(&mut out, key, value.get(*key).cloned());
    }
    (Value::Object(out), scripts)
}

pub(super) fn app_source_to_draft_value(app: &Value, fallback: Option<&Value>) -> Value {
    // A deployed app nests its source under `value`; a draft carries it at the top.
    let source = app.get("value").filter(|v| !v.is_null()).unwrap_or(app);
    let data = source.get("data");
    let creation = data.and_then(|d| d.get("creation"));
    let tables_only = |tables: &Value| json!({ "tables": tables });
    let data = if truthy(creation) {
        let mut out = Map::new();
        out.insert(
            "tables".into(),
            data.and_then(|d| non_null(d.get("tables")))
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        put(
            &mut out,
            "datatable",
            creation.and_then(|c| c.get("datatable")).cloned(),
        );
        put(
            &mut out,
            "schema",
            creation.and_then(|c| c.get("schema")).cloned(),
        );
        Value::Object(out)
    } else if truthy(data) {
        data.cloned().unwrap_or_default()
    } else if truthy(source.get("datatables")) {
        tables_only(&source["datatables"])
    } else if truthy(source.get("dataTableRefs")) {
        tables_only(&source["dataTableRefs"])
    } else {
        tables_only(&json!([]))
    };
    let or_fallback = |key: &str| {
        non_null(app.get(key))
            .cloned()
            .or_else(|| fallback?.get(key).cloned())
    };
    let mut out = Map::new();
    out.insert(
        "summary".into(),
        non_null(app.get("summary"))
            .cloned()
            .unwrap_or_else(|| json!("")),
    );
    out.insert("files".into(), object_or_empty(source.get("files")));
    out.insert("runnables".into(), object_or_empty(source.get("runnables")));
    out.insert("data".into(), data);
    put(&mut out, "policy", or_fallback("policy"));
    put(&mut out, "custom_path", or_fallback("custom_path"));
    // The fork base: the head of a deployed app's versions, or what a draft already pinned.
    let head = app
        .get("versions")
        .and_then(Value::as_array)
        .and_then(|versions| versions.last())
        .filter(|v| !v.is_null())
        .cloned();
    put(
        &mut out,
        "parent_version",
        non_null(app.get("parent_version"))
            .cloned()
            .or(head)
            .or_else(|| fallback?.get("parent_version").cloned()),
    );
    Value::Object(out)
}

pub(super) fn summarize_app_value(value: &Value) -> Value {
    let entries = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
    };
    let frontend: Vec<Value> = entries("files")
        .iter()
        .map(|(path, content)| json!({ "path": path, "size": content.as_str().map(js_len).unwrap_or(0) }))
        .collect();
    let backend: Vec<Value> = entries("runnables")
        .iter()
        .map(|(key, runnable)| {
            let mut out = Map::new();
            out.insert("key".into(), json!(key));
            if !truthy(Some(runnable)) {
                out.insert("name".into(), json!(key));
                out.insert("type".into(), json!("inline"));
                return Value::Object(out);
            }
            out.insert(
                "name".into(),
                non_null(runnable.get("name"))
                    .cloned()
                    .unwrap_or_else(|| json!(key)),
            );
            let kind = runnable.get("type").and_then(Value::as_str);
            let by_path = matches!(kind, Some("path" | "runnableByPath"));
            let run_type = match runnable.get("runType").and_then(Value::as_str) {
                _ if !by_path => "inline",
                Some("flow") => "flow",
                Some("hubscript") => "hubscript",
                _ => "script",
            };
            out.insert("type".into(), json!(run_type));
            if by_path && truthy(runnable.get("path")) {
                out.insert("path".into(), runnable["path"].clone());
            }
            let inline = runnable.get("inlineScript").filter(|s| truthy(Some(s)));
            if let Some(inline) =
                inline.filter(|_| matches!(kind, Some("inline" | "runnableByName")))
            {
                let language = match inline.get("language") {
                    Some(Value::String(l)) if l == "nativets" || l == "deno" => Some(json!("bun")),
                    other => other.cloned(),
                };
                put(&mut out, "language", language);
                let content = inline
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                out.insert("contentSize".into(), json!(js_len(content)));
            }
            let static_keys: Vec<&String> = runnable
                .get("fields")
                .and_then(Value::as_object)
                .map(|fields| {
                    fields
                        .iter()
                        .filter(|(_, v)| v.get("type").and_then(Value::as_str) == Some("static"))
                        .map(|(k, _)| k)
                        .collect()
                })
                .unwrap_or_default();
            if !static_keys.is_empty() {
                out.insert("staticInputKeys".into(), json!(static_keys));
            }
            Value::Object(out)
        })
        .collect();
    let mut out = Map::new();
    out.insert("frontend".into(), Value::Array(frontend));
    out.insert("backend".into(), Value::Array(backend));
    if truthy(value.get("data")) {
        out.insert("data".into(), value["data"].clone());
    }
    Value::Object(out)
}

/// `guest` is stored on an app whether or not anyone is admitted by it, so the mode
/// alone would say strangers can open an app that admits members only.
pub(super) async fn describe_app_exposure(ctx: &Ctx<'_>, mode: Option<&Value>) -> Option<Value> {
    if mode.and_then(Value::as_str) != Some("guest") {
        return mode.cloned();
    }
    let usage = ctx.get(&ctx.ws("workspaces/guest_usage")).await;
    let settings = ctx.get(&ctx.ws("workspaces/get_public_settings")).await;
    // A switch that could not be read proves nothing either way: report the mode bare.
    let (Ok(usage), Ok(settings)) = (usage, settings) else {
        return mode.cloned();
    };
    let live = truthy(usage.get("available"))
        && truthy(usage.get("instance_enabled"))
        && truthy(settings.get("guest_access_enabled"));
    if live {
        mode.cloned()
    } else {
        Some(json!(
            "guest, but inert: the instance or workspace admits no guest, so this app admits members only"
        ))
    }
}

pub(super) fn serialize_for_read(item: &Value, execution_mode: Option<Value>) -> Value {
    let item_type = item.get("type").and_then(Value::as_str);
    let mut out = Map::new();
    let copy = |out: &mut Map<String, Value>, keys: &[&str]| {
        for key in keys {
            put(out, key, item.get(*key).cloned());
        }
    };
    match item_type {
        Some("variable") => copy(
            &mut out,
            &["type", "path", "summary", "isSecret", "isDraft"],
        ),
        Some("app") if item.get("value").is_some_and(|v| v.get("files").is_some()) => {
            copy(&mut out, &["type", "path", "summary"]);
            out.insert("value".into(), summarize_app_value(&item["value"]));
            copy(&mut out, &["rawApp"]);
            put(
                &mut out,
                "executionMode",
                execution_mode.or_else(|| item.get("executionMode").cloned()),
            );
            copy(&mut out, &["isDraft"]);
        }
        Some("flow") if truthy(item.get("value")) => {
            let flow = &item["value"];
            // A draft carries `groups` beside the flow value; a set one outranks the value's.
            let mut value = flow.get("value").cloned().unwrap_or_else(|| json!({}));
            if let (Some(groups), Some(value)) = (flow.get("groups"), value.as_object_mut()) {
                if groups.is_null() {
                    value.remove("groups");
                } else {
                    value.insert("groups".into(), groups.clone());
                }
            }
            copy(&mut out, &["type", "path", "summary"]);
            out.insert(
                "value".into(),
                compact_flow(&value, non_null(flow.get("schema"))).0,
            );
            copy(&mut out, &["isDraft"]);
        }
        _ => {
            let mut item = item.clone();
            if let (Some(mode), Some(item)) = (execution_mode, item.as_object_mut()) {
                item.insert("executionMode".into(), mode);
            }
            return item;
        }
    }
    Value::Object(out)
}

pub(super) struct LoadedScript {
    pub content: String,
    pub language: String,
    pub summary: Option<String>,
    pub schema: Option<Value>,
}

pub(super) async fn load_script(ctx: &Ctx<'_>, path: &str) -> Result<LoadedScript, String> {
    let text = |value: &Value, key: &str| value.get(key).and_then(Value::as_str).map(String::from);
    if let Some((item, _)) = get_draft_item(ctx, "script", path, None).await? {
        let (Some(content), Some(language)) = (
            text(&item, "value"),
            text(&item, "language").filter(|l| !l.is_empty()),
        ) else {
            return Err(format!(
                "Draft script \"{path}\" is missing content or language."
            ));
        };
        return Ok(LoadedScript {
            content,
            language,
            summary: text(&item, "summary"),
            schema: non_null(item.get("schema")).cloned(),
        });
    }
    let script = get_deployed(ctx, "script", None, path).await?;
    Ok(LoadedScript {
        content: text(&script, "content").unwrap_or_default(),
        language: text(&script, "language").unwrap_or_default(),
        summary: text(&script, "summary"),
        schema: non_null(script.get("schema")).cloned(),
    })
}

pub(super) struct LoadedFlow {
    /// `{value, schema, groups}`, plus `description` when read from a draft.
    pub flow: Value,
    pub summary: Option<String>,
    pub is_draft: bool,
}

pub(super) async fn load_flow(ctx: &Ctx<'_>, path: &str) -> Result<LoadedFlow, String> {
    let summary = |value: &Value| {
        value
            .get("summary")
            .and_then(Value::as_str)
            .map(String::from)
    };
    if let Some((item, _)) = get_draft_item(ctx, "flow", path, None).await? {
        let Some(flow) = item.get("value").filter(|v| v.is_object()) else {
            return Err(format!("Draft flow \"{path}\" has no value."));
        };
        return Ok(LoadedFlow { flow: flow.clone(), summary: summary(&item), is_draft: true });
    }
    let flow = get_deployed(ctx, "flow", None, path).await?;
    Ok(LoadedFlow { flow: deployed_flow_value(&flow), summary: summary(&flow), is_draft: false })
}

fn deployed_flow_value(flow: &Value) -> Value {
    let mut out = Map::new();
    put(&mut out, "value", flow.get("value").cloned());
    put(&mut out, "schema", flow.get("schema").cloned());
    out.insert(
        "groups".into(),
        flow.get("value")
            .and_then(|v| non_null(v.get("groups")))
            .cloned()
            .unwrap_or(Value::Null),
    );
    Value::Object(out)
}

pub(super) struct WriteBase {
    pub base: Option<Value>,
    pub from_draft: bool,
    pub last_sync: Option<String>,
}

pub(super) async fn write_base(
    ctx: &Ctx<'_>,
    item_type: &str,
    path: &str,
    trigger_kind: Option<&str>,
) -> Result<WriteBase, String> {
    // A failed draft read must not fall through to the deployed item: the write would
    // then be built on it and replace the draft's own changes.
    let draft = match draft_kind(item_type, trigger_kind) {
        Some(kind) => get_draft(ctx, &kind, path).await?,
        None => None,
    };
    if let Some(draft) = draft {
        return Ok(WriteBase {
            base: Some(draft.value),
            from_draft: true,
            last_sync: draft.created_at,
        });
    }
    let base = if item_exists(ctx, item_type, trigger_kind, path).await? {
        Some(get_deployed(ctx, item_type, trigger_kind, path).await?)
    } else {
        None
    };
    Ok(WriteBase { base, from_draft: false, last_sync: None })
}

pub(super) enum Persist {
    Saved(Value),
    Failed(String),
}

pub(super) async fn persist_draft(
    ctx: &Ctx<'_>,
    item_type: &str,
    path: &str,
    trigger_kind: Option<&str>,
    value: &Value,
    last_sync: Option<&str>,
    force: bool,
) -> Result<Persist, String> {
    let kind = draft_kind(item_type, trigger_kind)
        .ok_or_else(|| format!("Unsupported draft type \"{item_type}\"."))?;
    let item = draft_item(&kind, path, value)
        .ok_or_else(|| format!("Could not synthesize {item_type} draft \"{path}\"."))?;
    let failure = match save_draft(ctx, &kind, path, Some(value), last_sync, force).await {
        Ok(true) => return Ok(Persist::Saved(item)),
        Ok(false) => json!({
            "success": false,
            "conflict": true,
            "message": format!("The {item_type} draft \"{path}\" changed externally since you last read it. Re-run this tool to merge onto the latest version, or pass override:true to overwrite. If an editor for it is open, a conflict dialog is also shown there."),
        }),
        Err(e) => json!({
            "success": false,
            "error": true,
            "message": format!("The {item_type} draft \"{path}\" could NOT be saved ({}). The change was not persisted — retry; do not assume it succeeded.", api_message(&e)),
        }),
    };
    Ok(Persist::Failed(pretty(&failure)))
}

pub(super) fn finish_draft_write(item: &Value, existed: bool) -> String {
    let text = |key: &str| item.get(key).and_then(Value::as_str).unwrap_or_default();
    let (item_type, path) = (text("type"), text("path"));
    let verb = if existed { "Updated" } else { "Created" };
    let mut out = Map::new();
    out.insert("success".into(), json!(true));
    out.insert(
        "message".into(),
        json!(format!("{verb} {item_type} \"{path}\" as a per-user draft (saved server-side, visible only to this user — not a deployed workspace item). It was not deployed.")),
    );
    // A flow is not echoed back: the model just sent it. A variable comes back redacted.
    match item_type {
        "flow" => {}
        "variable" => {
            out.insert("item".into(), serialize_for_read(item, None));
        }
        _ => {
            out.insert("item".into(), item.clone());
        }
    }
    pretty(&Value::Object(out))
}

pub(super) fn find_and_replace(
    content: &str,
    old: &str,
    new: &str,
    replace_all: bool,
    label: &str,
) -> Result<String, String> {
    let matches = if old.is_empty() {
        0
    } else {
        content.matches(old).count()
    };
    if matches == 0 {
        return Err(format!("old_string was not found in the {label}."));
    }
    if !replace_all && matches != 1 {
        return Err(format!(
            "old_string matched {matches} locations. Make it more specific or set replace_all to true."
        ));
    }
    Ok(content.replace(old, new))
}

pub(super) fn empty_schema() -> Value {
    json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "properties": {},
        "required": [],
        "type": "object",
    })
}

pub(super) fn infer_script_schema(
    language: &str,
    content: &str,
    previous: Option<&Value>,
    entrypoint: Option<&str>,
) -> Option<Value> {
    use windmill_common::scripts::{ScriptKind, ScriptLang};
    let language: ScriptLang = serde_json::from_value(Value::String(language.to_string())).ok()?;
    let kind = (entrypoint == Some("preprocessor")).then_some(ScriptKind::Preprocessor);
    let schema = windmill_api_scripts::scripts::infer_main_schema(
        &language,
        kind.as_ref(),
        content,
        previous,
    )?;
    serde_json::to_value(schema).ok()
}

/// Lines with their terminator kept, so a missing final newline is a difference.
fn patch_lines(text: &str) -> Vec<&str> {
    text.split_inclusive('\n').collect()
}

/// `(tag, line)` for every line of both sides, `' '` for a shared one, deletions ahead
/// of insertions within a change.
fn diff_lines<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(char, &'a str)> {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (a, b) = (
        &old[prefix..old.len() - suffix],
        &new[prefix..new.len() - suffix],
    );
    let mut ops: Vec<(char, &str)> = old[..prefix].iter().map(|l| (' ', *l)).collect();
    let (n, m) = (a.len(), b.len());
    // ponytail: quadratic LCS table. Past the cap the changed region is reported as
    // replaced wholesale, still a valid patch; switch to Myers if that ever shows.
    const MAX_CELLS: usize = 16_000_000;
    if (n + 1).saturating_mul(m + 1) > MAX_CELLS {
        ops.extend(a.iter().map(|l| ('-', *l)));
        ops.extend(b.iter().map(|l| ('+', *l)));
    } else {
        let width = m + 1;
        let mut lcs = vec![0u32; (n + 1) * width];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                lcs[i * width + j] = if a[i] == b[j] {
                    lcs[(i + 1) * width + j + 1] + 1
                } else {
                    lcs[(i + 1) * width + j].max(lcs[i * width + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        let mut inserted = Vec::new();
        while i < n || j < m {
            if i < n && j < m && a[i] == b[j] {
                ops.extend(inserted.drain(..));
                ops.push((' ', a[i]));
                i += 1;
                j += 1;
            } else if j == m || (i < n && lcs[(i + 1) * width + j] >= lcs[i * width + j + 1]) {
                ops.push(('-', a[i]));
                i += 1;
            } else {
                inserted.push(('+', b[j]));
                j += 1;
            }
        }
        ops.extend(inserted);
    }
    ops.extend(old[old.len() - suffix..].iter().map(|l| (' ', *l)));
    ops
}

pub(super) fn unified_patch(
    old_name: &str,
    new_name: &str,
    old: &str,
    new: &str,
    context: usize,
) -> String {
    let ops = diff_lines(&patch_lines(old), &patch_lines(new));
    let mut out = String::new();
    if old_name == new_name {
        out.push_str(&format!("Index: {old_name}\n"));
    }
    out.push_str(&format!(
        "===================================================================\n--- {old_name}\t\n+++ {new_name}\t\n"
    ));
    let changes: Vec<usize> = (0..ops.len()).filter(|i| ops[*i].0 != ' ').collect();
    let mut next = 0;
    while next < changes.len() {
        let first = changes[next];
        let mut last = first;
        // Two changes share a hunk when their contexts would touch.
        while next + 1 < changes.len() && changes[next + 1] - last - 1 <= 2 * context {
            next += 1;
            last = changes[next];
        }
        next += 1;
        let start = first.saturating_sub(context);
        let end = (last + context + 1).min(ops.len());
        let count =
            |range: &[(char, &str)], skip: char| range.iter().filter(|op| op.0 != skip).count();
        let (old_before, new_before) = (count(&ops[..start], '+'), count(&ops[..start], '-'));
        let (old_count, new_count) = (count(&ops[start..end], '+'), count(&ops[start..end], '-'));
        // An empty side is addressed by the line before it.
        let old_start = old_before + usize::from(old_count > 0);
        let new_start = new_before + usize::from(new_count > 0);
        out.push_str(&format!(
            "@@ -{old_start},{old_count} +{new_start},{new_count} @@\n"
        ));
        for (tag, line) in &ops[start..end] {
            out.push(*tag);
            match line.strip_suffix('\n') {
                Some(line) => {
                    out.push_str(line);
                    out.push('\n');
                }
                None => {
                    out.push_str(line);
                    out.push_str("\n\\ No newline at end of file\n");
                }
            }
        }
    }
    out
}

// ============================================================================
// Tools
// ============================================================================

fn deployed_item(item_type: &str, trigger_kind: Option<&str>, row: &Value) -> Value {
    let mut item = Map::new();
    item.insert("type".into(), json!(item_type));
    put(&mut item, "triggerKind", trigger_kind.map(Value::from));
    put(&mut item, "path", row.get("path").cloned());
    let mut is_draft = false;
    match item_type {
        "script" | "flow" => {
            put(&mut item, "summary", row.get("summary").cloned());
            if item_type == "script" {
                put(&mut item, "language", row.get("language").cloned());
            }
            // A listing synthesizes rows for drafts with no deployed counterpart.
            is_draft = truthy(row.get("draft_only"));
        }
        "schedule" | "trigger" => put(&mut item, "summary", non_null(row.get("summary")).cloned()),
        "resource" => put(&mut item, "summary", row.get("description").cloned()),
        "variable" => {
            put(&mut item, "summary", row.get("description").cloned());
            put(&mut item, "isSecret", row.get("is_secret").cloned());
        }
        _ => {
            put(&mut item, "summary", row.get("summary").cloned());
            // The server omits the flag rather than sending false.
            item.insert(
                "rawApp".into(),
                non_null(row.get("raw_app"))
                    .cloned()
                    .unwrap_or(json!(false)),
            );
        }
    }
    item.insert("isDraft".into(), json!(is_draft));
    Value::Object(item)
}

/// Put `value` (and a script's `schema`) where the session's item shape has them.
fn with_value(item: Value, value: Option<Value>, schema: Option<Value>) -> Value {
    let Value::Object(item) = item else {
        return item;
    };
    let mut out = Map::new();
    for (key, field) in item {
        if key == "isDraft" {
            put(&mut out, "value", value.clone());
            put(&mut out, "schema", schema.clone());
        }
        out.insert(key, field);
    }
    Value::Object(out)
}

async fn list_workspace_items(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let requested: Vec<&str> = args
        .get("types")
        .and_then(Value::as_array)
        .map(|types| types.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    let types = if requested.is_empty() {
        vec!["script", "flow"]
    } else {
        requested
    };
    let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(50);
    let page = args.get("page").and_then(Value::as_u64);
    let prefix = opt_str_arg(args, "path_prefix");

    let mut query = format!("per_page={}", limit.min(MAX_LIST_LIMIT));
    if let Some(page) = page {
        query.push_str(&format!("&page={page}"));
    }
    if let Some(prefix) = prefix {
        query.push_str(&format!("&path_start={}", enc(prefix)));
    }

    let mut by_key = Map::new();
    for item_type in ITEM_TYPES.iter().filter(|t| types.contains(t)) {
        let kinds: Vec<Option<&str>> = if *item_type == "trigger" {
            TRIGGER_KINDS.iter().map(|kind| Some(*kind)).collect()
        } else {
            vec![None]
        };
        let extra = if matches!(*item_type, "script" | "flow") {
            "&include_draft_only=true&without_description=true"
        } else {
            ""
        };
        for trigger_kind in kinds {
            let route = ctx.ws(&format!(
                "{}/list?{query}{extra}",
                service(item_type, trigger_kind)?
            ));
            let rows = match ctx.get(&route).await {
                Ok(rows) => rows,
                // A trigger kind this build does not serve answers 404: skip that kind
                // alone. Any other failure is real.
                Err(e) if trigger_kind.is_some() && is_not_found(&e) => continue,
                Err(e) => return Err(e),
            };
            for row in array(Some(&rows)) {
                let path = row.get("path").and_then(Value::as_str).unwrap_or_default();
                by_key.insert(
                    item_key(item_type, path, trigger_kind),
                    deployed_item(item_type, trigger_kind, row),
                );
            }
        }
    }

    // Drafts are not paginated: they overlay page 1 only, capped per type, so later
    // pages never repeat a page-1 item as its draft twin.
    if page.unwrap_or(1) == 1 {
        let mut count_by_type: HashMap<String, u64> = HashMap::new();
        for draft in list_drafts(ctx).await? {
            let text = |key: &str| draft.get(key).and_then(Value::as_str);
            let (item_type, path) = (
                text("type").unwrap_or_default(),
                text("path").unwrap_or_default(),
            );
            if !types.contains(&item_type) {
                continue;
            }
            // A draft's staged name is often not where it is stored, and this pass is
            // the only source of draft-only rows under a prefix: either name may match.
            if let Some(prefix) = prefix {
                let staged = text("draftPath").is_some_and(|p| p.starts_with(prefix));
                if !path.starts_with(prefix) && !staged {
                    continue;
                }
            }
            let count = count_by_type.entry(item_type.to_string()).or_default();
            if *count >= limit {
                continue;
            }
            *count += 1;
            by_key.insert(
                item_key(item_type, path, text("triggerKind")),
                draft.clone(),
            );
        }
    }

    let query = opt_str_arg(args, "query")
        .map(|q| q.trim().to_lowercase())
        .filter(|q| !q.is_empty());
    let results: Vec<Value> = by_key
        .into_iter()
        .map(|(_, item)| item)
        .filter(|item| {
            let Some(query) = &query else { return true };
            ["path", "summary"].iter().any(|key| {
                item.get(*key)
                    .and_then(Value::as_str)
                    .is_some_and(|text| text.to_lowercase().contains(query))
            })
        })
        .collect();
    Ok(pretty(&Value::Array(results)))
}

async fn read_workspace_item(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let item_type = item_type_arg(args)?;
    let path = str_arg(args, "path")?;
    let trigger_kind = opt_str_arg(args, "trigger_kind");
    if item_type == "trigger" && trigger_kind.is_none() {
        return Ok(
            json!({ "success": false, "error": "trigger_kind is required when type is trigger." })
                .to_string(),
        );
    }
    let deployed_only = opt_str_arg(args, "version") == Some("deployed");
    // Hub scripts live outside the workspace: no draft, no workspace lookup.
    let hub = path.starts_with("hub/");

    if !deployed_only && !hub {
        if let Some((item, _)) = get_draft_item(ctx, item_type, path, trigger_kind).await? {
            // The mode reported is the draft's, which is what deploying it will write.
            let mode = if item_type == "app" {
                let mode = item
                    .get("value")
                    .and_then(|v| v.get("policy"))
                    .and_then(|p| p.get("execution_mode"));
                describe_app_exposure(ctx, mode).await
            } else {
                None
            };
            return Ok(pretty(&serialize_for_read(&item, mode)));
        }
    }

    let item = match item_type {
        "script" if hub => {
            let script = ctx
                .get(&format!("/scripts/hub/get_full/{}", path_enc(path)))
                .await?;
            let mut row = script.clone();
            row["path"] = json!(path);
            let mut item = with_value(
                deployed_item("script", None, &row),
                script.get("content").cloned(),
                script.get("schema").cloned(),
            );
            item["isDraft"] = json!(false);
            item
        }
        "script" | "flow" => {
            let route = get_route(ctx, item_type, None, path)?;
            let deployed = ctx
                .get(&format!("{route}?get_draft={}", !deployed_only))
                .await?;
            // The draft stored on the item itself outranks its deployed version.
            let row = non_null(deployed.get("draft"))
                .cloned()
                .filter(|_| !deployed_only)
                .unwrap_or(deployed);
            if item_type == "script" {
                with_value(
                    deployed_item("script", None, &row),
                    row.get("content").cloned(),
                    row.get("schema").cloned(),
                )
            } else {
                let item = with_value(
                    deployed_item("flow", None, &row),
                    Some(deployed_flow_value(&row)),
                    None,
                );
                serialize_for_read(&item, None)
            }
        }
        "variable" => deployed_item(
            "variable",
            None,
            &get_deployed(ctx, "variable", None, path).await?,
        ),
        "app" => {
            // Metadata only: file and runnable contents come through `read_app_file`.
            let app = get_deployed(ctx, "app", None, path).await?;
            let mode =
                describe_app_exposure(ctx, app.get("policy").and_then(|p| p.get("execution_mode")))
                    .await;
            let mut item = Map::new();
            item.insert("type".into(), json!("app"));
            put(&mut item, "path", app.get("path").cloned());
            // A grid is not files and runnables: summarizing one would report an empty app.
            if app.get("raw_app") == Some(&json!(false)) {
                put(&mut item, "summary", app.get("summary").cloned());
            } else {
                let value = app_source_to_draft_value(&app, None);
                put(&mut item, "summary", value.get("summary").cloned());
                item.insert("value".into(), summarize_app_value(&value));
            }
            put(&mut item, "rawApp", app.get("raw_app").cloned());
            put(&mut item, "executionMode", mode);
            item.insert("isDraft".into(), json!(false));
            Value::Object(item)
        }
        _ => {
            let row = get_deployed(ctx, item_type, trigger_kind, path).await?;
            with_value(
                deployed_item(item_type, trigger_kind, &row),
                Some(row.clone()),
                None,
            )
        }
    };
    Ok(pretty(&item))
}

async fn write_script_draft(
    ctx: &Ctx<'_>,
    path: &str,
    summary: Option<&str>,
    language: &str,
    content: &str,
    force: bool,
) -> ToolResult {
    let base = write_base(ctx, "script", path, None).await?;
    let existed = base.base.is_some();
    let mut draft = match base.base {
        Some(Value::Object(mut script)) => {
            if !base.from_draft {
                let hash = script.get("hash").cloned().unwrap_or(Value::Null);
                script.insert("parent_hash".into(), hash);
            }
            script.insert("path".into(), json!(path));
            put(&mut script, "summary", summary.map(Value::from));
            script.insert("content".into(), json!(content));
            script.insert("language".into(), json!(language));
            script
        }
        _ => json!({
            "path": path,
            "summary": summary.unwrap_or_default(),
            "description": "",
            "content": content,
            "schema": empty_schema(),
            "is_template": false,
            "language": language,
            "kind": "script",
        })
        .as_object()
        .cloned()
        .unwrap_or_default(),
    };
    // Inferred into the schema the base carries: its properties are the only copy of
    // what no parser emits (`password`, enums, formats, titles). Code that does not
    // parse keeps the schema as it was.
    let previous = draft
        .get("schema")
        .filter(|schema| truthy(schema.get("properties")))
        .cloned()
        .unwrap_or_else(empty_schema);
    let code = if content.is_empty() { " " } else { content };
    // A language this build cannot parse, or code that does not parse yet, keeps the
    // schema the draft had.
    let schema = infer_script_schema(language, code, Some(&previous), None).unwrap_or(previous);
    draft.insert("schema".into(), schema);
    let persisted = persist_draft(
        ctx,
        "script",
        path,
        None,
        &Value::Object(draft),
        base.last_sync.as_deref(),
        force,
    )
    .await?;
    Ok(match persisted {
        Persist::Saved(item) => finish_draft_write(&item, existed),
        Persist::Failed(text) => text,
    })
}

async fn edit_script(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let script = load_script(ctx, path).await?;
    let updated = find_and_replace(
        &script.content,
        str_arg(args, "old_string")?,
        str_arg(args, "new_string")?,
        bool_arg(args, "replace_all"),
        "script source",
    )?;
    write_script_draft(
        ctx,
        path,
        script.summary.as_deref(),
        &script.language,
        &updated,
        false,
    )
    .await
}

async fn discard_local_draft(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let item_type = item_type_arg(args)?;
    let path = str_arg(args, "path")?;
    let trigger_kind = opt_str_arg(args, "trigger_kind");
    if item_type == "trigger" && trigger_kind.is_none() {
        return Err("trigger_kind is required when discarding a trigger draft.".to_string());
    }
    let not_found = || format!("No draft found for {item_type} \"{path}\".");
    let kind = draft_kind(item_type, trigger_kind).ok_or_else(not_found)?;
    let draft = get_draft(ctx, &kind, path).await?.ok_or_else(not_found)?;
    delete_draft(ctx, &kind, path, draft.created_at.as_deref()).await?;
    let mut out = Map::new();
    out.insert("success".into(), json!(true));
    out.insert(
        "message".into(),
        json!(format!(
            "Discarded the draft for {item_type} \"{path}\". The deployed workspace item was not changed."
        )),
    );
    out.insert("type".into(), json!(item_type));
    out.insert("path".into(), json!(path));
    put(&mut out, "triggerKind", trigger_kind.map(Value::from));
    Ok(pretty(&Value::Object(out)))
}

/// Discards a stale draft and hands back its own changes against the version it was
/// forked from. Discarding rather than resetting keeps the fork base honest: the next
/// write starts from the current head, and a premature deploy finds no draft instead
/// of shipping the latest version unchanged.
async fn rebase_draft(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let item_type = item_type_arg(args)?;
    let path = str_arg(args, "path")?;
    let (label, noun) = match item_type {
        "script" => ("script", "Script"),
        "flow" => ("flow", "Flow"),
        "app" => ("app", "App"),
        _ => return Err("rebase_draft currently supports scripts, flows, and apps.".to_string()),
    };
    let no_draft = || format!("No {label} draft found for \"{path}\".");
    let (item, draft) = get_draft_item(ctx, item_type, path, None)
        .await?
        .ok_or_else(no_draft)?;
    let value = item.get("value").cloned().unwrap_or(Value::Null);
    let usable = match item_type {
        "script" => value.is_string() && truthy(item.get("language")),
        _ => value.is_object(),
    };
    if !usable {
        return Err(no_draft());
    }
    if !item_exists(ctx, item_type, None, path).await? {
        return Err(format!(
            "{noun} \"{path}\" is not deployed; there is no newer version to rebase onto."
        ));
    }
    let latest = get_deployed(ctx, item_type, None, path).await?;
    let (base, head, head_key) = match item_type {
        "script" => (
            item.get("parentHash").filter(|h| truthy(Some(h))).cloned(),
            latest.get("hash").cloned(),
            "latest_hash",
        ),
        "flow" => (
            non_null(item.get("parentVersionId")).cloned(),
            latest.get("version_id").cloned(),
            "latest_version",
        ),
        _ => (
            non_null(item.get("parentVersionId")).cloned(),
            latest
                .get("versions")
                .and_then(Value::as_array)
                .and_then(|versions| versions.last())
                .cloned(),
            "latest_version",
        ),
    };
    let head_text = head
        .as_ref()
        .map_or_else(|| "undefined".to_string(), js_str);
    if base.is_some() && base == head {
        let mut out = Map::new();
        out.insert("success".into(), json!(true));
        out.insert("alreadyLatest".into(), json!(true));
        put(&mut out, head_key, head);
        out.insert(
            "message".into(),
            json!(format!(
                "Draft \"{path}\" is already based on the latest deployed version ({head_text})."
            )),
        );
        return Ok(Value::Object(out).to_string());
    }

    // Captured before the discard: this diff is the only record of what to replay. A
    // base version that can no longer be fetched diffs against nothing, so the whole
    // draft surfaces.
    let base_id = base.as_ref().map(js_str);
    let (theirs, ours) = match item_type {
        "script" => {
            let base_content = match &base_id {
                Some(hash) => ctx
                    .get(&ctx.ws(&format!("scripts/get/h/{}", enc(hash))))
                    .await
                    .ok()
                    .and_then(|s| s.get("content").and_then(Value::as_str).map(String::from)),
                None => None,
            };
            (
                base_content.unwrap_or_default(),
                value.as_str().unwrap_or_default().to_string(),
            )
        }
        "flow" => {
            let base_value = match &base_id {
                Some(version) => ctx
                    .get(&ctx.ws(&format!("flows/get/v/{}", enc(version))))
                    .await
                    .ok()
                    .and_then(|flow| flow.get("value").cloned()),
                None => None,
            };
            (
                pretty(&base_value.unwrap_or_else(|| json!({}))),
                pretty(value.get("value").unwrap_or(&Value::Null)),
            )
        }
        _ => {
            let source = |app: &Value| {
                let mut out = Map::new();
                for key in ["files", "runnables", "data"] {
                    put(&mut out, key, app.get(key).cloned());
                }
                Value::Object(out)
            };
            let base_app = match &base_id {
                Some(version) => ctx
                    .get(&ctx.ws(&format!("apps/get/v/{}", enc(version))))
                    .await
                    .ok()
                    .map(|app| app_source_to_draft_value(&app, Some(&app))),
                None => None,
            };
            (
                pretty(&source(
                    &base_app.unwrap_or_else(|| json!({ "files": {}, "runnables": {} })),
                )),
                pretty(&source(&value)),
            )
        }
    };
    let your_changes = unified_patch("fork-base", "your-draft", &theirs, &ours, 4);

    let kind = draft_kind(item_type, None).ok_or_else(no_draft)?;
    delete_draft(ctx, &kind, path, draft.created_at.as_deref()).await?;

    let (diff, tools, version) = match item_type {
        "script" => (
            "a diff",
            "edit_script / write_script",
            format!("hash {head_text}"),
        ),
        "flow" => (
            "a JSON diff",
            "the flow edit tools",
            format!("version {head_text}"),
        ),
        _ => (
            "a JSON diff",
            "the app edit tools",
            format!("version {head_text}"),
        ),
    };
    let mut out = Map::new();
    out.insert("success".into(), json!(true));
    out.insert(
        "message".into(),
        json!(format!(
            "Discarded the stale draft for \"{path}\". Your changes are in \"your_changes\" ({diff} against the version you forked from). Re-apply them with {tools} — the new draft will be based on the latest deployed version ({version}) — then deploy."
        )),
    );
    put(&mut out, head_key, head);
    out.insert("your_changes".into(), json!(your_changes));
    Ok(pretty(&Value::Object(out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_and_replace_requires_one_match_unless_replace_all() {
        let replace = |all| find_and_replace("a b a", "a", "x", all, "script source");
        assert_eq!(replace(true).unwrap(), "x b x");
        assert_eq!(
            replace(false).unwrap_err(),
            "old_string matched 2 locations. Make it more specific or set replace_all to true."
        );
        assert_eq!(
            find_and_replace("a", "z", "x", false, "script source").unwrap_err(),
            "old_string was not found in the script source."
        );
    }

    // Expected texts are jsdiff's `createTwoFilesPatch` output for the same inputs.
    #[test]
    fn unified_patch_matches_jsdiff() {
        let header =
            "===================================================================\n--- fork-base\t\n+++ your-draft\t\n";
        let patch = |old, new, context| unified_patch("fork-base", "your-draft", old, new, context);
        assert_eq!(
            patch(
                "a\nb\nc\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\no\np\n",
                "a\nb\nC\nd\ne\nf\ng\nh\ni\nj\nk\nl\nm\nn\nO\np\nq",
                4
            ),
            format!("{header}@@ -1,7 +1,7 @@\n a\n b\n-c\n+C\n d\n e\n f\n g\n@@ -11,6 +11,7 @@\n k\n l\n m\n n\n-o\n+O\n p\n+q\n\\ No newline at end of file\n")
        );
        assert_eq!(
            patch("", "x\ny\n", 4),
            format!("{header}@@ -0,0 +1,2 @@\n+x\n+y\n")
        );
        assert_eq!(
            patch("x\ny", "", 4),
            format!("{header}@@ -1,2 +0,0 @@\n-x\n-y\n\\ No newline at end of file\n")
        );
        assert_eq!(patch("same\n", "same\n", 4), header);
        assert_eq!(
            patch("a\nb\nc\n", "a\nX\nY\nc\n", 1),
            format!("{header}@@ -1,3 +1,4 @@\n a\n-b\n+X\n+Y\n c\n")
        );
        assert_eq!(
            patch("a\nb", "a\nb\n", 4),
            format!("{header}@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+b\n")
        );
    }

    #[test]
    fn compact_flow_replaces_nested_inline_scripts() {
        let raw = |id: &str| json!({ "id": id, "value": { "type": "rawscript", "content": id.to_uppercase(), "language": "bun" } });
        let value = json!({
            "modules": [
                raw("a"),
                { "id": "loop", "value": { "type": "forloopflow", "modules": [raw("b")] } },
                { "id": "br", "value": { "type": "branchone", "branches": [{ "modules": [raw("c")] }], "default": [raw("d")] } },
            ],
            "failure_module": raw("failure"),
            "same_worker": true,
        });
        let (compact, scripts) = compact_flow(&value, None);
        assert_eq!(scripts.len(), 5);
        assert_eq!(scripts["c"], "C");
        assert_eq!(
            compact["modules"][2]["value"]["default"][0]["value"]["content"],
            "inline_script.d"
        );
        assert_eq!(
            compact["failure_module"]["value"]["content"],
            "inline_script.failure"
        );
        let keys: Vec<&String> = compact.as_object().unwrap().keys().collect();
        assert_eq!(
            keys,
            [
                "modules",
                "schema",
                "preprocessor_module",
                "failure_module",
                "groups",
                "notes",
                "same_worker"
            ]
        );
    }

    #[test]
    fn draft_item_drops_a_staged_path_equal_to_the_storage_path() {
        let draft = json!({ "path": "u/a/s", "draft_path": "u/a/s", "summary": "s", "language": "bun", "content": "x", "parent_hash": "h" });
        let item = draft_item("script", "u/a/s", &draft).unwrap();
        assert_eq!(
            item,
            json!({ "type": "script", "path": "u/a/s", "summary": "s", "language": "bun", "value": "x", "parentHash": "h", "isDraft": true })
        );
        assert!(draft_item("app", "u/a/s", &draft).is_none());
    }
}
