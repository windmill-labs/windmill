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
    Router::new()
        .route("/sync", post(sync_drafts))
        .route(
            "/users_with_draft/{kind}/{*path}",
            get(list_users_with_draft_on_path),
        )
        .route("/get/{kind}/{*path}", get(get_draft_for_user))
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncomingDraft {
    pub path: String,
    /// Free-form item kind matching the frontend's `UserDraftItemKind`
    /// (`script`, `flow`, `app`, `raw_app`, `resource`, `variable`,
    /// `trigger_*`, ...). Stored verbatim — no server-side validation,
    /// since the set of kinds is owned by the client.
    pub typ: String,
    /// `null` (or omitted) means delete the draft at this path. Conflict
    /// semantics apply the same way to deletions as to upserts.
    #[serde(default)]
    pub value: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
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
    Deleted {
        path: String,
        typ: String,
    },
    Rejected {
        path: String,
        typ: String,
        /// Current server copy at conflict-detection time.
        server_value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
        server_created_at: chrono::DateTime<chrono::Utc>,
        /// The value the client tried to push. `None` when the client
        /// attempted a delete; the modal interprets this as "you tried to
        /// delete, but the server has a newer version".
        incoming_value: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
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
                        incoming_value: incoming.value.as_ref().map(|v| {
                            sqlx::types::Json(
                                serde_json::value::RawValue::from_string(v.0.get().to_string())
                                    .expect("RawValue round-trip"),
                            )
                        }),
                    });
                    continue;
                }
            }
        }

        match &incoming.value {
            Some(value) => {
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
                    serde_json::to_string(value).unwrap(),
                )
                .fetch_one(&db)
                .await?;

                statuses.push(DraftSyncStatus::Saved {
                    path: incoming.path.clone(),
                    typ: incoming.typ.clone(),
                    created_at: row.created_at,
                });
            }
            None => {
                // Delete-only path. Idempotent: the DELETE is a no-op if
                // the row was already gone (concurrent delete from another
                // tab) — we still report `Deleted` so the client clears
                // its pending state.
                sqlx::query!(
                    r#"DELETE FROM draft
                       WHERE workspace_id = $1
                         AND email = $2
                         AND path = $3
                         AND typ = $4"#,
                    &w_id,
                    email,
                    incoming.path,
                    incoming.typ,
                )
                .execute(&db)
                .await?;

                statuses.push(DraftSyncStatus::Deleted {
                    path: incoming.path.clone(),
                    typ: incoming.typ.clone(),
                });
            }
        }
    }

    // Compute after the inserts so the response's `current_timestamp` is
    // >= every just-saved row's `created_at`. Otherwise a client that
    // re-syncs immediately would see its own writes as newer than its
    // `last_sync` and get rejected on the next push.
    let current_timestamp = sqlx::query_scalar!("SELECT now()")
        .fetch_one(&db)
        .await?
        .expect("now() is never null");

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
    require_can_read_path(&authed, &user_db, &w_id, &kind, path).await?;

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

#[derive(Deserialize, Debug)]
pub struct GetDraftQuery {
    /// Owner of the draft to fetch. Omit to fetch the legacy
    /// workspace-level (NULL email) row, if any.
    pub email: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DraftForUser {
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Fetch a specific user's (or the legacy NULL row's) draft content at a
/// path. Used by the "other users' drafts" modal in editors after the list
/// endpoint has surfaced who has a draft. Same path-permission check as
/// the list endpoint.
async fn get_draft_for_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, String, windmill_common::utils::StripPath)>,
    axum::extract::Query(query): axum::extract::Query<GetDraftQuery>,
) -> Result<Json<DraftForUser>> {
    let path = path.to_path();
    require_can_read_path(&authed, &user_db, &w_id, &kind, path).await?;

    let row = sqlx::query_as!(
        DraftForUser,
        r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>", created_at
           FROM draft
           WHERE workspace_id = $1
             AND path = $2
             AND typ = $3
             AND email IS NOT DISTINCT FROM $4"#,
        &w_id,
        path,
        kind,
        query.email,
    )
    .fetch_optional(&db)
    .await?;

    row.map(Json).ok_or_else(|| {
        Error::NotFound(format!(
            "no draft for {} at {path}",
            query.email.as_deref().unwrap_or("<legacy>")
        ))
    })
}

/// Each `UserDraftItemKind` maps to either its own table (where RLS can
/// resolve item-level extra_perms grants that bypass folder/owner checks)
/// or `None` (kinds without a backing table fall through to the path-only
/// access check below).
fn table_for_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "script" => Some("script"),
        "flow" => Some("flow"),
        "app" | "raw_app" => Some("app"),
        "resource" => Some("resource"),
        "variable" => Some("variable"),
        "trigger_schedule" => Some("schedule"),
        "trigger_http" => Some("http_trigger"),
        "trigger_websocket" => Some("websocket_trigger"),
        "trigger_postgres" => Some("postgres_trigger"),
        "trigger_kafka" => Some("kafka_trigger"),
        "trigger_nats" => Some("nats_trigger"),
        "trigger_mqtt" => Some("mqtt_trigger"),
        "trigger_sqs" => Some("sqs_trigger"),
        "trigger_gcp" => Some("gcp_trigger"),
        "trigger_azure" => Some("azure_trigger"),
        "trigger_email" | "trigger_default_email" => Some("email_trigger"),
        "trigger_poll" | "trigger_cli" | "trigger_nextcloud" | "trigger_google"
        | "trigger_github" => Some("native_trigger"),
        // trigger_webhook is a property of script/flow rows, not its own row.
        _ => None,
    }
}

/// Resolves to `Ok(())` if `authed` can read at `path`. Three layers, in
/// order of cheapness:
///   1. admin → always.
///   2. Path-prefix match against the user's own namespace (`u/{username}`)
///      or any folder in `authed.folders` (which is the precomputed read
///      set used to seed UserDB's session context, so groups + direct
///      grants on the folder are already factored in).
///   3. RLS-aware `SELECT 1` against the kind's backing table — covers
///      item-level extra_perms grants that bypass folder/owner checks.
/// Both "not readable" and "doesn't exist" return a 404 — we don't leak
/// path existence to non-readers.
async fn require_can_read_path(
    authed: &ApiAuthed,
    user_db: &UserDB,
    w_id: &str,
    kind: &str,
    path: &str,
) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    }
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() >= 2 {
        match parts[0] {
            "u" if parts[1] == authed.username => return Ok(()),
            "f" => {
                let folder = parts[1];
                if authed.folders.iter().any(|(name, _, _)| name == folder) {
                    return Ok(());
                }
            }
            _ => {}
        }
    }
    if let Some(table) = table_for_kind(kind) {
        let mut tx = user_db.clone().begin(authed).await?;
        let query = format!("SELECT 1 FROM {table} WHERE path = $1 AND workspace_id = $2 LIMIT 1");
        let row = sqlx::query_scalar::<_, i32>(&query)
            .bind(path)
            .bind(w_id)
            .fetch_optional(&mut *tx)
            .await?;
        tx.commit().await?;
        if row.is_some() {
            return Ok(());
        }
    }
    Err(Error::NotFound(format!("no draft visible at {path}")))
}
