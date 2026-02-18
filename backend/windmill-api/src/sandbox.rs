use axum::{
    body::Bytes,
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::value::RawValue as JsonRawValue;
use std::collections::HashMap;
use windmill_common::{
    db::DB,
    error::{self, JsonResult},
    users::username_to_permissioned_as,
    utils::require_admin,
};
use windmill_queue::{push, PushArgs, PushIsolationLevel};
use windmill_sandbox::{SandboxSnapshot, SandboxVolume};
use windmill_types::jobs::JobPayload;

use crate::db::ApiAuthed;

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/snapshots", get(list_snapshots).post(create_snapshot))
        .route(
            "/snapshots/:name/:tag",
            get(get_snapshot).delete(delete_snapshot),
        )
        .route("/snapshots/:name", delete(delete_snapshot_all_tags))
        .route("/snapshots/:name/:tag/rebuild", post(rebuild_snapshot))
        .route("/snapshots/:name/:tag/upload", post(upload_snapshot))
        .route("/volumes", get(list_volumes).post(create_volume))
        .route("/volumes/:name", get(get_volume).delete(delete_volume))
}

#[derive(Deserialize)]
struct ListSnapshotsQuery {
    name: Option<String>,
}

async fn list_snapshots(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListSnapshotsQuery>,
) -> JsonResult<Vec<SandboxSnapshot>> {
    require_admin(authed.is_admin, &authed.username)?;
    let rows = if let Some(name) = query.name {
        sqlx::query_as!(
            SandboxSnapshot,
            "SELECT workspace_id, name, tag, s3_key, content_hash, docker_image, setup_script, \
             size_bytes, status, build_error, build_job_id, created_by, created_at, updated_at, \
             extra_perms \
             FROM sandbox_snapshot WHERE workspace_id = $1 AND name = $2 ORDER BY created_at DESC",
            &w_id,
            &name,
        )
        .fetch_all(&db)
        .await?
    } else {
        sqlx::query_as!(
            SandboxSnapshot,
            "SELECT workspace_id, name, tag, s3_key, content_hash, docker_image, setup_script, \
             size_bytes, status, build_error, build_job_id, created_by, created_at, updated_at, \
             extra_perms \
             FROM sandbox_snapshot WHERE workspace_id = $1 ORDER BY created_at DESC",
            &w_id,
        )
        .fetch_all(&db)
        .await?
    };
    Ok(Json(rows))
}

#[derive(Deserialize)]
struct CreateSnapshot {
    name: String,
    #[serde(default = "default_tag")]
    tag: String,
    docker_image: String,
    setup_script: Option<String>,
}

fn default_tag() -> String {
    "latest".to_string()
}

async fn create_snapshot(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(body): Json<CreateSnapshot>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let s3_key = format!(
        "sandbox/snapshots/{}/{}/{}.tar.gz",
        w_id, body.name, body.tag
    );
    sqlx::query!(
        "INSERT INTO sandbox_snapshot \
         (workspace_id, name, tag, s3_key, docker_image, setup_script, created_by, status) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, 'pending') \
         ON CONFLICT (workspace_id, name, tag) DO UPDATE SET \
         docker_image = $5, setup_script = $6, status = 'pending', \
         build_error = NULL, updated_at = now()",
        &w_id,
        &body.name,
        &body.tag,
        &s3_key,
        &body.docker_image,
        body.setup_script.as_deref(),
        &authed.username,
    )
    .execute(&db)
    .await?;

    let job_uuid = push_snapshot_build_job(
        &db,
        &w_id,
        &body.name,
        &body.tag,
        &body.docker_image,
        body.setup_script.as_deref(),
        &authed,
    )
    .await?;

    sqlx::query!(
        "UPDATE sandbox_snapshot SET build_job_id = $4 \
         WHERE workspace_id = $1 AND name = $2 AND tag = $3",
        &w_id,
        &body.name,
        &body.tag,
        job_uuid,
    )
    .execute(&db)
    .await?;

    Ok(format!(
        "Snapshot {}:{} created with build job {}",
        body.name, body.tag, job_uuid
    ))
}

async fn get_snapshot(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name, tag)): Path<(String, String, String)>,
) -> JsonResult<SandboxSnapshot> {
    require_admin(authed.is_admin, &authed.username)?;
    let row = sqlx::query_as!(
        SandboxSnapshot,
        "SELECT workspace_id, name, tag, s3_key, content_hash, docker_image, setup_script, \
         size_bytes, status, build_error, build_job_id, created_by, created_at, updated_at, \
         extra_perms \
         FROM sandbox_snapshot WHERE workspace_id = $1 AND name = $2 AND tag = $3",
        &w_id,
        &name,
        &tag,
    )
    .fetch_optional(&db)
    .await?;
    let row = windmill_common::utils::not_found_if_none(row, "sandbox_snapshot", &name)?;
    Ok(Json(row))
}

async fn delete_snapshot(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name, tag)): Path<(String, String, String)>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let row = sqlx::query!(
        "DELETE FROM sandbox_snapshot WHERE workspace_id = $1 AND name = $2 AND tag = $3 \
         RETURNING s3_key",
        &w_id,
        &name,
        &tag,
    )
    .fetch_optional(&db)
    .await?;

    if let Some(row) = row {
        windmill_object_store::delete_s3_object(&db, &w_id, &row.s3_key).await;
    }

    Ok(format!("Deleted snapshot {}:{}", name, tag))
}

async fn delete_snapshot_all_tags(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let rows = sqlx::query!(
        "DELETE FROM sandbox_snapshot WHERE workspace_id = $1 AND name = $2 RETURNING s3_key",
        &w_id,
        &name,
    )
    .fetch_all(&db)
    .await?;

    for row in &rows {
        windmill_object_store::delete_s3_object(&db, &w_id, &row.s3_key).await;
    }

    Ok(format!(
        "Deleted {} tag(s) for snapshot {}",
        rows.len(),
        name
    ))
}

async fn rebuild_snapshot(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name, tag)): Path<(String, String, String)>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let row = sqlx::query!(
        "SELECT docker_image, setup_script FROM sandbox_snapshot \
         WHERE workspace_id = $1 AND name = $2 AND tag = $3",
        &w_id,
        &name,
        &tag,
    )
    .fetch_optional(&db)
    .await?;

    let row = windmill_common::utils::not_found_if_none(row, "sandbox_snapshot", &name)?;

    sqlx::query!(
        "UPDATE sandbox_snapshot SET status = 'pending', build_error = NULL, updated_at = now() \
         WHERE workspace_id = $1 AND name = $2 AND tag = $3",
        &w_id,
        &name,
        &tag,
    )
    .execute(&db)
    .await?;

    let job_uuid = push_snapshot_build_job(
        &db,
        &w_id,
        &name,
        &tag,
        &row.docker_image,
        row.setup_script.as_deref(),
        &authed,
    )
    .await?;

    sqlx::query!(
        "UPDATE sandbox_snapshot SET build_job_id = $4 \
         WHERE workspace_id = $1 AND name = $2 AND tag = $3",
        &w_id,
        &name,
        &tag,
        job_uuid,
    )
    .execute(&db)
    .await?;

    Ok(format!(
        "Rebuild queued for snapshot {}:{} with build job {}",
        name, tag, job_uuid
    ))
}

async fn upload_snapshot(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name, tag)): Path<(String, String, String)>,
    body: Bytes,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    windmill_sandbox::upload_snapshot_bytes(&db, &w_id, &name, &tag, body, &authed.username).await
}

async fn push_snapshot_build_job(
    db: &DB,
    w_id: &str,
    name: &str,
    tag: &str,
    docker_image: &str,
    setup_script: Option<&str>,
    authed: &ApiAuthed,
) -> error::Result<uuid::Uuid> {
    let mut args = HashMap::new();
    args.insert(
        "snapshot_name".to_string(),
        JsonRawValue::from_string(serde_json::to_string(name).unwrap()).unwrap(),
    );
    args.insert(
        "snapshot_tag".to_string(),
        JsonRawValue::from_string(serde_json::to_string(tag).unwrap()).unwrap(),
    );
    args.insert(
        "docker_image".to_string(),
        JsonRawValue::from_string(serde_json::to_string(docker_image).unwrap()).unwrap(),
    );
    args.insert(
        "setup_script".to_string(),
        JsonRawValue::from_string(serde_json::to_string(&setup_script).unwrap()).unwrap(),
    );

    let tx = PushIsolationLevel::IsolatedRoot(db.clone());
    let (job_uuid, tx) = push(
        db,
        tx,
        w_id,
        JobPayload::SnapshotBuild {
            snapshot_name: name.to_string(),
            snapshot_tag: tag.to_string(),
        },
        PushArgs { args: &args, extra: None },
        &authed.username,
        &authed.email,
        username_to_permissioned_as(&authed.username),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
        false,
        None,
        true,
        None,
        None,
        None,
        None,
        None,
        false,
        None,
        None,
        None,
    )
    .await?;
    tx.commit().await?;

    Ok(job_uuid)
}

// --- Volumes ---

async fn list_volumes(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> JsonResult<Vec<SandboxVolume>> {
    require_admin(authed.is_admin, &authed.username)?;
    let rows = sqlx::query_as!(
        SandboxVolume,
        "SELECT workspace_id, name, s3_key, size_bytes, created_by, created_at, updated_at, \
         extra_perms \
         FROM sandbox_volume WHERE workspace_id = $1 ORDER BY created_at DESC",
        &w_id,
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(rows))
}

#[derive(Deserialize)]
struct CreateVolume {
    name: String,
}

async fn create_volume(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(body): Json<CreateVolume>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;
    let s3_key = format!("sandbox/volumes/{}/{}.tar.gz", w_id, body.name);
    sqlx::query!(
        "INSERT INTO sandbox_volume (workspace_id, name, s3_key, created_by) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (workspace_id, name) DO NOTHING",
        &w_id,
        &body.name,
        &s3_key,
        &authed.username,
    )
    .execute(&db)
    .await?;
    Ok(format!("Volume {} created", body.name))
}

async fn get_volume(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> JsonResult<SandboxVolume> {
    require_admin(authed.is_admin, &authed.username)?;
    let row = sqlx::query_as!(
        SandboxVolume,
        "SELECT workspace_id, name, s3_key, size_bytes, created_by, created_at, updated_at, \
         extra_perms \
         FROM sandbox_volume WHERE workspace_id = $1 AND name = $2",
        &w_id,
        &name,
    )
    .fetch_optional(&db)
    .await?;
    let row = windmill_common::utils::not_found_if_none(row, "sandbox_volume", &name)?;
    Ok(Json(row))
}

async fn delete_volume(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, name)): Path<(String, String)>,
) -> error::Result<String> {
    require_admin(authed.is_admin, &authed.username)?;

    let row = sqlx::query!(
        "DELETE FROM sandbox_volume WHERE workspace_id = $1 AND name = $2 RETURNING s3_key",
        &w_id,
        &name,
    )
    .fetch_optional(&db)
    .await?;

    if let Some(row) = row {
        windmill_object_store::delete_s3_object(&db, &w_id, &row.s3_key).await;
    }

    Ok(format!("Deleted volume {}", name))
}
