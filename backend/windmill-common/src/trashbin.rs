use serde::Serialize;
use sqlx::PgConnection;

use crate::error::Result;

#[derive(Serialize, sqlx::FromRow)]
pub struct TrashItem {
    pub id: i64,
    pub workspace_id: String,
    pub item_kind: String,
    pub item_path: String,
    pub deleted_by: String,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct TrashItemWithData {
    pub id: i64,
    pub workspace_id: String,
    pub item_kind: String,
    pub item_path: String,
    pub item_data: serde_json::Value,
    pub deleted_by: String,
    pub deleted_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

pub async fn move_to_trash(
    tx: &mut PgConnection,
    workspace_id: &str,
    item_kind: &str,
    item_path: &str,
    item_data: serde_json::Value,
    deleted_by: &str,
) -> Result<i64> {
    let id = sqlx::query_scalar!(
        "INSERT INTO trashbin (workspace_id, item_kind, item_path, item_data, deleted_by)
         VALUES ($1, $2, $3, $4, $5) RETURNING id",
        workspace_id,
        item_kind,
        item_path,
        item_data,
        deleted_by,
    )
    .fetch_one(&mut *tx)
    .await?;

    Ok(id)
}

pub async fn list_trash<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    workspace_id: &str,
    kind_filter: Option<&str>,
    page: Option<i64>,
    per_page: Option<i64>,
) -> Result<Vec<TrashItem>> {
    let per_page = per_page.unwrap_or(100).min(1000);
    let offset = page.unwrap_or(0) * per_page;

    let items = if let Some(kind) = kind_filter {
        sqlx::query_as!(
            TrashItem,
            "SELECT id, workspace_id, item_kind, item_path, deleted_by, deleted_at, expires_at
             FROM trashbin
             WHERE workspace_id = $1 AND item_kind = $2
             ORDER BY deleted_at DESC
             LIMIT $3 OFFSET $4",
            workspace_id,
            kind,
            per_page,
            offset,
        )
        .fetch_all(db)
        .await?
    } else {
        sqlx::query_as!(
            TrashItem,
            "SELECT id, workspace_id, item_kind, item_path, deleted_by, deleted_at, expires_at
             FROM trashbin
             WHERE workspace_id = $1
             ORDER BY deleted_at DESC
             LIMIT $2 OFFSET $3",
            workspace_id,
            per_page,
            offset,
        )
        .fetch_all(db)
        .await?
    };

    Ok(items)
}

pub async fn get_trash_item<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    workspace_id: &str,
    id: i64,
) -> Result<TrashItemWithData> {
    let item = sqlx::query_as!(
        TrashItemWithData,
        "SELECT id, workspace_id, item_kind, item_path, item_data, deleted_by, deleted_at, expires_at
         FROM trashbin
         WHERE workspace_id = $1 AND id = $2",
        workspace_id,
        id,
    )
    .fetch_optional(db)
    .await?
    .ok_or_else(|| crate::error::Error::NotFound("Trash item not found".to_string()))?;

    Ok(item)
}

/// Where a deleted flow's versions wait while it is in the trash. Jobs load versions by id,
/// so runs in flight keep working, while every lookup by path (history, relocks, versioned
/// runs, renames) stops seeing them. Nodes stay at the flow's path: they are shared by
/// content and referenced by id.
fn set_aside_path(trash_id: i64) -> String {
    format!("__deleted/{trash_id}")
}

/// No permission check: the caller must have deleted the flow at `path` itself, in `tx`.
pub async fn set_aside_flow_versions(
    tx: &mut PgConnection,
    workspace_id: &str,
    path: &str,
    trash_id: i64,
) -> Result<()> {
    sqlx::query("UPDATE flow_version SET path = $1 WHERE path = $2 AND workspace_id = $3")
        .bind(set_aside_path(trash_id))
        .bind(path)
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

/// No permission check: the caller must have authorized restoring trash item `trash_id`.
pub async fn restore_flow_versions(
    tx: &mut PgConnection,
    workspace_id: &str,
    path: &str,
    trash_id: i64,
) -> Result<()> {
    sqlx::query("UPDATE flow_version SET path = $1 WHERE path = $2 AND workspace_id = $3")
        .bind(path)
        .bind(set_aside_path(trash_id))
        .bind(workspace_id)
        .execute(&mut *tx)
        .await?;
    Ok(())
}

/// SQL deleting the trash rows matching `cond` and selecting how many there were, along with
/// the versions their flows set aside, except those a queued job still references: those
/// are kept for good. `v2_job.runnable_id` has no index, so the queue is read once against
/// the set-aside ids: a check correlated per version scans `v2_job` once per version.
fn remove_trash_sql(cond: &str) -> String {
    format!(
        "WITH removed AS (
            DELETE FROM trashbin WHERE {cond} RETURNING id, workspace_id, item_kind
        ), aside AS MATERIALIZED (
            SELECT v.workspace_id, v.id FROM flow_version v JOIN removed r
            ON r.item_kind = 'flow' AND v.workspace_id = r.workspace_id
            AND v.path = '__deleted/' || r.id
        ), in_use AS MATERIALIZED (
            SELECT DISTINCT j.runnable_id AS id
            FROM v2_job_queue q JOIN v2_job j ON j.id = q.id
            WHERE q.workspace_id IN (SELECT workspace_id FROM aside)
            AND j.runnable_id IN (SELECT id FROM aside)
        ), _versions AS (
            DELETE FROM flow_version v USING aside a
            WHERE v.workspace_id = a.workspace_id AND v.id = a.id
            AND NOT EXISTS (SELECT 1 FROM in_use u WHERE u.id = v.id)
        )
        SELECT count(*) FROM removed"
    )
}

pub async fn permanently_delete_item<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    workspace_id: &str,
    id: i64,
) -> Result<()> {
    let rows: i64 = sqlx::query_scalar(&remove_trash_sql("workspace_id = $1 AND id = $2"))
        .bind(workspace_id)
        .bind(id)
        .fetch_one(db)
        .await?;

    if rows == 0 {
        return Err(crate::error::Error::NotFound(
            "Trash item not found".to_string(),
        ));
    }

    Ok(())
}

pub async fn empty_trash<'e, E: sqlx::PgExecutor<'e>>(db: E, workspace_id: &str) -> Result<i64> {
    let rows = sqlx::query_scalar(&remove_trash_sql("workspace_id = $1"))
        .bind(workspace_id)
        .fetch_one(db)
        .await?;

    Ok(rows)
}

pub async fn delete_expired_trash<'e, E: sqlx::PgExecutor<'e>>(db: E) -> Result<i64> {
    let rows = sqlx::query_scalar(&remove_trash_sql("expires_at <= now()"))
        .fetch_one(db)
        .await?;

    Ok(rows)
}
