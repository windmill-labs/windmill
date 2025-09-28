//! Database operations for MCP server
//!
//! Contains all database query functions and database-related utilities
//! used by the MCP server implementation.

use rmcp::ErrorData;
use sql_builder::prelude::*;
use windmill_common::db::UserDB;
use windmill_common::scripts::{get_full_hub_script_by_path, Schema};
use windmill_common::utils::{query_elems_from_hub, StripPath};
use windmill_common::{DB, HUB_BASE_URL};

use super::models::*;
use crate::db::ApiAuthed;
use crate::HTTP_CLIENT;

/// Check if the user has proper MCP scopes
pub fn check_scopes(authed: &ApiAuthed) -> Result<(), ErrorData> {
    let scopes = authed.scopes.as_ref();
    if scopes.is_none()
        || scopes.unwrap().iter().all(|scope| {
            !scope.starts_with("mcp:all")
                && !scope.starts_with("mcp:favorites")
                && !scope.starts_with("mcp:hub:")
        })
    {
        tracing::error!("Unauthorized: missing mcp scope");
        return Err(ErrorData::internal_error(
            "Unauthorized: missing mcp scope".to_string(),
            None,
        ));
    }
    Ok(())
}

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
    let sql = sqlb.sql().map_err(|_e| {
        tracing::error!("failed to build sql: {}", _e);
        ErrorData::internal_error("failed to build sql", None)
    })?;
    let mut tx = user_db
        .clone()
        .begin(authed)
        .await
        .map_err(|_e| ErrorData::internal_error("failed to begin transaction", None))?;
    let item = sqlx::query_as::<_, ItemSchema>(&sql)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_e| {
            tracing::error!("failed to fetch item schema: {}", _e);
            ErrorData::internal_error("failed to fetch item schema", None)
        })?;
    tx.commit()
        .await
        .map_err(|_e| ErrorData::internal_error("failed to commit transaction", None))?;
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
    let sql = sqlb.sql().map_err(|_e| {
        tracing::error!("failed to build sql: {}", _e);
        ErrorData::internal_error("failed to build sql", None)
    })?;
    let mut tx = user_db
        .clone()
        .begin(authed)
        .await
        .map_err(|_e| ErrorData::internal_error("failed to begin transaction", None))?;
    let rows = sqlx::query_as::<_, ResourceType>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_e| {
            tracing::error!("Failed to fetch resource types: {}", _e);
            ErrorData::internal_error("failed to fetch resource types", None)
        })?;
    tx.commit()
        .await
        .map_err(|_e| ErrorData::internal_error("failed to commit transaction", None))?;
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
    let sql = sqlb.sql().map_err(|_e| {
        tracing::error!("failed to build sql: {}", _e);
        ErrorData::internal_error("failed to build sql", None)
    })?;
    let mut tx = user_db
        .clone()
        .begin(authed)
        .await
        .map_err(|_e| ErrorData::internal_error("failed to begin transaction", None))?;
    let rows = sqlx::query_as::<_, ResourceInfo>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_e| {
            tracing::error!("Failed to fetch resources: {}", _e);
            ErrorData::internal_error("failed to fetch resources", None)
        })?;
    tx.commit()
        .await
        .map_err(|_e| ErrorData::internal_error("failed to commit transaction", None))?;

    Ok(rows)
}

/// Generic function to get items (scripts or flows) from the database
pub async fn get_items<T: for<'a> sqlx::FromRow<'a, sqlx::postgres::PgRow> + Send + Unpin>(
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
    scope_type: &str,
    item_type: &str,
    scope_path: Option<&str>,
) -> Result<Vec<T>, ErrorData> {
    let mut sqlb = SqlBuilder::select_from(&format!("{} as o", item_type));
    let fields = if item_type == "script" {
        vec!["o.path", "o.hash", "o.summary", "o.description", "o.schema"]
    } else {
        // For flows, we need to join with flow_version to get the latest version ID
        sqlb.join(&format!("{}_version as fv", item_type))
            .on(&format!("fv.workspace_id = o.workspace_id AND fv.path = o.path AND fv.id = (SELECT MAX(id) FROM {}_version WHERE workspace_id = o.workspace_id AND path = o.path)", item_type));
        vec!["o.path", "fv.id", "o.summary", "o.description", "o.schema"]
    };
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

    // scope path is always a folder path, format is f/my_folder/*
    if let Some(scope_path) = scope_path {
        if scope_path.split("/").count() != 3
            || !scope_path.starts_with("f/")
            || !scope_path.ends_with("/*")
        {
            return Err(ErrorData::internal_error(
                format!(
                    "Invalid folder format: {}, expected format is f/my_folder/*",
                    scope_path
                ),
                None,
            ));
        }
        sqlb.and_where_like_left("o.path", &scope_path[..scope_path.len() - 2]);
    }

    sqlb.order_by(
        if item_type == "flow" {
            "o.edited_at"
        } else {
            "o.created_at"
        },
        false,
    )
    .limit(100);
    let sql = sqlb.sql().map_err(|_e| {
        tracing::error!("failed to build sql: {}", _e);
        ErrorData::internal_error("failed to build sql", None)
    })?;
    let mut tx = user_db
        .clone()
        .begin(authed)
        .await
        .map_err(|_e| ErrorData::internal_error("failed to begin transaction", None))?;
    let rows = sqlx::query_as::<_, T>(&sql)
        .fetch_all(&mut *tx)
        .await
        .map_err(|_e| {
            tracing::error!("Failed to fetch {}: {}", item_type, _e);
            ErrorData::internal_error(format!("failed to fetch {}", item_type), None)
        })?;
    tx.commit()
        .await
        .map_err(|_e| ErrorData::internal_error("failed to commit transaction", None))?;
    Ok(rows)
}

/// Get scripts from the Hub
pub async fn get_scripts_from_hub(
    db: &DB,
    scope_integrations: Option<&str>,
) -> Result<Vec<HubScriptInfo>, ErrorData> {
    let query_params = Some(vec![
        ("limit", "100".to_string()),
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

/// Get script path by hash
pub async fn get_script_path_by_hash(
    hash: &str,
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
) -> Result<String, ErrorData> {
    let hash_i64 = hash
        .parse::<i64>()
        .map_err(|_| ErrorData::internal_error("Invalid hash format", None))?;

    let mut sqlb = SqlBuilder::select_from("script as o");
    sqlb.fields(&["o.path"]);
    sqlb.and_where("o.hash = ?".bind(&hash_i64));
    sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
    sqlb.and_where("o.archived = false");
    sqlb.and_where("o.draft_only IS NOT TRUE");
    sqlb.and_where("(o.no_main_func IS NOT TRUE OR o.no_main_func IS NULL)");

    let sql = sqlb.sql().map_err(|_e| {
        tracing::error!("failed to build sql: {}", _e);
        ErrorData::internal_error("failed to build sql", None)
    })?;

    let mut tx = user_db
        .clone()
        .begin(authed)
        .await
        .map_err(|_e| ErrorData::internal_error("failed to begin transaction", None))?;

    let row = sqlx::query_scalar::<_, String>(&sql)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_e| {
            tracing::error!("failed to fetch script path by hash: {}", _e);
            ErrorData::internal_error("failed to fetch script path by hash", None)
        })?;

    tx.commit()
        .await
        .map_err(|_e| ErrorData::internal_error("failed to commit transaction", None))?;

    Ok(row)
}

/// Get flow path by version id
pub async fn get_flow_path_by_id(
    version_id: &str,
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
) -> Result<String, ErrorData> {
    let mut sqlb = SqlBuilder::select_from("flow_version as fv");
    sqlb.join("flow as o")
        .on("o.workspace_id = fv.workspace_id AND o.path = fv.path");
    sqlb.fields(&["o.path"]);
    sqlb.and_where(
        "fv.id = ?".bind(
            &version_id
                .parse::<i64>()
                .map_err(|_| ErrorData::internal_error("Invalid version ID format", None))?,
        ),
    );
    sqlb.and_where("fv.workspace_id = ?".bind(&workspace_id));
    sqlb.and_where("o.archived = false");
    sqlb.and_where("o.draft_only IS NOT TRUE");

    let sql = sqlb.sql().map_err(|_e| {
        tracing::error!("failed to build sql: {}", _e);
        ErrorData::internal_error("failed to build sql", None)
    })?;

    let mut tx = user_db
        .clone()
        .begin(authed)
        .await
        .map_err(|_e| ErrorData::internal_error("failed to begin transaction", None))?;

    let row = sqlx::query_scalar::<_, String>(&sql)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_e| {
            tracing::error!("failed to fetch flow path by version id: {}", _e);
            ErrorData::internal_error("failed to fetch flow path by version id", None)
        })?;

    tx.commit()
        .await
        .map_err(|_e| ErrorData::internal_error("failed to commit transaction", None))?;

    Ok(row)
}

/// Get script path and schema by hash
pub async fn get_script_path_and_schema_by_hash(
    hash: &str,
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
) -> Result<ItemPathAndSchema, ErrorData> {
    let hash_i64 = hash
        .parse::<i64>()
        .map_err(|_| ErrorData::internal_error("Invalid hash format", None))?;

    let mut sqlb = SqlBuilder::select_from("script as o");
    sqlb.fields(&["o.path", "o.schema"]);
    sqlb.and_where("o.hash = ?".bind(&hash_i64));
    sqlb.and_where("o.workspace_id = ?".bind(&workspace_id));
    sqlb.and_where("o.archived = false");
    sqlb.and_where("o.draft_only IS NOT TRUE");
    sqlb.and_where("(o.no_main_func IS NOT TRUE OR o.no_main_func IS NULL)");

    let sql = sqlb.sql().map_err(|_e| {
        tracing::error!("failed to build sql: {}", _e);
        ErrorData::internal_error("failed to build sql", None)
    })?;

    let mut tx = user_db
        .clone()
        .begin(authed)
        .await
        .map_err(|_e| ErrorData::internal_error("failed to begin transaction", None))?;

    let row = sqlx::query_as::<_, ItemPathAndSchema>(&sql)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_e| {
            tracing::error!("failed to fetch script path and schema by hash: {}", _e);
            ErrorData::internal_error("failed to fetch script path and schema by hash", None)
        })?;

    tx.commit()
        .await
        .map_err(|_e| ErrorData::internal_error("failed to commit transaction", None))?;

    Ok(row)
}

/// Get flow path and schema by version id
pub async fn get_flow_path_and_schema_by_id(
    version_id: &str,
    user_db: &UserDB,
    authed: &ApiAuthed,
    workspace_id: &str,
) -> Result<ItemPathAndSchema, ErrorData> {
    let mut sqlb = SqlBuilder::select_from("flow_version as fv");
    sqlb.join("flow as o")
        .on("o.workspace_id = fv.workspace_id AND o.path = fv.path");
    sqlb.fields(&["o.path", "fv.schema"]);
    sqlb.and_where(
        "fv.id = ?".bind(
            &version_id
                .parse::<i64>()
                .map_err(|_| ErrorData::internal_error("Invalid version ID format", None))?,
        ),
    );
    sqlb.and_where("fv.workspace_id = ?".bind(&workspace_id));
    sqlb.and_where("o.archived = false");
    sqlb.and_where("o.draft_only IS NOT TRUE");

    let sql = sqlb.sql().map_err(|_e| {
        tracing::error!("failed to build sql: {}", _e);
        ErrorData::internal_error("failed to build sql", None)
    })?;

    let mut tx = user_db
        .clone()
        .begin(authed)
        .await
        .map_err(|_e| ErrorData::internal_error("failed to begin transaction", None))?;

    let row = sqlx::query_as::<_, ItemPathAndSchema>(&sql)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_e| {
            tracing::error!("failed to fetch flow path and schema by version id: {}", _e);
            ErrorData::internal_error("failed to fetch flow path and schema by version id", None)
        })?;

    tx.commit()
        .await
        .map_err(|_e| ErrorData::internal_error("failed to commit transaction", None))?;

    Ok(row)
}
