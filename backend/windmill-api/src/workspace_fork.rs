/*
 * Author: Windmill Labs
 * Copyright: Windmill Labs, Inc 2025
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};
use axum::{extract::{Extension, Path}, routing::{get, post}, Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};
use windmill_common::error::{Error, JsonResult, Result};
use windmill_common::utils::{rd_string, require_admin};

#[derive(FromRow, Serialize, Deserialize)]
pub struct WorkspaceFork {
    pub fork_workspace_id: String,
    pub parent_workspace_id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub created_by: String,
    pub fork_point: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct ForkedResourceRef {
    pub id: i64,
    pub fork_workspace_id: String,
    pub resource_type: String,
    pub resource_path: String,
    pub is_reference: bool,
    pub parent_resource_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateForkRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct ForkResponse {
    pub fork_workspace_id: String,
    pub parent_workspace_id: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Get fork information for a workspace
pub async fn get_fork_info(
    db: &DB,
    workspace_id: &str,
) -> Result<Option<WorkspaceFork>> {
    let fork_info = sqlx::query_as!(
        WorkspaceFork,
        "SELECT fork_workspace_id, parent_workspace_id, created_at, created_by, fork_point 
         FROM workspace_fork 
         WHERE fork_workspace_id = $1",
        workspace_id
    )
    .fetch_optional(db)
    .await?;

    Ok(fork_info)
}

/// Get all resource references for a forked workspace
pub async fn get_forked_resource_refs(
    db: &DB,
    workspace_id: &str,
) -> Result<Vec<ForkedResourceRef>> {
    let refs = sqlx::query_as!(
        ForkedResourceRef,
        "SELECT id, fork_workspace_id, resource_type, resource_path, is_reference, parent_resource_id, created_at
         FROM forked_resource_refs 
         WHERE fork_workspace_id = $1
         ORDER BY resource_type, resource_path",
        workspace_id
    )
    .fetch_all(db)
    .await?;

    Ok(refs)
}

/// Create a resource reference in a forked workspace
pub async fn create_resource_reference(
    tx: &mut Transaction<'_, Postgres>,
    fork_workspace_id: &str,
    resource_type: &str,
    resource_path: &str,
    parent_resource_id: &str,
) -> Result<()> {
    sqlx::query!(
        "INSERT INTO forked_resource_refs (fork_workspace_id, resource_type, resource_path, is_reference, parent_resource_id)
         VALUES ($1, $2, $3, true, $4)",
        fork_workspace_id,
        resource_type,
        resource_path,
        parent_resource_id
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Mark a resource as cloned (no longer a reference)
pub async fn mark_resource_as_cloned(
    tx: &mut Transaction<'_, Postgres>,
    fork_workspace_id: &str,
    resource_type: &str,
    resource_path: &str,
) -> Result<()> {
    sqlx::query!(
        "UPDATE forked_resource_refs 
         SET is_reference = false, parent_resource_id = NULL
         WHERE fork_workspace_id = $1 AND resource_type = $2 AND resource_path = $3",
        fork_workspace_id,
        resource_type,
        resource_path
    )
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Check if a resource is a reference in a forked workspace
pub async fn is_resource_reference(
    db: &DB,
    workspace_id: &str,
    resource_type: &str,
    resource_path: &str,
) -> Result<Option<String>> {
    let result = sqlx::query_scalar!(
        "SELECT parent_resource_id 
         FROM forked_resource_refs 
         WHERE fork_workspace_id = $1 AND resource_type = $2 AND resource_path = $3 AND is_reference = true",
        workspace_id,
        resource_type,
        resource_path
    )
    .fetch_optional(db)
    .await?;

    Ok(result.flatten())
}

/// Create a new workspace fork
pub async fn create_fork(
    db: &DB,
    parent_workspace_id: &str,
    created_by: &str,
    fork_name: &str,
    description: Option<&str>,
) -> Result<ForkResponse> {
    let mut tx = db.begin().await?;
    
    // Generate fork workspace ID
    let fork_workspace_id = format!("{}-fork-{}", parent_workspace_id, rd_string(6));
    
    // Create the fork workspace
    sqlx::query!(
        "INSERT INTO workspace (id, name, owner, deleted, premium) 
         SELECT $1, $2, owner, false, premium FROM workspace WHERE id = $3",
        fork_workspace_id,
        fork_name,
        parent_workspace_id
    )
    .execute(&mut *tx)
    .await?;
    
    // Create fork relationship
    sqlx::query!(
        "INSERT INTO workspace_fork (fork_workspace_id, parent_workspace_id, created_by, fork_point)
         VALUES ($1, $2, $3, NOW())",
        fork_workspace_id,
        parent_workspace_id,
        created_by
    )
    .execute(&mut *tx)
    .await?;
    
    // Copy workspace resources
    copy_workspace_resources(&mut tx, parent_workspace_id, &fork_workspace_id).await?;
    
    tx.commit().await?;
    
    let fork_info = sqlx::query_as!(
        WorkspaceFork,
        "SELECT fork_workspace_id, parent_workspace_id, created_at, created_by, fork_point 
         FROM workspace_fork 
         WHERE fork_workspace_id = $1",
        fork_workspace_id
    )
    .fetch_one(db)
    .await?;
    
    Ok(ForkResponse {
        fork_workspace_id: fork_info.fork_workspace_id,
        parent_workspace_id: fork_info.parent_workspace_id,
        name: fork_name.to_string(),
        created_at: fork_info.created_at,
    })
}

/// Copy all basic resources from parent to fork (creates initial references)
pub async fn copy_workspace_resources(
    tx: &mut Transaction<'_, Postgres>,
    parent_workspace_id: &str,
    fork_workspace_id: &str,
) -> Result<()> {
    tracing::info!(
        "Copying resources from workspace {} to fork {}",
        parent_workspace_id,
        fork_workspace_id
    );
    
    // Copy core workspace resources by creating references
    let resource_types = vec![
        ("script", "SELECT path FROM script WHERE workspace_id = $1 AND NOT deleted AND NOT archived"),
        ("flow", "SELECT path FROM flow WHERE workspace_id = $1 AND NOT archived"),
        ("app", "SELECT path FROM app WHERE workspace_id = $1"),
        ("raw_app", "SELECT path FROM raw_app WHERE workspace_id = $1"),
        ("variable", "SELECT path FROM variable WHERE workspace_id = $1"),
        ("resource", "SELECT path FROM resource WHERE workspace_id = $1"),
        ("resource_type", "SELECT name as path FROM resource_type WHERE workspace_id = $1"),
        ("folder", "SELECT name as path FROM folder WHERE workspace_id = $1"),
        ("schedule", "SELECT path FROM schedule WHERE workspace_id = $1"),
    ];
    
    for (resource_type, query) in resource_types {
        let paths: Vec<String> = sqlx::query_scalar(query)
            .bind(parent_workspace_id)
            .fetch_all(&mut **tx)
            .await?;
            
        for path in paths {
            create_resource_reference(
                tx,
                fork_workspace_id,
                resource_type,
                &path,
                &format!("{}:{}", parent_workspace_id, path),
            ).await?;
        }
    }
    
    Ok(())
}

/// Handle resource modification in a forked workspace
/// This should be called whenever a resource is modified in a fork
pub async fn handle_resource_modification(
    tx: &mut Transaction<'_, Postgres>,
    workspace_id: &str,
    resource_type: &str,
    resource_path: &str,
) -> Result<()> {
    // Check if this is a forked workspace
    let fork_info = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace_fork WHERE fork_workspace_id = $1",
        workspace_id
    )
    .fetch_optional(&mut **tx)
    .await?;

    if let Some(_parent_workspace_id) = fork_info {
        // Check if this resource is currently a reference
        let is_ref = sqlx::query_scalar!(
            "SELECT is_reference FROM forked_resource_refs 
             WHERE fork_workspace_id = $1 AND resource_type = $2 AND resource_path = $3",
            workspace_id,
            resource_type,
            resource_path
        )
        .fetch_optional(&mut **tx)
        .await?
        .unwrap_or(false);

        if is_ref {
            // Mark as cloned since it's being modified
            mark_resource_as_cloned(tx, workspace_id, resource_type, resource_path).await?;
            
            tracing::info!(
                "Resource {}:{} in workspace {} is now cloned due to modification",
                resource_type,
                resource_path,
                workspace_id
            );
        }
    }

    Ok(())
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create_fork", post(create_fork_handler))
        .route("/fork_info", get(get_fork_info_handler))
        .route("/list_forks", get(list_forks_handler))
        .route("/resource_refs", get(list_resource_refs_handler))
}

async fn create_fork_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Json(request): Json<CreateForkRequest>,
) -> JsonResult<ForkResponse> {
    require_admin(authed.is_admin, &authed.username)?;
    
    let fork = create_fork(
        &db,
        &workspace_id,
        &authed.email,
        &request.name,
        request.description.as_deref(),
    ).await?;
    
    Ok(Json(fork))
}

async fn get_fork_info_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Option<WorkspaceFork>> {
    let fork_info = get_fork_info(&db, &workspace_id).await?;
    Ok(Json(fork_info))
}

async fn list_forks_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<WorkspaceFork>> {
    require_admin(authed.is_admin, &authed.username)?;
    
    let forks = sqlx::query_as!(
        WorkspaceFork,
        "SELECT fork_workspace_id, parent_workspace_id, created_at, created_by, fork_point 
         FROM workspace_fork 
         WHERE parent_workspace_id = $1
         ORDER BY created_at DESC",
        workspace_id
    )
    .fetch_all(&db)
    .await?;
    
    Ok(Json(forks))
}

async fn list_resource_refs_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<ForkedResourceRef>> {
    let refs = get_forked_resource_refs(&db, &workspace_id).await?;
    Ok(Json(refs))
}