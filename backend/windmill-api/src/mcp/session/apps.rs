//! Raw app drafts: files, backend runnables and their test runs.

use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::LazyLock;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use super::runs::finish_run;
use super::workspace::{
    api_message, app_source_to_draft_value, deployed_item_exists, find_and_replace, get_deployed,
    get_draft, item_exists, normalize_app_draft, path_enc, persist_draft, Persist,
};
use super::{
    bool_arg, js_head, js_len, matches_glob, opt_str_arg, pretty, str_arg, truthy, Ctx, ToolResult,
};

// `init_app` is dispatched below but stays out of this list until `gen/app_templates.json`
// is emitted by the generator.
pub(super) const TOOLS: &[&str] = &[
    "init_app",
    "read_app_file",
    "search_app",
    "write_app_file",
    "delete_app_file",
    "patch_app_file",
    "write_app_runnable",
    "delete_app_runnable",
    "test_run_app_runnable",
];

pub(super) async fn call(ctx: &Ctx<'_>, name: &str, args: &Value) -> ToolResult {
    match name {
        "init_app" => init_app(ctx, args).await,
        "read_app_file" => read_app_file(ctx, args).await,
        "search_app" => search_app(ctx, args).await,
        "write_app_file" => write_app_file(ctx, args).await,
        "delete_app_file" => delete_app_file(ctx, args).await,
        "patch_app_file" => patch_app_file(ctx, args).await,
        "write_app_runnable" => write_app_runnable(ctx, args).await,
        "delete_app_runnable" => delete_app_runnable(ctx, args).await,
        "test_run_app_runnable" => test_run_app_runnable(ctx, args).await,
        _ => unreachable!("{name} is not a apps tool"),
    }
}

const GENERATED_FILE: &str = "/wmill.d.ts";

const READ_DEFAULT_LINE_LIMIT: i64 = 1500;
const READ_CHAR_BUDGET: usize = 50_000;

const SEARCH_DEFAULT_MAX_MATCHES: i64 = 100;
const SEARCH_MAX_MATCHES_CEILING: i64 = 200;
const SEARCH_MAX_LINE_CHARS: usize = 200;
const SEARCH_TOTAL_CHAR_BUDGET: usize = 12_000;
const SEARCH_CONTEXT_LINES: usize = 2;

const MAX_TOOL_ERROR_LENGTH: usize = 2000;

/// `{templates: {<framework>: {<file path>: content}}, starter_runnables: {<key>: runnable}}`,
/// the frontend's `FRAMEWORK_TEMPLATES` and `STARTER_RUNNABLES`.
static APP_TEMPLATES: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!("gen/app_templates.json"))
        .expect("gen/app_templates.json is valid")
});

// ---------------------------------------------------------------------------
// JS-shaped helpers
// ---------------------------------------------------------------------------

fn entries(value: &Value, key: &str) -> Map<String, Value> {
    value
        .get(key)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// The object under `key`, created (or replacing a non-object) when missing.
fn object_mut<'a>(value: &'a mut Value, key: &str) -> &'a mut Map<String, Value> {
    let slot = &mut value[key];
    if !slot.is_object() {
        *slot = json!({});
    }
    slot.as_object_mut().expect("just made an object")
}

fn text_of<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn is_inline(runnable: &Value) -> bool {
    matches!(text_of(runnable, "type"), Some("inline" | "runnableByName"))
}

fn is_by_path(runnable: &Value) -> bool {
    matches!(text_of(runnable, "type"), Some("path" | "runnableByPath"))
}

// ---------------------------------------------------------------------------
// File targets
// ---------------------------------------------------------------------------

enum Target {
    Frontend(String),
    Backend { file_path: String, key: String, extension: &'static str },
}

impl Target {
    fn file_path(&self) -> &str {
        match self {
            Target::Frontend(file_path) | Target::Backend { file_path, .. } => file_path,
        }
    }
}

fn resolve_target(raw: &str) -> Target {
    let trimmed = raw.trim();
    let backend = trimmed.strip_prefix("backend/").and_then(|rest| {
        let (key, extension) = rest
            .strip_suffix("/main.ts")
            .map(|key| (key, "ts"))
            .or_else(|| rest.strip_suffix("/main.py").map(|key| (key, "py")))?;
        (!key.is_empty() && !key.contains('/')).then_some((key, extension))
    });
    match backend {
        Some((key, extension)) => {
            Target::Backend { file_path: trimmed.to_string(), key: key.to_string(), extension }
        }
        None if trimmed.starts_with('/') => Target::Frontend(trimmed.to_string()),
        None => Target::Frontend(format!("/{trimmed}")),
    }
}

fn assert_not_generated(file_path: &str) -> Result<(), String> {
    if file_path == GENERATED_FILE {
        return Err(format!(
            "\"{file_path}\" is generated automatically from backend runnables and cannot be modified directly."
        ));
    }
    Ok(())
}

fn inline_extension(runnable: &Value) -> &'static str {
    let language = runnable.get("inlineScript").and_then(|s| s.get("language"));
    if language.and_then(Value::as_str) == Some("python3") {
        "py"
    } else {
        "ts"
    }
}

/// The inline script behind a `backend/<key>/main.<ext>` target, and its runnable.
fn inline_runnable_content<'a>(
    value: &'a Value,
    key: &str,
    extension: &str,
    app_path: &str,
) -> Result<(&'a str, &'a Value), String> {
    let runnable = value
        .get("runnables")
        .and_then(|r| r.get(key))
        .filter(|r| truthy(Some(r)))
        .ok_or_else(|| format!("Backend runnable \"{key}\" not found in app \"{app_path}\"."))?;
    if !is_inline(runnable) {
        return Err(format!(
            "Runnable \"{key}\" is not inline. Use read_workspace_item on the referenced {} instead.",
            text_of(runnable, "runType").unwrap_or("item")
        ));
    }
    let expected = inline_extension(runnable);
    if extension != expected {
        return Err(format!(
            "Runnable \"{key}\" language is {expected}. Use backend/{key}/main.{expected}."
        ));
    }
    let content = runnable
        .get("inlineScript")
        .and_then(|s| text_of(s, "content"))
        .unwrap_or_default();
    Ok((content, runnable))
}

// ---------------------------------------------------------------------------
// Loading and saving the draft
// ---------------------------------------------------------------------------

struct Loaded {
    value: Value,
    last_sync: Option<String>,
}

/// The caller's draft of the app, else the deployed app shaped as one.
async fn load_app(ctx: &Ctx<'_>, path: &str) -> Result<Loaded, String> {
    let draft = get_draft(ctx, "raw_app", path).await?;
    let mut last_sync = None;
    if let Some(draft) = draft {
        if draft.value.get("files").is_some() {
            return Ok(Loaded { value: draft.value, last_sync: draft.created_at });
        }
        last_sync = draft.created_at;
    }
    let app = get_deployed(ctx, "app", None, path)
        .await
        .map_err(|e| api_message(&e).to_string())?;
    // Only an explicit false: an app known by its draft alone carries no version, so no
    // `raw_app`, and must not read as low-code.
    if app.get("raw_app") == Some(&Value::Bool(false)) {
        return Err(format!(
            "\"{path}\" is a low-code app. This chat only edits code-based apps — open it in the app editor instead."
        ));
    }
    Ok(Loaded { value: app_source_to_draft_value(&app, Some(&app)), last_sync })
}

/// What the backend requires of a draft from an operator with builder rights: sandboxed,
/// never in viewer mode, and running as nobody but its builder.
async fn conform_builder_policy(ctx: &Ctx<'_>, value: &mut Value) {
    if !ctx.auth.is_operator {
        return;
    }
    let Ok(memberships) = ctx.get("/workspaces/users").await else {
        return;
    };
    let builder = memberships
        .get("workspaces")
        .and_then(Value::as_array)
        .and_then(|list| list.iter().find(|w| text_of(w, "id") == Some(ctx.w_id)))
        .and_then(|w| w.get("operator_settings"))
        .and_then(|s| s.get("builder_apps"))
        == Some(&Value::Bool(true));
    if !builder {
        return;
    }
    let policy = object_mut(value, "policy");
    policy.insert("sandbox".into(), json!(true));
    if policy.get("execution_mode").and_then(Value::as_str) == Some("viewer") {
        policy.insert("execution_mode".into(), json!("publisher"));
    }
    let username = &ctx.auth.username;
    let this_user = if username.contains('@') {
        username.clone()
    } else {
        format!("u/{username}")
    };
    let differs = |key: &str, own: &str| {
        policy
            .get(key)
            .is_some_and(|v| !v.is_null() && v.as_str() != Some(own))
    };
    if differs("on_behalf_of", &this_user) || differs("on_behalf_of_email", &ctx.auth.email) {
        policy.shift_remove("on_behalf_of");
        policy.shift_remove("on_behalf_of_email");
    }
}

/// Save the draft and report: the success JSON, or the conflict / failed-save JSON.
async fn save_app(
    ctx: &Ctx<'_>,
    path: &str,
    mut value: Value,
    last_sync: Option<&str>,
    message: String,
    warning: Option<String>,
) -> ToolResult {
    conform_builder_policy(ctx, &mut value).await;
    let value = normalize_app_draft(&value);
    match persist_draft(ctx, "app", path, None, &value, last_sync, false).await? {
        Persist::Failed(text) => Ok(text),
        Persist::Saved(_) => {
            let mut out = Map::new();
            out.insert("success".into(), json!(true));
            out.insert("message".into(), json!(message));
            out.insert(
                "preview_url".into(),
                json!(ctx.page_url(&format!("/apps_raw/edit/{path}"))),
            );
            if let Some(warning) = warning {
                out.insert("warning".into(), json!(warning));
            }
            Ok(pretty(&Value::Object(out)))
        }
    }
}

fn sha256_hex(text: &str) -> String {
    hex::encode(Sha256::digest(text.as_bytes()))
}

/// Rebuild `policy.triggerables_v2` from the runnables: the key names what a runnable
/// may execute (an inline script by the hash of its code), the entry which of its inputs
/// the caller cannot set.
fn recompute_policy(value: &mut Value) {
    let mut triggerables = Map::new();
    for (id, runnable) in entries(value, "runnables") {
        let target = if is_inline(&runnable) {
            let content = runnable
                .get("inlineScript")
                .and_then(|s| text_of(s, "content"))
                .unwrap_or_default();
            format!("rawscript/{}", sha256_hex(content))
        } else if is_by_path(&runnable) {
            let run_type = match text_of(&runnable, "runType") {
                Some("hubscript") => "script",
                Some(other) => other,
                None => "undefined",
            };
            format!(
                "{run_type}/{}",
                text_of(&runnable, "path").unwrap_or("undefined")
            )
        } else {
            continue;
        };
        let fields = entries(&runnable, "fields");
        let flagged = |flag: &str| -> Vec<&String> {
            fields
                .iter()
                .filter(|(_, field)| truthy(field.get(flag)))
                .map(|(name, _)| name)
                .collect()
        };
        let static_inputs: Map<String, Value> = fields
            .iter()
            .filter(|(_, field)| text_of(field, "type") == Some("static"))
            .filter_map(|(name, field)| Some((name.clone(), field.get("value")?.clone())))
            .collect();
        let mut entry = Map::new();
        entry.insert("static_inputs".into(), Value::Object(static_inputs));
        entry.insert("one_of_inputs".into(), json!({}));
        entry.insert(
            "allow_user_resources".into(),
            json!(flagged("allowUserResources")),
        );
        if let Some(secs) = runnable
            .get("delete_after_secs")
            .filter(|s| s.as_f64().is_some_and(|s| s >= 0.0))
        {
            entry.insert("delete_after_secs".into(), secs.clone());
        }
        let sensitive = flagged("sensitive");
        if !sensitive.is_empty() {
            entry.insert("sensitive_inputs".into(), json!(sensitive));
        }
        if let Some(tag) = runnable
            .get("inlineScript")
            .and_then(|s| s.get("tag"))
            .filter(|tag| truthy(Some(tag)))
        {
            entry.insert("tag".into(), tag.clone());
        }
        // The server honors it on inline runnables only, whose code is deployed with the app.
        if is_inline(&runnable) && runnable.get("viewer_data_access") == Some(&Value::Bool(true)) {
            entry.insert("viewer_data_access".into(), json!(true));
        }
        triggerables.insert(format!("{id}:{target}"), Value::Object(entry));
    }
    let policy = object_mut(value, "policy");
    policy.insert("triggerables_v2".into(), Value::Object(triggerables));
    if !truthy(policy.get("execution_mode")) {
        policy.insert("execution_mode".into(), json!("publisher"));
    }
}

// ---------------------------------------------------------------------------
// init_app
// ---------------------------------------------------------------------------

/// A table reference canonicalized to `<datatable>/<table>` or `<datatable>/<schema>:<table>`.
/// A bare name is a table of the app's own schema.
fn app_table_ref(datatable: &str, schema: Option<&str>, reference: &str) -> String {
    if reference.contains('/') {
        return reference.to_string();
    }
    let separator = if reference.contains(':') { ':' } else { '.' };
    let (schema, table) = match reference.split_once(separator) {
        Some((schema, table)) => (Some(schema), table),
        None => (schema, reference),
    };
    match schema {
        _ if table.is_empty() => datatable.to_string(),
        None | Some("" | "public") => format!("{datatable}/{table}"),
        Some(schema) => format!("{datatable}/{schema}:{table}"),
    }
}

async fn init_app(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let framework = str_arg(args, "framework")?;
    let template = APP_TEMPLATES["templates"]
        .get(framework)
        .ok_or_else(|| format!("Invalid framework \"{framework}\""))?;

    if get_draft(ctx, "raw_app", path).await?.is_some() {
        return Err(format!(
            "A draft for app \"{path}\" already exists. Use write_app_file / write_app_runnable to modify it, or delete the existing draft first."
        ));
    }
    if item_exists(ctx, "app", None, path).await? {
        return Err(format!(
            "An app already exists at \"{path}\". Use read_workspace_item + write_app_file / write_app_runnable to modify it."
        ));
    }

    let starters = &APP_TEMPLATES["starter_runnables"];
    let mut value = Map::new();
    if let Some(summary) = opt_str_arg(args, "summary") {
        value.insert("summary".into(), json!(summary));
    }
    value.insert("files".into(), template.clone());
    value.insert("runnables".into(), starters.clone());
    value.insert("policy".into(), json!({ "sandbox": true }));
    let data = args.get("data").filter(|d| truthy(Some(d)));
    let schema = data.and_then(|d| text_of(d, "schema"));
    let datatable = data
        .and_then(|d| text_of(d, "datatable"))
        .unwrap_or_default();
    if let Some(data) = data {
        let tables: Vec<String> = data
            .get("tables")
            .and_then(Value::as_array)
            .map(|tables| {
                tables
                    .iter()
                    .filter_map(Value::as_str)
                    .map(|reference| app_table_ref(datatable, schema, reference))
                    .collect()
            })
            .unwrap_or_default();
        let mut out = Map::new();
        out.insert("tables".into(), json!(tables));
        out.insert("datatable".into(), json!(datatable));
        if let Some(schema) = schema {
            out.insert("schema".into(), json!(schema));
        }
        value.insert("data".into(), Value::Object(out));
    }
    let mut value = Value::Object(value);
    recompute_policy(&mut value);

    let starter_keys: Vec<&str> = starters
        .as_object()
        .map(|s| s.keys().map(String::as_str).collect())
        .unwrap_or_default();
    let data_note = match schema.filter(|s| !s.is_empty()) {
        Some(schema) => format!(" Its data config names schema \"{schema}\" of data table \"{datatable}\" without creating it: if it does not exist yet, create it before the tables that go in it."),
        None => String::new(),
    };
    let message = format!(
        "Initialized a per-user draft app \"{path}\" from the {framework} template (saved server-side, not a deployed workspace item). The template is a demo tour with starter runnables {}: replace its UI and delete the runnables the app does not need (delete_app_runnable). Use write_app_file / write_app_runnable to evolve it.{data_note}",
        starter_keys.join(", ")
    );
    save_app(ctx, path, value, None, message, None).await
}

// ---------------------------------------------------------------------------
// read_app_file
// ---------------------------------------------------------------------------

fn format_file_read(content: &str, offset: Option<i64>, limit: Option<i64>) -> String {
    let lines: Vec<&str> = content.split('\n').collect();
    let total = lines.len();
    let requested_start = offset.unwrap_or(1);
    let start = (requested_start.saturating_sub(1).max(0) as usize).min(total);
    let line_limit = limit.unwrap_or(READ_DEFAULT_LINE_LIMIT).max(0) as usize;
    let end = start.saturating_add(line_limit).min(total);
    let window = lines[start..end].join("\n");
    let window_chars = js_len(&window);
    let char_truncated = window_chars > READ_CHAR_BUDGET;
    let body = js_head(&window, READ_CHAR_BUDGET);

    if requested_start > total as i64 {
        return format!("offset {requested_start} is past the end of the file ({total} lines).");
    }
    if start == 0 && end == total && !char_truncated {
        return body.to_string();
    }
    let mut label = format!("lines {}-{end} of {total}", start + 1);
    let mut more = String::new();
    if char_truncated {
        label.push_str(&format!(
            ", truncated to the first {READ_CHAR_BUDGET} of {window_chars} chars"
        ));
        more = format!(" Reached the {READ_CHAR_BUDGET}-char limit; re-read with a smaller limit (fewer lines). If a single line exceeds the limit the file is likely minified and not readable this way.");
    } else if end < total {
        more = format!(
            " Call read_app_file again with offset={} to continue.",
            end + 1
        );
    }
    format!("{label}.{more}\n\n{body}")
}

async fn read_app_file(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let target = resolve_target(str_arg(args, "file_path")?);
    let value = load_app(ctx, path).await?.value;
    let content = match &target {
        Target::Frontend(file_path) => value
            .get("files")
            .and_then(|files| text_of(files, file_path))
            .ok_or_else(|| format!("Frontend file \"{file_path}\" not found in app \"{path}\"."))?,
        Target::Backend { key, extension, .. } => {
            inline_runnable_content(&value, key, extension, path)?.0
        }
    };
    let int = |name: &str| args.get(name).and_then(Value::as_i64);
    Ok(format_file_read(content, int("offset"), int("limit")))
}

// ---------------------------------------------------------------------------
// search_app
// ---------------------------------------------------------------------------

/// Frontend files (the generated one aside) and inline runnables, each under the path
/// `read_app_file` takes.
fn searchable_files(value: &Value) -> Vec<(String, String)> {
    let mut files: Vec<(String, String)> = entries(value, "files")
        .into_iter()
        .filter(|(file_path, _)| file_path != GENERATED_FILE)
        .filter_map(|(file_path, content)| Some((file_path, content.as_str()?.to_string())))
        .collect();
    for (key, runnable) in entries(value, "runnables") {
        let Some(content) = runnable
            .get("inlineScript")
            .and_then(|s| text_of(s, "content"))
        else {
            continue;
        };
        files.push((
            format!("backend/{key}/main.{}", inline_extension(&runnable)),
            content.to_string(),
        ));
    }
    files
}

// ponytail: approximates `localeCompare` (punctuation, then digits, then letters, case
// ignored until a tie) rather than full ICU collation; only the order files are listed in
// depends on it.
fn locale_compare(a: &str, b: &str) -> Ordering {
    let key = |text: &str| -> Vec<(u8, char)> {
        text.chars()
            .flat_map(char::to_lowercase)
            .map(|c| {
                let class = if c.is_alphabetic() {
                    2
                } else if c.is_numeric() {
                    1
                } else {
                    0
                };
                (class, c)
            })
            .collect()
    };
    key(a).cmp(&key(b)).then_with(|| b.cmp(a))
}

fn search_files(
    mut files: Vec<(String, String)>,
    query: &str,
    file_glob: Option<&str>,
    max_matches: i64,
) -> String {
    let needle = query.to_lowercase();
    files.sort_by(|a, b| locale_compare(&a.0, &b.0));
    if let Some(glob) = file_glob {
        files.retain(|(file_path, _)| matches_glob(file_path, glob));
    }

    // (file, 1-based line, text) rows: every matching line with its context window.
    let mut rows: Vec<(&str, usize, &str)> = Vec::new();
    let (mut total, mut rendered, mut file_count, mut truncated) = (0usize, 0i64, 0usize, false);
    for (file_path, content) in &files {
        let lines: Vec<&str> = content.split('\n').collect();
        let mut file_had_match = false;
        for (i, line) in lines.iter().enumerate() {
            if !line.to_lowercase().contains(&needle) {
                continue;
            }
            total += 1;
            file_had_match = true;
            if rendered >= max_matches {
                truncated = true;
                continue;
            }
            rendered += 1;
            let lo = i.saturating_sub(SEARCH_CONTEXT_LINES);
            let hi = (i + SEARCH_CONTEXT_LINES).min(lines.len() - 1);
            for j in lo..=hi {
                rows.push((file_path, j + 1, lines[j]));
            }
        }
        if file_had_match {
            file_count += 1;
        }
    }

    if total == 0 {
        return format!(
            "No matches. Try a broader or differently-spelled term{}.",
            if file_glob.is_some() {
                ", or drop the file_glob"
            } else {
                ""
            }
        );
    }

    let plural = |n: usize, one: &str, many: &str| {
        if n == 1 {
            one.to_string()
        } else {
            many.to_string()
        }
    };
    let header = format!(
        "{total} {} in {file_count} {}{}",
        plural(total, "match", "matches"),
        plural(file_count, "file", "files"),
        if truncated {
            format!(" (showing the first {max_matches}; narrow with file_glob or a more specific query)")
        } else {
            String::new()
        }
    );

    let mut spent = js_len(&header);
    let mut out = vec![header];
    let mut current_file = "";
    let mut budget_hit = false;
    let mut seen = HashSet::new();
    for (file_path, line, text) in rows {
        if !seen.insert((file_path, line)) {
            continue;
        }
        let text = if js_len(text) > SEARCH_MAX_LINE_CHARS {
            format!("{}… [line truncated]", js_head(text, SEARCH_MAX_LINE_CHARS))
        } else {
            text.to_string()
        };
        let new_file = file_path != current_file;
        let row = if new_file {
            format!("{file_path}\n  {line}: {text}")
        } else {
            format!("  {line}: {text}")
        };
        if spent + js_len(&row) + 1 > SEARCH_TOTAL_CHAR_BUDGET {
            budget_hit = true;
            break;
        }
        if new_file {
            current_file = file_path;
        }
        spent += js_len(&row) + 1;
        out.push(row);
    }
    if budget_hit {
        out.push(
            "… output truncated at the context budget — narrow with file_glob or a more specific query."
                .to_string(),
        );
    }
    out.join("\n")
}

async fn search_app(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let query = str_arg(args, "query")?;
    if query.is_empty() {
        return Err("search_app requires a non-empty query.".to_string());
    }
    let value = load_app(ctx, path).await?.value;
    let max_matches = args
        .get("max_matches")
        .and_then(Value::as_i64)
        .unwrap_or(SEARCH_DEFAULT_MAX_MATCHES)
        .min(SEARCH_MAX_MATCHES_CEILING);
    let file_glob = opt_str_arg(args, "file_glob").filter(|glob| !glob.is_empty());
    Ok(search_files(
        searchable_files(&value),
        query,
        file_glob,
        max_matches,
    ))
}

// ---------------------------------------------------------------------------
// File writes
// ---------------------------------------------------------------------------

async fn write_app_file(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let content = str_arg(args, "content")?;
    let Target::Frontend(file_path) = resolve_target(str_arg(args, "file_path")?) else {
        return Err("write_app_file only writes frontend files. Use write_app_runnable to set inline backend script content.".to_string());
    };
    assert_not_generated(&file_path)?;

    let Loaded { mut value, last_sync } = load_app(ctx, path).await?;
    object_mut(&mut value, "files").insert(file_path.clone(), json!(content));
    let message = format!("Updated draft app \"{path}\" with frontend file \"{file_path}\".");
    save_app(ctx, path, value, last_sync.as_deref(), message, None).await
}

async fn delete_app_file(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let Target::Frontend(file_path) = resolve_target(str_arg(args, "file_path")?) else {
        return Err("delete_app_file only deletes frontend files. Use delete_app_runnable for backend runnables.".to_string());
    };
    assert_not_generated(&file_path)?;

    let Loaded { mut value, last_sync } = load_app(ctx, path).await?;
    if object_mut(&mut value, "files")
        .shift_remove(&file_path)
        .is_none()
    {
        return Err(format!(
            "Frontend file \"{file_path}\" not found in app \"{path}\"."
        ));
    }
    let message = format!("Removed \"{file_path}\" from draft app \"{path}\".");
    save_app(ctx, path, value, last_sync.as_deref(), message, None).await
}

async fn patch_app_file(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let old = str_arg(args, "old_string")?;
    let new = str_arg(args, "new_string")?;
    let replace_all = bool_arg(args, "replace_all");
    let target = resolve_target(str_arg(args, "file_path")?);
    if let Target::Frontend(file_path) = &target {
        assert_not_generated(file_path)?;
    }

    let Loaded { mut value, last_sync } = load_app(ctx, path).await?;
    let label = format!("{} content", target.file_path());
    match &target {
        Target::Frontend(file_path) => {
            let current = value
                .get("files")
                .and_then(|files| text_of(files, file_path))
                .ok_or_else(|| {
                    format!("Frontend file \"{file_path}\" not found in app \"{path}\".")
                })?;
            let updated = find_and_replace(current, old, new, replace_all, &label)?;
            object_mut(&mut value, "files").insert(file_path.clone(), json!(updated));
        }
        Target::Backend { key, extension, .. } => {
            let (current, runnable) = inline_runnable_content(&value, key, extension, path)?;
            let updated = find_and_replace(current, old, new, replace_all, &label)?;
            let language = runnable
                .get("inlineScript")
                .and_then(|s| s.get("language"))
                .filter(|l| !l.is_null())
                .cloned()
                .unwrap_or_else(|| json!(if *extension == "py" { "python3" } else { "bun" }));
            let mut runnable = runnable.clone();
            runnable["inlineScript"] = json!({ "language": language, "content": updated });
            object_mut(&mut value, "runnables").insert(key.clone(), runnable);
        }
    }
    let message = format!(
        "Patched \"{}\" in draft app \"{path}\".",
        target.file_path()
    );
    save_app(ctx, path, value, last_sync.as_deref(), message, None).await
}

// ---------------------------------------------------------------------------
// Runnable writes
// ---------------------------------------------------------------------------

/// The stored runnable for a `write_app_runnable` input. Converting between kinds drops
/// the other kind's fields, and a path runnable's `schema` survives only while its target
/// is unchanged: it describes the item pointed at. `viewer_data_access` is inline-only.
fn build_persisted_runnable(input: &Value, existing: Option<&Value>) -> Result<Value, String> {
    let name = str_arg(input, "name")?;
    let kind = str_arg(input, "type")?;
    let existing = existing.and_then(Value::as_object);
    let fields = match input.get("staticInputs").and_then(Value::as_object) {
        Some(static_inputs) => Value::Object(
            static_inputs
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        json!({ "type": "static", "value": v, "fieldType": "object" }),
                    )
                })
                .collect(),
        ),
        None => existing
            .and_then(|e| e.get("fields"))
            .filter(|f| !f.is_null())
            .cloned()
            .unwrap_or_else(|| json!({})),
    };
    let mut out = existing.cloned().unwrap_or_default();
    let carried_as_viewer = out.shift_remove("viewer_data_access") == Some(Value::Bool(true));
    for key in ["runType", "path", "inlineScript"] {
        out.shift_remove(key);
    }
    out.insert("name".into(), json!(name));

    if kind == "inline" {
        let inline = input
            .get("inlineScript")
            .filter(|s| truthy(Some(s)))
            .ok_or("inlineScript is required when runnable type is \"inline\".")?;
        out.insert("type".into(), json!("inline"));
        out.insert(
            "inlineScript".into(),
            json!({ "content": inline.get("content"), "language": inline.get("language") }),
        );
        let viewer_data_access = input
            .get("viewerDataAccess")
            .and_then(Value::as_bool)
            .unwrap_or(carried_as_viewer);
        if viewer_data_access {
            out.insert("viewer_data_access".into(), json!(true));
        }
        out.insert("fields".into(), fields);
        return Ok(Value::Object(out));
    }

    let path = text_of(input, "path")
        .filter(|p| !p.is_empty())
        .ok_or("path is required when runnable type is \"script\", \"flow\", or \"hubscript\".")?;
    let same_target = existing.is_some_and(|e| {
        matches!(
            e.get("type").and_then(Value::as_str),
            Some("path" | "runnableByPath")
        ) && e.get("runType").and_then(Value::as_str) == Some(kind)
            && e.get("path").and_then(Value::as_str) == Some(path)
    });
    let schema = existing
        .filter(|_| same_target)
        .and_then(|e| e.get("schema"))
        .filter(|s| !s.is_null())
        .cloned()
        .unwrap_or_else(|| json!({}));
    out.insert("type".into(), json!("path"));
    out.insert("runType".into(), json!(kind));
    out.insert("path".into(), json!(path));
    out.insert("fields".into(), fields);
    out.insert("schema".into(), schema);
    Ok(Value::Object(out))
}

/// Whether the script or flow a path runnable names is deployed. None when it names
/// neither, or the probe failed: the answer only annotates a write.
async fn path_target_deployed(ctx: &Ctx<'_>, runnable: &Value) -> Option<bool> {
    let kind = text_of(runnable, "runType").filter(|k| matches!(*k, "script" | "flow"))?;
    let path = text_of(runnable, "path").filter(|p| !p.is_empty())?;
    deployed_item_exists(ctx, kind, None, path).await.ok()
}

async fn write_app_runnable(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let key = str_arg(args, "key")?;
    let input = args
        .get("runnable")
        .filter(|r| r.is_object())
        .ok_or("Missing required object argument `runnable`")?;

    let Loaded { mut value, last_sync } = load_app(ctx, path).await?;
    let existing = value.get("runnables").and_then(|r| r.get(key));
    let persisted = build_persisted_runnable(input, existing)?;
    let warning = match path_target_deployed(ctx, &persisted).await {
        Some(false) => Some(format!(
            "This runnable points at an item that is NOT deployed (\"{key}\" → {} \"{}\"), so it fails at runtime — a path runnable runs the deployed item, never a draft. Offer to deploy just that item with deploy_workspace_item; the app itself does not need deploying, since the preview runs its draft.",
            text_of(&persisted, "runType").unwrap_or_default(),
            text_of(&persisted, "path").unwrap_or_default(),
        )),
        _ => None,
    };
    object_mut(&mut value, "runnables").insert(key.to_string(), persisted);
    recompute_policy(&mut value);
    let message = format!("Updated draft app \"{path}\" with runnable \"{key}\".");
    save_app(ctx, path, value, last_sync.as_deref(), message, warning).await
}

async fn delete_app_runnable(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let key = str_arg(args, "key")?;
    let Loaded { mut value, last_sync } = load_app(ctx, path).await?;
    if object_mut(&mut value, "runnables")
        .shift_remove(key)
        .is_none()
    {
        return Err(format!(
            "Backend runnable \"{key}\" not found in app \"{path}\"."
        ));
    }
    recompute_policy(&mut value);
    let message = format!("Removed runnable \"{key}\" from draft app \"{path}\".");
    save_app(ctx, path, value, last_sync.as_deref(), message, None).await
}

// ---------------------------------------------------------------------------
// test_run_app_runnable
// ---------------------------------------------------------------------------

/// The `execute_component` body that runs a runnable the way the editor preview does.
fn execute_component_body(app_path: &str, key: &str, runnable: &Value, args: &Value) -> Value {
    let fields = entries(runnable, "fields");
    let mut test_args = args.as_object().cloned().unwrap_or_default();
    let mut static_fields = Map::new();
    let mut sensitive = Vec::new();
    for (name, field) in &fields {
        match text_of(field, "type") {
            Some("static") => {
                if let Some(value) = field.get("value") {
                    static_fields.insert(name.clone(), value.clone());
                }
            }
            // Filled by the server from `$ctx:<prop>`; left out, the argument is missing.
            Some("ctx") => {
                if let Some(prop) = text_of(field, "ctx").filter(|p| !p.is_empty()) {
                    test_args.insert(name.clone(), json!(format!("$ctx:{prop}")));
                }
            }
            Some("user") if truthy(field.get("sensitive")) => sensitive.push(name.clone()),
            _ => {}
        }
    }

    let mut body = Map::new();
    body.insert("component".into(), json!(key));
    body.insert("args".into(), Value::Object(test_args));
    // Sent even when empty: its presence is what puts the server in preview mode, which
    // is what lets inline draft code run at all.
    body.insert(
        "force_viewer_static_fields".into(),
        Value::Object(static_fields),
    );
    // The server encrypts a queued argument only when it is named here.
    if !sensitive.is_empty() {
        body.insert("force_viewer_sensitive_inputs".into(), json!(sensitive));
    }
    if is_inline(runnable) {
        if let Some(inline) = runnable.get("inlineScript").filter(|s| truthy(Some(s))) {
            let id = inline.get("id").filter(|id| !id.is_null());
            let mut raw_code = Map::new();
            raw_code.insert(
                "content".into(),
                if id.is_some() {
                    json!("")
                } else {
                    inline.get("content").cloned().unwrap_or_default()
                },
            );
            raw_code.insert(
                "language".into(),
                inline
                    .get("language")
                    .filter(|l| !l.is_null())
                    .cloned()
                    .unwrap_or_else(|| json!("")),
            );
            raw_code.insert("path".into(), json!(format!("{app_path}/{key}")));
            match id {
                Some(id) => {
                    body.insert("id".into(), id.clone());
                }
                None => {
                    if let Some(lock) = inline.get("lock") {
                        raw_code.insert("lock".into(), lock.clone());
                    }
                }
            }
            if let Some(cache_ttl) = inline.get("cache_ttl") {
                raw_code.insert("cache_ttl".into(), cache_ttl.clone());
            }
            if let Some(tag) = inline.get("tag").filter(|tag| truthy(Some(tag))) {
                raw_code.insert("tag".into(), tag.clone());
            }
            body.insert("raw_code".into(), Value::Object(raw_code));
        }
    } else if is_by_path(runnable) {
        let run_type = match text_of(runnable, "runType") {
            Some("hubscript") | None => "script",
            Some(other) => other,
        };
        body.insert(
            "path".into(),
            json!(format!(
                "{run_type}/{}",
                text_of(runnable, "path").unwrap_or_default()
            )),
        );
    }
    Value::Object(body)
}

async fn test_run_app_runnable(ctx: &Ctx<'_>, args: &Value) -> ToolResult {
    let path = str_arg(args, "path")?;
    let key = str_arg(args, "key")?;
    let value = load_app(ctx, path).await?.value;
    let runnables = entries(&value, "runnables");
    let Some(runnable) = runnables.get(key).filter(|r| truthy(Some(r))) else {
        let known: Vec<&str> = runnables.keys().map(String::as_str).collect();
        return Err(format!(
            "App \"{path}\" has no backend runnable \"{key}\".{}",
            if known.is_empty() {
                String::new()
            } else {
                format!(" Available runnables: {}.", known.join(", "))
            }
        ));
    };

    let body = execute_component_body(
        path,
        key,
        runnable,
        args.get("args").unwrap_or(&Value::Null),
    );
    let id = ctx
        .request(
            "POST",
            &ctx.ws(&format!("apps_u/execute_component/{}", path_enc(path))),
            Some(body),
        )
        .await
        .map_err(|e| {
            let message = api_message(&e);
            let message = if js_len(message) > MAX_TOOL_ERROR_LENGTH {
                format!("{}... (truncated)", js_head(message, MAX_TOOL_ERROR_LENGTH))
            } else {
                message.to_string()
            };
            format!("Failed to execute test run: {message}")
        })?;
    let id = id.trim().trim_matches('"').to_string();

    // A path runnable aimed at a flow queues a flow job, whose steps are worth a look.
    let is_flow = text_of(runnable, "runType") == Some("flow");
    let label = format!("{path} / {key}");
    finish_run(ctx, Ok(id), &label, is_flow, "test", args).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_resolve_to_frontend_files_or_inline_runnables() {
        assert!(matches!(resolve_target(" App.tsx "), Target::Frontend(p) if p == "/App.tsx"));
        assert!(matches!(
            resolve_target("backend/a/main.py"),
            Target::Backend { key, extension: "py", .. } if key == "a"
        ));
        // Not the virtual file of a runnable: a frontend path like any other.
        assert!(matches!(
            resolve_target("backend/a/b/main.ts"),
            Target::Frontend(p) if p == "/backend/a/b/main.ts"
        ));
    }

    #[test]
    fn file_reads_are_whole_or_annotated_windows() {
        assert_eq!(format_file_read("a\nb", None, None), "a\nb");
        assert_eq!(
            format_file_read("a\nb\nc", Some(2), Some(1)),
            "lines 2-2 of 3. Call read_app_file again with offset=3 to continue.\n\nb"
        );
        assert_eq!(
            format_file_read("a\nb\nc", Some(2), None),
            "lines 2-3 of 3.\n\nb\nc"
        );
        assert_eq!(
            format_file_read("a", Some(5), None),
            "offset 5 is past the end of the file (1 lines)."
        );
        let long = "x".repeat(READ_CHAR_BUDGET + 1);
        assert!(format_file_read(&long, None, None).starts_with(
            "lines 1-1 of 1, truncated to the first 50000 of 50001 chars. Reached the 50000-char limit;"
        ));
    }

    #[test]
    fn search_caps_matching_lines_and_dedupes_context() {
        let files = vec![
            (
                "/b.ts".to_string(),
                "one\nFoo()\nfoo()\nfour\nfive\nsix".to_string(),
            ),
            ("/App.tsx".to_string(), "foo".to_string()),
        ];
        assert_eq!(
            search_files(files.clone(), "foo", None, 100),
            "3 matches in 2 files\n/App.tsx\n  1: foo\n/b.ts\n  1: one\n  2: Foo()\n  3: foo()\n  4: four\n  5: five"
        );
        assert_eq!(
            search_files(files.clone(), "foo", Some("*.ts"), 1),
            "2 matches in 1 file (showing the first 1; narrow with file_glob or a more specific query)\n/b.ts\n  1: one\n  2: Foo()\n  3: foo()\n  4: four"
        );
        assert_eq!(
            search_files(files, "nope", Some("*.ts"), 100),
            "No matches. Try a broader or differently-spelled term, or drop the file_glob."
        );
    }

    #[test]
    fn globs_match_the_name_or_the_full_path() {
        assert!(matches_glob("/components/Button.tsx", "*.tsx"));
        assert!(!matches_glob("/components/Button.tsx", "/*.tsx"));
        assert!(matches_glob("/components/Button.tsx", "/components/*"));
        assert!(matches_glob("backend/a/main.ts", "backend/**"));
        assert!(!matches_glob("/a.tsx", "?.ts"));
    }

    #[test]
    fn runnable_kind_changes_drop_the_other_kinds_fields() {
        let existing = json!({
            "name": "old", "type": "path", "runType": "script", "path": "u/a/s",
            "fields": { "x": { "type": "static", "value": 1 } }, "schema": { "k": 1 }
        });
        let same = build_persisted_runnable(
            &json!({ "name": "n", "type": "script", "path": "u/a/s" }),
            Some(&existing),
        )
        .unwrap();
        assert_eq!(same["schema"], json!({ "k": 1 }));
        assert_eq!(same["fields"], existing["fields"]);

        let retargeted = build_persisted_runnable(
            &json!({ "name": "n", "type": "flow", "path": "u/a/s" }),
            Some(&existing),
        )
        .unwrap();
        assert_eq!(retargeted["schema"], json!({}));

        let inline = build_persisted_runnable(
            &json!({
                "name": "n", "type": "inline", "staticInputs": { "y": 2 },
                "inlineScript": { "language": "bun", "content": "c" }
            }),
            Some(&existing),
        )
        .unwrap();
        assert!(inline.get("path").is_none() && inline.get("runType").is_none());
        assert_eq!(
            inline["fields"],
            json!({ "y": { "type": "static", "value": 2, "fieldType": "object" } })
        );
        assert_eq!(
            build_persisted_runnable(&json!({ "name": "n", "type": "inline" }), None),
            Err("inlineScript is required when runnable type is \"inline\".".to_string())
        );
    }

    #[test]
    fn viewer_data_access_is_set_kept_or_cleared_on_inline_runnables_only() {
        let inline = |viewer: Value| {
            let mut input = json!({
                "name": "n", "type": "inline",
                "inlineScript": { "language": "bun", "content": "c" }
            });
            if !viewer.is_null() {
                input["viewerDataAccess"] = viewer;
            }
            input
        };
        let set = build_persisted_runnable(&inline(json!(true)), None).unwrap();
        assert_eq!(set["viewer_data_access"], json!(true));
        let kept = build_persisted_runnable(&inline(Value::Null), Some(&set)).unwrap();
        assert_eq!(kept["viewer_data_access"], json!(true));
        let cleared = build_persisted_runnable(&inline(json!(false)), Some(&set)).unwrap();
        assert!(cleared.get("viewer_data_access").is_none());
        let by_path = build_persisted_runnable(
            &json!({ "name": "n", "type": "script", "path": "u/a/s", "viewerDataAccess": true }),
            Some(&set),
        )
        .unwrap();
        assert!(by_path.get("viewer_data_access").is_none());

        let mut value = json!({ "runnables": { "a": set, "b": {
            "type": "path", "runType": "script", "path": "u/a/s", "viewer_data_access": true
        } } });
        recompute_policy(&mut value);
        let triggerables = &value["policy"]["triggerables_v2"];
        let flagged: Vec<&String> = triggerables
            .as_object()
            .unwrap()
            .iter()
            .filter(|(_, e)| e["viewer_data_access"] == json!(true))
            .map(|(k, _)| k)
            .collect();
        assert_eq!(flagged.len(), 1);
        assert!(flagged[0].starts_with("a:rawscript/"));
    }

    #[test]
    fn policy_keys_inline_runnables_by_the_hash_of_their_code() {
        let mut value = json!({
            "runnables": {
                "a": {
                    "type": "inline", "inlineScript": { "content": "", "language": "bun" },
                    "fields": {
                        "s": { "type": "static", "value": 1 },
                        "u": { "type": "user", "sensitive": true }
                    }
                },
                "b": { "type": "path", "runType": "hubscript", "path": "hub/1/x" }
            },
            "policy": { "on_behalf_of": "u/x", "triggerables_v2": { "stale": {} } }
        });
        recompute_policy(&mut value);
        assert_eq!(
            value["policy"],
            json!({
                "on_behalf_of": "u/x",
                "triggerables_v2": {
                    "a:rawscript/e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855": {
                        "static_inputs": { "s": 1 }, "one_of_inputs": {},
                        "allow_user_resources": [], "sensitive_inputs": ["u"]
                    },
                    "b:script/hub/1/x": {
                        "static_inputs": {}, "one_of_inputs": {}, "allow_user_resources": []
                    }
                },
                "execution_mode": "publisher"
            })
        );
    }

    #[test]
    fn preview_body_fills_static_ctx_and_sensitive_inputs() {
        let runnable = json!({
            "type": "inline",
            "inlineScript": { "content": "code", "language": "bun" },
            "fields": {
                "s": { "type": "static", "value": 1 },
                "who": { "type": "ctx", "ctx": "username" },
                "pw": { "type": "user", "sensitive": true }
            }
        });
        assert_eq!(
            execute_component_body("u/a/app", "k", &runnable, &json!({ "x": 1 })),
            json!({
                "component": "k",
                "args": { "x": 1, "who": "$ctx:username" },
                "force_viewer_static_fields": { "s": 1 },
                "force_viewer_sensitive_inputs": ["pw"],
                "raw_code": { "content": "code", "language": "bun", "path": "u/a/app/k" }
            })
        );
        let by_path = json!({ "type": "path", "runType": "flow", "path": "u/a/f" });
        assert_eq!(
            execute_component_body("u/a/app", "k", &by_path, &Value::Null),
            json!({
                "component": "k", "args": {}, "force_viewer_static_fields": {},
                "path": "flow/u/a/f"
            })
        );
    }

    #[test]
    fn table_refs_are_canonicalized_against_the_apps_schema() {
        assert_eq!(
            app_table_ref("main", Some("shop"), "orders"),
            "main/shop:orders"
        );
        assert_eq!(app_table_ref("main", None, "orders"), "main/orders");
        assert_eq!(
            app_table_ref("main", Some("shop"), "public.orders"),
            "main/orders"
        );
        assert_eq!(
            app_table_ref("main", Some("shop"), "crm:leads"),
            "main/crm:leads"
        );
        assert_eq!(app_table_ref("main", Some("shop"), "other/x"), "other/x");
    }
}
