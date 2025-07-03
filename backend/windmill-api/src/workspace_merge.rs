/*
 * Author: Windmill Labs
 * Copyright: Windmill Labs, Inc 2025
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{Extension, Path},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Postgres, Transaction};
use std::collections::HashMap;
use windmill_common::error::{Error, JsonResult, Result};
use windmill_common::utils::require_admin;

#[derive(FromRow, Serialize, Deserialize)]
pub struct WorkspaceMergeRequest {
    pub id: i64,
    pub source_workspace_id: String,
    pub target_workspace_id: String,
    pub created_by: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub status: String,
    pub title: String,
    pub description: Option<String>,
    pub merged_at: Option<chrono::DateTime<chrono::Utc>>,
    pub merged_by: Option<String>,
    pub rejected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub rejected_by: Option<String>,
    pub rejection_reason: Option<String>,
    pub auto_merge: bool,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct WorkspaceMergeChange {
    pub id: i64,
    pub merge_request_id: i64,
    pub resource_type: String,
    pub resource_path: String,
    pub change_type: String,
    pub source_content_hash: Option<String>,
    pub target_content_hash: Option<String>,
    pub has_conflict: bool,
    pub conflict_reason: Option<String>,
    pub resolved: bool,
    pub resolution_strategy: Option<String>,
    pub resolved_by: Option<String>,
    pub resolved_at: Option<chrono::DateTime<chrono::Utc>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(FromRow, Serialize, Deserialize)]
pub struct WorkspaceMergeContent {
    pub id: i64,
    pub merge_change_id: i64,
    pub content_type: String,
    pub content_hash: String,
    pub content_data: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateMergeRequestRequest {
    pub title: String,
    pub description: Option<String>,
    pub auto_merge: Option<bool>,
}

#[derive(Serialize, Deserialize)]
pub struct MergeRequestResponse {
    pub merge_request: WorkspaceMergeRequest,
    pub changes: Vec<WorkspaceMergeChange>,
    pub conflicts_count: usize,
}

#[derive(Serialize, Deserialize)]
pub struct ResolveMergeConflictRequest {
    pub change_id: i64,
    pub resolution_strategy: String, // 'take_source', 'take_target', 'manual'
    pub manual_content: Option<serde_json::Value>,
}

/// Create a merge request to merge a fork back to its parent
pub async fn create_merge_request(
    db: &DB,
    source_workspace_id: &str,
    target_workspace_id: &str,
    created_by: &str,
    title: &str,
    description: Option<&str>,
    auto_merge: bool,
) -> Result<MergeRequestResponse> {
    let mut tx = db.begin().await?;

    // Verify that source is a fork of target
    let fork_info = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace_fork WHERE fork_workspace_id = $1",
        source_workspace_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let parent_workspace_id =
        fork_info.ok_or_else(|| Error::BadRequest("Source workspace is not a fork".to_string()))?;

    if parent_workspace_id != target_workspace_id {
        return Err(Error::BadRequest(
            "Source workspace is not a fork of the target workspace".to_string(),
        ));
    }

    // Create merge request
    let merge_request_id = sqlx::query_scalar!(
        "INSERT INTO workspace_merge_request 
         (source_workspace_id, target_workspace_id, created_by, title, description, auto_merge)
         VALUES ($1, $2, $3, $4, $5, $6)
         RETURNING id",
        source_workspace_id,
        target_workspace_id,
        created_by,
        title,
        description,
        auto_merge
    )
    .fetch_one(&mut *tx)
    .await?;

    // Analyze changes and create merge changes
    let changes = analyze_workspace_changes(
        &mut tx,
        source_workspace_id,
        target_workspace_id,
        merge_request_id,
    )
    .await?;

    tx.commit().await?;

    // Fetch the created merge request
    let merge_request = get_merge_request(db, merge_request_id).await?;

    let conflicts_count = changes.iter().filter(|c| c.has_conflict).count();

    Ok(MergeRequestResponse { merge_request, changes, conflicts_count })
}

/// Analyze changes between fork and parent workspace
async fn analyze_workspace_changes(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
    merge_request_id: i64,
) -> Result<Vec<WorkspaceMergeChange>> {
    let mut changes = Vec::new();

    // Get all resources that have been modified in the fork (is_reference = false)
    let modified_resources = sqlx::query!(
        "SELECT resource_type, resource_path FROM forked_resource_refs 
         WHERE fork_workspace_id = $1 AND is_reference = false",
        source_workspace_id
    )
    .fetch_all(&mut **tx)
    .await?;

    for resource in modified_resources {
        let resource_type = &resource.resource_type;
        let resource_path = &resource.resource_path;

        // Get content hashes for comparison
        let (source_hash, target_hash) = match resource_type.as_str() {
            "script" => {
                let source = sqlx::query_scalar!(
                    "SELECT encode(sha256(content::bytea), 'hex') FROM script 
                     WHERE workspace_id = $1 AND path = $2",
                    source_workspace_id,
                    resource_path
                )
                .fetch_optional(&mut **tx)
                .await?
                .flatten();

                let target = sqlx::query_scalar!(
                    "SELECT encode(sha256(content::bytea), 'hex') FROM script 
                     WHERE workspace_id = $1 AND path = $2",
                    target_workspace_id,
                    resource_path
                )
                .fetch_optional(&mut **tx)
                .await?
                .flatten();

                (source, target)
            }
            "flow" => {
                let source = sqlx::query_scalar!(
                    "SELECT encode(sha256(value::text::bytea), 'hex') FROM flow 
                     WHERE workspace_id = $1 AND path = $2",
                    source_workspace_id,
                    resource_path
                )
                .fetch_optional(&mut **tx)
                .await?
                .flatten();

                let target = sqlx::query_scalar!(
                    "SELECT encode(sha256(value::text::bytea), 'hex') FROM flow 
                     WHERE workspace_id = $1 AND path = $2",
                    target_workspace_id,
                    resource_path
                )
                .fetch_optional(&mut **tx)
                .await?
                .flatten();

                (source, target)
            }
            _ => (None, None), // Add more resource types as needed
        };

        let change_type = match (&source_hash, &target_hash) {
            (Some(_), None) => "added",
            (Some(s), Some(t)) if s != t => "modified",
            (None, Some(_)) => "deleted",
            _ => continue, // No change
        };

        // Check for conflicts (if target has been modified since fork point)
        let fork_point = sqlx::query_scalar!(
            "SELECT fork_point FROM workspace_fork WHERE fork_workspace_id = $1",
            source_workspace_id
        )
        .fetch_one(&mut **tx)
        .await?;

        let target_modified_after_fork = match resource_type.as_str() {
            "script" => sqlx::query_scalar!(
                "SELECT created_at > $1 FROM script 
                     WHERE workspace_id = $2 AND path = $3",
                fork_point,
                target_workspace_id,
                resource_path
            )
            .fetch_optional(&mut **tx)
            .await?
            .flatten()
            .unwrap_or(false),
            "flow" => sqlx::query_scalar!(
                "SELECT edited_at > $1 FROM flow 
                     WHERE workspace_id = $2 AND path = $3",
                fork_point,
                target_workspace_id,
                resource_path
            )
            .fetch_optional(&mut **tx)
            .await?
            .flatten()
            .unwrap_or(false),
            _ => false,
        };

        let has_conflict = target_modified_after_fork && change_type == "modified";
        let conflict_reason = if has_conflict {
            Some("Resource was modified in both source and target workspace".to_string())
        } else {
            None
        };

        // Create merge change record
        let change_id = sqlx::query_scalar!(
            "INSERT INTO workspace_merge_change 
             (merge_request_id, resource_type, resource_path, change_type, 
              source_content_hash, target_content_hash, has_conflict, conflict_reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id",
            merge_request_id,
            resource_type,
            resource_path,
            change_type,
            source_hash,
            target_hash,
            has_conflict,
            conflict_reason
        )
        .fetch_one(&mut **tx)
        .await?;

        changes.push(WorkspaceMergeChange {
            id: change_id,
            merge_request_id,
            resource_type: resource_type.clone(),
            resource_path: resource_path.clone(),
            change_type: change_type.to_string(),
            source_content_hash: source_hash,
            target_content_hash: target_hash,
            has_conflict,
            conflict_reason,
            resolved: false,
            resolution_strategy: None,
            resolved_by: None,
            resolved_at: None,
            created_at: chrono::Utc::now(),
        });
    }

    Ok(changes)
}

/// Get a merge request by ID
pub async fn get_merge_request(db: &DB, merge_request_id: i64) -> Result<WorkspaceMergeRequest> {
    let merge_request = sqlx::query_as!(
        WorkspaceMergeRequest,
        "SELECT id, source_workspace_id, target_workspace_id, created_by, created_at,
                status, title, description, merged_at, merged_by, rejected_at, rejected_by,
                rejection_reason, auto_merge
         FROM workspace_merge_request WHERE id = $1",
        merge_request_id
    )
    .fetch_one(db)
    .await?;

    Ok(merge_request)
}

/// Execute a merge request (apply changes to target workspace)
pub async fn execute_merge(db: &DB, merge_request_id: i64, merged_by: &str) -> Result<()> {
    let mut tx = db.begin().await?;

    // Get merge request details
    let merge_request = get_merge_request(db, merge_request_id).await?;

    if merge_request.status != "pending" && merge_request.status != "approved" {
        return Err(Error::BadRequest(
            "Merge request is not in a mergeable state".to_string(),
        ));
    }

    // Get all unresolved conflicts
    let unresolved_conflicts = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM workspace_merge_change 
         WHERE merge_request_id = $1 AND has_conflict = true AND resolved = false",
        merge_request_id
    )
    .fetch_one(&mut *tx)
    .await?
    .unwrap_or(0);

    if unresolved_conflicts > 0 {
        return Err(Error::BadRequest(format!(
            "Cannot merge: {} unresolved conflicts remain",
            unresolved_conflicts
        )));
    }

    // Apply all changes to target workspace
    let changes = sqlx::query_as!(
        WorkspaceMergeChange,
        "SELECT id, merge_request_id, resource_type, resource_path, change_type,
                source_content_hash, target_content_hash, has_conflict, conflict_reason,
                resolved, resolution_strategy, resolved_by, resolved_at, created_at
         FROM workspace_merge_change WHERE merge_request_id = $1",
        merge_request_id
    )
    .fetch_all(&mut *tx)
    .await?;

    for change in changes {
        apply_change_to_workspace(
            &mut tx,
            &merge_request.source_workspace_id,
            &merge_request.target_workspace_id,
            &change,
        )
        .await?;
    }

    // Mark merge request as merged
    sqlx::query!(
        "UPDATE workspace_merge_request 
         SET status = 'merged', merged_at = NOW(), merged_by = $1
         WHERE id = $2",
        merged_by,
        merge_request_id
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

/// Apply a single change to the target workspace
async fn apply_change_to_workspace(
    tx: &mut Transaction<'_, Postgres>,
    source_workspace_id: &str,
    target_workspace_id: &str,
    change: &WorkspaceMergeChange,
) -> Result<()> {
    match change.resource_type.as_str() {
        "script" => {
            match change.change_type.as_str() {
                "added" | "modified" => {
                    // Copy script from source to target
                    sqlx::query!(
                        "INSERT INTO script 
                         (workspace_id, hash, path, parent_hashes, summary, description, content,
                          created_by, created_at, archived, schema, deleted, is_template, 
                          extra_perms, lock, lock_error_logs, language, kind, tag, draft_only,
                          envs, concurrent_limit, concurrency_time_window_s, cache_ttl, 
                          dedicated_worker, ws_error_handler_muted, priority, timeout, 
                          delete_after_use, restart_unless_cancelled, concurrency_key, 
                          visible_to_runner_only, no_main_func, codebase, has_preprocessor,
                          on_behalf_of_email, schema_validation)
                         SELECT $1 as workspace_id, hash, path, parent_hashes, summary, description, content,
                                created_by, created_at, archived, schema, deleted, is_template, 
                                extra_perms, lock, lock_error_logs, language, kind, tag, draft_only,
                                envs, concurrent_limit, concurrency_time_window_s, cache_ttl, 
                                dedicated_worker, ws_error_handler_muted, priority, timeout, 
                                delete_after_use, restart_unless_cancelled, concurrency_key, 
                                visible_to_runner_only, no_main_func, codebase, has_preprocessor,
                                on_behalf_of_email, schema_validation
                         FROM script WHERE workspace_id = $2 AND path = $3
                         ON CONFLICT (workspace_id, hash) DO UPDATE SET
                         path = EXCLUDED.path, summary = EXCLUDED.summary, description = EXCLUDED.description,
                         content = EXCLUDED.content",
                        target_workspace_id,
                        source_workspace_id,
                        change.resource_path
                    )
                    .execute(&mut **tx)
                    .await?;
                }
                "deleted" => {
                    sqlx::query!(
                        "UPDATE script SET deleted = true 
                         WHERE workspace_id = $1 AND path = $2",
                        target_workspace_id,
                        change.resource_path
                    )
                    .execute(&mut **tx)
                    .await?;
                }
                _ => {
                    return Err(Error::InternalErr(format!(
                        "Unknown change type: {}",
                        change.change_type
                    )))
                }
            }
        }
        "flow" => match change.change_type.as_str() {
            "added" | "modified" => {
                sqlx::query!(
                        "INSERT INTO flow 
                         (workspace_id, path, summary, description, value, edited_by, edited_at,
                          archived, schema, extra_perms, dependency_job, draft_only, tag,
                          ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only,
                          concurrency_key, versions, on_behalf_of_email, lock_error_logs)
                         SELECT $1 as workspace_id, path, summary, description, value, edited_by, edited_at,
                                archived, schema, extra_perms, dependency_job, draft_only, tag,
                                ws_error_handler_muted, dedicated_worker, timeout, visible_to_runner_only,
                                concurrency_key, versions, on_behalf_of_email, lock_error_logs
                         FROM flow WHERE workspace_id = $2 AND path = $3
                         ON CONFLICT (workspace_id, path) DO UPDATE SET
                         summary = EXCLUDED.summary, description = EXCLUDED.description,
                         value = EXCLUDED.value, edited_by = EXCLUDED.edited_by, 
                         edited_at = EXCLUDED.edited_at",
                        target_workspace_id,
                        source_workspace_id,
                        change.resource_path
                    )
                    .execute(&mut **tx)
                    .await?;
            }
            "deleted" => {
                sqlx::query!(
                    "UPDATE flow SET archived = true 
                         WHERE workspace_id = $1 AND path = $2",
                    target_workspace_id,
                    change.resource_path
                )
                .execute(&mut **tx)
                .await?;
            }
            _ => {
                return Err(Error::InternalErr(format!(
                    "Unknown change type: {}",
                    change.change_type
                )))
            }
        },
        // Add more resource types as needed (apps, variables, etc.)
        _ => {
            tracing::warn!(
                "Merge not implemented for resource type: {}",
                change.resource_type
            );
        }
    }

    Ok(())
}

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/create_merge_request", post(create_merge_request_handler))
        .route("/list_merge_requests", get(list_merge_requests_handler))
        .route("/merge_request/:id", get(get_merge_request_handler))
        .route("/merge_request/:id/execute", post(execute_merge_handler))
        .route("/merge_request/:id/changes", get(get_merge_changes_handler))
        .route(
            "/merge_request/:id/resolve_conflict",
            post(resolve_conflict_handler),
        )
}

async fn create_merge_request_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
    Json(request): Json<CreateMergeRequestRequest>,
) -> JsonResult<MergeRequestResponse> {
    require_admin(authed.is_admin, &authed.username)?;

    // Get parent workspace ID for this fork
    let parent_workspace_id = sqlx::query_scalar!(
        "SELECT parent_workspace_id FROM workspace_fork WHERE fork_workspace_id = $1",
        workspace_id
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| Error::BadRequest("Workspace is not a fork".to_string()))?;

    let merge_response = create_merge_request(
        &db,
        &workspace_id,
        &parent_workspace_id,
        &authed.email,
        &request.title,
        request.description.as_deref(),
        request.auto_merge.unwrap_or(false),
    )
    .await?;

    Ok(Json(merge_response))
}

async fn list_merge_requests_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(workspace_id): Path<String>,
) -> JsonResult<Vec<WorkspaceMergeRequest>> {
    let merge_requests = sqlx::query_as!(
        WorkspaceMergeRequest,
        "SELECT id, source_workspace_id, target_workspace_id, created_by, created_at,
                status, title, description, merged_at, merged_by, rejected_at, rejected_by,
                rejection_reason, auto_merge
         FROM workspace_merge_request 
         WHERE source_workspace_id = $1 OR target_workspace_id = $1
         ORDER BY created_at DESC",
        workspace_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(merge_requests))
}

async fn get_merge_request_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, merge_request_id)): Path<(String, i64)>,
) -> JsonResult<WorkspaceMergeRequest> {
    let merge_request = get_merge_request(&db, merge_request_id).await?;
    Ok(Json(merge_request))
}

async fn execute_merge_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, merge_request_id)): Path<(String, i64)>,
) -> JsonResult<()> {
    require_admin(authed.is_admin, &authed.username)?;

    execute_merge(&db, merge_request_id, &authed.email).await?;
    Ok(Json(()))
}

async fn get_merge_changes_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, merge_request_id)): Path<(String, i64)>,
) -> JsonResult<Vec<WorkspaceMergeChange>> {
    let changes = sqlx::query_as!(
        WorkspaceMergeChange,
        "SELECT id, merge_request_id, resource_type, resource_path, change_type,
                source_content_hash, target_content_hash, has_conflict, conflict_reason,
                resolved, resolution_strategy, resolved_by, resolved_at, created_at
         FROM workspace_merge_change WHERE merge_request_id = $1
         ORDER BY resource_type, resource_path",
        merge_request_id
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(changes))
}

async fn resolve_conflict_handler(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((workspace_id, merge_request_id)): Path<(String, i64)>,
    Json(request): Json<ResolveMergeConflictRequest>,
) -> JsonResult<()> {
    require_admin(authed.is_admin, &authed.username)?;

    sqlx::query!(
        "UPDATE workspace_merge_change 
         SET resolved = true, resolution_strategy = $1, resolved_by = $2, resolved_at = NOW()
         WHERE id = $3 AND merge_request_id = $4",
        request.resolution_strategy,
        authed.email,
        request.change_id,
        merge_request_id
    )
    .execute(&db)
    .await?;

    Ok(Json(()))
}
