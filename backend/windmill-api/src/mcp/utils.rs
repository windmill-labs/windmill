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
use windmill_common::scripts::{get_full_hub_script_by_path, Schema};
use windmill_common::utils::{query_elems_from_hub, StripPath};
use windmill_common::worker::to_raw_value;
use windmill_common::{DB, HUB_BASE_URL};
use windmill_mcp::server::{BackendResult, ErrorData, PathFilter};
use windmill_mcp::{HubResponse, HubScriptInfo, ItemSchema, ResourceInfo, ResourceType};

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
/// field_renames maps renamed_key -> original_key (e.g. {"path__path": "path"}).
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
    path_field_renames: &Option<Value>,
) -> BackendResult<String> {
    let mut path_template = path.replace("{workspace}", workspace_id);

    if let Some(schema) = path_schema {
        if let Some(props) = schema.get("properties").and_then(|p| p.as_object()) {
            for (param_name, _) in props {
                // param_name may be renamed (e.g. "path__path"), get original for URL placeholder
                let original_name = get_original_name(param_name, path_field_renames);
                let placeholder = format!("{{{}}}", original_name);
                match args_map.get(param_name) {
                    Some(param_value) => {
                        if let Some(str_val) = param_value.as_str() {
                            validate_path_param_value(&original_name, str_val)?;
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

/// Build request body from arguments
pub fn build_request_body(
    method: &str,
    args_map: &serde_json::Map<String, Value>,
    body_schema: &Option<Value>,
    body_field_renames: &Option<Value>,
    path_params_schema: &Option<Value>,
    query_params_schema: &Option<Value>,
) -> Option<Value> {
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

    let body_map: serde_json::Map<String, Value> = props
        .keys()
        .filter_map(|param_name| {
            args_map.get(param_name).map(|value| {
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

/// Create HTTP request with authentication
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

    // Bound the minted JWT to exactly this proxied route so a scope-restricted
    // MCP token can't be widened into a full-privilege blank check. The
    // endpoint-name gate (in the MCP runner) already authorized *which* endpoint
    // may be called; this constrains what the resulting request can do. Unscoped
    // callers (cookie / full-privilege tokens) keep an unscoped JWT to preserve
    // existing behavior. A scope-restricted caller whose route can't be resolved
    // fails closed.
    let caller_restricted = api_authed
        .scopes
        .as_deref()
        .is_some_and(|s| s.iter().any(|x| !x.starts_with("if_jobs:filter_tags:")));
    let scopes = if caller_restricted {
        let parsed = reqwest::Url::parse(url)
            .map_err(|e| ErrorData::internal_error(format!("Invalid proxied URL: {}", e), None))?;
        let scope =
            windmill_api_auth::scopes::scope_for_route(method, parsed.path()).ok_or_else(|| {
                ErrorData::internal_error(
                    "Could not derive route scope for proxied MCP endpoint".to_string(),
                    None,
                )
            })?;
        Some(vec![scope])
    } else {
        None
    };

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

/// Convert a JSON Value into PushArgsOwned for job execution
pub fn prepare_push_args(args: Value) -> windmill_queue::PushArgsOwned {
    if let Value::Object(map) = args {
        let mut args_hash = HashMap::new();
        for (k, v) in map {
            args_hash.insert(k, to_raw_value(&v));
        }
        windmill_queue::PushArgsOwned { extra: None, args: args_hash }
    } else {
        windmill_queue::PushArgsOwned::default()
    }
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

    #[test]
    fn build_request_body_passthrough_forwards_script_args_minus_path() {
        // runScriptByPath-shaped body: additionalProperties, no declared props.
        // `path` is a path param and must be excluded; the rest are the script's
        // arguments and must be forwarded verbatim.
        let body_schema = Some(json!({ "type": "object", "additionalProperties": true }));
        let path_schema = Some(json!({
            "type": "object",
            "properties": { "path": { "type": "string" } },
            "required": ["path"]
        }));
        let args: serde_json::Map<String, Value> = json!({
            "path": "u/admin/my_script",
            "name": "alice",
            "count": 3
        })
        .as_object()
        .unwrap()
        .clone();

        let body = build_request_body("POST", &args, &body_schema, &None, &path_schema, &None)
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
        let body_schema = Some(json!({
            "type": "object",
            "properties": { "value": { "type": "string" } },
            "required": ["value"]
        }));
        let args: serde_json::Map<String, Value> = json!({ "value": "x", "sneaky": "y" })
            .as_object()
            .unwrap()
            .clone();
        let body = build_request_body("POST", &args, &body_schema, &None, &None, &None).unwrap();
        let obj = body.as_object().unwrap();
        assert_eq!(obj.get("value"), Some(&json!("x")));
        assert!(
            !obj.contains_key("sneaky"),
            "undeclared args must be dropped"
        );
    }

    #[test]
    fn build_request_body_get_has_no_body() {
        let body_schema = Some(json!({ "type": "object", "additionalProperties": true }));
        let args: serde_json::Map<String, Value> = json!({ "a": 1 }).as_object().unwrap().clone();
        assert!(build_request_body("GET", &args, &body_schema, &None, &None, &None).is_none());
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
            &None,
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
            &None,
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
            token_prefix: None,
            read_only: false,
            job_id,
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

        let header = captured.lock().unwrap().clone().expect("no auth header captured");
        let token = header.strip_prefix("Bearer ").unwrap().to_string();
        let jwt = token.strip_prefix("jwt_").expect("expected an internal jwt_ token");
        let claims: JWTAuthClaims =
            windmill_common::jwt::decode_with_internal_secret(jwt).await.unwrap();
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
