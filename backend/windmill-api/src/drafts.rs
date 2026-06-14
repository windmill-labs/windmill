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
        .route("/save_draft/{kind}/{*path}", post(save_draft))
}

#[derive(Serialize, sqlx::FromRow)]
pub struct DraftListItem {
    pub kind: UserDraftItemKind,
    pub path: String,
    /// Best-effort, read from the draft JSON's `summary` field when the
    /// editor shape carries one (scripts, flows, schedules, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// No deployed counterpart exists at this path — the draft is the
    /// whole item. Kinds without a per-path backing table (webhook,
    /// native triggers) report `true`.
    pub draft_only: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Every draft the authed user has in this workspace, across all kinds —
/// the single source for the "Review & deploy drafts" page and the
/// home-page draft-count banner. One query over the `draft` table; the
/// `draft_only` flag is computed per kind against the deployed table so
/// the frontend doesn't fan out a dozen list calls just to find drafts.
async fn list_drafts(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path(w_id): Path<String>,
) -> Result<Json<Vec<DraftListItem>>> {
    // Operators can't create drafts (see `require_can_write_path`) and
    // are excluded from every other draft surface.
    if authed.is_operator {
        return Ok(Json(vec![]));
    }
    // `draft_only` (no deployed counterpart at this path) is computed by a
    // per-kind existence check. Generate the CASE from
    // `UserDraftItemKind::deployed_table()` so the kind→table mapping has a
    // SINGLE source — drift between this and the access check is impossible
    // by construction. Table names come from the closed enum, never user
    // input, so the interpolation can't inject. Kinds with no path-keyed
    // table (`deployed_table() == None`: webhook, native triggers) get no
    // arm and fall to the `ELSE true` default.
    let rows = sqlx::query_as::<_, DraftListItem>(&list_drafts_query())
        .bind(&w_id)
        .bind(&authed.email)
        .fetch_all(&db)
        .await?;
    Ok(Json(rows))
}

/// Build the `list_drafts` SQL, generating the `draft_only` CASE from the
/// per-kind `deployed_table()` mapping. `$1` = workspace_id, `$2` = email.
fn list_drafts_query() -> String {
    let mut case = String::from("CASE d.typ::text\n");
    for kind in UserDraftItemKind::ALL {
        let Some(table) = kind.deployed_table() else {
            continue;
        };
        // `script` rows are soft-deleted — a deleted script is "not
        // deployed" for draft_only purposes. No other backing table has a
        // `deleted` flag.
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
    // `(d.email = $2 OR d.email IS NULL)` lists the user's own drafts AND
    // the legacy NULL-email workspace drafts (pre-per-user drafts +
    // `draft_only` migration rows). `DISTINCT ON (d.path, d.typ)` with
    // `d.email IS NULL` last collapses a (path, kind) that has both to the
    // owned row.
    format!(
        r#"SELECT DISTINCT ON (d.path, d.typ)
                  d.path,
                  d.typ AS kind,
                  d.created_at,
                  d.value ->> 'summary' AS summary,
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
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    Json(req): Json<SaveDraftRequest>,
) -> Result<Json<SaveDraftResponse>> {
    let email = &authed.email;
    let path = path.to_path();
    require_can_write_path(&authed, &db, &user_db, &w_id, kind, path).await?;

    let applied_at = if let Some(value) = &req.value {
        // Secret variable values must never sit in the draft table in
        // plaintext — deployed secrets are encrypted with the workspace
        // crypt key precisely so DB dumps don't leak them. Encrypt them
        // with the same key, marked with the `$encrypted:` prefix; the
        // variable deploy endpoints decrypt the marker back before
        // persisting for real.
        let serialized = if kind == UserDraftItemKind::Variable {
            encrypt_secret_variable_value(&db, &w_id, value.0.get()).await?
        } else {
            serde_json::to_string(value).unwrap()
        };
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
            serialized,
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

/// For variable-kind drafts: when the JSON says `variable.is_secret ==
/// true`, encrypt `variable.value` with the workspace crypt key and mark
/// it `$encrypted:<base64>` so the typed secret never persists in
/// plaintext at rest. Already-marked values (a restored draft echoed back
/// by autosave) pass through untouched. Unexpected shapes pass through
/// unchanged — the draft store is schema-less by design and a malformed
/// draft is the editor's problem, not a save error.
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
    /// Workspace username of the draft owner to fetch. Omit to fetch the
    /// legacy workspace-level (NULL email) row, if any. Emails are not
    /// part of the public draft API — the username is resolved to an
    /// email server-side.
    pub username: Option<String>,
}

#[derive(Serialize, Debug)]
pub struct DraftForUser {
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Fetch a specific user's (or the legacy NULL row's) draft content at a
/// path. Used by the "other users' drafts" banner in editors after the
/// list of other owners has been surfaced on the deployed-overlay
/// response. The caller identifies the owner by workspace username so
/// emails never reach the client.
async fn get_draft_for_user(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    axum::extract::Query(query): axum::extract::Query<GetDraftQuery>,
) -> Result<Json<DraftForUser>> {
    let path = path.to_path();
    // Cross-user draft viewing (View JSON / Fork) is disabled for the
    // drawer kinds (resource/variable/triggers): their drafts stay private
    // to their owner. For secret variables this is also what prevents a
    // viewer from reading another user's `$encrypted:` ciphertext and
    // laundering it into plaintext via deploy.
    if !kind.shares_drafts_across_users() {
        return Err(Error::NotFound(
            "drafts for this item kind are private to their owner".to_string(),
        ));
    }
    require_can_read_path(&authed, &user_db, &w_id, kind, path).await?;

    // Username -> email lookup, scoped to the workspace. None signals
    // "fetch the legacy NULL-email row" (kept distinct from a username
    // that simply has no draft, which falls through to 404 below).
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

/// The deployed table whose `(workspace_id, path)` row RLS resolves
/// item-level `extra_perms` grants against. Delegates to
/// `UserDraftItemKind::deployed_table()` — the SINGLE source shared with
/// the `draft_only` existence check — so the two can't drift. `None`
/// kinds (webhook, native triggers) fall through to the path-only access
/// check.
fn table_for_kind(kind: UserDraftItemKind) -> Option<&'static str> {
    kind.deployed_table()
}

/// Resolves to `Ok(())` if `authed` may SAVE a draft at `path`. Two
/// layers (operators are rejected outright):
///   1. Claim-based namespace rules — admin, own `u/`, member `g/`,
///      writable `f/` — which mirror what RLS evaluates from the same
///      JWT claims, and are the ENTIRE check for draft-only paths
///      (no deployed row exists for RLS to evaluate).
///   2. An RLS write-probe on the deployed row (`SELECT ... FOR UPDATE`
///      through UserDB) for everything the path string can't answer —
///      above all item-level extra_perms grants. The canonical policies
///      decide; no write rule is re-implemented here.
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
    // Operators are read-only users — they're excluded from every draft
    // surface (list synthesis, badges) and must not write drafts either.
    if authed.is_operator {
        return Err(Error::NotAuthorized(
            "operators cannot save drafts".to_string(),
        ));
    }
    // Cheap claim-based namespace checks first — these evaluate the SAME
    // JWT claims RLS reads from the session context (`session.user`,
    // `session.groups`, `session.folders_write`), so the outcome matches
    // the policies' `see_own` / `see_member` / folder-write lanes; doing
    // them as string checks just spares the autosave hot path (a POST
    // every debounce tick) a DB round-trip for the common cases. They are
    // also the ENTIRE check for draft-only paths, where no deployed row
    // exists for RLS to evaluate — without them, any workspace member
    // could plant drafts in another user's `u/` namespace, surfaced to
    // every reader of the path (home-page circles, others'-drafts modal).
    // `require_owner_of_path` is the shared helper (admin / `u/{own}` /
    // folder owner); group membership and the folder WRITE bit (with the
    // same JWT-claim refresh the deploy endpoints use for fresh grants)
    // are layered on top.
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
    // Defer to RLS for everything the path string can't answer — above
    // all item-level extra_perms grants (the Share dialog). Lock-probe
    // the deployed row through UserDB: Postgres applies UPDATE policies
    // to rows locked via `SELECT ... FOR UPDATE`, so a returned row
    // means the CANONICAL write policies would let this user UPDATE it —
    // nothing re-implemented here, no rule drift possible. The row lock
    // is released by the immediate commit. Draft-only paths have no row,
    // so the probe finds nothing and the namespace rules above were the
    // whole check.
    if let Some(table) = kind.deployed_table() {
        // `table` is from the closed `deployed_table()` enum, never user
        // input. LIMIT 1 keeps the probe to a single row lock — `script`
        // has one row per version at the same path, and locking the whole
        // version history would serialize against deploys for no gain.
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
