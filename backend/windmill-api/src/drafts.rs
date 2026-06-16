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
    user_drafts::{UserDraftItemKind, ENCRYPTED_DRAFT_PREFIX},
    variables::{build_crypt, encrypt},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_drafts))
        .route("/get/{kind}/{*path}", get(get_draft_for_user))
        .route("/get_own/{kind}/{*path}", get(get_own_draft))
        .route("/update/{kind}/{*path}", post(update_draft))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct DraftListItem {
    pub kind: UserDraftItemKind,
    pub path: String,
    /// Best-effort, read from the draft JSON's `summary` field when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// User-typed friendly path read from the draft JSON's `draft_path` (set by
    /// the editors when it differs from the storage path, e.g. a never-deployed
    /// item parked at `u/{user}/draft_{uuid}`). `None` when absent. Lets the
    /// review page show the friendly name instead of the storage path, like the
    /// home-page list endpoints.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_path: Option<String>,
    /// No deployed counterpart exists at this path — the draft is the whole
    /// item. Kinds without a per-path backing table report `true`.
    pub draft_only: bool,
    /// The listed row is a legacy workspace-level draft (`email IS NULL`),
    /// predating the per-user drafts migration. Only `true` when no per-user
    /// row exists at this (path, kind) — the DISTINCT ON prefers an owned row.
    pub legacy_draft: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Every draft the authed user has in this workspace, across all kinds — the
/// single source for the "Review & deploy drafts" page and the home-page
/// draft-count banner. One query over `draft`; `draft_only` is computed per
/// kind against the deployed table.
async fn list_drafts(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<Json<Vec<DraftListItem>>> {
    // Operators have no drafts of their own (they can't write any, see
    // `require_can_write_path`), so this list is always empty for them. They
    // can still READ some collaborators' drafts via `/drafts/get`.
    if authed.is_operator {
        return Ok(Json(vec![]));
    }
    let rows = sqlx::query_as::<_, DraftListItem>(&list_drafts_query())
        .bind(&w_id)
        .bind(&authed.email)
        .fetch_all(&db)
        .await?;
    Ok(Json(rows))
}

/// Build the `list_drafts` SQL, generating the `draft_only` CASE from
/// `deployed_table()` (shared single source — can't drift from the access
/// check). Table names come from the closed enum, never user input. Kinds
/// with no path-keyed table get no arm and fall to `ELSE true`.
/// `$1` = workspace_id, `$2` = email.
fn list_drafts_query() -> String {
    let mut case = String::from("CASE d.typ::text\n");
    for kind in UserDraftItemKind::ALL {
        let Some(table) = kind.deployed_table() else {
            continue;
        };
        // `script` rows are soft-deleted — a deleted script counts as "not
        // deployed". No other backing table has a `deleted` flag.
        let extra = if matches!(kind, UserDraftItemKind::Script) {
            " AND t.deleted = false"
        } else {
            ""
        };
        case.push_str(&format!(
            "  WHEN '{}' THEN NOT EXISTS(SELECT 1 FROM {} t WHERE t.workspace_id = d.workspace_id AND t.path = d.path{})\n",
            kind.as_str(),
            table,
            extra
        ));
    }
    case.push_str("  ELSE true\nEND");
    // `(d.email = $2 OR d.email IS NULL)` lists the user's own drafts AND the
    // legacy NULL-email rows; `DISTINCT ON (d.path, d.typ)` with `email IS NULL`
    // last collapses a (path, kind) that has both to the owned row.
    format!(
        r#"SELECT DISTINCT ON (d.path, d.typ)
                  d.path,
                  d.typ AS kind,
                  d.created_at,
                  d.value ->> 'summary' AS summary,
                  -- Friendly typed path, by kind (mirrors the home-page list
                  -- endpoints): scripts bind the Path widget to `script.path`,
                  -- so it round-trips through the draft JSON's own `path`;
                  -- flows/apps/raw-apps carry a separate `draft_path`. NULLIF
                  -- drops it when empty or equal to the storage path.
                  NULLIF(
                    NULLIF(
                      CASE WHEN d.typ::text = 'script'
                           THEN d.value ->> 'path'
                           ELSE d.value ->> 'draft_path' END,
                      ''),
                    d.path
                  ) AS draft_path,
                  (d.email IS NULL) AS legacy_draft,
                  {case} AS draft_only
           FROM draft d
           WHERE d.workspace_id = $1 AND (d.email = $2 OR d.email IS NULL)
           ORDER BY d.path, d.typ, (d.email IS NULL)"#
    )
}

#[derive(Deserialize, Debug)]
pub struct SaveDraftRequest {
    /// Draft content to save. `null` (or omitted) signals a delete — the
    /// row is removed under the same conflict rules as an upsert.
    #[serde(default)]
    pub value: Option<sqlx::types::Json<Box<serde_json::value::RawValue>>>,
    /// Client's last known sync timestamp. When present and `force` is false,
    /// the save is rejected if the server's `created_at` is more recent
    /// (another writer moved the row forward). Omit on a first save.
    #[serde(default)]
    pub last_sync: Option<chrono::DateTime<chrono::Utc>>,
    /// Skip the conflict check and unconditionally overwrite the server
    /// copy. Use after the client has resolved the conflict locally.
    #[serde(default)]
    pub force: bool,
    /// Delete-only: target the legacy workspace-level row (`email IS NULL`)
    /// rather than the authed user's row. An upsert ignores it (always writes
    /// the user's own row). Lets the review page discard a legacy draft, which
    /// the email-scoped delete otherwise can't reach.
    #[serde(default)]
    pub legacy: bool,
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
    /// On `saved`: when the change was applied (client remembers it as the
    /// next `last_sync`). On `conflict`: the existing row's `created_at`.
    pub current_timestamp: chrono::DateTime<chrono::Utc>,
}

/// Apply the current user's draft at (workspace, kind, path): non-null `value`
/// upserts, `null` (or omitted) deletes. Either way, when the existing row is
/// newer than `last_sync` (and `force` is false) the op is skipped and the
/// response is `status = conflict` + the server's current timestamp.
async fn update_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    Json(req): Json<SaveDraftRequest>,
) -> Result<Json<SaveDraftResponse>> {
    let email = &authed.email;
    let path = path.to_path();
    require_can_write_path(&authed, &db, &user_db, &w_id, kind, path).await?;

    let applied_at = if let Some(value) = &req.value {
        // Secret variable values must never sit in `draft.value` in plaintext
        // (see `encrypt_secret_variable_value`).
        let serialized = if kind == UserDraftItemKind::Variable {
            encrypt_secret_variable_value(&db, &w_id, value.0.get()).await?
        } else {
            serde_json::to_string(value).unwrap()
        };
        // Upsert. The conflict check rides on the DO UPDATE WHERE clause —
        // when the row is newer than `last_sync`, RETURNING yields nothing.
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
            serialized,
            req.last_sync,
            req.force,
        )
        .fetch_optional(&db)
        .await?
    } else {
        // Delete, same conflict rule in the WHERE clause. Returns NULL when
        // the row was too new (conflict) OR already absent (idempotent) —
        // disambiguated below. `legacy` ($7) retargets to the NULL-email row.
        sqlx::query_scalar!(
            r#"DELETE FROM draft
               WHERE workspace_id = $1
                 AND email IS NOT DISTINCT FROM (CASE WHEN $7::bool THEN NULL::text ELSE $2 END)
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
            req.legacy,
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

    // No row affected: either the row was newer than `last_sync` (conflict),
    // or it was a delete with no row present (idempotent ok). Distinguished
    // by re-reading.
    let existing = sqlx::query_scalar!(
        r#"SELECT created_at FROM draft
           WHERE workspace_id = $1
             AND email IS NOT DISTINCT FROM (CASE WHEN $5::bool THEN NULL::text ELSE $2 END)
             AND path = $3 AND typ = $4"#,
        &w_id,
        email,
        path,
        kind as UserDraftItemKind,
        req.legacy,
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

/// For variable-kind drafts with `variable.is_secret == true`, encrypt
/// `variable.value` with the workspace crypt key and mark it
/// `$encrypted:<base64>` so the secret never persists in plaintext at rest.
/// Already-marked values pass through untouched. Unexpected/malformed shapes
/// pass through unchanged — the draft store is schema-less by design.
async fn encrypt_secret_variable_value(db: &DB, w_id: &str, raw: &str) -> Result<String> {
    let Ok(mut v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Ok(raw.to_string());
    };
    let is_secret = v
        .get("variable")
        .and_then(|x| x.get("is_secret"))
        .and_then(|x| x.as_bool())
        .unwrap_or(false);
    if is_secret {
        if let Some(serde_json::Value::String(s)) =
            v.get_mut("variable").and_then(|x| x.get_mut("value"))
        {
            if !s.is_empty() && !s.starts_with(ENCRYPTED_DRAFT_PREFIX) {
                let mc = build_crypt(db, w_id).await?;
                *s = format!("{ENCRYPTED_DRAFT_PREFIX}{}", encrypt(&mc, s));
            }
        }
    }
    Ok(v.to_string())
}

#[derive(Deserialize, Debug)]
pub struct GetDraftQuery {
    /// Workspace username of the draft owner. Omit to fetch the legacy
    /// NULL-email row, if any. Resolved to an email server-side — emails are
    /// not part of the public draft API.
    pub username: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DraftForUser {
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Fetch a specific user's (or the legacy NULL row's) draft content at a path.
/// Backs the "other users' drafts" banner in editors. The owner is identified
/// by workspace username so emails never reach the client.
async fn get_draft_for_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    axum::extract::Query(query): axum::extract::Query<GetDraftQuery>,
) -> Result<Json<DraftForUser>> {
    let path = path.to_path();
    // Drawer kinds keep drafts private to their owner (see
    // `shares_drafts_across_users`) — also what blocks reading another user's
    // secret-variable `$encrypted:` ciphertext.
    if !kind.shares_drafts_across_users() {
        return Err(Error::NotFound(
            "drafts for this item kind are private to their owner".to_string(),
        ));
    }
    require_can_read_path(&authed, &user_db, &w_id, kind, path).await?;

    // Username -> email, scoped to the workspace. None signals "fetch the
    // legacy NULL-email row" (distinct from a username with no draft, which
    // 404s below).
    let owner_email: Option<String> = if let Some(username) = &query.username {
        let email = sqlx::query_scalar!(
            r#"SELECT email FROM usr WHERE workspace_id = $1 AND username = $2"#,
            &w_id,
            username,
        )
        .fetch_optional(&db)
        .await?;
        match email {
            Some(e) => Some(e),
            // The `admins` workspace has no `usr` rows (username IS the email
            // there), so accept it as the owner email directly.
            None if w_id == "admins" => Some(username.clone()),
            None => {
                return Err(Error::NotFound(format!(
                    "no user with username {username} in workspace"
                )))
            }
        }
    } else {
        None
    };

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
        owner_email,
    )
    .fetch_optional(&db)
    .await?;

    row.map(Json).ok_or_else(|| {
        Error::NotFound(format!(
            "no draft for {} at {path}",
            query.username.as_deref().unwrap_or("<legacy>")
        ))
    })
}

/// Fetch the AUTHED user's OWN draft at a path, for any kind — including
/// private kinds (`shares_drafts_across_users() == false`). Backs editors with
/// no deployed-item GET to overlay a draft onto: the `data_pipeline` bundle is
/// keyed at a folder path with no runnable to hang `get_draft` on, so it loads
/// its in-flight state from here. Returns `null` (200) when the user has no
/// draft there, so a fresh pipeline isn't a 404. Secret-variable values come
/// back `$encrypted:`-prefixed, same as `get_draft_for_user` — variable editors
/// use their own overlay GET, not this route.
async fn get_own_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
) -> Result<Json<Option<DraftForUser>>> {
    let path = path.to_path();
    require_can_read_path(&authed, &user_db, &w_id, kind, path).await?;
    let row = sqlx::query_as!(
        DraftForUser,
        r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>", created_at
           FROM draft
           WHERE workspace_id = $1 AND path = $2 AND typ = $3 AND email = $4"#,
        &w_id,
        path,
        kind as UserDraftItemKind,
        &authed.email,
    )
    .fetch_optional(&db)
    .await?;
    Ok(Json(row))
}

/// The deployed table RLS resolves item-level `extra_perms` against.
/// Delegates to `UserDraftItemKind::deployed_table()` (the shared single
/// source); `None` kinds fall through to the path-only access check.
fn table_for_kind(kind: UserDraftItemKind) -> Option<&'static str> {
    kind.deployed_table()
}

/// Resolves to `Ok(())` if `authed` may SAVE a draft at `path`. Operators are
/// rejected outright. Two layers:
///   1. Claim-based namespace rules (admin, own `u/`, member `g/`, writable
///      `f/`) — mirror what RLS reads from the same JWT claims, and are the
///      ENTIRE check for draft-only paths (no deployed row for RLS to use).
///   2. An RLS write-probe on the deployed row (`SELECT ... FOR UPDATE`) for
///      what the path can't answer, above all item-level extra_perms grants.
async fn require_can_write_path(
    authed: &ApiAuthed,
    db: &DB,
    user_db: &UserDB,
    w_id: &str,
    kind: UserDraftItemKind,
    path: &str,
) -> Result<()> {
    if authed.is_admin {
        return Ok(());
    }
    // Operators are read-only and never WRITE drafts. Read access is
    // deliberately asymmetric: `require_can_read_path` has no operator block,
    // so an operator can still READ a draft they can read via `/drafts/get`,
    // mirroring their read access to deployed content. Intended.
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "operators cannot save drafts".to_string(),
        ));
    }
    // Cheap claim-based namespace checks first: they evaluate the same JWT
    // claims RLS reads, so the outcome matches the policies while sparing the
    // autosave hot path a DB round-trip. They are also the ENTIRE check for
    // draft-only paths (no deployed row for RLS) — without them any member
    // could plant a draft in another user's `u/` namespace, surfaced to every
    // reader of the path. `require_owner_of_path` covers admin / `u/{own}` /
    // folder owner; group membership and the folder WRITE bit are layered on.
    if windmill_api_auth::require_owner_of_path(authed, path).is_ok() {
        return Ok(());
    }
    let parts: Vec<&str> = path.splitn(3, '/').collect();
    if parts.len() >= 3 {
        match parts[0] {
            "g" if authed.groups.iter().any(|g| g == parts[1]) => return Ok(()),
            "f" => {
                let folder = parts[1];
                let has_write = |a: &ApiAuthed| {
                    a.folders
                        .iter()
                        .any(|(name, write, owner)| name == folder && (*write || *owner))
                };
                if has_write(authed) {
                    return Ok(());
                }
                let refreshed =
                    windmill_api_auth::maybe_refresh_folders(path, w_id, authed.clone(), db).await;
                if has_write(&refreshed) {
                    return Ok(());
                }
            }
            _ => {}
        }
    }
    // Defer to RLS for what the path can't answer (item-level extra_perms
    // grants). Postgres applies UPDATE policies to rows locked via `SELECT
    // ... FOR UPDATE`, so a returned row means the canonical write policies
    // would let this user UPDATE it — no rule re-implemented here. Draft-only
    // paths have no row, so the namespace rules above were the whole check.
    if let Some(table) = kind.deployed_table() {
        // `table` is from the closed enum, never user input. LIMIT 1 keeps the
        // probe to one row lock — `script` has a row per version at the path,
        // and locking the whole history would serialize against deploys.
        let query = format!(
            "SELECT 1 FROM {table} WHERE path = $1 AND workspace_id = $2 LIMIT 1 FOR UPDATE"
        );
        let mut tx = user_db.clone().begin(authed).await?;
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
    Err(Error::NotAuthorized(format!(
        "you don't have write permission on {path}"
    )))
}

/// Resolves to `Ok(())` if `authed` can read at `path`. Three layers:
///   1. admin → always.
///   2. Path-prefix match against own `u/{username}` or any folder in
///      `authed.folders` (the precomputed read set, with groups + direct
///      grants already factored in).
///   3. RLS-aware `SELECT 1` against the backing table — covers item-level
///      extra_perms grants that bypass folder/owner checks.
/// Both "not readable" and "doesn't exist" return 404 — don't leak existence.
///
/// Operators are deliberately NOT rejected here (unlike
/// `require_can_write_path`): read-only users keep their read access to
/// deployed content, so an operator can view a collaborator's draft for the
/// cross-user kinds they can already read, while never writing one. Drawer
/// kinds never reach this (`get_draft_for_user` rejects them up front).
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
