//! Utility functions for MCP server
//!
//! Contains database query functions and HTTP request helpers
//! used by the MCP server implementation.

use std::collections::HashMap;

use axum::body::{to_bytes, Body};
use axum::response::Response;
use serde_json::Value;
use sql_builder::prelude::*;
use windmill_common::auth::create_jwt_token;
use windmill_common::db::{Authed, UserDB};
use windmill_common::error::Error;
use windmill_common::scripts::{get_full_hub_script_by_path, Schema};
use windmill_common::triggers::{RunnableFormat, RunnableFormatVersion, TriggerKind};
use windmill_common::utils::{query_elems_from_hub, StripPath};
use windmill_common::worker::to_raw_value;
use windmill_common::{DB, HUB_BASE_URL};
use windmill_mcp::server::{
    non_empty_body_fields, BackendResult, EndpointTool, ErrorData, McpRequest, PathFilter,
};
use windmill_mcp::{HubResponse, HubScriptInfo, ItemSchema, ResourceInfo, ResourceType};
use windmill_trigger::trigger_helpers::{get_runnable_format, RunnableId};

use crate::args::build_headers;
use crate::db::ApiAuthed;
use crate::HTTP_CLIENT;

// items max limit
const ITEMS_FETCH_MAX_LIMIT: usize = 100;

/// Escape LIKE wildcards so a literal path is matched as a prefix, not a pattern.
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

/// Build the SQL condition matching any MCP scope pattern, mirroring
/// `is_resource_allowed`. Returns `None` when no filter should be applied (a `*`
/// pattern grants everything); `Some("false")` when the list is empty (grants
/// nothing); otherwise an OR of per-pattern `o.path` conditions.
fn scope_patterns_condition(patterns: &[String]) -> Option<String> {
    if patterns.iter().any(|p| p == "*") {
        return None;
    }
    if patterns.is_empty() {
        return Some("false".to_string());
    }
    let conds: Vec<String> = patterns
        .iter()
        .map(|p| {
            if let Some(prefix) = p.strip_suffix("/*") {
                // A subtree pattern matches the folder itself or anything under it.
                let subtree = format!("{}/%", escape_like(prefix));
                format!(
                    "({} OR {})",
                    "o.path = ?".bind(&prefix),
                    "o.path LIKE ? ESCAPE '\\'".bind(&subtree),
                )
            } else {
                "o.path = ?".bind(p)
            }
        })
        .collect();
    Some(format!("({})", conds.join(" OR ")))
}

// ============================================================================
// Database utilities
// ============================================================================

/// Get the schema for a specific item (script or flow)
pub async fn get_item_schema(
    path: &str,
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
    item_type: &str,
) -> Result<Option<Schema>, ErrorData> {
    let mut sqlb = SqlBuilder::select_from(&format!("{} as o", item_type));
    sqlb.fields(&["o.schema"]);
    sqlb.and_where("o.path = ?".bind(&path));
    sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
    sqlb.and_where("o.archived = false");
    let sql = sqlb.sql().map_err(|e| {
        tracing::error!("failed to build sql: {}", e);
        ErrorData::internal_error(format!("failed to build sql: {}", e), None)
    })?;
    let mut tx = user_db.clone().begin(authed).await.map_err(|e| {
        tracing::error!("failed to begin transaction: {}", e);
        ErrorData::internal_error(format!("failed to begin transaction: {}", e), None)
    })?;
    let item = sqlx::query_as::<_, ItemSchema>(&sql)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch item schema: {}", e);
            ErrorData::internal_error(format!("failed to fetch item schema: {}", e), None)
        })?;
    tx.commit().await.map_err(|e| {
        tracing::error!("failed to commit transaction: {}", e);
        ErrorData::internal_error(format!("failed to commit transaction: {}", e), None)
    })?;
    Ok(item.schema)
}

/// Get all resource types from the database
pub async fn get_resources_types(
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
) -> Result<Vec<ResourceType>, ErrorData> {
    let mut sqlb = SqlBuilder::select_from("resource_type as o");
    sqlb.fields(&["o.name", "o.description"]);
    sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
    let sql = sqlb.sql().map_err(|e| {
        tracing::error!("failed to build sql: {}", e);
        ErrorData::internal_error(format!("failed to build sql: {}", e), None)
    })?;
    let mut tx = user_db.clone().begin(authed).await.map_err(|e| {
        tracing::error!("failed to begin transaction: {}", e);
        ErrorData::internal_error(format!("failed to begin transaction: {}", e), None)
    })?;
    let rows = sqlx::query_as::<_, ResourceType>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch resource types: {}", e);
            ErrorData::internal_error(format!("failed to fetch resource types: {}", e), None)
        })?;
    tx.commit().await.map_err(|e| {
        tracing::error!("failed to commit transaction: {}", e);
        ErrorData::internal_error(format!("failed to commit transaction: {}", e), None)
    })?;
    Ok(rows)
}

/// Get resources by type from the database
pub async fn get_resources(
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
    resource_type: &str,
) -> Result<Vec<ResourceInfo>, ErrorData> {
    let mut sqlb = SqlBuilder::select_from("resource as o");
    sqlb.fields(&["o.path", "o.description", "o.resource_type"]);
    sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
    sqlb.and_where("o.resource_type = ?".bind(&resource_type));
    let sql = sqlb.sql().map_err(|e| {
        tracing::error!("failed to build sql: {}", e);
        ErrorData::internal_error(format!("failed to build sql: {}", e), None)
    })?;
    let mut tx = user_db.clone().begin(authed).await.map_err(|e| {
        tracing::error!("failed to begin transaction: {}", e);
        ErrorData::internal_error(format!("failed to begin transaction: {}", e), None)
    })?;
    let rows = sqlx::query_as::<_, ResourceInfo>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch resources: {}", e);
            ErrorData::internal_error(format!("failed to fetch resources: {}", e), None)
        })?;
    tx.commit().await.map_err(|e| {
        tracing::error!("failed to commit transaction: {}", e);
        ErrorData::internal_error(format!("failed to commit transaction: {}", e), None)
    })?;

    Ok(rows)
}

/// Generic function to get items (scripts or flows) from the database
pub async fn get_items<T: for<'a> sqlx::FromRow<'a, sqlx::postgres::PgRow> + Send + Unpin>(
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
    scope_type: &str,
    item_type: &str,
    path_filter: Option<PathFilter<'_>>,
) -> Result<Vec<T>, ErrorData> {
    let mut sqlb = SqlBuilder::select_from(&format!("{} as o", item_type));
    let fields = vec!["o.path", "o.summary", "o.description", "o.schema"];
    sqlb.fields(&fields);
    if scope_type == "favorites" {
        sqlb.join("favorite")
            .on("favorite.favorite_kind = ? AND favorite.workspace_id = o.workspace_id AND favorite.path = o.path AND favorite.usr = ?".bind(&item_type)
                .bind(&authed.username));
    }
    sqlb.and_where("o.workspace_id = ?".bind(&workspace_id))
        .and_where("o.archived = false");

    if item_type == "script" {
        // only exclude library scripts (no main function); pipeline, test, WAC,
        // and any future `auto_kind` values remain callable. Mirrors the scripts
        // list API deny-list.
        sqlb.and_where("(o.auto_kind IS NULL OR o.auto_kind <> 'lib')");
    }

    match path_filter {
        None => {}
        Some(PathFilter::Prefix(prefix)) => {
            let escaped = format!("{}%", escape_like(prefix));
            sqlb.and_where("o.path LIKE ? ESCAPE '\\'".bind(&escaped));
        }
        Some(PathFilter::Patterns(patterns)) => {
            if let Some(cond) = scope_patterns_condition(patterns) {
                sqlb.and_where(cond);
            }
        }
    }

    sqlb.order_by(
        if item_type == "flow" {
            "o.edited_at"
        } else {
            "o.created_at"
        },
        false,
    )
    .limit(ITEMS_FETCH_MAX_LIMIT);
    let sql = sqlb.sql().map_err(|e| {
        tracing::error!("failed to build sql: {}", e);
        ErrorData::internal_error(format!("failed to build sql: {}", e), None)
    })?;
    let mut tx = user_db.clone().begin(authed).await.map_err(|e| {
        tracing::error!("failed to begin transaction: {}", e);
        ErrorData::internal_error(format!("failed to begin transaction: {}", e), None)
    })?;
    let rows = sqlx::query_as::<_, T>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| {
            tracing::error!("failed to fetch {}: {}", item_type, e);
            ErrorData::internal_error(format!("failed to fetch {}: {}", item_type, e), None)
        })?;
    tx.commit().await.map_err(|e| {
        tracing::error!("failed to commit transaction: {}", e);
        ErrorData::internal_error(format!("failed to commit transaction: {}", e), None)
    })?;
    Ok(rows)
}

/// Get scripts from the Hub
pub async fn get_scripts_from_hub(
    db: &DB,
    scope_integrations: Option<&str>,
) -> Result<Vec<HubScriptInfo>, ErrorData> {
    let query_params = Some(vec![
        ("limit", ITEMS_FETCH_MAX_LIMIT.to_string()),
        ("with_schema", "true".to_string()),
        ("apps", scope_integrations.unwrap_or("").to_string()),
    ]);
    let url = format!("{}/scripts/top", **HUB_BASE_URL.load());
    let (_status_code, _headers, response) =
        query_elems_from_hub(&HTTP_CLIENT, &url, query_params, &db)
            .await
            .map_err(|e| {
                tracing::error!("Failed to get items from hub: {}", e);
                ErrorData::internal_error(format!("Failed to get items from hub: {}", e), None)
            })?;

    use axum::body::to_bytes;
    let body_bytes = to_bytes(response, usize::MAX).await.map_err(|e| {
        tracing::error!("Failed to read response body: {}", e);
        ErrorData::internal_error(format!("Failed to read response body: {}", e), None)
    })?;
    let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|e| {
        tracing::error!("Failed to decode response body: {}", e);
        ErrorData::internal_error(format!("Failed to decode response body: {}", e), None)
    })?;
    let hub_response: HubResponse = serde_json::from_str(&body_str).map_err(|e| {
        tracing::error!("Failed to parse hub response: {}", e);
        ErrorData::internal_error(format!("Failed to parse hub response: {}", e), None)
    })?;

    Ok(hub_response.asks)
}

/// Get the schema for a Hub script
pub async fn get_hub_script_schema(path: &str, db: &DB) -> Result<Option<Schema>, ErrorData> {
    let strip_path = StripPath(path.to_string());
    let res = get_full_hub_script_by_path(strip_path, &HTTP_CLIENT, Some(db))
        .await
        .map_err(|e| {
            tracing::error!("Failed to get hub script: {}", e);
            ErrorData::internal_error(format!("Failed to get hub script: {}", e), None)
        })?;
    match serde_json::from_str::<Schema>(res.schema.get()) {
        Ok(schema) => Ok(Some(schema)),
        Err(e) => {
            tracing::warn!("Failed to convert schema: {}", e);
            Ok(None)
        }
    }
}

// ============================================================================
// HTTP request utilities for endpoint tools
// ============================================================================

/// Look up the original field name from a field_renames map.
/// field_renames maps renamed_key -> original_key (e.g. {"path__body": "path"}).
fn get_original_name(renamed_key: &str, field_renames: &Option<Value>) -> String {
    field_renames
        .as_ref()
        .and_then(|v| v.as_object())
        .and_then(|m| m.get(renamed_key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| renamed_key.to_string())
}

/// Reject path parameter values that could alter the URL structure of the
/// internal backend request (path traversal, query/fragment injection,
/// percent-encoded and backslash bypasses).
///
/// MCP endpoint tools build internal API URLs by string-substituting these
/// values into a fixed path template. A value containing `..` segments would
/// let a narrowly-scoped tool reach unrelated same-method endpoints once the
/// HTTP client normalizes the URL (e.g. `scripts/get/p/../../../resources/...`
/// collapses to `resources/...`).
///
/// Only structural escapes are rejected — not every character the backend
/// happens not to use. Windmill paths legitimately contain spaces (app paths)
/// and `@` (email-style usernames, e.g. `u/admin@windmill.dev/...`); those are
/// ordinary path-segment data in an absolute URL and cannot redirect the
/// request, so rejecting them would regress valid MCP calls.
fn validate_path_param_value(param_name: &str, value: &str) -> BackendResult<()> {
    let reject = |reason: &str| {
        tracing::warn!(
            "Rejected MCP endpoint path parameter '{}': {}",
            param_name,
            reason
        );
        Err(ErrorData::invalid_params(
            format!("Invalid path parameter '{}': {}", param_name, reason),
            None,
        ))
    };

    if value.is_empty() {
        return reject("must not be empty");
    }

    // Structurally dangerous characters only:
    // - control chars (incl. tab/CR/LF): the WHATWG URL parser strips these,
    //   so `.<TAB>.` could be reassembled into `..`
    // - `\`: WHATWG converts it to `/` for http(s), enabling `..\..\` traversal
    // - `%`: would let `%2e%2e%2f` decode to `../` server-side
    // - `?` / `#`: query/fragment delimiters that truncate or redirect the path
    // A literal space is *not* rejected: the URL crate percent-encodes it
    // (`%20`) so it cannot alter routing, and app paths legitimately use it.
    if let Some(bad) = value
        .chars()
        .find(|c| c.is_control() || matches!(*c, '\\' | '%' | '?' | '#'))
    {
        return reject(&format!("contains disallowed character {:?}", bad));
    }

    // No leading/trailing slash, no empty/dot/dot-dot segments. Splitting on
    // `/` keeps legitimate Windmill paths (`u/alice/db`, `f/folder/name`)
    // valid while catching `..`, `.`, `//`, leading and trailing `/`.
    for segment in value.split('/') {
        match segment {
            "" => return reject("contains an empty path segment or leading/trailing slash"),
            "." | ".." => return reject("contains a '.' or '..' path segment"),
            _ => {}
        }
    }

    Ok(())
}

/// Substitute path parameters in the URL template
pub fn substitute_path_params(
    path: &str,
    workspace_id: &str,
    args_map: &serde_json::Map<String, Value>,
    path_schema: &Option<Value>,
) -> BackendResult<String> {
    let mut path_template = path.replace("{workspace}", workspace_id);

    if let Some(schema) = path_schema {
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (param_name, _) in props {
                let placeholder = format!("{{{}}}", param_name);
                match args_map.get(param_name) {
                    Some(param_value) => {
                        if let Some(str_val) = param_value.as_str() {
                            validate_path_param_value(param_name, str_val)?;
                            path_template = path_template.replace(&placeholder, str_val);
                        }
                    }
                    None => {
                        tracing::warn!("Missing required path parameter: {}", param_name);
                        return Err(ErrorData::invalid_params(
                            format!("Missing required path parameter: {}", param_name),
                            None,
                        ));
                    }
                }
            }
        }
    }

    Ok(path_template)
}

/// Build query string from arguments
pub fn build_query_string(
    args_map: &serde_json::Map<String, Value>,
    query_schema: &Option<Value>,
    query_field_renames: &Option<Value>,
) -> String {
    let Some(schema) = query_schema else {
        return String::new();
    };
    let Some(props) = schema.get("properties").and_then(|p| p.as_object()) else {
        return String::new();
    };

    let query_params: Vec<String> = props
        .keys()
        .filter_map(|param_name| {
            args_map
                .get(param_name)
                .filter(|v| !v.is_null())
                .map(|value| {
                    // Use the original name for the query parameter key
                    let original_name = get_original_name(param_name, query_field_renames);
                    // For string values, use the raw content: to_string() would JSON-encode
                    // it, and stripping the outer quotes leaves inner quotes backslash-escaped
                    // (e.g. `{\"k\":\"v\"}`), which breaks downstream JSON parsing of params
                    // like `args`/`result`. Non-string values keep their JSON serialization.
                    let str_val = value
                        .as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| value.to_string());
                    format!(
                        "{}={}",
                        urlencoding::encode(&original_name),
                        urlencoding::encode(&str_val)
                    )
                })
        })
        .collect();

    if query_params.is_empty() {
        String::new()
    } else {
        format!("?{}", query_params.join("&"))
    }
}

/// Build request body from arguments, refusing a call that would reach an
/// endpoint requiring a body without one.
pub fn build_request_body(
    tool: &EndpointTool,
    args_map: &serde_json::Map<String, Value>,
) -> BackendResult<Option<Value>> {
    let body = assemble_request_body(tool, args_map);

    // The fields that would have satisfied it are not expressible in the tool's input
    // schema, so name them here rather than dispatching a call that cannot succeed.
    if body.is_none() {
        if let Some(fields) = non_empty_body_fields(tool) {
            return Err(ErrorData::invalid_params(
                format!(
                    "{} needs a request body: provide at least one of {}",
                    tool.name,
                    fields.join(", ")
                ),
                None,
            ));
        }
    }

    Ok(body)
}

fn assemble_request_body(
    tool: &EndpointTool,
    args_map: &serde_json::Map<String, Value>,
) -> Option<Value> {
    let EndpointTool {
        method,
        body_schema,
        body_field_renames,
        path_params_schema,
        query_params_schema,
        ..
    } = tool;
    if method == "GET" {
        return None;
    }

    let schema = body_schema.as_ref()?;

    let has_declared_props = schema
        .get("properties")
        .and_then(|p| p.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false);

    // Pass-through body: the schema declares no explicit properties (e.g.
    // runScriptByPath / runFlowByPath, whose body is `additionalProperties: true`
    // and carries the script/flow arguments verbatim). Forward every argument
    // that isn't already consumed by a path or query parameter — without this the
    // request body would be empty and parameterized runs would lose their args.
    if !has_declared_props {
        if schema.get("type").and_then(|t| t.as_str()) != Some("object") {
            return None;
        }
        let consumed: std::collections::HashSet<&str> = [path_params_schema, query_params_schema]
            .into_iter()
            .filter_map(|s| s.as_ref())
            .filter_map(|s| s.get("properties").and_then(|p| p.as_object()))
            .flat_map(|props| props.keys().map(|k| k.as_str()))
            .collect();

        let body_map: serde_json::Map<String, Value> = args_map
            .iter()
            .filter(|(k, v)| !consumed.contains(k.as_str()) && !v.is_null())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        return if body_map.is_empty() {
            None
        } else {
            Some(Value::Object(body_map))
        };
    }

    let props = schema.get("properties")?.as_object()?;

    // A null argument is how a client that must fill in every declared argument says
    // "no value". Forwarding it would reach a field the API declares as a bare
    // `String`, which rejects null outright rather than falling back to its default.
    let body_map: serde_json::Map<String, Value> = props
        .keys()
        .filter_map(|param_name| {
            args_map
                .get(param_name)
                .filter(|value| !value.is_null())
                .map(|value| {
                    // Use the original name as the key in the request body
                    let original_name = get_original_name(param_name, body_field_renames);
                    (original_name, value.clone())
                })
        })
        .collect();

    if body_map.is_empty() {
        None
    } else {
        Some(Value::Object(body_map))
    }
}

/// Scopes to embed in the JWT minted for a proxied MCP endpoint request. The MCP
/// runner already authorized *which* endpoint may be called; this bounds *what
/// the resulting internal request can do*.
///
/// - Unscoped caller (cookie / full-privilege token): unscoped JWT.
/// - Scope-restricted caller whose own scopes already authorize the route: keep
///   those scopes verbatim, so the target handler's per-path `check_scopes` still
///   enforces the caller's path caps (e.g. a `variables:read:u/admin/safe/*`
///   token can't read `u/admin/secret` via the getVariable proxy).
/// - Otherwise the caller has no route scope for this domain (the common
///   `mcp:`-only token): mint a least-privilege scope for exactly this route,
///   failing closed if the route can't be resolved.
fn jwt_scopes_for_proxied_route(
    caller_scopes: Option<&[String]>,
    method: &str,
    route_path: &str,
) -> BackendResult<Option<Vec<String>>> {
    let caller_restricted =
        caller_scopes.is_some_and(|s| s.iter().any(|x| !x.starts_with("if_jobs:filter_tags:")));
    if !caller_restricted {
        return Ok(None);
    }
    if windmill_api_auth::scopes::check_scopes_for_route(caller_scopes, route_path, method).is_ok()
    {
        return Ok(caller_scopes.map(|s| s.to_vec()));
    }
    let scope =
        windmill_api_auth::scopes::scope_for_route(method, route_path).ok_or_else(|| {
            ErrorData::internal_error(
                "Could not derive route scope for proxied MCP endpoint".to_string(),
                None,
            )
        })?;
    let mut scopes = vec![scope];
    if let Some((tool, extras)) = extra_scopes_for_route(route_path) {
        // Only when the token names this tool. `mcp:all` and `mcp:favorites` reach
        // every endpoint without naming any, and they are the create-token and
        // OAuth defaults — a token whose consent screen talks about scripts and
        // flows must not come with this.
        if caller_scopes.is_some_and(|s| selects_endpoint_tool(s, tool)) {
            scopes.extend(extras.iter().map(|s| (*s).to_string()));
        }
    }
    Ok(Some(scopes))
}

/// The tool a route belongs to and the scopes its handler requires beyond the
/// one the route's own domain implies, added to the JWT minted for that single
/// proxied request.
///
/// Minting is what keeps the grant *confined*: the JWT is built here and handed
/// to the internal request, never to the client, so the MCP token itself stays
/// `mcp:`-only and can't reach `/jobs/run/preview`. Putting the scope on the
/// token instead would widen every request it makes, which is a far larger grant
/// than the tool needs.
fn extra_scopes_for_route(route_path: &str) -> Option<(&'static str, &'static [&'static str])> {
    // Matched on segments: a runnable path is caller-chosen and can contain
    // anything, so a script called `f/apps/update_raw_source/x` must not decide
    // what its own request is minted.
    let mut segments = route_path.split('/').skip_while(|s| *s != "w");
    let (_, _ws) = (segments.next()?, segments.next()?);
    if segments.next() != Some("apps") {
        return None;
    }
    // Both deploy an app by compiling its sources, which runs the app's own
    // dependencies on a worker — a token reaches that only by naming the tool.
    // The names are the ones agents see (`x-mcp-tool-name`), not the operation ids.
    //
    // They are deliberately the names the retired low-code tools used, so a token
    // issued before that switch reaches these instead — an accepted upgrade, not
    // an oversight: MCP has one pair of app-write tools and they are these.
    match segments.next() {
        Some("create_raw_source") => Some(("createApp", &["jobs:run"])),
        Some("update_raw_source") => Some(("updateApp", &["jobs:run"])),
        _ => None,
    }
}

/// Whether the token names `tool` rather than reaching it through a blanket
/// grant. Parsed by `windmill-mcp`, which owns the scope grammar; `mcp:all` and
/// `mcp:favorites` yield `*` or nothing, neither of which names a tool.
fn selects_endpoint_tool(caller_scopes: &[String], tool: &str) -> bool {
    windmill_mcp::common::scope::parse_mcp_scopes(caller_scopes)
        .is_ok_and(|config| config.endpoints.iter().any(|e| e == tool))
}

/// Create HTTP request with authentication.
pub async fn create_http_request(
    method: &str,
    url: &str,
    workspace_id: &str,
    api_authed: &ApiAuthed,
    body_json: Option<Value>,
) -> BackendResult<reqwest::Response> {
    let client = &HTTP_CLIENT;
    let mut request_builder = match method {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "DELETE" => client.delete(url),
        "PATCH" => client.patch(url),
        _ => {
            return Err(ErrorData::invalid_params(
                format!("Unsupported HTTP method: {}", method),
                None,
            ));
        }
    };

    // Scope the minted JWT to the proxied route so a scope-restricted MCP token
    // can't be widened into a full-privilege blank check. See
    // `jwt_scopes_for_proxied_route`.
    let parsed = reqwest::Url::parse(url)
        .map_err(|e| ErrorData::internal_error(format!("Invalid proxied URL: {}", e), None))?;
    let scopes = jwt_scopes_for_proxied_route(api_authed.scopes.as_deref(), method, parsed.path())?;

    // Add authorization header. Carry the caller's job provenance into the proxy
    // JWT: a job's WM_TOKEN is capped at workspace admin (GHSA-hfh4-cx4h-3fcr), and
    // dropping `job_id` here would re-mint an uncapped token that satisfies
    // require_super_admin / require_devops_role on the proxied route.
    let authed = Authed::from(api_authed.clone());
    let token = create_jwt_token(
        authed,
        workspace_id,
        3600,
        api_authed.job_id,
        None,
        None,
        scopes,
    )
    .await
    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
    request_builder = request_builder.header("Authorization", format!("Bearer {}", token));

    // Add body if present
    if let Some(body) = body_json {
        request_builder = request_builder
            .header("Content-Type", "application/json")
            .json(&body);
    }

    request_builder
        .send()
        .await
        .map_err(|e| ErrorData::internal_error(format!("Failed to execute request: {}", e), None))
}

/// The `kind` an MCP-invoked runnable sees on its preprocessor event, alongside
/// `webhook`, `http` and the trigger kinds.
const MCP_TRIGGER_KEY: &str = "mcp";

/// A preprocessor's view of the MCP request that ran it. Mirrors the HTTP
/// trigger event: `body` is what the model sent, everything else describes the
/// call itself.
#[derive(serde::Serialize)]
struct McpPreprocessorEvent<'a> {
    kind: &'a str,
    body: Box<serde_json::value::RawValue>,
    headers: HashMap<String, Box<serde_json::value::RawValue>>,
    tool_name: &'a str,
}

/// Headers withheld from a preprocessor because they authenticate the connection.
///
/// Not a security boundary: a webhook preprocessor receives all three. Withheld
/// because nothing needs them yet, and releasing one later is additive while
/// withdrawing one after runnables read it is not.
const WITHHELD_FROM_PREPROCESSOR: &[&str] = &["authorization", "cookie", "proxy-authorization"];

/// Every header a preprocessor may see.
fn preprocessor_headers(
    headers: &http::HeaderMap,
) -> HashMap<String, Box<serde_json::value::RawValue>> {
    let mut selected = build_headers(headers, None, true);
    selected.retain(|name, _| {
        !WITHHELD_FROM_PREPROCESSOR
            .iter()
            .any(|withheld| withheld.eq_ignore_ascii_case(name))
    });
    selected
}

/// Build the job arguments for a script or flow run as an MCP tool.
///
/// Shaped by the runnable's own format: a preprocessor receives the request as
/// an event, and a runnable without one receives only what the model sent.
pub async fn prepare_push_args(
    db: &DB,
    w_id: &str,
    path: &str,
    is_flow: bool,
    args: Value,
    request: &McpRequest<'_>,
) -> Result<windmill_queue::PushArgsOwned, Error> {
    let mut main_args = HashMap::new();
    if let Value::Object(map) = args {
        for (k, v) in map {
            main_args.insert(k, to_raw_value(&v));
        }
    }

    let runnable_id = if is_flow {
        RunnableId::from_flow_path(path)
    } else {
        // Resolves a `hub/<version_id>` path to the hub script on its own.
        RunnableId::from_script_path(path)
    };

    // MCP is not one of the `TRIGGER_KIND` enum values and does not need to be:
    // the per-kind arms of the no-preprocessor heuristic are payload-shape
    // special cases for message triggers, and `Webhook` reaches the same generic
    // arm MCP wants while sharing that kind's format cache.
    let runnable_format = get_runnable_format(runnable_id, w_id, db, &TriggerKind::Webhook).await?;

    Ok(match runnable_format {
        // Without a preprocessor there is nowhere for a header to go that the
        // model does not also write: its arguments *are* the runnable's
        // parameters, so a header bound to one of them would be a value the model
        // could set. The request is reachable through a preprocessor, where it
        // arrives in a key of the event the model never fills.
        RunnableFormat { has_preprocessor: false, .. } => {
            windmill_queue::PushArgsOwned { args: main_args, extra: None }
        }
        RunnableFormat { has_preprocessor: true, version } => {
            let headers = preprocessor_headers(request.headers);
            match version {
                RunnableFormatVersion::V2 => {
                    let event = McpPreprocessorEvent {
                        kind: MCP_TRIGGER_KEY,
                        body: to_raw_value(&main_args),
                        headers,
                        tool_name: request.tool_name,
                    };
                    windmill_queue::PushArgsOwned {
                        args: HashMap::from([("event".to_string(), to_raw_value(&event))]),
                        extra: None,
                    }
                }
                RunnableFormatVersion::V1 => windmill_queue::PushArgsOwned {
                    args: main_args,
                    extra: Some(HashMap::from([(
                        "wm_trigger".to_string(),
                        to_raw_value(&serde_json::json!({
                            "kind": MCP_TRIGGER_KEY,
                            MCP_TRIGGER_KEY: {
                                "headers": headers,
                                "tool_name": request.tool_name,
                            }
                        })),
                    )])),
                },
            }
        }
    })
}

/// Parse an HTTP response body into a JSON Value
pub async fn parse_response_body(response: Response<Body>) -> BackendResult<Value> {
    let body_bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .map_err(|e| {
            ErrorData::internal_error(format!("Failed to read response body: {}", e), None)
        })?;

    let body_str = String::from_utf8(body_bytes.to_vec()).map_err(|e| {
        ErrorData::internal_error(format!("Failed to decode response body: {}", e), None)
    })?;

    Ok(serde_json::from_str(&body_str).unwrap_or_else(|_| Value::String(body_str)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scopes(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn proxy_jwt_unscoped_caller_keeps_none() {
        // No scopes, or filter-tags-only, is treated as unscoped -> unscoped JWT.
        assert_eq!(
            jwt_scopes_for_proxied_route(None, "GET", "/api/w/ws/variables/get/u/a/b").unwrap(),
            None
        );
        let ft = scopes(&["if_jobs:filter_tags:foo"]);
        assert_eq!(
            jwt_scopes_for_proxied_route(Some(&ft), "GET", "/api/w/ws/variables/get/u/a/b")
                .unwrap(),
            None
        );
    }

    #[test]
    fn proxy_jwt_bare_mcp_token_falls_back_to_route_scope() {
        // A token whose only authority is its mcp: scope has no variables route
        // scope, so the JWT gets a least-privilege route scope for this request.
        let s = scopes(&["mcp:endpoints:getVariable"]);
        assert_eq!(
            jwt_scopes_for_proxied_route(Some(&s), "GET", "/api/w/ws/variables/get/u/admin/secret")
                .unwrap(),
            Some(scopes(&["variables:read"]))
        );
    }

    #[test]
    fn proxy_jwt_raw_app_create_needs_the_tool_named() {
        // Same as the update route: creating an app compiles its sources too.
        let route = "/api/w/ws/apps/create_raw_source";
        let named = scopes(&["mcp:endpoints:createApp"]);
        assert_eq!(
            jwt_scopes_for_proxied_route(Some(&named), "POST", route).unwrap(),
            Some(scopes(&["apps:write", "jobs:run"]))
        );
        let implicit = scopes(&["mcp:all"]);
        assert_eq!(
            jwt_scopes_for_proxied_route(Some(&implicit), "POST", route).unwrap(),
            Some(scopes(&["apps:write"]))
        );
    }

    #[test]
    fn proxy_jwt_raw_app_source_deploy_needs_the_tool_named() {
        // The handler requires jobs:run as well: compiling an app's sources runs
        // its dependencies on a worker. It goes in the per-request JWT, not on the
        // token — the token stays mcp:-only and so can't reach /jobs/run/preview —
        // and only for a token that named this tool. The defaults (mcp:favorites
        // on create, mcp:all through OAuth) reach every endpoint without naming
        // one, and must not carry a capability their consent screen never showed.
        let route = "/api/w/ws/apps/update_raw_source/u/admin/app";
        let named = scopes(&["mcp:endpoints:listScripts,updateApp"]);
        assert_eq!(
            jwt_scopes_for_proxied_route(Some(&named), "POST", route).unwrap(),
            Some(scopes(&["apps:write", "jobs:run"]))
        );
        for implicit in [
            scopes(&["mcp:all"]),
            scopes(&["mcp:favorites"]),
            scopes(&["mcp:endpoints:*"]),
        ] {
            assert_eq!(
                jwt_scopes_for_proxied_route(Some(&implicit), "POST", route).unwrap(),
                Some(scopes(&["apps:write"])),
                "a token that never named the tool must not get jobs:run"
            );
        }
        // A runnable path is caller-chosen, so it must not select the extras of a
        // route it merely spells out.
        assert_eq!(
            jwt_scopes_for_proxied_route(
                Some(&named),
                "POST",
                "/api/w/ws/jobs/run/p/f/apps/update_raw_source/x"
            )
            .unwrap(),
            Some(scopes(&["jobs:run:scripts"]))
        );
    }

    #[test]
    fn proxy_jwt_mixed_token_passes_through_caller_route_scope() {
        // The caller's route scope is preserved so the target handler's per-path
        // check_scopes enforces the cap; the coarse route match here is path-blind.
        let s = scopes(&["mcp:endpoints:getVariable", "variables:read:u/admin/safe/*"]);
        assert_eq!(
            jwt_scopes_for_proxied_route(Some(&s), "GET", "/api/w/ws/variables/get/u/admin/secret")
                .unwrap(),
            Some(s.clone())
        );
    }

    #[test]
    fn proxy_jwt_run_script_bare_mcp_falls_back_to_jobs_run_scripts() {
        let s = scopes(&["mcp:scripts:f/team/*", "mcp:endpoints:*"]);
        assert_eq!(
            jwt_scopes_for_proxied_route(Some(&s), "POST", "/api/w/ws/jobs/run/p/f/team/deploy")
                .unwrap(),
            Some(scopes(&["jobs:run:scripts"]))
        );
    }

    fn endpoint_tool(
        method: &'static str,
        path_params_schema: Option<Value>,
        body_schema: Option<Value>,
        body_field_renames: Option<Value>,
    ) -> EndpointTool {
        EndpointTool {
            name: std::borrow::Cow::Borrowed("testTool"),
            description: std::borrow::Cow::Borrowed("desc"),
            instructions: std::borrow::Cow::Borrowed(""),
            path: std::borrow::Cow::Borrowed("/w/{workspace}/test/{path}"),
            method: std::borrow::Cow::Borrowed(method),
            path_params_schema,
            query_params_schema: None,
            body_schema,
            query_field_renames: None,
            body_field_renames,
        }
    }

    fn args_of(value: Value) -> serde_json::Map<String, Value> {
        value.as_object().unwrap().clone()
    }

    fn path_param_schema() -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        }))
    }

    fn generated_tool(name: &str) -> EndpointTool {
        crate::mcp::auto_generated_endpoints::all_tools()
            .into_iter()
            .find(|t| t.name.as_ref() == name)
            .unwrap_or_else(|| panic!("{name} must be a generated endpoint tool"))
    }

    /// A script/flow tool the URL addresses by path, but `endpoint_path_policy` does
    /// not name, falls through to no policy — which is no path confinement at all, so
    /// a token scoped to `mcp:scripts:f/team/*` reaches every script through it. The
    /// catalogue lives here and the policy in windmill-mcp, so neither crate notices
    /// a tool added on one side and forgotten on the other; this is where they meet.
    #[test]
    fn every_path_addressed_script_or_flow_tool_is_path_confined() {
        let unpoliced: Vec<String> = crate::mcp::auto_generated_endpoints::all_tools()
            .into_iter()
            .filter(|t| {
                (t.path.contains("/scripts/") || t.path.contains("/flows/"))
                    && t.path.contains("{path}")
                    && !windmill_mcp::server::has_endpoint_path_policy(&t.name)
            })
            .map(|t| t.name.to_string())
            .collect();
        assert!(
            unpoliced.is_empty(),
            "path-addressed script/flow tools with no path policy: {unpoliced:?}"
        );
    }

    #[test]
    fn build_request_body_passthrough_forwards_script_args_minus_path() {
        // runScriptByPath-shaped body: additionalProperties, no declared props.
        // `path` is a path param and must be excluded; the rest are the script's
        // arguments and must be forwarded verbatim.
        let tool = endpoint_tool(
            "POST",
            path_param_schema(),
            Some(json!({ "type": "object", "additionalProperties": true })),
            None,
        );
        let args = args_of(json!({
            "path": "u/admin/my_script",
            "name": "alice",
            "count": 3
        }));

        let body = build_request_body(&tool, &args)
            .unwrap()
            .expect("passthrough body should be built");
        let obj = body.as_object().unwrap();
        assert_eq!(obj.get("name"), Some(&json!("alice")));
        assert_eq!(obj.get("count"), Some(&json!(3)));
        assert!(
            !obj.contains_key("path"),
            "path param must be excluded from body"
        );
    }

    #[test]
    fn build_request_body_declared_props_only_forwards_declared() {
        // Endpoints with explicit properties keep the strict declared-only behavior.
        let tool = endpoint_tool(
            "POST",
            None,
            Some(json!({
                "type": "object",
                "properties": { "value": { "type": "string" } },
                "required": ["value"]
            })),
            None,
        );
        let args = args_of(json!({ "value": "x", "sneaky": "y" }));
        let body = build_request_body(&tool, &args).unwrap().unwrap();
        let obj = body.as_object().unwrap();
        assert_eq!(obj.get("value"), Some(&json!("x")));
        assert!(
            !obj.contains_key("sneaky"),
            "undeclared args must be dropped"
        );
    }

    // updateFlow-shaped: `path` is both a path parameter and a body field. The path
    // parameter keeps the plain name; only the body side is mangled.
    fn update_flow_tool() -> EndpointTool {
        endpoint_tool(
            "POST",
            path_param_schema(),
            Some(json!({
                "type": "object",
                "properties": {
                    "summary": { "type": "string" },
                    "value": { "type": "object" },
                    "path__body": { "type": "string" }
                },
                "required": ["summary", "value"],
                "minProperties": 1
            })),
            Some(json!({ "path__body": "path" })),
        )
    }

    #[test]
    fn build_request_body_maps_body_path_alias_for_rename() {
        // The mangled body field carries the *new* path when renaming; it must reach the
        // API under its original name `path`. (An omitted `path__body` is intentionally
        // absent from the body; the server defaults it from the URL path parameter.)
        let args = args_of(json!({
            "path": "f/team/my_flow",
            "path__body": "f/team/renamed_flow",
            "summary": "s",
            "value": {}
        }));

        let body = build_request_body(&update_flow_tool(), &args)
            .unwrap()
            .expect("body should be built");
        assert_eq!(
            body.as_object().unwrap().get("path"),
            Some(&json!("f/team/renamed_flow")),
            "path__body must be sent as `path` so a rename takes effect"
        );
    }

    #[test]
    fn build_request_body_get_has_no_body() {
        let tool = endpoint_tool(
            "GET",
            None,
            Some(json!({ "type": "object", "additionalProperties": true })),
            None,
        );
        assert!(build_request_body(&tool, &args_of(json!({ "a": 1 })))
            .unwrap()
            .is_none());
    }

    #[test]
    fn build_request_body_refuses_bodyless_call_to_required_body_endpoint() {
        // updateVariable's only required argument is the path parameter, so a
        // path-only call satisfies the tool's input schema while leaving the body
        // empty. Dispatching it would reach the API with no JSON body and come back
        // as a 415, so it is refused here with the fields it could have set.
        let tool = generated_tool("updateVariable");
        let err = build_request_body(&tool, &args_of(json!({ "path": "u/admin/a_var" })))
            .expect_err("a path-only updateVariable call must be refused");
        assert!(
            err.message.contains("value"),
            "the error must name the body fields, got: {}",
            err.message
        );

        let body = build_request_body(
            &tool,
            &args_of(json!({ "path": "u/admin/a_var", "value": "v" })),
        )
        .unwrap()
        .expect("a call carrying an update must still build a body");
        assert_eq!(body.as_object().unwrap().get("value"), Some(&json!("v")));
    }

    #[test]
    fn build_request_body_allows_bodyless_run_of_a_no_arg_runnable() {
        // The counterpart: a runnable that takes no arguments has nothing to put in
        // the body, and the run endpoints accept an empty one.
        let tool = generated_tool("runScriptByPath");
        assert!(
            build_request_body(&tool, &args_of(json!({ "path": "u/admin/no_args"})))
                .unwrap()
                .is_none()
        );
    }

    /// Neither script tool asks for a hash: `createScript` never has a parent, and
    /// `updateScript` names the version it supersedes in its URL. A caller that must
    /// fill in every declared argument has none to invent a value for, and the one it
    /// invents anyway is not a field either tool declares, so it never reaches the API.
    #[test]
    fn script_tools_take_no_parent_hash() {
        for name in ["createScript", "updateScript"] {
            let tool = generated_tool(name);
            let props = tool.body_schema.as_ref().unwrap()["properties"]
                .as_object()
                .unwrap();
            assert!(
                !props.contains_key("parent_hash"),
                "{name} must not ask for a parent_hash"
            );
        }

        let body = build_request_body(
            &generated_tool("createScript"),
            &args_of(json!({
                "path": "u/admin/s",
                "summary": "s",
                "content": "export async function main() {}",
                "language": "bun",
                "parent_hash": "0000000000000000",
                // A client that must fill in every argument says "no value" with null;
                // forwarded, it would reach a field the API declares as a bare String.
                "description": null,
            })),
        )
        .unwrap()
        .expect("createScript body should be built");
        let obj = body.as_object().unwrap();

        assert!(!obj.contains_key("parent_hash"));
        assert!(!obj.contains_key("description"));
    }

    /// The path an `updateScript` call omits is the one its URL already carries, so the
    /// tool must not oblige an agent to restate it — a value that drifts from the URL's
    /// moves the script instead of editing it.
    #[test]
    fn update_script_does_not_require_the_destination_path() {
        let tool = generated_tool("updateScript");
        let body_schema = tool.body_schema.as_ref().unwrap();
        // Renamed off the URL's own `path` parameter, and mapped back on the way out.
        assert!(
            body_schema["properties"]
                .as_object()
                .unwrap()
                .contains_key("path__body"),
            "updateScript must still offer a destination path, which is what moves a script"
        );
        assert!(
            !body_schema["required"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r == "path__body" || r == "path"),
            "updateScript must not require the destination path, got: {}",
            body_schema["required"]
        );
    }

    #[test]
    fn validate_path_param_value_accepts_legitimate_windmill_paths() {
        for ok in [
            "u/alice/prod_db",
            "f/folder/sub/my-script",
            "g/all",
            "myscript",
            "01h00000-0000-0000-0000-000000000000",
            "123",
            "u/admin/My App",         // app paths legitimately contain spaces
            "u/admin@windmill.dev/x", // email-style usernames contain '@'
            "f/folder/tag:v1",        // ':' is valid path-segment data
        ] {
            assert!(
                validate_path_param_value("path", ok).is_ok(),
                "expected {ok:?} to be accepted"
            );
        }
    }

    #[test]
    fn validate_path_param_value_rejects_traversal_and_injection() {
        for bad in [
            "../../../resources/get/u/alice/prod_db", // path traversal (the report PoC)
            "..",
            ".",
            "a/../b",
            "a/./b",
            "/leading",
            "trailing/",
            "double//slash",
            "",
            "back\\slash",         // WHATWG converts '\' -> '/'
            "with\nnewline",       // control char (stripped by URL parser)
            "tab\there",           // control char
            "query?x=1",           // query delimiter truncates the path
            "frag#ment",           // fragment delimiter truncates the path
            "pct%2e%2e%2fencoded", // percent-encoded `../`
        ] {
            assert!(
                validate_path_param_value("path", bad).is_err(),
                "expected {bad:?} to be rejected"
            );
        }
    }

    #[test]
    fn substitute_path_params_blocks_cross_endpoint_traversal() {
        let path_schema = Some(json!({
            "type": "object",
            "properties": { "path": { "type": "string" } }
        }));
        let mut args = serde_json::Map::new();
        args.insert(
            "path".to_string(),
            json!("../../../resources/get/u/alice/prod_db"),
        );

        let result = substitute_path_params(
            "/w/{workspace}/scripts/get/p/{path}",
            "dev",
            &args,
            &path_schema,
        );
        assert!(
            result.is_err(),
            "traversal payload must be rejected before URL substitution"
        );
    }

    #[test]
    fn substitute_path_params_allows_normal_path() {
        let path_schema = Some(json!({
            "type": "object",
            "properties": { "path": { "type": "string" } }
        }));
        let mut args = serde_json::Map::new();
        args.insert("path".to_string(), json!("u/alice/my_script"));

        let result = substitute_path_params(
            "/w/{workspace}/scripts/get/p/{path}",
            "dev",
            &args,
            &path_schema,
        )
        .expect("legitimate path should substitute");
        assert_eq!(result, "/w/dev/scripts/get/p/u/alice/my_script");
    }

    fn single_query_schema(param: &str) -> Option<Value> {
        Some(json!({
            "type": "object",
            "properties": { param: { "type": "string" } }
        }))
    }

    #[test]
    fn build_query_string_preserves_json_string_content() {
        // A string param carrying JSON (e.g. the `args` filter on listJobs) must be
        // emitted as its raw content so the backend can `serde_json::from_str` it.
        let mut args = serde_json::Map::new();
        args.insert("args".to_string(), json!("{\"key\":\"val\"}"));

        let qs = build_query_string(&args, &single_query_schema("args"), &None);

        // No backslash escaping: %5C must not appear; the encoded braces/quotes are exact.
        assert_eq!(qs, "?args=%7B%22key%22%3A%22val%22%7D");
        assert!(
            !qs.contains("%5C"),
            "must not contain backslash escapes: {qs}"
        );
    }

    #[test]
    fn build_query_string_keeps_non_string_serialization() {
        let mut args = serde_json::Map::new();
        args.insert("per_page".to_string(), json!(42));
        assert_eq!(
            build_query_string(&args, &single_query_schema("per_page"), &None),
            "?per_page=42"
        );

        let mut args = serde_json::Map::new();
        args.insert("running".to_string(), json!(true));
        assert_eq!(
            build_query_string(&args, &single_query_schema("running"), &None),
            "?running=true"
        );
    }

    #[test]
    fn build_query_string_encodes_plain_string() {
        let mut args = serde_json::Map::new();
        args.insert("path".to_string(), json!("u/alice/my script"));
        assert_eq!(
            build_query_string(&args, &single_query_schema("path"), &None),
            "?path=u%2Falice%2Fmy%20script"
        );
    }

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn scope_patterns_condition_wildcard_disables_filter() {
        // A `*` pattern grants everything, so no SQL condition should be added.
        assert_eq!(scope_patterns_condition(&strings(&["*"])), None);
        assert_eq!(scope_patterns_condition(&strings(&["f/team/*", "*"])), None);
    }

    #[test]
    fn scope_patterns_condition_empty_matches_nothing() {
        // An empty pattern list grants no items of this type.
        assert_eq!(scope_patterns_condition(&[]), Some("false".to_string()));
    }

    #[test]
    fn scope_patterns_condition_exact_path() {
        assert_eq!(
            scope_patterns_condition(&strings(&["u/admin/my_script"])),
            Some("(o.path = 'u/admin/my_script')".to_string())
        );
    }

    #[test]
    fn scope_patterns_condition_subtree() {
        // `f/team/*` matches the folder itself or anything beneath it, mirroring
        // resource_matches_pattern. Underscores in the prefix are LIKE-escaped.
        assert_eq!(
            scope_patterns_condition(&strings(&["f/team/*"])),
            Some("((o.path = 'f/team' OR o.path LIKE 'f/team/%' ESCAPE '\\'))".to_string())
        );
    }

    #[test]
    fn scope_patterns_condition_mixed_ored() {
        assert_eq!(
            scope_patterns_condition(&strings(&["u/admin/one", "f/team/*"])),
            Some(
                "(o.path = 'u/admin/one' OR (o.path = 'f/team' OR o.path LIKE 'f/team/%' ESCAPE '\\'))"
                    .to_string()
            )
        );
    }

    #[test]
    fn scope_patterns_condition_escapes_like_wildcards() {
        // A subtree prefix containing `%`/`_` must be escaped so it isn't treated
        // as a LIKE pattern; the exact-match arm is quoted verbatim by bind.
        assert_eq!(
            scope_patterns_condition(&strings(&["f/a_b/*"])),
            Some("((o.path = 'f/a_b' OR o.path LIKE 'f/a\\_b/%' ESCAPE '\\'))".to_string())
        );
    }

    fn test_api_authed(job_id: Option<uuid::Uuid>) -> ApiAuthed {
        ApiAuthed {
            email: "admin@windmill.dev".to_string(),
            username: "admin".to_string(),
            is_admin: true,
            is_operator: false,
            groups: vec![],
            folders: vec![],
            scopes: None,
            username_override: None,
            username_override_is_token_label: false,
            is_session_token: false,
            token_prefix: None,
            read_only: false,
            job_id,
            credential_expiry: None,
        }
    }

    /// Capture the `Authorization` header of the single request `create_http_request`
    /// proxies, decode the minted JWT, and return its `job_id` claim.
    async fn proxied_jwt_job_id(caller: &ApiAuthed) -> Option<String> {
        use axum::{extract::State, routing::get, Router};
        use std::sync::{Arc, Mutex};
        use windmill_common::auth::JWTAuthClaims;

        // The internal JWT secret must be non-empty for encode/decode to round-trip.
        windmill_common::jwt::JWT_SECRET.store(Arc::new("mytestsecret".to_string()));

        let captured: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let app = Router::new()
            .route(
                "/",
                get(
                    |State(state): State<Arc<Mutex<Option<String>>>>,
                     headers: axum::http::HeaderMap| async move {
                        if let Some(auth) = headers.get(axum::http::header::AUTHORIZATION) {
                            *state.lock().unwrap() =
                                Some(auth.to_str().unwrap_or_default().to_string());
                        }
                        "ok"
                    },
                ),
            )
            .with_state(captured.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let url = format!("http://{addr}/");
        create_http_request("GET", &url, "test-workspace", caller, None)
            .await
            .expect("proxied request should succeed");

        server.abort();

        let header = captured
            .lock()
            .unwrap()
            .clone()
            .expect("no auth header captured");
        let token = header.strip_prefix("Bearer ").unwrap().to_string();
        let jwt = token
            .strip_prefix("jwt_")
            .expect("expected an internal jwt_ token");
        let claims: JWTAuthClaims = windmill_common::jwt::decode_with_internal_secret(jwt)
            .await
            .unwrap();
        claims.job_id
    }

    /// Regression for GHSA-hfh4-cx4h-3fcr: the MCP proxy must carry the caller's
    /// job provenance into the JWT it mints, otherwise a job's WM_TOKEN — capped at
    /// workspace admin — would be re-minted uncapped and pass require_super_admin /
    /// require_devops_role on the proxied route (e.g. listWorkers).
    #[tokio::test]
    async fn create_http_request_preserves_job_id_provenance() {
        let job_id = uuid::Uuid::from_u128(0x0123_4567_89ab_cdef_0123_4567_89ab_cdef);
        assert_eq!(
            proxied_jwt_job_id(&test_api_authed(Some(job_id))).await,
            Some(job_id.to_string()),
            "a job-token caller's job_id must be preserved in the proxied JWT"
        );
    }

    /// The mirror invariant: a non-job caller must not gain a spurious job_id (which
    /// would wrongly cap a legitimate interactive/superadmin MCP token).
    #[tokio::test]
    async fn create_http_request_keeps_non_job_caller_unstamped() {
        assert_eq!(
            proxied_jwt_job_id(&test_api_authed(None)).await,
            None,
            "a non-job caller must not be stamped with a job_id"
        );
    }
}
