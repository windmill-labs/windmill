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
    user_drafts::UserDraftItemKind,
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route(
            "/users_with_draft/{kind}/{*path}",
            get(list_users_with_draft_on_path),
        )
        .route("/get/{kind}/{*path}", get(get_draft_for_user))
        .route("/save_draft/{kind}/{*path}", post(save_draft))
        .route("/list_drafts", get(list_drafts))
        .route("/get_draft/{kind}/{*path}", get(get_draft))
}

#[derive(Deserialize, Debug)]
pub struct SaveDraftRequest {
    /// Draft content to save. `null` (or omitted) signals a delete — the
    /// row is removed under the same conflict rules as an upsert.
    #[serde(default)]
    pub value: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    /// Server timestamp of the client's last known sync for this draft. When
    /// present and `force` is false, the save is rejected if the server's
    /// `created_at` is more recent (i.e. another writer moved the row
    /// forward since this client last saw it). Omit on a first save.
    #[serde(default)]
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    /// Skip the conflict check and unconditionally overwrite the server
    /// copy. Use after the client has resolved the conflict locally.
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum SaveDraftStatus {
    Saved,
    Conflict,
}

#[derive(Serialize, Debug)]
pub struct SaveDraftResponse {
    pub status: SaveDraftStatus,
    /// On `saved`: the timestamp at which the change was applied (the
    /// client should remember it as the next `last_sync`). On `conflict`:
    /// the existing row's `created_at`, so the client knows what the
    /// server has.
    pub current_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Apply the current user's draft at (workspace, kind, path). With a
/// non-null `value` this upserts; with `null` (or omitted) it deletes.
/// Either way, the same conflict rule applies: when the existing row is
/// newer than `last_sync` (and `force` is false), the operation is
/// skipped and the response carries `status = conflict` + the server's
/// current timestamp so the client can rebase.
async fn save_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    Json(req): Json<SaveDraftRequest>,
) -> Result<Json<SaveDraftResponse>> {
    let email = &authed.email;
    let path = path.to_path();

    let applied_at = if let Some(value) = &req.value {
        // Upsert branch. Conflict check rides on a WHERE clause attached
        // to DO UPDATE — when the existing row is newer than `last_sync`,
        // the statement is a no-op and RETURNING yields nothing.
        sqlx::query_scalar!(
            r#"INSERT INTO draft (workspace_id, email, path, typ, value, created_at)
               VALUES ($1, $2, $3, $4, $5::text::json, now())
               ON CONFLICT (workspace_id, path, typ, email) WHERE email IS NOT NULL
               DO UPDATE SET value = EXCLUDED.value, created_at = now()
               WHERE $7::bool = true
                  OR $6::timestamptz IS NULL
                  OR draft.created_at <= $6::timestamptz
               RETURNING created_at"#,
            &w_id,
            email,
            path,
            kind as UserDraftItemKind,
            serde_json::to_string(value).unwrap(),
            req.last_sync,
            req.force,
        )
        .fetch_optional(&db)
        .await?
    } else {
        // Delete branch. Same conflict rule lifted into the WHERE clause.
        // Returns NULL when the row was either too new (conflict) OR
        // already absent (idempotent delete) — disambiguated below.
        sqlx::query_scalar!(
            r#"DELETE FROM draft
               WHERE workspace_id = $1
                 AND email = $2
                 AND path = $3
                 AND typ = $4
                 AND ($6::bool = true
                      OR $5::timestamptz IS NULL
                      OR created_at <= $5::timestamptz)
               RETURNING now() as "now!""#,
            &w_id,
            email,
            path,
            kind as UserDraftItemKind,
            req.last_sync,
            req.force,
        )
        .fetch_optional(&db)
        .await?
    };

    if let Some(ts) = applied_at {
        return Ok(Json(SaveDraftResponse {
            status: SaveDraftStatus::Saved,
            current_timestamp: ts,
        }));
    }

    // No row affected. Either:
    //   - the existing row was newer than `last_sync` (conflict), or
    //   - this was a delete request and no row existed (idempotent ok).
    let existing = sqlx::query_scalar!(
        r#"SELECT created_at FROM draft
           WHERE workspace_id = $1 AND email = $2 AND path = $3 AND typ = $4"#,
        &w_id,
        email,
        path,
        kind as UserDraftItemKind,
    )
    .fetch_optional(&db)
    .await?;

    match existing {
        Some(ts) => Ok(Json(SaveDraftResponse {
            status: SaveDraftStatus::Conflict,
            current_timestamp: ts,
        })),
        // Delete + nothing-was-there ⇒ report success with server's NOW().
        None => {
            let now = sqlx::query_scalar!(r#"SELECT now() as "now!""#)
                .fetch_one(&db)
                .await?;
            Ok(Json(SaveDraftResponse {
                status: SaveDraftStatus::Saved,
                current_timestamp: now,
            }))
        }
    }
}

#[derive(Serialize, Debug)]
pub struct DraftListItem {
    pub path: String,
    pub typ: UserDraftItemKind,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

/// Metadata-only listing of the current user's drafts in a workspace.
/// Excludes the legacy NULL-email rows. Ordered most-recently-saved first.
async fn list_drafts(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<Json<Vec<DraftListItem>>> {
    let rows = sqlx::query_as!(
        DraftListItem,
        r#"SELECT path,
                  typ as "typ!: UserDraftItemKind",
                  created_at as "saved_at!"
           FROM draft
           WHERE workspace_id = $1 AND email = $2
           ORDER BY created_at DESC"#,
        &w_id,
        &authed.email,
    )
    .fetch_all(&db)
    .await?;
    Ok(Json(rows))
}

#[derive(Serialize, Debug)]
pub struct OwnDraft {
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

/// Fetch the current user's draft content at (kind, path). 404 if no draft.
async fn get_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
) -> Result<Json<OwnDraft>> {
    let path = path.to_path();
    let row = sqlx::query_as!(
        OwnDraft,
        r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>",
                  created_at as "saved_at!"
           FROM draft
           WHERE workspace_id = $1 AND email = $2 AND path = $3 AND typ = $4"#,
        &w_id,
        &authed.email,
        path,
        kind as UserDraftItemKind,
    )
    .fetch_optional(&db)
    .await?;
    row.map(Json)
        .ok_or_else(|| Error::NotFound(format!("no draft for current user at {path}")))
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
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
) -> Result<Json<Vec<UserWithDraft>>> {
    let path = path.to_path();
    require_can_read_path(&authed, &user_db, &w_id, kind, path).await?;

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
        kind as UserDraftItemKind,
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
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    axum::extract::Query(query): axum::extract::Query<GetDraftQuery>,
) -> Result<Json<DraftForUser>> {
    let path = path.to_path();
    require_can_read_path(&authed, &user_db, &w_id, kind, path).await?;

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
        kind as UserDraftItemKind,
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
fn table_for_kind(kind: UserDraftItemKind) -> Option<&'static str> {
    use UserDraftItemKind::*;
    match kind {
        Script => Some("script"),
        Flow => Some("flow"),
        App | RawApp => Some("app"),
        Resource => Some("resource"),
        Variable => Some("variable"),
        TriggerSchedule => Some("schedule"),
        TriggerHttp => Some("http_trigger"),
        TriggerWebsocket => Some("websocket_trigger"),
        TriggerPostgres => Some("postgres_trigger"),
        TriggerKafka => Some("kafka_trigger"),
        TriggerNats => Some("nats_trigger"),
        TriggerMqtt => Some("mqtt_trigger"),
        TriggerSqs => Some("sqs_trigger"),
        TriggerGcp => Some("gcp_trigger"),
        TriggerAzure => Some("azure_trigger"),
        TriggerEmail | TriggerDefaultEmail => Some("email_trigger"),
        TriggerPoll | TriggerCli | TriggerNextcloud | TriggerGoogle | TriggerGithub => {
            Some("native_trigger")
        }
        // trigger_webhook is a property of script/flow rows, not its own row.
        TriggerWebhook => None,
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
    kind: UserDraftItemKind,
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
