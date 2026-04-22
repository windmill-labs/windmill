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

pub async fn permanently_delete_item<'e, E: sqlx::PgExecutor<'e>>(
    db: E,
    workspace_id: &str,
    id: i64,
) -> Result<()> {
    let rows = sqlx::query!(
        "DELETE FROM trashbin WHERE workspace_id = $1 AND id = $2",
        workspace_id,
        id,
    )
    .execute(db)
    .await?
    .rows_affected();

    if rows == 0 {
        return Err(crate::error::Error::NotFound(
            "Trash item not found".to_string(),
        ));
    }

    Ok(())
}

pub async fn empty_trash<'e, E: sqlx::PgExecutor<'e>>(db: E, workspace_id: &str) -> Result<i64> {
    let rows = sqlx::query!("DELETE FROM trashbin WHERE workspace_id = $1", workspace_id,)
        .execute(db)
        .await?
        .rows_affected();

    Ok(rows as i64)
}

pub async fn delete_expired_trash<'e, E: sqlx::PgExecutor<'e>>(db: E) -> Result<i64> {
    let rows = sqlx::query!("DELETE FROM trashbin WHERE expires_at <= now()")
        .execute(db)
        .await?
        .rows_affected();

    Ok(rows as i64)
}
