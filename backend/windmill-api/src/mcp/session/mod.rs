//! The "Windmill workspace" MCP server's tools: an AI session's toolset, for a client
//! outside the Windmill UI.
//!
//! Everything a model reads here — tool names, descriptions, input schemas, the
//! instructions, the `get_instructions` texts — is generated from the session's own
//! definitions into `gen/` (`npm run gen:mcp-session` in `frontend/`). What a tool
//! *does* is written here, mirroring its session counterpart in
//! `frontend/src/lib/components/copilot/chat/global/core.ts`: a change to one side's
//! behaviour belongs on the other.
//!
//! A tool acts through the HTTP API as the caller, the way the session's does, so
//! every permission check is the API's own.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;
use serde_json::{Map, Value};
use windmill_mcp::server::{
    BackendResult, CallToolResult, ContentBlock, ErrorData, MetaObject, Tool, ToolAnnotations,
};

use crate::db::ApiAuthed;

use super::utils::send_as_caller;

mod apps;
mod data;
mod deploy;
mod flows;
mod guidance;
mod instructions;
mod items;
mod misc;
mod runs;
mod workspace;

pub(super) use instructions::session_instructions;

#[derive(Deserialize)]
struct ToolDef {
    name: String,
    description: String,
    parameters: Map<String, Value>,
    read_only: bool,
}

static TOOL_DEFS: LazyLock<Vec<ToolDef>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("gen/tools.json")).expect("gen/tools.json is valid")
});

/// A tool's input schema, as generated from the session's zod schema.
pub(super) fn tool_parameters(name: &str) -> Option<&'static Map<String, Value>> {
    TOOL_DEFS
        .iter()
        .find(|def| def.name == name)
        .map(|def| &def.parameters)
}

/// The JSON Schema text of each trigger kind's config, and of a schedule's `advanced` bag,
/// as the session renders them from its request schemas.
#[derive(Deserialize)]
pub(super) struct Schemas {
    pub trigger: HashMap<String, String>,
    pub schedule: String,
}

pub(super) static SCHEMAS: LazyLock<Schemas> = LazyLock::new(|| {
    serde_json::from_str(include_str!("gen/schemas.json")).expect("gen/schemas.json is valid")
});

/// Session tools that need the Windmill UI and have no meaning for an external client.
/// Every generated tool is either here or implemented by a group below.
#[cfg(test)]
const EDITOR_ONLY: &[&str] = &[
    // The side panel, the question card and the browser-held user instructions.
    "open_page",
    "askUserQuestion",
    "update_user_instructions",
    // Files attached to the chat.
    "search_files",
    "read_file",
];

/// Tools whose effect reaches the deployed workspace and cannot be undone by discarding
/// a draft. Stands in for the session's confirmation card.
const DESTRUCTIVE: &[&str] = &["deploy_workspace_item", "delete_workspace_item"];

/// Tools the session counts as reads that still start a job. A read-only token runs
/// nothing on the tools server, so it runs nothing here either.
const RUNS_A_JOB: &[&str] = &["get_db_schema"];

fn allowed_read_only(def: &ToolDef) -> bool {
    def.read_only && !RUNS_A_JOB.contains(&def.name.as_str())
}

/// Each group's tools and its dispatcher, tried in order.
macro_rules! groups {
    ($($group:ident),*) => {
        fn implemented() -> impl Iterator<Item = &'static str> {
            std::iter::empty()$(.chain($group::TOOLS.iter().copied()))*
        }

        async fn dispatch(ctx: &Ctx<'_>, name: &str, args: &Value) -> Option<ToolResult> {
            $(if $group::TOOLS.contains(&name) {
                return Some($group::call(ctx, name, args).await);
            })*
            None
        }
    };
}
groups!(guidance, workspace, items, misc, flows, apps, runs, deploy, data);

/// Claude clients keep an MCP result in context only up to a token budget well under the
/// authoring references `get_instructions` returns, unless the tool raises it. Above the
/// runner's own cap on a result, so that cap is the one that applies.
fn result_size_meta() -> MetaObject {
    let mut meta = Map::new();
    meta.insert("anthropic/maxResultSizeChars".to_string(), 100_000.into());
    MetaObject(meta)
}

pub(super) fn session_tools(read_only: bool) -> Vec<Tool> {
    TOOL_DEFS
        .iter()
        .filter(|def| implemented().any(|name| name == def.name))
        .filter(|def| !read_only || allowed_read_only(def))
        .map(|def| {
            Tool::new(
                def.name.clone(),
                def.description.clone(),
                def.parameters.clone(),
            )
            .with_annotations(
                ToolAnnotations::new()
                    .read_only(def.read_only)
                    .destructive(DESTRUCTIVE.contains(&def.name.as_str())),
            )
            .with_meta(result_size_meta())
        })
        .collect()
}

/// What a tool hands back: the text the model reads, or the message of a failed call.
pub(super) type ToolResult = Result<String, String>;

/// The caller, and the API reached as them.
pub(super) struct Ctx<'a> {
    pub auth: &'a ApiAuthed,
    pub w_id: &'a str,
    pub base_internal_url: &'a str,
}

impl Ctx<'_> {
    /// Call the API as the caller. `path` follows `/api`; a non-2xx answer is the error.
    pub async fn request(&self, method: &str, path: &str, body: Option<Value>) -> ToolResult {
        let url = format!("{}/api{}", self.base_internal_url, path);
        // Unscoped: this server only admits `mcp:all`, the routes are chosen by its own
        // handlers, and several of them (hub, embeddings) map to no route scope at all.
        let response = send_as_caller(method, &url, self.w_id, self.auth, body, None)
            .await
            .map_err(|e| e.message.to_string())?;
        let status = response.status();
        let text = response.text().await.map_err(|e| e.to_string())?;
        if status.is_success() {
            Ok(text)
        } else {
            Err(format!("{}: {}", status.as_u16(), text))
        }
    }

    pub async fn get(&self, path: &str) -> Result<Value, String> {
        let text = self.request("GET", path, None).await?;
        serde_json::from_str(&text).map_err(|e| format!("unexpected response from {path}: {e}"))
    }

    /// `POST` a JSON body. Several endpoints answer with plain text (a path, a job id):
    /// that comes back as a JSON string.
    pub async fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        let text = self.request("POST", path, Some(body)).await?;
        Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }

    pub async fn delete(&self, path: &str) -> Result<Value, String> {
        let text = self.request("DELETE", path, None).await?;
        Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }

    /// A workspace-scoped path: `ws("scripts/list")` is `/w/<workspace>/scripts/list`.
    pub fn ws(&self, rest: &str) -> String {
        format!("/w/{}/{}", self.w_id, rest)
    }
}

/// A string argument, or the error the model gets for leaving it out.
pub(super) fn str_arg<'a>(args: &'a Value, name: &str) -> Result<&'a str, String> {
    args.get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing required string argument `{name}`"))
}

pub(super) fn opt_str_arg<'a>(args: &'a Value, name: &str) -> Option<&'a str> {
    args.get(name).and_then(Value::as_str)
}

/// `JSON.stringify(value, null, 2)`, the form the session's tools return objects in.
pub(super) fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_default()
}

/// Percent-encode one query-string value.
pub(super) fn enc(value: &str) -> String {
    urlencoding::encode(value).into_owned()
}

pub(super) fn bool_arg(args: &Value, name: &str) -> bool {
    args.get(name).and_then(Value::as_bool).unwrap_or(false)
}

/// Whether a `Ctx` error is the API answering 404.
pub(super) fn is_not_found(error: &str) -> bool {
    error.starts_with("404: ")
}

/// JavaScript truthiness, for the fields the session keeps only `if (value)`.
pub(super) fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|n| n != 0.0),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// `value ?? undefined`: the value unless it is missing or null.
pub(super) fn non_null(value: Option<&Value>) -> Option<&Value> {
    value.filter(|v| !v.is_null())
}

/// Set `key` unless the value is JS `undefined`, which `JSON.stringify` drops.
pub(super) fn put(map: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    if let Some(value) = value {
        map.insert(key.to_string(), value);
    }
}

/// The elements of an array; none for anything else.
pub(super) fn array(value: Option<&Value>) -> &[Value] {
    value
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

/// A value as a JS template literal prints it.
pub(super) fn js_str(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// `String.length`: UTF-16 code units.
pub(super) fn js_len(text: &str) -> usize {
    text.encode_utf16().count()
}

/// `text.slice(0, max)` in UTF-16 units, never splitting a character.
pub(super) fn js_head(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut units = 0;
    for (i, c) in text.char_indices() {
        units += c.len_utf16();
        if units > max {
            return &text[..i];
        }
    }
    text
}

/// `text.slice(-max)` in UTF-16 units, never splitting a character.
pub(super) fn js_tail(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut units = 0;
    for (i, c) in text.char_indices().rev() {
        units += c.len_utf16();
        if units > max {
            return &text[i + c.len_utf8()..];
        }
    }
    text
}

/// `*` any characters but `/`, `**` any characters, `?` one character but `/`. A pattern
/// without `/` is matched against the file name alone, so `*.tsx` finds nested files.
pub(super) fn matches_glob(subject: &str, glob: &str) -> bool {
    let subject = if glob.contains('/') {
        subject
    } else {
        subject.rsplit('/').next().unwrap_or(subject)
    };
    let mut pattern = String::from("^");
    let mut chars = glob.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                pattern.push_str(".*");
            }
            '*' => pattern.push_str("[^/]*"),
            '?' => pattern.push_str("[^/]"),
            c => pattern.push_str(&regex::escape(c.encode_utf8(&mut [0; 4]))),
        }
    }
    pattern.push('$');
    regex::Regex::new(&pattern).is_ok_and(|re| re.is_match(subject))
}

pub(super) async fn call_session_tool(
    ctx: &Ctx<'_>,
    name: &str,
    args: Value,
    read_only: bool,
) -> BackendResult<CallToolResult> {
    let Some(def) = TOOL_DEFS.iter().find(|def| def.name == name) else {
        return Err(ErrorData::invalid_params(
            format!("Unknown tool '{name}'"),
            None,
        ));
    };
    if read_only && !allowed_read_only(def) {
        return Err(ErrorData::invalid_request(
            format!("Tool '{name}' changes or runs something, and this token is read-only"),
            None,
        ));
    }
    let Some(result) = dispatch(ctx, name, &args).await else {
        return Err(ErrorData::invalid_params(
            format!("Tool '{name}' is not available on this server"),
            None,
        ));
    };
    windmill_common::feature_usage::log_feature_usage("mcp_workspace", "tool_call", name);
    Ok(match result {
        Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
        // Without the status code `Ctx` prefixes an API failure with, as the session reads it.
        Err(message) => {
            let body = workspace::api_message(&message);
            let text = if body.is_empty() { &message } else { body };
            CallToolResult::error(vec![ContentBlock::text(text.to_string())])
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_session_tool_is_implemented_or_editor_only() {
        let implemented: Vec<&str> = implemented().collect();
        for def in TOOL_DEFS.iter() {
            let name = def.name.as_str();
            assert!(
                implemented.contains(&name) != EDITOR_ONLY.contains(&name),
                "session tool `{name}` must be implemented here or listed in EDITOR_ONLY, not both"
            );
        }
        for name in implemented.iter().chain(EDITOR_ONLY) {
            assert!(
                TOOL_DEFS.iter().any(|def| def.name == *name),
                "`{name}` is not a session tool any more"
            );
        }
    }
}
