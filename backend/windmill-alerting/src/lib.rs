use axum::Json;
use serde::Deserialize;

use windmill_common::error::{self, JsonResult};
use windmill_common::worker::CLOUD_HOSTED;
use windmill_common::DB;

#[derive(serde::Serialize)]
pub struct CriticalAlert {
    id: i32,
    alert_type: String,
    message: String,
    created_at: chrono::DateTime<chrono::Utc>,
    acknowledged: Option<bool>,
    workspace_id: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct AlertQueryParams {
    pub page: Option<i32>,
    pub page_size: Option<i32>,
    pub acknowledged: Option<bool>,
}

pub async fn get_critical_alerts(
    db: DB,
    params: AlertQueryParams,
    workspace_id: Option<String>,
) -> JsonResult<serde_json::Value> {
    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(10).min(100) as i64;
    let offset = ((page - 1) * page_size as i32) as i64;

    let total_rows = if let Some(workspace_id) = &workspace_id {
        if params.acknowledged.is_none() {
            sqlx::query_scalar!(
                "SELECT COUNT(*)
                 FROM alerts
                 WHERE workspace_id = $1",
                workspace_id
            )
            .fetch_one(&db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*)
                 FROM alerts
                 WHERE workspace_id = $1 AND COALESCE(acknowledged_workspace, false) = $2",
                workspace_id,
                params.acknowledged
            )
            .fetch_one(&db)
            .await?
        }
    } else {
        if params.acknowledged.is_none() {
            sqlx::query_scalar!(
                "SELECT COUNT(*)
                 FROM alerts"
            )
            .fetch_one(&db)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(*)
                 FROM alerts
                 WHERE COALESCE(acknowledged, false) = $1",
                params.acknowledged
            )
            .fetch_one(&db)
            .await?
        }
    };

    let alerts = if let Some(workspace_id) = workspace_id {
        if params.acknowledged.is_none() {
            sqlx::query_as!(
                CriticalAlert,
                "SELECT id, alert_type, message, created_at, COALESCE(acknowledged_workspace, false) AS acknowledged, workspace_id
                 FROM alerts
                 WHERE workspace_id = $1
                 ORDER BY created_at DESC
                 LIMIT $2 OFFSET $3",
                workspace_id,
                page_size,
                offset
            )
            .fetch_all(&db)
            .await?
        } else {
            sqlx::query_as!(
                CriticalAlert,
                "SELECT id, alert_type, message, created_at, COALESCE(acknowledged_workspace, false) AS acknowledged, workspace_id
                 FROM alerts
                 WHERE workspace_id = $1 AND COALESCE(acknowledged_workspace, false) = $2
                 ORDER BY created_at DESC
                 LIMIT $3 OFFSET $4",
                workspace_id,
                params.acknowledged,
                page_size,
                offset
            )
            .fetch_all(&db)
            .await?
        }
    } else {
        if params.acknowledged.is_none() {
            sqlx::query_as!(
                CriticalAlert,
                "SELECT id, alert_type, message, created_at, COALESCE(acknowledged, false) AS acknowledged, workspace_id
                 FROM alerts
                 ORDER BY created_at DESC
                 LIMIT $1 OFFSET $2",
                page_size,
                offset
            )
            .fetch_all(&db)
            .await?
        } else {
            sqlx::query_as!(
                CriticalAlert,
                "SELECT id, alert_type, message, created_at, COALESCE(acknowledged, false) AS acknowledged, workspace_id
                 FROM alerts
                 WHERE COALESCE(acknowledged, false) = $1
                 ORDER BY created_at DESC
                 LIMIT $2 OFFSET $3",
                params.acknowledged,
                page_size,
                offset
            )
            .fetch_all(&db)
            .await?
        }
    };

    let total_rows = total_rows.unwrap_or(0);
    let total_pages = ((total_rows as f64) / (page_size as f64)).ceil() as i64;

    Ok(Json(serde_json::json!({
        "alerts": alerts,
        "total_rows": total_rows,
        "total_pages": total_pages
    })))
}

pub async fn acknowledge_critical_alert(
    db: DB,
    workspace_id: Option<String>,
    id: i32,
) -> error::Result<String> {
    sqlx::query!(
        "UPDATE alerts
         SET
           acknowledged = true,
           acknowledged_workspace = CASE
             WHEN $3 THEN
               CASE
                 WHEN $2::text IS NOT NULL AND workspace_id = $2 THEN true
                 ELSE acknowledged_workspace
               END
             ELSE true
           END
         WHERE id = $1",
        id,
        workspace_id,
        *CLOUD_HOSTED
    )
    .execute(&db)
    .await?;

    tracing::info!(
        "Acknowledged critical alert with id: {}{}",
        id,
        workspace_id.map_or_else(|| "".to_string(), |w| format!(" for workspace_id: {}", w))
    );
    Ok("Critical alert acknowledged".to_string())
}

pub async fn acknowledge_all_critical_alerts(
    db: DB,
    workspace_id: Option<String>,
) -> error::Result<String> {
    sqlx::query!(
        "UPDATE alerts
         SET
           acknowledged = true,
           acknowledged_workspace = CASE
             WHEN $2 THEN
               CASE
                 WHEN $1::text IS NOT NULL THEN true
                 ELSE acknowledged_workspace
               END
             ELSE true
           END
         WHERE ($1::text IS NOT NULL AND workspace_id = $1)
            OR ($1::text IS NULL)",
        workspace_id,
        *CLOUD_HOSTED
    )
    .execute(&db)
    .await?;

    tracing::info!(
        "Acknowledged all unacknowledged critical alerts{}",
        workspace_id.map_or_else(|| "".to_string(), |w| format!(" for workspace_id: {}", w))
    );
    Ok("All unacknowledged critical alerts acknowledged".to_string())
}
