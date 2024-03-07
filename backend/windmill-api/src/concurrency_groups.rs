#[cfg(feature = "enterprise")]
use crate::db::{ApiAuthed, DB};
#[cfg(feature = "enterprise")]
use axum::extract::Path;
#[cfg(feature = "enterprise")]
use axum::routing::{delete, get};
#[cfg(feature = "enterprise")]
use axum::{Extension, Json};

use axum::Router;

#[cfg(feature = "enterprise")]
use serde::Serialize;
#[cfg(feature = "enterprise")]
use std::collections::HashMap;
#[cfg(feature = "enterprise")]
use windmill_common::error::Error::{InternalErr, PermissionDenied};
#[cfg(feature = "enterprise")]
use windmill_common::error::JsonResult;

#[cfg(feature = "enterprise")]
pub fn global_service() -> Router {
    Router::new()
        .route("/list", get(list_concurrency_groups))
        .route("/*id", delete(delete_concurrency_group))
}

#[cfg(not(feature = "enterprise"))]
pub fn global_service() -> Router {
    Router::new()
}

#[cfg(feature = "enterprise")]
#[derive(Serialize)]
pub struct ConcurrencyGroups {
    concurrency_id: String,
    job_uuids: Vec<String>,
}

#[cfg(feature = "enterprise")]
async fn list_concurrency_groups(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
) -> JsonResult<Vec<ConcurrencyGroups>> {
    if !authed.is_admin {
        return Err(PermissionDenied(
            "Only administrators can see concurrency groups".to_string(),
        ));
    }
    let concurrency_groups_raw = sqlx::query_as::<_, (String, serde_json::Value)>(
        "SELECT * FROM concurrency_counter ORDER BY concurrency_id ASC",
    )
    .fetch_all(&db)
    .await?;

    let mut concurrency_groups: Vec<ConcurrencyGroups> = vec![];
    for (concurrency_id, job_uuids_json) in concurrency_groups_raw {
        let job_uuids_map = serde_json::from_value::<HashMap<String, serde_json::Value>>(
            job_uuids_json,
        )
        .map_err(|err| {
            tracing::error!(
                "Error deserializing concurrency_counter table content: {:?}",
                err
            );
            InternalErr(format!(
                "Error deserializing concurrency_counter table content: {}",
                err.to_string()
            ))
        })?;
        concurrency_groups.push(ConcurrencyGroups {
            concurrency_id: concurrency_id.clone(),
            job_uuids: job_uuids_map.keys().cloned().collect(),
        })
    }

    return Ok(Json(concurrency_groups));
}

#[cfg(feature = "enterprise")]
async fn delete_concurrency_group(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(concurrency_id): Path<String>,
) -> JsonResult<()> {
    if !authed.is_admin {
        return Err(PermissionDenied(
            "Only administrators can delete concurrency groups".to_string(),
        ));
    }
    let mut tx = db.begin().await?;

    let concurrency_group = sqlx::query_as::<_, (String, i64)>(
        "SELECT concurrency_id, (select COUNT(*) from jsonb_object_keys(job_uuids)) as n_job_uuids FROM concurrency_counter WHERE concurrency_id = $1 FOR UPDATE",
    )
    .bind(concurrency_id.clone())
    .fetch_optional(&mut *tx)
    .await?;

    let n_job_uuids = concurrency_group.map(|cg| cg.1).unwrap_or_default();

    if n_job_uuids > 0 {
        tx.commit().await?;
        return Err(InternalErr(
            "Concurrency group is currently in use, unable to remove it. Retry later.".to_string(),
        ));
    }

    sqlx::query!(
        "DELETE FROM concurrency_counter WHERE concurrency_id = $1",
        concurrency_id.clone(),
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query!(
        "DELETE FROM custom_concurrency_key_ended  WHERE key = $1",
        concurrency_id.clone(),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(()))
}
