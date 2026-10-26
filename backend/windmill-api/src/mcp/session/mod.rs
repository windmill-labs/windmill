//! The Windmill MCP server's toolset (`/api/mcp`, tokens with `mcp:all`): an AI session's toolset, for a client
//! outside the Windmill UI.
//!
//! Everything a model reads here — tool names, descriptions, input schemas, the
//! instructions, the `get_instructions` texts — is generated from the session's own
//! definitions into `gen/` (`npm run gen:mcp-session` in `frontend/`). What a tool
//! *does* is written here, mirroring its session counterpart in
//! `frontend/src/lib/components/copilot/chat/global/core.ts`: a change to one side's
//! behaviour belongs on the other.
//!
//! A tool acts through the API as the caller, the way the session's does, so every
//! permission check is the API's own. It calls the server's router in-process rather
//! than over a socket.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde::Deserialize;
use serde_json::{Map, Value};
use windmill_mcp::server::{
    BackendResult, CallToolResult, ContentBlock, ErrorData, MetaObject, SessionAccess, Tool,
    ToolAnnotations,
};

use crate::db::ApiAuthed;

use super::utils::{mint_caller_jwt, send_as_caller};

/// Each running server's fully layered router, by its `base_internal_url`. Keyed rather
/// than global because the backend tests run one server per test in a single process,
/// each on its own database: a tool must reach the server it was called on.
static IN_PROCESS_ROUTERS: LazyLock<std::sync::RwLock<HashMap<String, axum::Router>>> =
    LazyLock::new(Default::default);

/// Let session tools of the server at `base_internal_url` call its API in-process.
/// Without a registration they fall back to HTTP.
pub(crate) fn register_router(base_internal_url: &str, router: Option<axum::Router>) {
    let mut routers = IN_PROCESS_ROUTERS.write().unwrap();
    match router {
        Some(router) => routers.insert(base_internal_url.to_string(), router),
        None => routers.remove(base_internal_url),
    };
}

mod apps;
mod data;
mod deploy;
mod flows;
mod guidance;
mod instructions;
mod items;
mod misc;
mod runs;
mod settings;
mod workspace;

pub(super) use instructions::{
    generic_session_instructions, session_instructions, workspace_context,
};

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
const DESTRUCTIVE: &[&str] = &[
    "deploy_workspace_item",
    "delete_workspace_item",
    "create_resource_type",
    "set_workspace_settings",
];

/// Tools the session counts as reads that still start a job. A read-only token runs
/// nothing on the tools server, so it runs nothing here either.
const RUNS_A_JOB: &[&str] = &["get_db_schema"];

/// Tools that run a deployed script or flow, the one write a read-and-run token keeps.
const RUNS_DEPLOYED: &[&str] = &["run_script", "run_flow"];

fn allowed(def: &ToolDef, access: SessionAccess) -> bool {
    let read = def.read_only && !RUNS_A_JOB.contains(&def.name.as_str());
    match access {
        SessionAccess::Full => true,
        SessionAccess::ReadAndRun => read || RUNS_DEPLOYED.contains(&def.name.as_str()),
        SessionAccess::ReadOnly => read,
    }
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
groups!(guidance, workspace, items, misc, flows, apps, runs, deploy, data, settings);

/// Claude clients keep an MCP result in context only up to a token budget well under the
/// authoring references `get_instructions` returns, unless the tool raises it. Above the
/// runner's own cap on a result, so that cap is the one that applies.
fn result_size_meta() -> MetaObject {
    let mut meta = Map::new();
    meta.insert("anthropic/maxResultSizeChars".to_string(), 100_000.into());
    MetaObject(meta)
}

pub(super) fn session_tools(access: SessionAccess) -> Vec<Tool> {
    TOOL_DEFS
        .iter()
        .filter(|def| implemented().any(|name| name == def.name))
        .filter(|def| allowed(def, access))
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
    pub access: SessionAccess,
    /// The scheme and host the MCP client reached, when known.
    pub origin: Option<&'a str>,
    /// Minted once per tool call: every JWT the API sees is cached by the auth layer, and a
    /// tool can make dozens of calls, which would evict other users' cached sessions.
    pub jwt: tokio::sync::OnceCell<String>,
}

impl Ctx<'_> {
    /// Call the API as the caller. `path` follows `/api`; a non-2xx answer is the error.
    pub async fn request(&self, method: &str, path: &str, body: Option<Value>) -> ToolResult {
        let url = guarded_url(self.base_internal_url, self.w_id, method, path, self.access)?;
        // No route scope: this server only admits `mcp:all`, several routes (hub, embeddings)
        // map to none, and `guarded_url` is what keeps a call on its route. The tag fence
        // is not a route scope and stays: it limits which jobs the caller runs and reads.
        let tag_scopes: Vec<String> = self
            .auth
            .scopes
            .iter()
            .flatten()
            .filter(|s| s.starts_with("if_jobs:filter_tags:"))
            .cloned()
            .collect();
        let scopes = (!tag_scopes.is_empty()).then_some(tag_scopes);
        let router = IN_PROCESS_ROUTERS
            .read()
            .unwrap()
            .get(self.base_internal_url)
            .cloned();
        let (status, text) = match router {
            Some(router) => self.call_router(router, method, &url, body, scopes).await?,
            None => {
                let response = send_as_caller(method, &url, self.w_id, self.auth, body, scopes)
                    .await
                    .map_err(|e| e.message.to_string())?;
                let status = response.status();
                (status, response.text().await.map_err(|e| e.to_string())?)
            }
        };
        if status.is_success() {
            Ok(text)
        } else {
            Err(format!("{}: {}", status.as_u16(), text))
        }
    }

    /// `request` through the server's own router: the same layers and handlers as over
    /// HTTP, without a socket.
    async fn call_router(
        &self,
        router: axum::Router,
        method: &str,
        url: &str,
        body: Option<Value>,
        scopes: Option<Vec<String>>,
    ) -> Result<(http::StatusCode, String), String> {
        use tower::ServiceExt;
        let parsed = reqwest::Url::parse(url).map_err(|e| e.to_string())?;
        // The router does not see the base URL's own path prefix.
        let at = reqwest::Url::parse(self.base_internal_url)
            .map_or(0, |base| base.path().trim_end_matches('/').len());
        let uri = match parsed.query() {
            Some(q) => format!("{}?{q}", &parsed.path()[at..]),
            None => parsed.path()[at..].to_string(),
        };
        let token = self
            .jwt
            .get_or_try_init(|| mint_caller_jwt(self.w_id, self.auth, scopes))
            .await
            .map_err(|e| e.message.to_string())?;
        let mut request = http::Request::builder()
            .method(method)
            .uri(uri)
            .header(http::header::AUTHORIZATION, format!("Bearer {token}"));
        let body = match body {
            Some(body) => {
                request = request.header(http::header::CONTENT_TYPE, "application/json");
                axum::body::Body::from(serde_json::to_vec(&body).map_err(|e| e.to_string())?)
            }
            None => axum::body::Body::empty(),
        };
        let request = request.body(body).map_err(|e| e.to_string())?;
        // The total timeout of the HTTP client this replaces.
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let response = router.oneshot(request).await.map_err(|e| e.to_string())?;
            let status = response.status();
            let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .map_err(|e| e.to_string())?;
            Ok((status, String::from_utf8_lossy(&bytes).into_owned()))
        })
        .await
        .map_err(|_| format!("Failed to execute request: {method} {url} timed out"))?
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

    /// A Windmill page in this workspace, as a link the user can open: an external client
    /// has no preview panel, so a link is how it shows what a tool made or ran.
    pub fn page_url(&self, page: &str) -> String {
        let (page, hash) = page
            .split_once('#')
            .map_or((page, None), |(p, h)| (p, Some(h)));
        let sep = if page.contains('?') { '&' } else { '?' };
        let hash = hash.map(|h| format!("#{h}")).unwrap_or_default();
        // An instance that never set its base URL holds this placeholder, which no client
        // can open; the host the client reached is the better guess then.
        let base = windmill_common::BASE_URL.load();
        let base = match self.origin {
            Some(origin) if base.as_str() == "http://localhost" => origin,
            _ => base.as_str(),
        };
        format!("{base}{page}{sep}workspace={}{hash}", enc(self.w_id))
    }
}

/// Routes outside the workspace that a tool reads.
const INSTANCE_ROUTES: &[&str] = &[
    "/docs/",
    "/embeddings/",
    "/integrations/hub/",
    "/scripts/hub/",
    "/users/whoami",
    "/workers/list",
    "/workspaces/users",
];

/// The URL of an API call, refused unless it lands on the route its handler wrote.
///
/// The JWT behind a call carries the caller's full rights, and paths embed arguments a
/// model chose: a `..` segment, or an id carrying `#`, would otherwise turn a read of
/// one job into a run of any script. A read-only or read-and-run token is held to its rule
/// here too, since that JWT carries neither.
fn guarded_url(
    base: &str,
    w_id: &str,
    method: &str,
    path: &str,
    access: SessionAccess,
) -> Result<String, String> {
    let refused = || format!("Refusing to call `{path}`: not a route of this tool");
    let route = path.split('?').next().unwrap_or_default();
    let in_workspace = route
        .strip_prefix("/w/")
        .and_then(|rest| rest.strip_prefix(w_id))
        .is_some_and(|rest| rest.starts_with('/'));
    let workspace_is_plain = w_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if path.contains('#')
        || !workspace_is_plain
        || !(in_workspace || INSTANCE_ROUTES.iter().any(|r| route.starts_with(r)))
    {
        return Err(refused());
    }
    let url = format!("{base}/api{path}");
    let parsed = reqwest::Url::parse(&url).map_err(|_| refused())?;
    // Parsing resolves dot segments, encoded or not: a path that changed went elsewhere.
    if !parsed.path().ends_with(&format!("/api{route}")) {
        return Err(refused());
    }
    // Runs of deployed items by path only: the preview routes take code, so they are writes.
    // A run also stores its secret arguments as expiring variables; the write tools that
    // could create other variables are not offered to a read-and-run token.
    let run_writes = method == "POST"
        && ["jobs/run/p/", "jobs/run/f/"]
            .iter()
            .any(|run| route.starts_with(&format!("/w/{w_id}/{run}")))
        || method == "POST" && route == format!("/w/{w_id}/variables/create");
    let held_to_reads = match access {
        SessionAccess::Full => false,
        SessionAccess::ReadAndRun => !run_writes,
        SessionAccess::ReadOnly => true,
    };
    if held_to_reads {
        windmill_api_auth::scopes::check_read_only_for_route(parsed.path(), method)
            .map_err(|e| e.to_string())?;
    }
    Ok(url)
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
) -> BackendResult<CallToolResult> {
    let Some(def) = TOOL_DEFS.iter().find(|def| def.name == name) else {
        return Err(ErrorData::invalid_params(
            format!("Unknown tool '{name}'"),
            None,
        ));
    };
    if !allowed(def, ctx.access) {
        let token = match ctx.access {
            SessionAccess::ReadAndRun => "can only read and run deployed scripts and flows",
            _ => "is read-only",
        };
        return Err(ErrorData::invalid_request(
            format!("Tool '{name}' changes or runs something, and this token {token}"),
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
    fn a_call_stays_on_the_route_its_handler_wrote() {
        use SessionAccess::*;
        let url = |method, path, access| guarded_url("http://h", "ws", method, path, access);
        assert!(url("GET", "/w/ws/jobs_u/get/0191?no_logs=true", Full).is_ok());
        assert!(url("GET", "/docs/search?query=a%2Fb", ReadOnly).is_ok());
        for path in [
            "/w/ws/jobs_u/get/../../jobs/run_wait_result/p/u/a/b",
            "/w/ws/jobs_u/get/%2e%2e/%2E%2E/variables/get_value/u/a/b",
            "/w/ws/jobs_u/get/x#",
            "/w/other/scripts/list",
            "/w/ws",
            "/settings/global/x",
            "/docs/../users/tokens/list",
        ] {
            assert!(url("GET", path, Full).is_err(), "{path} should be refused");
        }
        assert!(guarded_url("http://h", "a/../b", "GET", "/w/a/../b/scripts/list", Full).is_err());
        // A read-only token neither writes nor runs, whatever the tool.
        assert!(url("POST", "/w/ws/drafts/update", ReadOnly).is_err());
        assert!(url("GET", "/w/ws/jobs/run_wait_result/p/u/a/b", ReadOnly).is_err());
        assert!(url("POST", "/w/ws/jobs/run/p/u/a/b", ReadOnly).is_err());
        // A read-and-run token runs deployed items by path, and writes nothing else: a
        // preview takes code.
        assert!(url("POST", "/w/ws/jobs/run/p/u/a/b", ReadAndRun).is_ok());
        assert!(url("POST", "/w/ws/jobs/run/f/u/a/b", ReadAndRun).is_ok());
        assert!(url("POST", "/w/ws/jobs/run/preview", ReadAndRun).is_err());
        assert!(url("POST", "/w/ws/drafts/update", ReadAndRun).is_err());
        assert!(url("POST", "/w/ws/variables/create", ReadAndRun).is_ok());
        assert!(url("POST", "/w/ws/variables/update/u/a/b", ReadAndRun).is_err());
        assert!(url("POST", "/w/ws/variables/create", ReadOnly).is_err());
    }

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
