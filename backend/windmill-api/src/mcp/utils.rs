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
use windmill_mcp::server::{BackendResult, ErrorData};
use windmill_mcp::{HubResponse, HubScriptInfo, ItemSchema, ResourceInfo, ResourceType};

use crate::db::ApiAuthed;
use crate::HTTP_CLIENT;

// items max limit
const ITEMS_FETCH_MAX_LIMIT: usize = 100;

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
    sqlb.and_where("o.draft_only IS NOT TRUE");
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
        .and_where("o.archived = false")
        .and_where("o.draft_only IS NOT TRUE");

    if item_type == "script" {
        sqlb.and_where("(o.no_main_func IS NOT TRUE OR o.no_main_func IS NULL)");
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
    let url = format!("{}/scripts/top", *HUB_BASE_URL.read().await);
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
                    let value_str = value.to_string();
                    let str_val = value_str.trim_matches('"');
                    format!(
                        "{}={}",
                        urlencoding::encode(&original_name),
                        urlencoding::encode(str_val)
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
) -> Option<Value> {
    if method == "GET" {
        return None;
    }

    let schema = body_schema.as_ref()?;
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

    // Add authorization header
    let authed = Authed::from(api_authed.clone());
    let token = create_jwt_token(authed, workspace_id, 3600, None, None, None, None)
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
