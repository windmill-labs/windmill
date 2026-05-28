/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
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
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
};

pub fn workspaced_service() -> Router {
    Router::new().route("/sync", post(sync_drafts)).route(
        "/users_with_draft/{kind}/{*path}",
        get(list_users_with_draft_on_path),
    )
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncomingDraft {
    pub path: String,
    /// Free-form item kind matching the frontend's `UserDraftItemKind`
    /// (`script`, `flow`, `app`, `raw_app`, `resource`, `variable`,
    /// `trigger_*`, ...). Stored verbatim — no server-side validation,
    /// since the set of kinds is owned by the client.
    pub typ: String,
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    /// When true, skip the conflict check for this entry and overwrite the
    /// server copy. Only the matching entry is forced — other entries in
    /// the same batch still run through the normal conflict check.
    #[serde(default)]
    pub force: bool,
}

#[derive(Deserialize, Debug)]
pub struct SyncDraftsRequest {
    /// Server timestamp of the client's last successful sync. Used both to
    /// stream back drafts written by other sessions since then
    /// (`missed_drafts`) and to detect conflicts when the client tries to
    /// push a draft whose server copy moved forward (`status: rejected`).
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub drafts: Vec<IncomingDraft>,
}

#[derive(Serialize, Debug)]
pub struct MissedDraft {
    pub path: String,
    pub typ: String,
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum DraftSyncStatus {
    Saved {
        path: String,
        typ: String,
        created_at: chrono::DateTime<chrono::Utc>,
    },
    Rejected {
        path: String,
        typ: String,
        /// Current server copy at conflict-detection time.
        server_value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
        server_created_at: chrono::DateTime<chrono::Utc>,
        /// The value the client tried to push. Echoed back so the client can
        /// retry with `force = true` without re-reading its local state.
        incoming_value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    },
}

#[derive(Serialize, Debug)]
pub struct SyncDraftsResponse {
    pub missed_drafts: Vec<MissedDraft>,
    pub statuses: Vec<DraftSyncStatus>,
    pub current_timestamp: chrono::DateTime<chrono::Utc>,
}

async fn sync_drafts(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
    Json(req): Json<SyncDraftsRequest>,
) -> Result<Json<SyncDraftsResponse>> {
    let email = &authed.email;
    let current_timestamp = sqlx::query_scalar!("SELECT now()")
        .fetch_one(&db)
        .await?
        .expect("now() is never null");

    let missed_drafts = if let Some(last_sync) = req.last_sync {
        sqlx::query_as!(
            MissedDraft,
            r#"SELECT path, typ, value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>", created_at
               FROM draft
               WHERE workspace_id = $1
                 AND email = $2
                 AND created_at > $3"#,
            &w_id,
            email,
            last_sync,
        )
        .fetch_all(&db)
        .await?
    } else {
        // Initial sync — return everything the user has on the server.
        sqlx::query_as!(
            MissedDraft,
            r#"SELECT path, typ, value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>", created_at
               FROM draft
               WHERE workspace_id = $1
                 AND email = $2"#,
            &w_id,
            email,
        )
        .fetch_all(&db)
        .await?
    };

    let mut statuses = Vec::with_capacity(req.drafts.len());

    for incoming in &req.drafts {
        if !incoming.force {
            if let Some(last_sync) = req.last_sync {
                let conflict = sqlx::query!(
                    r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>", created_at
                       FROM draft
                       WHERE workspace_id = $1
                         AND email = $2
                         AND path = $3
                         AND typ = $4
                         AND created_at > $5"#,
                    &w_id,
                    email,
                    incoming.path,
                    incoming.typ,
                    last_sync,
                )
                .fetch_optional(&db)
                .await?;

                if let Some(row) = conflict {
                    statuses.push(DraftSyncStatus::Rejected {
                        path: incoming.path.clone(),
                        typ: incoming.typ.clone(),
                        server_value: row.value,
                        server_created_at: row.created_at,
                        incoming_value: sqlx::types::Json(
                            serde_json::value::RawValue::from_string(
                                incoming.value.0.get().to_string(),
                            )
                            .expect("RawValue round-trip"),
                        ),
                    });
                    continue;
                }
            }
        }

        let row = sqlx::query!(
            r#"INSERT INTO draft (workspace_id, email, path, typ, value, created_at)
               VALUES ($1, $2, $3, $4, $5::text::json, now())
               ON CONFLICT (workspace_id, path, typ, email) WHERE email IS NOT NULL
               DO UPDATE SET value = EXCLUDED.value, created_at = now()
               RETURNING created_at"#,
            &w_id,
            email,
            incoming.path,
            incoming.typ,
            serde_json::to_string(&incoming.value).unwrap(),
        )
        .fetch_one(&db)
        .await?;

        statuses.push(DraftSyncStatus::Saved {
            path: incoming.path.clone(),
            typ: incoming.typ.clone(),
            created_at: row.created_at,
        });
    }

    Ok(Json(SyncDraftsResponse {
        missed_drafts,
        statuses,
        current_timestamp,
    }))
}

#[derive(Serialize, Debug)]
pub struct UserWithDraft {
    /// `None` represents a legacy workspace-level draft (no owner).
    pub email: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn list_users_with_draft_on_path(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, String, windmill_common::utils::StripPath)>,
) -> Result<Json<Vec<UserWithDraft>>> {
    let path = path.to_path();
    require_access_to_path(&authed, &user_db, &w_id, &kind, path).await?;

    let rows = sqlx::query_as!(
        UserWithDraft,
        r#"SELECT email, created_at
           FROM draft
           WHERE workspace_id = $1
             AND path = $2
             AND typ = $3
           ORDER BY email NULLS LAST"#,
        &w_id,
        path,
        kind,
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}

/// Resolves to `Ok(())` only if `authed` can read the underlying item at
/// `path`. Implemented by issuing a `SELECT 1` through `user_db` (which
/// applies row-level extra_perms via `set_session_user`), so a user who
/// can't see the script/flow/app gets back zero rows. We don't leak the
/// set of paths that exist either way — both "not readable" and "doesn't
/// exist" return the same 404.
async fn require_access_to_path(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    kind: &str,
    path: &str,
) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    }
    let table = match kind {
        "script" => "script",
        "flow" => "flow",
        "app" => "app",
        // Item kinds without a backing path-permission table (raw_app,
        // resource, variable, trigger_*, ...) fall back to the user's own
        // `u/{authed.username}/...` path namespace.
        _ => {
            let prefix = format!("u/{}/", authed.username);
            if path.starts_with(&prefix) {
                return Ok(());
            }
            return Err(Error::NotFound(format!("no draft visible at {path}")));
        }
    };
    let mut tx = user_db.clone().begin(authed).await?;
    let query =
        format!("SELECT 1 AS exists FROM {table} WHERE path = $1 AND workspace_id = $2 LIMIT 1");
    let row = sqlx::query_scalar::<_, i32>(&query)
        .bind(path)
        .bind(w_id)
        .fetch_optional(&mut *tx)
        .await?;
    tx.commit().await?;
    if row.is_none() {
        return Err(Error::NotFound(format!("no {kind} visible at {path}")));
    }
    Ok(())
}
