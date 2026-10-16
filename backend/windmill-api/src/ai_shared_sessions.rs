/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Read-only snapshots of AI session transcripts, shared by their author.
//!
//! Sessions live in the author's browser; a row here exists only because the author asked for
//! a link. A share is readable by the members of its workspace, or by anyone holding the link
//! once its author makes it public. The snapshot is whatever the author's browser sent, so the
//! viewer treats it as untrusted data.

use crate::db::{ApiAuthed, DB};
use axum::{
    extract::{DefaultBodyLimit, Extension, Json, Path, Query},
    routing::{delete, get, post},
    Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use uuid::Uuid;
use windmill_api_auth::{is_effectively_unscoped, OptAuthed};
use windmill_audit::{audit_oss::audit_log, ActionKind};
use windmill_common::error::{Error, JsonResult, Result};

const MAX_SNAPSHOT_BYTES: usize = 10 * 1024 * 1024;
const MAX_NAME_CHARS: usize = 255;
const MAX_SESSION_ID_CHARS: usize = 255;
const SHARE_BODY_LIMIT: usize = MAX_SNAPSHOT_BYTES + 64 * 1024;

pub fn workspaced_service() -> Router {
    Router::new()
        .route(
            "/share",
            post(share_session).layer(DefaultBodyLimit::max(SHARE_BODY_LIMIT)),
        )
        .route("/status", get(get_share_status))
        .route("/set_public/{id}", post(set_public))
        .route("/delete/{id}", delete(unshare_session))
}

/// Mounted outside the authenticated router: a public share is served to anyone.
pub fn workspaced_unauthed_service() -> Router {
    Router::new().route("/get/{id}", get(get_shared_session))
}

#[derive(Deserialize)]
struct ShareSession {
    session_id: String,
    name: String,
    /// Left as it is on an existing share when absent; a new share starts private.
    is_public: Option<bool>,
    message_count: i32,
    snapshot: Box<RawValue>,
}

#[derive(Serialize)]
struct SharedSessionInfo {
    id: Uuid,
    name: String,
    is_public: bool,
    message_count: i32,
    created_by: String,
    shared_at: DateTime<Utc>,
}

#[derive(Serialize)]
struct SharedSession {
    #[serde(flatten)]
    info: SharedSessionInfo,
    snapshot: Box<RawValue>,
    /// Whether the caller may stop sharing it: its author, or a workspace admin.
    can_unshare: bool,
}

#[derive(Serialize)]
struct ShareStatus {
    #[serde(skip_serializing_if = "Option::is_none")]
    share: Option<SharedSessionInfo>,
}

#[derive(Deserialize)]
struct ShareStatusQuery {
    session_id: String,
}

#[derive(Deserialize)]
struct SetPublic {
    is_public: bool,
}

/// Sharing exposes the author's own browser state, so only the author's own unscoped token may
/// do it: a job token can carry an `on_behalf_of` identity, and every scoped token (guest,
/// embed, app policy, MCP) was minted for something narrower.
fn require_plain_user_token(authed: &ApiAuthed) -> Result<()> {
    if authed.job_id.is_some() || !is_effectively_unscoped(authed.scopes.as_deref()) {
        return Err(Error::PermissionDenied(
            "Shared AI sessions are only reachable with an unscoped user token".to_string(),
        ));
    }
    Ok(())
}

/// It is compared against a `VARCHAR(255)` column, and Postgres answers a NUL in a text
/// parameter with an opaque 500.
fn check_session_id(session_id: &str) -> Result<()> {
    if session_id.is_empty() || session_id.chars().count() > MAX_SESSION_ID_CHARS {
        return Err(Error::BadRequest(format!(
            "Session id must be between 1 and {MAX_SESSION_ID_CHARS} characters"
        )));
    }
    if session_id.contains('\0') {
        return Err(Error::BadRequest(
            "Session id cannot contain NUL characters".to_string(),
        ));
    }
    Ok(())
}

/// Share a session, or move the caller's existing link for it to this snapshot.
async fn share_session(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(payload): Json<ShareSession>,
) -> JsonResult<SharedSessionInfo> {
    require_plain_user_token(&authed)?;
    check_session_id(&payload.session_id)?;
    let name = payload.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS || name.contains('\0') {
        return Err(Error::BadRequest(format!(
            "Session name must be between 1 and {MAX_NAME_CHARS} characters, without NUL"
        )));
    }
    let snapshot = payload.snapshot.get();
    if snapshot.len() > MAX_SNAPSHOT_BYTES {
        return Err(Error::BadRequest(format!(
            "Session snapshot is {} bytes, above the {MAX_SNAPSHOT_BYTES} byte limit",
            snapshot.len()
        )));
    }
    if !snapshot.trim_start().starts_with('{') {
        return Err(Error::BadRequest(
            "Session snapshot must be a JSON object".to_string(),
        ));
    }
    if payload.message_count < 0 {
        return Err(Error::BadRequest(
            "Message count cannot be negative".to_string(),
        ));
    }

    let mut tx = db.begin().await?;
    let row = sqlx::query!(
        r#"INSERT INTO ai_shared_session
               (workspace_id, session_id, email, created_by, name, is_public, message_count, snapshot)
           VALUES ($1, $2, $3, $4, $5, COALESCE($6, false), $7, $8)
           ON CONFLICT (workspace_id, email, session_id) DO UPDATE
           SET created_by = EXCLUDED.created_by,
               name = EXCLUDED.name,
               is_public = COALESCE($6, ai_shared_session.is_public),
               message_count = EXCLUDED.message_count,
               snapshot = EXCLUDED.snapshot,
               shared_at = now()
           RETURNING id, is_public, shared_at, (xmax = 0) AS "inserted!""#,
        &w_id,
        &payload.session_id,
        &authed.email,
        &authed.username,
        name,
        payload.is_public,
        payload.message_count,
        snapshot,
    )
    .fetch_one(&mut *tx)
    .await?;

    let id = row.id.to_string();
    let public = row.is_public.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "ai.shared_sessions.share",
        if row.inserted {
            ActionKind::Create
        } else {
            ActionKind::Update
        },
        &w_id,
        Some(&id),
        Some([("name", name), ("public", public.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(SharedSessionInfo {
        id: row.id,
        name: name.to_string(),
        is_public: row.is_public,
        message_count: payload.message_count,
        created_by: authed.username.clone(),
        shared_at: row.shared_at,
    }))
}

/// The caller's own share of one of their sessions, if any.
async fn get_share_status(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Query(query): Query<ShareStatusQuery>,
) -> JsonResult<ShareStatus> {
    require_plain_user_token(&authed)?;
    check_session_id(&query.session_id)?;
    let share = sqlx::query_as!(
        SharedSessionInfo,
        "SELECT id, name, is_public, message_count, created_by, shared_at FROM ai_shared_session
         WHERE workspace_id = $1 AND email = $2 AND session_id = $3",
        &w_id,
        &authed.email,
        &query.session_id,
    )
    .fetch_optional(&db)
    .await?;

    Ok(Json(ShareStatus { share }))
}

/// Only the author changes who can open their link; an admin may only take it down.
async fn set_public(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
    Json(payload): Json<SetPublic>,
) -> JsonResult<SharedSessionInfo> {
    require_plain_user_token(&authed)?;
    let mut tx = db.begin().await?;
    let info = sqlx::query_as!(
        SharedSessionInfo,
        "UPDATE ai_shared_session SET is_public = $4
         WHERE workspace_id = $1 AND id = $2 AND email = $3
         RETURNING id, name, is_public, message_count, created_by, shared_at",
        &w_id,
        id,
        &authed.email,
        payload.is_public,
    )
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| {
        Error::NotFound(format!(
            "Shared session {id} not found, or not shared by you"
        ))
    })?;

    let id_str = id.to_string();
    let public = info.is_public.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "ai.shared_sessions.set_public",
        ActionKind::Update,
        &w_id,
        Some(&id_str),
        Some([("name", info.name.as_str()), ("public", public.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(Json(info))
}

/// A public share is served to anyone with the link; any other one only to a member of its
/// workspace, which is what an `ApiAuthed` resolved on a `/w/{workspace_id}` path is.
async fn get_shared_session(
    OptAuthed(opt_authed): OptAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> JsonResult<SharedSession> {
    let row = sqlx::query!(
        "SELECT id, email, name, is_public, message_count, created_by, snapshot, shared_at
         FROM ai_shared_session
         WHERE workspace_id = $1 AND id = $2",
        &w_id,
        id,
    )
    .fetch_optional(&db)
    .await?;

    let not_found = || {
        Error::NotFound(format!(
            "Shared session {id} not found: it may have been unshared, or be visible only to members of {w_id}"
        ))
    };
    let member = opt_authed
        .as_ref()
        .filter(|a| a.job_id.is_none() && is_effectively_unscoped(a.scopes.as_deref()));
    let row = match row {
        Some(row) if row.is_public || member.is_some() => row,
        // A missing share and a private one answer a stranger alike, so the link does not
        // confirm that a private share exists.
        _ if member.is_none() => {
            return Err(Error::NotAuthorized(format!(
                "Shared session {id} is not public: sign in as a member of {w_id} to open it"
            )))
        }
        _ => return Err(not_found()),
    };

    Ok(Json(SharedSession {
        can_unshare: member.is_some_and(|a| row.email == a.email || a.is_admin),
        snapshot: RawValue::from_string(row.snapshot).map_err(|e| {
            Error::internal_err(format!("Stored session snapshot is not valid JSON: {e}"))
        })?,
        info: SharedSessionInfo {
            id: row.id,
            name: row.name,
            is_public: row.is_public,
            message_count: row.message_count,
            created_by: row.created_by,
            shared_at: row.shared_at,
        },
    }))
}

async fn unshare_session(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, id)): Path<(String, Uuid)>,
) -> Result<String> {
    require_plain_user_token(&authed)?;
    let mut tx = db.begin().await?;
    let name = sqlx::query_scalar!(
        "DELETE FROM ai_shared_session
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
            "Shared session {id} not found, or not shared by you"
        ))
    })?;

    let id_str = id.to_string();
    audit_log(
        &mut *tx,
        &authed,
        "ai.shared_sessions.unshare",
        ActionKind::Delete,
        &w_id,
        Some(&id_str),
        Some([("name", name.as_str())].into()),
    )
    .await?;
    tx.commit().await?;

    Ok(format!("Stopped sharing {name}"))
}
