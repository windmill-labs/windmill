/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Read-only copies of AI session artifacts, shared with the workspace by their author.
//!
//! Artifacts live in the author's browser; a row here exists only because the author asked
//! for a link. Every read filters on the retention window as well as the monitor sweeping
//! it, so a share past its window is never served in the gap before the sweep reaches it.

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{DefaultBodyLimit, Extension, Json, Path, Query},
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::{
    ai_shared_artifact_retention_secs,
    error::{Error, JsonResult, Result},
};

/// The frontend's `MAX_ARTIFACT_BYTES`: no artifact the viewer holds is larger.
const MAX_CONTENT_BYTES: usize = 256 * 1024;
const MAX_NAME_CHARS: usize = 255;
const MAX_ARTIFACT_ID_CHARS: usize = 255;
/// JSON escapes a control character into six bytes, so a full-size artifact made entirely of
/// them still fits.
const SHARE_BODY_LIMIT: usize = MAX_CONTENT_BYTES * 6 + 64 * 1024;

pub fn workspaced_service() -> Router {
    Router::new()
        .route(
            "/share",
            post(share_artifact).layer(DefaultBodyLimit::max(SHARE_BODY_LIMIT)),
        )
        .route("/status", get(get_share_status))
        .route("/get/{id}", get(get_shared_artifact))
        .route("/delete/{id}", delete(unshare_artifact))
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum ArtifactKind {
    Md,
    Html,
}

impl ArtifactKind {
    fn as_str(self) -> &'static str {
        match self {
            ArtifactKind::Md => "md",
            ArtifactKind::Html => "html",
        }
    }
}

#[derive(Deserialize)]
struct ShareArtifact {
    artifact_id: String,
    name: String,
    kind: ArtifactKind,
    version: i32,
    content: String,
}

#[derive(Serialize)]
struct SharedArtifactInfo {
    id: Uuid,
    name: String,
    kind: String,
    version: i32,
    created_by: String,
    shared_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct SharedArtifact {
    #[serde(flatten)]
    info: SharedArtifactInfo,
    content: String,
    /// Whether the caller may stop sharing it: its author, or a workspace admin.
    can_unshare: bool,
}

#[derive(Serialize)]
struct ShareStatus {
    retention_secs: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    share: Option<SharedArtifactInfo>,
}

#[derive(Deserialize)]
struct ShareStatusQuery {
    artifact_id: String,
}

fn expires_at(shared_at: DateTime<Utc>, retention_secs: i64) -> DateTime<Utc> {
    shared_at + chrono::Duration::seconds(retention_secs)
}

/// The browser-side artifact id, as every handler that takes one must check it: it is compared
/// against a `VARCHAR(255)` column, and Postgres answers a NUL in a text parameter with an
/// opaque 500.
fn check_artifact_id(artifact_id: &str) -> Result<()> {
    if artifact_id.is_empty() || artifact_id.chars().count() > MAX_ARTIFACT_ID_CHARS {
        return Err(Error::BadRequest(format!(
            "Artifact id must be between 1 and {MAX_ARTIFACT_ID_CHARS} characters"
        )));
    }
    if artifact_id.contains('\0') {
        return Err(Error::BadRequest(
            "Artifact id cannot contain NUL characters".to_string(),
        ));
    }
    Ok(())
}

/// Share an artifact, or move the caller's existing link for it to this content. Re-sharing
/// keeps the link and restarts its retention window.
async fn share_artifact(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(payload): Json<ShareArtifact>,
) -> JsonResult<SharedArtifactInfo> {
    let name = payload.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS {
        return Err(Error::BadRequest(format!(
            "Artifact name must be between 1 and {MAX_NAME_CHARS} characters"
        )));
    }
    check_artifact_id(&payload.artifact_id)?;
    if payload.content.len() > MAX_CONTENT_BYTES {
        return Err(Error::BadRequest(format!(
            "Artifact content is {} bytes, above the {MAX_CONTENT_BYTES} byte limit",
            payload.content.len()
        )));
    }
    if payload.version < 1 {
        return Err(Error::BadRequest(
            "Artifact version must be at least 1".to_string(),
        ));
    }
    // Postgres rejects NUL in text columns with an opaque 500.
    if name.contains('\0') || payload.content.contains('\0') {
        return Err(Error::BadRequest(
            "Artifact name and content cannot contain NUL characters".to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    let row = sqlx::query!(
        r#"INSERT INTO ai_shared_artifact
               (workspace_id, artifact_id, email, created_by, name, kind, version, content)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           ON CONFLICT (workspace_id, email, artifact_id) DO UPDATE
           SET created_by = EXCLUDED.created_by,
               name = EXCLUDED.name,
               kind = EXCLUDED.kind,
               version = EXCLUDED.version,
               content = EXCLUDED.content,
               shared_at = now()
           RETURNING id, shared_at, (xmax = 0) AS "inserted!""#,
        &w_id,
        &payload.artifact_id,
        &authed.email,
        &authed.username,
        name,
        payload.kind.as_str(),
        payload.version,
        &payload.content,
    )
    .fetch_one(&mut *tx)
    .await?;

    let id = row.id.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "ai.shared_artifacts.share",
        if row.inserted {
            ActionKind::Create
        } else {
            ActionKind::Update
        },
        &w_id,
        Some(&id),
        Some([("name", name)].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(SharedArtifactInfo {
        id: row.id,
        name: name.to_string(),
        kind: payload.kind.as_str().to_string(),
        version: payload.version,
        created_by: authed.username.clone(),
        shared_at: row.shared_at,
        expires_at: expires_at(row.shared_at, ai_shared_artifact_retention_secs()),
    }))
}

/// The caller's own live share of one of their artifacts, if any, and how long a share lasts.
async fn get_share_status(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<ShareStatusQuery>,
) -> JsonResult<ShareStatus> {
    check_artifact_id(&query.artifact_id)?;
    let retention_secs = ai_shared_artifact_retention_secs();
    let share = sqlx::query!(
        "SELECT id, name, kind, version, created_by, shared_at FROM ai_shared_artifact
         WHERE workspace_id = $1 AND email = $2 AND artifact_id = $3
           AND shared_at > now() - ($4::bigint::text || ' s')::interval",
        &w_id,
        &authed.email,
        &query.artifact_id,
        retention_secs,
    )
    .fetch_optional(&db)
    .await?
    .map(|r| SharedArtifactInfo {
        id: r.id,
        name: r.name,
        kind: r.kind,
        version: r.version,
        created_by: r.created_by,
        shared_at: r.shared_at,
        expires_at: expires_at(r.shared_at, retention_secs),
    });

    Ok(Json(ShareStatus { retention_secs, share }))
}

/// Any member of the workspace may read a live share: that is what sharing it granted.
async fn get_shared_artifact(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> JsonResult<SharedArtifact> {
    let retention_secs = ai_shared_artifact_retention_secs();
    let row = sqlx::query!(
        "SELECT id, email, name, kind, version, created_by, content, shared_at
         FROM ai_shared_artifact
         WHERE workspace_id = $1 AND id = $2
           AND shared_at > now() - ($3::bigint::text || ' s')::interval",
        &w_id,
        id,
        retention_secs,
    )
    .fetch_optional(&db)
    .await?
    .ok_or_else(|| {
        Error::NotFound(format!(
            "Shared artifact {id} not found: it may have expired or been unshared"
        ))
    })?;

    Ok(Json(SharedArtifact {
        can_unshare: row.email == authed.email || authed.is_admin,
        content: row.content,
        info: SharedArtifactInfo {
            id: row.id,
            name: row.name,
            kind: row.kind,
            version: row.version,
            created_by: row.created_by,
            shared_at: row.shared_at,
            expires_at: expires_at(row.shared_at, retention_secs),
        },
    }))
}

async fn unshare_artifact(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> Result<String> {
    let mut tx = db.begin().await?;
    let name = sqlx::query_scalar!(
        "DELETE FROM ai_shared_artifact
         WHERE workspace_id = $1 AND id = $2 AND (email = $3 OR $4::bool)
         RETURNING name",
        &w_id,
        id,
        &authed.email,
        authed.is_admin,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        Error::NotFound(format!(
            "Shared artifact {id} not found, or not shared by you"
        ))
    })?;

    let id_str = id.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "ai.shared_artifacts.unshare",
        ActionKind::Delete,
        &w_id,
        Some(&id_str),
        Some([("name", name.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Stopped sharing {name}"))
}
