/*
 * Author: Windmill Labs
 * Copyright: Windmill Labs, Inc 2025
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::DB;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};
use windmill_common::error::{Error, Result};

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

/// Copy all basic resources from parent to fork (creates initial references)
/// This is left as a placeholder for the actual implementation
pub async fn copy_workspace_resources(
    tx: &mut Transaction<'_, Postgres>,
    parent_workspace_id: &str,
    fork_workspace_id: &str,
) -> Result<()> {
    // TODO: Implement the actual resource copying logic
    // This should:
    // 1. Find all resources in the parent workspace (scripts, flows, apps, variables, etc.)
    // 2. Create references in the forked_resource_refs table
    // 3. Optionally create lightweight copies of essential resources
    
    // For now, this is intentionally left blank as planned
    // We can implement this later with the specific resource copying strategy
    
    tracing::info!(
        "Copying resources from workspace {} to fork {}",
        parent_workspace_id,
        fork_workspace_id
    );
    
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