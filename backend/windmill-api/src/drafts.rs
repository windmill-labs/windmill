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
use windmill_common::error::Result;

pub fn workspaced_service() -> Router {
    Router::new().route("/sync", post(sync_drafts)).route(
        "/users_with_draft/{kind}/{*path}",
        get(list_users_with_draft_on_path),
    )
}

#[derive(sqlx::Type, Serialize, Deserialize, Debug, PartialEq, Clone, Copy)]
#[sqlx(type_name = "DRAFT_TYPE", rename_all = "lowercase")]
#[serde(rename_all(serialize = "lowercase", deserialize = "lowercase"))]
pub enum DraftType {
    Script,
    Flow,
    App,
}

#[derive(Deserialize, Debug, Clone)]
pub struct IncomingDraft {
    pub path: String,
    pub typ: DraftType,
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
}

#[derive(Deserialize, Debug)]
pub struct SyncDraftsRequest {
    /// Server timestamp of the client's last successful sync. Used both to
    /// stream back drafts written by other sessions since then
    /// (`missed_drafts`) and to detect conflicts when the client tries to
    /// push a draft whose server copy moved forward (`status: rejected`).
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    pub drafts: Vec<IncomingDraft>,
    /// When true, skip the conflict check and always overwrite the server copy.
    #[serde(default)]
    pub force: bool,
}

#[derive(Serialize, Debug)]
pub struct MissedDraft {
    pub path: String,
    pub typ: DraftType,
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Serialize, Debug)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum DraftSyncStatus {
    Saved {
        path: String,
        typ: DraftType,
        created_at: chrono::DateTime<chrono::Utc>,
    },
    Rejected {
        path: String,
        typ: DraftType,
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
    let username = &authed.username;
    let current_timestamp = sqlx::query_scalar!("SELECT now()")
        .fetch_one(&db)
        .await?
        .expect("now() is never null");

    let missed_drafts = if let Some(last_sync) = req.last_sync {
        sqlx::query_as!(
            MissedDraft,
            r#"SELECT path, typ as "typ: DraftType", value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>", created_at
               FROM draft
               WHERE workspace_id = $1
                 AND username = $2
                 AND created_at > $3"#,
            &w_id,
            username,
            last_sync,
        )
        .fetch_all(&db)
        .await?
    } else {
        // Initial sync — return everything the user has on the server.
        sqlx::query_as!(
            MissedDraft,
            r#"SELECT path, typ as "typ: DraftType", value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>", created_at
               FROM draft
               WHERE workspace_id = $1
                 AND username = $2"#,
            &w_id,
            username,
        )
        .fetch_all(&db)
        .await?
    };

    let mut statuses = Vec::with_capacity(req.drafts.len());

    for incoming in &req.drafts {
        if !req.force {
            if let Some(last_sync) = req.last_sync {
                let conflict = sqlx::query!(
                    r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>", created_at
                       FROM draft
                       WHERE workspace_id = $1
                         AND username = $2
                         AND path = $3
                         AND typ = $4
                         AND created_at > $5"#,
                    &w_id,
                    username,
                    incoming.path,
                    incoming.typ as DraftType,
                    last_sync,
                )
                .fetch_optional(&db)
                .await?;

                if let Some(row) = conflict {
                    statuses.push(DraftSyncStatus::Rejected {
                        path: incoming.path.clone(),
                        typ: incoming.typ,
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
            r#"INSERT INTO draft (workspace_id, username, path, typ, value, created_at)
               VALUES ($1, $2, $3, $4, $5::text::json, now())
               ON CONFLICT (workspace_id, path, typ, username) WHERE username IS NOT NULL
               DO UPDATE SET value = EXCLUDED.value, created_at = now()
               RETURNING created_at"#,
            &w_id,
            username,
            incoming.path,
            incoming.typ as DraftType,
            serde_json::to_string(&incoming.value).unwrap(),
        )
        .fetch_one(&db)
        .await?;

        statuses.push(DraftSyncStatus::Saved {
            path: incoming.path.clone(),
            typ: incoming.typ,
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
    pub username: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

async fn list_users_with_draft_on_path(
    _authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, kind, path)): Path<(String, DraftType, windmill_common::utils::StripPath)>,
) -> Result<Json<Vec<UserWithDraft>>> {
    let rows = sqlx::query_as!(
        UserWithDraft,
        r#"SELECT username, created_at
           FROM draft
           WHERE workspace_id = $1
             AND path = $2
             AND typ = $3
           ORDER BY username NULLS LAST"#,
        &w_id,
        path.to_path(),
        kind as DraftType,
    )
    .fetch_all(&db)
    .await?;

    Ok(Json(rows))
}
