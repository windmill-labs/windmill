/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use crate::db::{ApiAuthed, DB};

use axum::{
    extract::{Extension, Path, Query},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use windmill_common::{
    db::UserDB,
    error::{Error, Result},
    user_drafts::{DraftUserRef, UserDraftItemKind, ENCRYPTED_DRAFT_PREFIX},
    users::resolve_username_to_email,
    variables::{build_crypt, encrypt},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_drafts))
        .route("/get/{kind}/{*path}", get(get_draft_for_user))
        .route("/get_own/{kind}/{*path}", get(get_own_draft))
        .route("/update/{kind}/{*path}", post(update_draft))
        .route("/migrate_legacy/{kind}/{*path}", post(migrate_legacy_draft))
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
    /// All draft authors at this `(path, kind)`, for the shared full-page-editor
    /// kinds (script/flow/app/raw_app) only — feeds the home-page-style owner
    /// circles on the review page. `None` for drawer kinds, which keep their
    /// drafts private.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_users: Option<sqlx::types::Json<Vec<DraftUserRef>>>,
    /// Whether the authed user may deploy/discard this draft — the same check
    /// the deploy/discard endpoints enforce. Computed per row after the query,
    /// so it defaults to `false` when read from the row.
    #[sqlx(default)]
    pub can_write: bool,
    /// The listed row belongs to the authed user (own draft or the legacy
    /// no-owner row) and is therefore actionable by them. Always `true` in the
    /// default (own-drafts) listing; only meaningful with `all_users=true`,
    /// where other users' rows surface as `false` (view-only — you can't deploy
    /// someone else's draft).
    pub mine: bool,
}

#[derive(Deserialize)]
pub struct ListDraftsQuery {
    /// List every draft in the workspace (all users), not just the authed
    /// user's own + legacy rows. Other users' rows come back with `mine=false`.
    pub all_users: Option<bool>,
}

/// Every draft the authed user has in this workspace, across all kinds — the
/// single source for the "Review & deploy drafts" page and the home-page
/// draft-count banner. One query over `draft`; `draft_only` is computed per
/// kind against the deployed table.
async fn list_drafts(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path(w_id): Path<String>,
    Query(query): Query<ListDraftsQuery>,
) -> Result<Json<Vec<DraftListItem>>> {
    // Operators have no drafts of their own (they can't write any, see
    // `require_can_write_path`), so this list is always empty for them. They
    // can still READ some collaborators' drafts via `/drafts/get`.
    if authed.is_operator {
        return Ok(Json(vec![]));
    }
    let all_users = query.all_users.unwrap_or(false);
    let rows = sqlx::query_as::<_, DraftListItem>(&list_drafts_query(all_users))
        .bind(&w_id)
        .bind(&authed.email)
        .fetch_all(&db)
        .await?;
    // Per-row permission gating:
    //  - own drafts (incl. legacy no-owner rows, `mine = true`): the actionable
    //    gate is write permission — run the exact check deploy/discard enforce so
    //    the UI never offers an action that would 403.
    //  - other users' drafts (only present with `all_users`, `mine = false`): the
    //    UI never lets you act on them (`isSelectable` requires `mine`), so skip
    //    the write probe (`can_write = false`) and instead require READ access —
    //    otherwise the broadened listing would disclose the path/summary/authors
    //    of items the caller can't see. Unreadable rows are dropped, mirroring the
    //    `require_can_read_path` gate on `/drafts/get`.
    let mut out = Vec::with_capacity(rows.len());
    for mut row in rows {
        if row.mine {
            row.can_write =
                match require_can_write_path(&authed, &db, &user_db, &w_id, row.kind, &row.path)
                    .await
                {
                    Ok(()) => true,
                    // A stored draft can sit at an unwritable path — unauthorized,
                    // or malformed (`BadRequest`; the `draft` table has no path
                    // constraint). Either way it's not writable, and one bad row
                    // must not 400 the whole listing.
                    Err(Error::NotAuthorized(_)) | Err(Error::BadRequest(_)) => false,
                    Err(e) => return Err(e),
                };
            out.push(row);
        } else {
            // `require_can_read_path` denies with `NotFound` (it hides existence)
            // and, for some paths, `NotAuthorized` — both mean "not visible to the
            // caller", so drop the row. Any other error is a real failure.
            match require_can_read_path(&authed, &user_db, &w_id, row.kind, &row.path).await {
                Ok(()) => {
                    row.can_write = false;
                    out.push(row);
                }
                Err(Error::NotFound(_)) | Err(Error::NotAuthorized(_)) => {}
                Err(e) => return Err(e),
            }
        }
    }
    Ok(Json(out))
}

/// Build the `list_drafts` SQL, generating the `draft_only` CASE from
/// `deployed_table()` (shared single source — can't drift from the access
/// check). Table names come from the closed enum, never user input. Kinds
/// with no path-keyed table get no arm and fall to `ELSE true`.
/// `$1` = workspace_id, `$2` = email. With `all_users` the owner filter is
/// dropped so every workspace draft is listed (others' rows get `mine=false`).
fn list_drafts_query(all_users: bool) -> String {
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
    // Owner circles, mirroring the home-page list subquery (see apps.rs): every
    // draft author at this (path, kind), legacy NULL-email row surfaced as a
    // null username. Restricted to the shared full-page-editor kinds — drawer
    // kinds keep their drafts private, so we never reveal their authors.
    // A superadmin authoring in a workspace they are not a member of has no `usr`
    // row: fall back to their instance-derived username (`password.username`), or
    // their email when derivation is disabled (`password.username` is NULL). This
    // keeps the raw email out of the payload whenever a derived username exists.
    let draft_users = r#"CASE WHEN d.typ::text IN ('script', 'flow', 'app', 'raw_app') THEN (
                      SELECT json_agg(json_build_object('username', COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN du.email END))
                                      ORDER BY COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN du.email END) NULLS LAST)
                      FROM draft du
                      LEFT JOIN usr u ON u.workspace_id = du.workspace_id AND u.email = du.email
                      LEFT JOIN password p ON p.email = du.email AND p.super_admin = true
                      WHERE du.workspace_id = d.workspace_id AND du.path = d.path AND du.typ = d.typ
                    ) ELSE NULL END"#;
    // Default lists the user's own drafts AND the legacy NULL-email rows; with
    // `all_users` the filter is dropped to list every workspace draft.
    let owner_filter = if all_users {
        ""
    } else {
        " AND (d.email = $2 OR d.email IS NULL)"
    };
    // `DISTINCT ON (d.path, d.typ)` keeps one row per item; the ORDER BY
    // priority below picks the user's own row first, then the legacy NULL row,
    // then (only with `all_users`) another user's row. `mine`/`legacy_draft`
    // describe that kept row.
    format!(
        r#"SELECT DISTINCT ON (d.path, d.typ)
                  d.path,
                  d.typ AS kind,
                  d.created_at,
                  d.value ->> 'summary' AS summary,
                  {draft_users} AS draft_users,
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
                  (d.email = $2 OR d.email IS NULL) AS mine,
                  {case} AS draft_only
           FROM draft d
           WHERE d.workspace_id = $1{owner_filter}
           ORDER BY d.path, d.typ,
                    CASE WHEN d.email = $2 THEN 0 WHEN d.email IS NULL THEN 1 ELSE 2 END"#
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
    /// Upsert-only override for the stored `created_at`. Normal saves omit it
    /// and the row is stamped `now()`; the localStorage→DB migration passes the
    /// draft's original write time (or epoch 0 when unknown) so migrated drafts
    /// keep their age instead of all resurfacing to the top as freshly created.
    #[serde(default)]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
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
    // Saving a draft requires write permission on the underlying path. Deleting
    // (discarding) one's OWN draft does not: the email-scoped row belongs to the
    // authed user, so they can always discard it even after losing write access
    // to the underlying item (e.g. a draft-only item whose folder perms changed).
    // The DELETE below is scoped to `email = authed.email`, so it can only ever
    // touch the caller's own row. Legacy (NULL-email) rows aren't owned by anyone
    // — they keep the write gate.
    let is_own_discard = req.value.is_none() && !req.legacy;
    if !is_own_discard {
        require_can_write_path(&authed, &db, &user_db, &w_id, kind, path).await?;
    }

    let applied_at = if let Some(value) = &req.value {
        // Secret variable values must never sit in `draft.value` in plaintext
        // (see `encrypt_secret_variable_value`).
        let serialized = if kind == UserDraftItemKind::Variable {
            encrypt_secret_variable_value(&db, &w_id, value.0.get()).await?
        } else {
            serde_json::to_string(value).unwrap()
        };
        // `draft.value` is a `json` column, so a U+0000 (NUL) would persist as an
        // escape and later make any `->>`/`to_jsonb` extraction raise `22P05`.
        // Strip it here so a NUL never reaches the column.
        let serialized = strip_json_nul(serialized);
        // Upsert. The conflict check rides on the DO UPDATE WHERE clause —
        // when the row is newer than `last_sync`, RETURNING yields nothing.
        // `created_at` defaults to `now()` but the migration overrides it ($8)
        // so a migrated draft keeps its original age instead of jumping to top.
        sqlx::query_scalar!(
            r#"INSERT INTO draft (workspace_id, email, path, typ, value, created_at)
               VALUES ($1, $2, $3, $4, $5::text::json, COALESCE($8::timestamptz, now()))
               ON CONFLICT (workspace_id, path, typ, email) WHERE email IS NOT NULL
               DO UPDATE SET value = EXCLUDED.value, created_at = EXCLUDED.created_at
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
            req.created_at,
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum MigrateLegacyDraftAction {
    /// Discard the legacy row entirely.
    Delete,
    /// Move the legacy row's content onto the authed admin's own row, then
    /// drop the legacy row — so it becomes a normal per-user draft.
    AssignToSelf,
}

#[derive(Deserialize, Debug)]
pub struct MigrateLegacyDraftRequest {
    pub action: MigrateLegacyDraftAction,
}

/// Resolve a LEGACY (workspace-level, `email IS NULL`) draft. These predate the
/// per-user drafts migration and have no owner, so only workspace admins (and
/// superadmins, which carry `is_admin` in a workspace) may delete one or claim
/// it as their own.
async fn migrate_legacy_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    Json(req): Json<MigrateLegacyDraftRequest>,
) -> Result<String> {
    if !authed.is_admin {
        return Err(Error::NotAuthorized(
            "only workspace admins can migrate legacy drafts".to_string(),
        ));
    }
    let path = path.to_path();
    match req.action {
        MigrateLegacyDraftAction::Delete => {
            sqlx::query!(
                r#"DELETE FROM draft
                   WHERE workspace_id = $1 AND path = $2 AND typ = $3 AND email IS NULL"#,
                &w_id,
                path,
                kind as UserDraftItemKind,
            )
            .execute(&db)
            .await?;
            Ok(format!("Deleted legacy draft at {path}"))
        }
        MigrateLegacyDraftAction::AssignToSelf => {
            // Take ownership: move the legacy value onto the admin's own row
            // (replacing any existing own draft) and drop the legacy row, in one
            // statement. `ON CONFLICT` matches the partial unique index that
            // covers `email IS NOT NULL`.
            let moved = sqlx::query_scalar!(
                r#"WITH legacy AS (
                       DELETE FROM draft
                       WHERE workspace_id = $1 AND path = $2 AND typ = $3 AND email IS NULL
                       RETURNING value
                   )
                   INSERT INTO draft (workspace_id, email, path, typ, value, created_at)
                   SELECT $1, $4, $2, $3, value, now() FROM legacy
                   ON CONFLICT (workspace_id, path, typ, email) WHERE email IS NOT NULL
                   DO UPDATE SET value = EXCLUDED.value, created_at = now()
                   RETURNING 1 as "one!""#,
                &w_id,
                path,
                kind as UserDraftItemKind,
                &authed.email,
            )
            .fetch_optional(&db)
            .await?;
            if moved.is_none() {
                return Err(Error::NotFound(format!("no legacy draft at {path}")));
            }
            Ok(format!("Assigned legacy draft at {path} to you"))
        }
    }
}

/// Remove every U+0000 (NUL) from a serialized JSON document so it is safe to
/// store in the `json`-typed `draft.value` (a NUL there would later make any
/// `->>`/`to_jsonb` extraction raise `22P05`).
///
/// A NUL can only appear in JSON text as a backslash-u0000 escape, and a
/// backslash only ever occurs inside a string, so one backslash-parity-aware
/// pass removes every real NUL escape — covering values and keys alike — while
/// leaving a legitimate `\\u0000` (an escaped backslash followed by the literal
/// text `u0000`) intact. O(n) over the bytes with no `serde_json::Value` tree to
/// allocate, and the fast path (no such substring at all) returns the input
/// untouched. The slow path is reached not only by genuinely poisoned values but
/// by any value that legitimately contains `u0000` after a backslash (e.g. script
/// source), so it must stay allocation-light for potentially large drafts.
fn strip_json_nul(serialized: String) -> String {
    if !serialized.contains("\\u0000") {
        return serialized;
    }
    let bytes = serialized.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'\\' {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        // Consume the whole run of backslashes. An even run is N/2 escaped
        // backslashes and leaves the next char unescaped; an odd run ends in an
        // escaping backslash, so a following `u0000` is a real NUL escape.
        let run_start = i;
        while i < bytes.len() && bytes[i] == b'\\' {
            i += 1;
        }
        let run = i - run_start;
        if run % 2 == 1 && bytes[i..].starts_with(b"u0000") {
            // Drop the escaping backslash + `u0000`; keep the leading literal pairs.
            out.extend(std::iter::repeat(b'\\').take(run - 1));
            i += 5;
        } else {
            out.extend(std::iter::repeat(b'\\').take(run));
        }
    }
    // Only whole ASCII backslash-u0000 escapes were removed, so the bytes remain
    // valid UTF-8 (and valid JSON).
    String::from_utf8(out).expect("removing a NUL escape preserves valid UTF-8")
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
    // 404s below). Resolution falls back to the instance `password` table so a
    // superadmin's draft (they are not a `usr` member of the workspace, and
    // their username is their instance-derived one) still resolves.
    let owner_email: Option<String> = if let Some(username) = &query.username {
        match resolve_username_to_email(&w_id, username, &db).await? {
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
    // A path without a recognized namespace prefix (u/, f/, g/) can never be
    // writable — no namespace rule and no deployed row can apply — so report it
    // as malformed rather than as a plain permission denial.
    if !(path.starts_with("u/") || path.starts_with("f/") || path.starts_with("g/")) {
        return Err(Error::BadRequest(format!(
            "Invalid path '{path}': a valid path starts with 'u/<user>/', 'f/<folder>/' or 'g/<group>/'"
        )));
    }
    Err(Error::NotAuthorized(format!(
        "You don't have write permission on '{path}'. It must be in your own 'u/{}/' namespace, or in a folder ('f/<folder>/') or group ('g/<group>/') you can write to.",
        authed.username
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

#[cfg(test)]
mod tests {
    use super::strip_json_nul;

    // Parse the (NUL-free) result so assertions read clearly.
    fn parsed(s: String) -> serde_json::Value {
        serde_json::from_str(&s).expect("strip_json_nul must return valid JSON")
    }

    #[test]
    fn clean_value_is_returned_byte_for_byte() {
        let s = r#"{"summary":"all good","n":1}"#.to_string();
        assert_eq!(strip_json_nul(s.clone()), s);
    }

    #[test]
    fn real_nul_in_value_is_stripped() {
        let out = strip_json_nul(r#"{"summary":"hi\u0000there"}"#.to_string());
        assert!(!out.contains(r"\u0000"));
        assert_eq!(parsed(out)["summary"], "hithere");
    }

    #[test]
    fn legit_escaped_backslash_is_a_noop() {
        // JSON "a\\u0000b" decodes to the 8-char string a,backslash,u,0,0,0,0,b
        // — not a NUL — so the value is already clean and round-trips byte-for-byte.
        let s = r#"{"summary":"a\\u0000b"}"#.to_string();
        assert_eq!(strip_json_nul(s.clone()), s);
    }

    #[test]
    fn collision_real_and_literal_both_handled() {
        // "a" carries a real NUL; "b" carries the literal text backslash-u0000.
        // The value walk strips the former and leaves the latter intact — the
        // pathological case that needed a fallback in SQL is trivial in Rust.
        let v = parsed(strip_json_nul(
            r#"{"a":"x\u0000y","b":"p\\u0000q"}"#.to_string(),
        ));
        assert_eq!(v["a"], "xy");
        assert_eq!(v["b"], "p\\u0000q");
    }

    #[test]
    fn nested_values_and_keys_are_cleaned() {
        let out = strip_json_nul(
            r#"{"o":{"k\u0000":["a\u0000b",{"deep\u0000":"v\u0000"}]}}"#.to_string(),
        );
        assert!(!out.contains(r"\u0000"));
        let v = parsed(out);
        assert_eq!(v["o"]["k"][0], "ab");
        assert_eq!(v["o"]["k"][1]["deep"], "v");
    }

    #[test]
    fn odd_backslash_run_keeps_literal_drops_nul() {
        // JSON "a\\\u0000b" is an escaped backslash (kept) immediately followed by
        // a real NUL escape (dropped) -> decodes to a,backslash,b.
        let v = parsed(strip_json_nul(r#"{"x":"a\\\u0000b"}"#.to_string()));
        assert_eq!(v["x"], "a\\b");
    }
}
