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
    utils::{check_proper_path, strip_json_nul},
    variables::{build_crypt, encrypt},
};

pub fn workspaced_service() -> Router {
    Router::new()
        .route("/list", get(list_drafts))
        .route("/get/{kind}/{*path}", get(get_draft_for_user))
        .route("/get_own/{kind}/{*path}", get(get_own_draft))
        .route("/update/{kind}/{*path}", post(update_draft))
        .route("/move/{kind}/{*path}", post(move_draft))
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
    /// `true` when this draft is identical (jsonb-equal) to the parent's draft at
    /// the same (path, kind, owner). `None` unless the request passed a valid
    /// `compare_to_workspace`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unchanged_from_parent: Option<bool>,
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
    /// A fork passes its parent workspace id here to have each row flagged with
    /// `unchanged_from_parent`. Honored only when it is this workspace's actual
    /// `parent_workspace_id` (enforced in `list_drafts`).
    pub compare_to_workspace: Option<String>,
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
    // Only honor `compare_to_workspace` when it is genuinely this workspace's
    // parent (a fork comparing against its source). Any other value is dropped
    // so the value-equality subquery can't be used to probe an unrelated
    // workspace's draft contents.
    let compare_to_workspace = match &query.compare_to_workspace {
        Some(candidate) => {
            let parent: Option<String> = sqlx::query_scalar::<_, Option<String>>(
                "SELECT parent_workspace_id FROM workspace WHERE id = $1",
            )
            .bind(&w_id)
            .fetch_optional(&db)
            .await?
            .flatten();
            if parent.as_deref() == Some(candidate.as_str()) {
                Some(candidate.clone())
            } else {
                None
            }
        }
        None => None,
    };
    let rows = sqlx::query_as::<_, DraftListItem>(&list_drafts_query(all_users))
        .bind(&w_id)
        .bind(&authed.email)
        .bind(&compare_to_workspace)
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
/// `$1` = workspace_id, `$2` = email, `$3` = the parent workspace to compare
/// against (nullable; drives `unchanged_from_parent`). With `all_users` the
/// owner filter is dropped so every workspace draft is listed (others' rows get
/// `mine=false`).
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
    // A null username means the legacy row downstream, so an owner that resolves to
    // no name at all — an external JWT's subject has neither row — is dropped rather
    // than surfaced as a second legacy entry.
    let draft_users = r#"CASE WHEN d.typ::text IN ('script', 'flow', 'app', 'raw_app') THEN (
                      SELECT json_agg(json_build_object('username', COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN du.email END))
                                      ORDER BY COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN du.email END) NULLS LAST)
                      FROM draft du
                      LEFT JOIN usr u ON u.workspace_id = du.workspace_id AND u.email = du.email
                      LEFT JOIN password p ON p.email = du.email AND p.super_admin = true
                      WHERE du.workspace_id = d.workspace_id AND du.path = d.path AND du.typ = d.typ
                        AND (du.email IS NULL OR u.username IS NOT NULL OR p.email IS NOT NULL)
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
                  {case} AS draft_only,
                  -- value is a `json` column (no `=` operator), so compare as jsonb.
                  CASE WHEN $3::text IS NULL THEN NULL::bool
                       ELSE EXISTS(
                         SELECT 1 FROM draft pd
                         WHERE pd.workspace_id = $3
                           AND pd.path = d.path
                           AND pd.typ = d.typ
                           AND pd.email IS NOT DISTINCT FROM d.email
                           AND pd.value::jsonb = d.value::jsonb
                       )
                  END AS unchanged_from_parent
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
    /// `saved` only: where the write landed. Differs from the URL path when the item
    /// had moved away from it; the editor follows it there. Absent when a delete found
    /// nothing to remove and the caller cannot read the path it moved to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// The version a draft forked from, as the editors write it into `draft.value`.
/// Each kind names it differently and only one is ever set.
#[derive(Deserialize)]
struct DraftBaseVersion {
    /// Scripts: hex-encoded script hash.
    #[serde(default)]
    parent_hash: Option<String>,
    /// Flows: `flow_version.id`.
    #[serde(default)]
    version_id: Option<i64>,
    /// Apps / raw apps: `app_version.id`.
    #[serde(default)]
    parent_version: Option<i64>,
}

impl DraftBaseVersion {
    /// The base as the `draft.base` column stores it: one opaque text id whatever
    /// the kind, so a reader compares it to the head without knowing the kind's
    /// own field name or type.
    fn as_text(&self, kind: UserDraftItemKind) -> Option<String> {
        use UserDraftItemKind::*;
        match kind {
            Script => self.parent_hash.clone(),
            Flow => self.version_id.map(|v| v.to_string()),
            _ => self.parent_version.map(|v| v.to_string()),
        }
    }
}

/// The version this draft forked from, or `None` when it has none — a kind
/// that keeps no lineage, a malformed payload, or a draft that was never forked
/// from a deploy. Pure: no queries.
fn draft_lineage(kind: UserDraftItemKind, value: &str) -> Option<DraftBaseVersion> {
    use UserDraftItemKind::*;
    if !matches!(kind, Script | Flow | App | RawApp) {
        return None;
    }
    let base = serde_json::from_str::<DraftBaseVersion>(value).ok()?;
    let has_base = match kind {
        Script => base
            .parent_hash
            .as_deref()
            .and_then(|h| windmill_common::scripts::to_i64(h).ok())
            .is_some(),
        Flow => base.version_id.is_some(),
        _ => base.parent_version.is_some(),
    };
    has_base.then_some(base)
}

/// Apply the current user's draft: non-null `value` upserts, `null` (or
/// omitted) deletes. Either way, when the existing row is newer than
/// `last_sync` (and `force` is false) the op is skipped and the response is
/// `status = conflict` + the server's current timestamp.
///
/// A save addressed to a path its item moved away from lands where the move took
/// the drafts (`draft_move`), unless the caller still has a draft of their own at
/// that path. The response names where it landed.
async fn update_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    Json(req): Json<SaveDraftRequest>,
) -> Result<Json<SaveDraftResponse>> {
    let email = &authed.email;
    let url_path = path.to_path();
    // Saving a draft requires write permission on the underlying path. Deleting
    // (discarding) one's OWN draft does not: the email-scoped row belongs to the
    // authed user, so they can always discard it even after losing write access
    // to the underlying item (e.g. a draft-only item whose folder perms changed).
    // The DELETE below is scoped to `email = authed.email`, so it can only ever
    // touch the caller's own row. Legacy (NULL-email) rows aren't owned by anyone
    // — they keep the write gate.
    let is_own_discard = req.value.is_none() && !req.legacy;
    // `legacy` targets the workspace-level row and is delete-only: an upsert writes the
    // caller's own row whatever it says. Every read of that rule goes through this.
    let legacy_delete = req.value.is_none() && req.legacy;

    // Whose row this write is for: the caller's, or the workspace-level one on a legacy
    // DELETE (`legacy` is delete-only, so an upsert is the caller's own row either way).
    // It picks both the record that applies — an item's move (`email IS NULL`) covers the
    // legacy row too, since the same rename carried it — and the draft whose presence
    // means this path is still the write's own.
    let owner: Option<&str> = (!legacy_delete).then_some(email.as_str());
    // The caller's own draft-only move outranks the move of the deployed item.
    let moved_to = sqlx::query_scalar!(
        r#"SELECT m.new_path FROM draft_move m
           WHERE m.workspace_id = $1 AND m.typ = $2 AND m.old_path = $3
             AND (m.email IS NULL OR m.email = $4)
             AND NOT EXISTS (
                 SELECT 1 FROM draft d
                 WHERE d.workspace_id = $1 AND d.typ = $2 AND d.path = $3
                   AND d.email IS NOT DISTINCT FROM $4
             )
           ORDER BY m.email IS NULL
           LIMIT 1"#,
        &w_id,
        kind as UserDraftItemKind,
        url_path,
        owner,
    )
    .fetch_optional(&db)
    .await?;
    let path: &str = moved_to.as_deref().unwrap_or(url_path);

    // Everything past here writes, so the gate applies from here on. Answered
    // without the path when the item moved: it may have gone somewhere the caller
    // cannot see.
    if !is_own_discard {
        match require_can_write_path(&authed, &db, &user_db, &w_id, kind, path).await {
            Err(Error::NotAuthorized(_)) if moved_to.is_some() => {
                return Err(Error::NotAuthorized(
                    "this draft's item was moved to a path you cannot write".to_string(),
                ));
            }
            other => other?,
        }
    }

    let applied = if let Some(value) = &req.value {
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
        let serialized = strip_json_nul(&serialized);
        // `base` is derived here from the value's per-kind field rather than sent
        // by the client, so every writer (editors, chat, CLI) fills it the same way.
        // Read from the sanitized text, which is what the value column gets: a NUL in the
        // lineage field otherwise costs the draft its base (the hash no longer parses),
        // leaving a draft that reads as up to date whatever the head is.
        let base = draft_lineage(kind, serialized.as_ref()).and_then(|l| l.as_text(kind));
        // Upsert. The conflict check rides on the DO UPDATE WHERE clause —
        // when the row is newer than `last_sync`, RETURNING yields nothing.
        // `created_at` defaults to `now()` but the migration overrides it ($8)
        // so a migrated draft keeps its original age instead of jumping to top.
        //
        // A moved save ($10) carries the path keys its editor had before the move.
        // One naming the path it addressed ($11) follows to where it landed; a draft
        // already there keeps the keys the move gave it. A pre-sanitizer NUL escape
        // in that draft makes `to_jsonb` raise, so it takes the incoming keys.
        sqlx::query!(
            r#"INSERT INTO draft (workspace_id, email, path, typ, value, created_at, base)
               VALUES ($1, $2, $3::text, $4,
                       CASE WHEN $10::bool
                            THEN to_json($5::text::jsonb || jsonb_strip_nulls(jsonb_build_object(
                                'path', CASE WHEN $5::text::jsonb -> 'path' = to_jsonb($11::text)
                                             THEN to_jsonb($3::text) END,
                                'draft_path', CASE WHEN $5::text::jsonb -> 'draft_path' = to_jsonb($11::text)
                                                   THEN to_jsonb($3::text) END)))
                            ELSE $5::text::json
                       END,
                       COALESCE($8::timestamptz, now()), $9)
               ON CONFLICT (workspace_id, path, typ, email) WHERE email IS NOT NULL
               DO UPDATE SET value = CASE
                                 WHEN NOT $10::bool
                                   OR position(chr(92) || 'u0000' in replace(draft.value::text, chr(92) || chr(92), '')) > 0
                                     THEN EXCLUDED.value
                                 ELSE to_json((to_jsonb(EXCLUDED.value) - 'path' - 'draft_path')
                                     || jsonb_strip_nulls(jsonb_build_object(
                                         'path', to_jsonb(draft.value) -> 'path',
                                         'draft_path', to_jsonb(draft.value) -> 'draft_path')))
                             END,
                             created_at = EXCLUDED.created_at,
                             base = EXCLUDED.base
               WHERE $7::bool = true
                  OR $6::timestamptz IS NULL
                  OR draft.created_at <= $6::timestamptz
               RETURNING path, created_at"#,
            &w_id,
            email,
            path,
            kind as UserDraftItemKind,
            serialized.as_ref(),
            req.last_sync,
            req.force,
            req.created_at,
            base.as_deref(),
            moved_to.is_some(),
            url_path,
        )
        .fetch_optional(&db)
        .await?
        .map(|r| (r.created_at, Some(r.path)))
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
            legacy_delete,
        )
        .fetch_optional(&db)
        .await?
        // Named for the same reason an upsert is: the editor that discarded is still on
        // the path the item left, and reloading there would land on nothing.
        .map(|ts| (ts, moved_to.clone()))
    };

    if let Some((ts, path)) = applied {
        return Ok(Json(SaveDraftResponse {
            status: SaveDraftStatus::Saved,
            current_timestamp: ts,
            path,
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
        legacy_delete,
    )
    .fetch_optional(&db)
    .await?;

    match existing {
        Some(ts) => Ok(Json(SaveDraftResponse {
            status: SaveDraftStatus::Conflict,
            current_timestamp: ts,
            path: None,
        })),
        // Delete + nothing-was-there ⇒ report success with server's NOW().
        None => {
            let now = sqlx::query_scalar!(r#"SELECT now() as "now!""#)
                .fetch_one(&db)
                .await?;
            // A retry of a routed discard whose answer was lost lands here: the row is
            // gone but the editor is still on the path the item left, so it needs the
            // destination as much as the first attempt did. Unlike the arm above there is
            // no deleted row proving the caller ever held that draft, and an own discard
            // is not gated, so this names a path to someone who may have none of it: the
            // read gate, which is what keeps a path from being disclosed elsewhere in this
            // module. Without it, discarding at a guessed path reads `draft_move`.
            let disclosed = match moved_to {
                Some(dest) => {
                    match require_can_read_path(&authed, &user_db, &w_id, kind, &dest).await {
                        Ok(()) => Some(dest),
                        Err(Error::NotFound(_))
                        | Err(Error::NotAuthorized(_))
                        | Err(Error::BadRequest(_)) => None,
                        Err(e) => return Err(e),
                    }
                }
                None => None,
            };
            Ok(Json(SaveDraftResponse {
                status: SaveDraftStatus::Saved,
                current_timestamp: now,
                path: disclosed,
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct MoveDraftRequest {
    pub new_path: String,
    /// Also restate the draft's summary, so the same drawer that renames a
    /// deployed item can retitle a draft-only one.
    #[serde(default)]
    pub summary: Option<String>,
}

/// Relocate the authed user's own DRAFT-ONLY item. Such an item is nothing but
/// its draft row, so moving it is a rewrite of that row's path plus both path
/// keys inside its value — there is no deployed row, schedule or trigger to
/// cascade to.
///
/// The owner's own open editor follows: its next save, still addressed to the old
/// path, lands at the new one through the move record, and it is told where.
///
/// Scoped to the caller's own row on purpose: two users can each have a draft
/// at the same never-deployed path, and those are two separate items.
///
/// A DEPLOYED item must move through its own deploy endpoint instead, which
/// cascades everything that references the path and carries every draft along.
async fn move_draft(
    authed: ApiAuthed,
    Extension(db): Extension<DB>,
    Extension(user_db): Extension<UserDB>,
    Path((w_id, kind, path)): Path<(String, UserDraftItemKind, windmill_common::utils::StripPath)>,
    Json(req): Json<MoveDraftRequest>,
) -> Result<String> {
    let path = path.to_path();
    let new_path = req.new_path.as_str();
    // Only the full-page editor kinds, which is exactly the set that has a typed
    // path to rewrite. Reading the movable set off the same mapping the rewrite
    // uses keeps them from drifting apart: a resource, a variable or a trigger
    // keeps its deploy target in `value.path` with no editor to stage a rename,
    // so moving one would leave the real target naming the old location and the
    // next deploy would recreate it where it came from.
    let (Some(typed_field), Some(mirror_field)) =
        (kind.typed_path_field(), kind.mirror_path_field())
    else {
        return Err(Error::BadRequest(format!(
            "moving a {kind:?} draft is not supported — only scripts, flows and apps"
        )));
    };
    // Validate before authorizing: `require_can_write_path` is not a format check
    // (an admin returns immediately, and a user returns early inside their own
    // namespace), so without this a malformed destination is stored as-is, and an
    // over-long or NUL-bearing one reaches Postgres as a raw server error.
    check_proper_path(new_path)?;
    // A summary-only edit is a legitimate use of this endpoint: the drawer edits
    // both fields, and for a draft-only script the path it posts back is the row
    // path unchanged (`list_scripts` only reports `draft_path` when it differs).
    // Returning early on the path alone would drop the new summary silently.
    if new_path == path && req.summary.is_none() {
        return Ok("unchanged".to_string());
    }
    require_can_write_path(&authed, &db, &user_db, &w_id, kind, path).await?;
    if new_path != path {
        require_can_write_path(&authed, &db, &user_db, &w_id, kind, new_path).await?;
    }

    if let Some(table) = kind.deployed_table() {
        // `table` is from the closed `deployed_table()` enum, never user input.
        // Archived and soft-deleted rows keep sitting at their path — a script
        // move archives its parent in place — so an existence check that counted
        // them would refuse a move away from, or into, a path nothing occupies.
        // `create_script_internal` resolves its own path clashes the same way.
        let archived_filter = if table == "script" {
            " AND NOT archived AND NOT deleted"
        } else {
            ""
        };
        let query = format!(
            "SELECT 1 FROM {table} WHERE path = $1 AND workspace_id = $2{archived_filter} LIMIT 1"
        );
        let mut tx = user_db.clone().begin(&authed).await?;
        let deployed_at_old = sqlx::query_scalar::<_, i32>(&query)
            .bind(path)
            .bind(&w_id)
            .fetch_optional(&mut *tx)
            .await?;
        let deployed_at_new = sqlx::query_scalar::<_, i32>(&query)
            .bind(new_path)
            .bind(&w_id)
            .fetch_optional(&mut *tx)
            .await?;
        tx.commit().await?;
        if deployed_at_old.is_some() {
            return Err(Error::BadRequest(format!(
                "'{path}' is deployed — move it from its editor so schedules and triggers follow"
            )));
        }
        if deployed_at_new.is_some() {
            return Err(Error::BadRequest(format!(
                "'{new_path}' already has a deployed item — moving there would turn this into a draft on top of it"
            )));
        }
    }

    // A NUL in the summary reaches Postgres as a raw byte and fails the statement with
    // an encoding error, so it is dropped here the way `strip_json_nul` drops one from a
    // draft value. `json` cannot store it either.
    let summary = req.summary.as_ref().map(|s| s.replace('\0', ""));

    // A classic app and a raw app share the `app` table, so a draft of either kind
    // occupies the destination for both: deploying there deletes the caller's drafts
    // of both kinds, taking the item that lost the collision with it.
    let collision_typs: Vec<&str> = match kind {
        UserDraftItemKind::App | UserDraftItemKind::RawApp => vec![
            UserDraftItemKind::App.as_str(),
            UserDraftItemKind::RawApp.as_str(),
        ],
        _ => vec![kind.as_str()],
    };

    // One transaction with the move record, so a save addressed to the old path
    // never sees the row gone without knowing where it went.
    let mut tx = db.begin().await?;
    let moved = sqlx::query_scalar!(
        r#"UPDATE draft
           SET path = $3,
               -- Both path keys, not just the typed one: the editors mirror the
               -- typed path into the other while it differs from the row's path,
               -- and the loaders prefer the mirror — left naming the old location
               -- it un-does this move on the next save. `create_missing = false`
               -- on both, so a draft carrying only one keeps only one.
               value = to_json(
                   jsonb_set(
                       jsonb_set(
                           CASE WHEN $7::text IS NULL THEN to_jsonb(value)
                                ELSE jsonb_set(to_jsonb(value), ARRAY['summary'], to_jsonb($7::text))
                           END,
                           ARRAY[$5::text], to_jsonb($3::text), false
                       ),
                       ARRAY[$8::text], to_jsonb($3::text), false
                   )
               )
           WHERE workspace_id = $1
             AND path = $2
             AND typ = $4
             AND email = $6
             -- A pre-sanitizer NUL escape makes `to_jsonb` raise 22P05. Excluded
             -- here so the statement can't 500; reported below instead. Unlike the
             -- passive carry, rewriting the value IS this operation, so skipping it
             -- silently would move the row and leave its typed path stale.
             AND position(chr(92) || 'u0000' in replace(value::text, chr(92) || chr(92), '')) = 0
             -- Skipped on a summary-only edit, where the "target" row is this
             -- row and the guard would refuse the update against itself.
             AND ($2 = $3 OR NOT EXISTS (
                 SELECT 1 FROM draft o
                 WHERE o.workspace_id = $1 AND o.path = $3 AND o.typ::text = ANY($9::text[])
                   -- Of this kind only the caller's own row and the legacy one collide:
                   -- teammates' drafts of one item share its path by design, but a deploy
                   -- there wipes those two together, so a second would discard edits the
                   -- caller never saw. The other app kind is a different item on the same
                   -- deployed path, so it collides whoever owns it.
                   AND (o.typ <> $4 OR o.email = $6 OR o.email IS NULL)
             ))
           RETURNING id"#,
        &w_id,
        path,
        new_path,
        kind as UserDraftItemKind,
        typed_field,
        &authed.email,
        summary,
        mirror_field,
        &collision_typs as &[&str],
    )
    .fetch_optional(&mut *tx)
    .await?;
    if moved.is_some() && new_path != path {
        windmill_common::user_drafts::record_draft_move(
            &mut tx,
            &w_id,
            &[kind],
            path,
            new_path,
            Some(&authed.email),
        )
        .await?;
    }
    tx.commit().await?;

    if moved.is_none() {
        let row = sqlx::query!(
            r#"SELECT
                 -- The guard's own predicate, ordered own row, then legacy, then another
                 -- user's other-kind row: each needs a different remedy, and a bare
                 -- LIMIT 1 would name an arbitrary one.
                 (SELECT typ::text FROM draft WHERE workspace_id = $1 AND path = $3
                  AND typ::text = ANY($6::text[])
                  AND (typ <> $2 OR email = $4 OR email IS NULL)
                  ORDER BY CASE WHEN email = $4 THEN 0 WHEN email IS NULL THEN 1 ELSE 2 END
                  LIMIT 1) as "at_target",
                 EXISTS(SELECT 1 FROM draft WHERE workspace_id = $1 AND path = $3
                        AND typ::text = ANY($6::text[]) AND email = $4) as "at_target_own!",
                 EXISTS(SELECT 1 FROM draft WHERE workspace_id = $1 AND path = $3
                        AND typ::text = ANY($6::text[]) AND email IS NULL) as "at_target_legacy!",
                 EXISTS(SELECT 1 FROM draft WHERE workspace_id = $1 AND path = $5
                        AND typ = $2 AND email = $4
                        AND position(chr(92) || 'u0000' in replace(value::text, chr(92) || chr(92), '')) > 0
                       ) as "poisoned!",
                 EXISTS(SELECT 1 FROM draft WHERE workspace_id = $1 AND path = $5
                        AND typ = $2 AND email IS NULL) as "legacy!" "#,
            &w_id,
            kind as UserDraftItemKind,
            new_path,
            &authed.email,
            path,
            &collision_typs as &[&str],
        )
        .fetch_one(&db)
        .await?;
        return Err(Error::BadRequest(if row.poisoned {
            // This endpoint also serves a summary-only edit, so name the operation
            // the caller actually asked for rather than always saying "moved".
            let attempted = if new_path == path { "updated" } else { "moved" };
            format!(
                "'{path}' contains a NUL character and predates the sanitizer, so it cannot be \
                 {attempted}. Reopen it, re-save to rewrite it cleanly, then retry."
            )
        } else if row.legacy {
            // The home list synthesizes a draft-only row for the legacy draft with the
            // caller's own name on it, so this is reachable from the row menu. Only an
            // admin can claim or discard that row, and only from the drafts page.
            format!(
                "'{path}' is a legacy workspace draft with no owner, so it cannot be moved. \
                 A workspace admin can claim or discard it on the Review & deploy drafts page."
            )
        } else if let Some(occupant) = row.at_target.filter(|_| new_path != path) {
            // Naming the kind matters for the app pair: a classic-app draft refusing a
            // raw-app move is invisible in the raw-app list the caller is looking at.
            let occupant = occupant.replace('_', " ");
            if row.at_target_own {
                format!("You already have a draft at '{new_path}' ({occupant})")
            } else if row.at_target_legacy {
                // An ownerless row the caller cannot clear themselves, so send them to
                // the one place it can be resolved rather than to "discard your draft".
                format!(
                    "A legacy workspace draft with no owner is already at '{new_path}' \
                     ({occupant}). A workspace admin can claim or discard it on the Review & \
                     deploy drafts page."
                )
            } else {
                // The other app kind, owned by someone else: one deployed path cannot hold
                // both, so this is the other item's path, not a teammate's copy of this one.
                format!(
                    "Another user has a {occupant} draft at '{new_path}', and an app and a \
                     raw app cannot share a path."
                )
            }
        } else {
            format!("You have no draft at '{path}'")
        }));
    }

    if new_path == path {
        return Ok(format!("updated draft {path}"));
    }
    Ok(format!("moved draft {path} to {new_path}"))
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
                       RETURNING value, base
                   )
                   INSERT INTO draft (workspace_id, email, path, typ, value, created_at, base)
                   SELECT $1, $4, $2, $3, value, now(), base FROM legacy
                   ON CONFLICT (workspace_id, path, typ, email) WHERE email IS NOT NULL
                   DO UPDATE SET value = EXCLUDED.value, created_at = now(), base = EXCLUDED.base
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
///   2. Path-prefix match against own `u/{username}`, a group in `authed.groups`, or any
///      folder in `authed.folders` (the precomputed read set, with groups + direct
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
            // As `require_can_write_path` and the `see_member` RLS policy read it: a
            // draft-only `g/` path has no row for the probe below to fall back on, so
            // without this a member cannot see a draft their group owns.
            "g" if authed.groups.iter().any(|g| g == parts[1]) => return Ok(()),
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
