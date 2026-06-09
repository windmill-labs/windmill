/*
 * Author: Diego Imbert
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Shared types and helpers for the per-user `draft` table.
//!
//! Lives in `windmill-common` so each entity crate (`windmill-api-scripts`,
//! `windmill-api-flows`, the trigger crates, etc.) can pull the helper
//! directly without taking a dependency on the top-level `windmill-api`
//! crate. Keep this file tiny and free of HTTP/axum concerns.

// `DraftUserRef` lives in `windmill-types` so the list-endpoint row structs
// (`ListableScript`, `ListableFlow`) — which sit in `windmill-types` and
// can't reach `windmill-common` without a dependency cycle — can declare
// `Vec<DraftUserRef>` aggregates. Re-exported here so the list/get
// handlers in `windmill-api-scripts` / `windmill-api-flows` / `windmill-api`
// keep a single import path for the per-user-draft surface.
pub use windmill_types::user_drafts::DraftUserRef;

use crate::db::DB;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Closed set of item kinds a user can have an autosaved draft on. Mirrors
/// the frontend's `USER_DRAFT_ITEM_KINDS`; the Postgres `DRAFT_KIND` enum
/// must stay in lockstep — adding a kind requires both a new variant here
/// and an `ALTER TYPE ... ADD VALUE` migration.
///
/// `snake_case` matches the wire/DB encoding so the same string round-trips
/// through HTTP path params, JSON bodies, and the `draft.typ` column without
/// per-edge mapping.
#[derive(sqlx::Type, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[sqlx(type_name = "DRAFT_KIND", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserDraftItemKind {
    Script,
    Flow,
    App,
    RawApp,
    Resource,
    Variable,
    TriggerSchedule,
    TriggerWebhook,
    TriggerDefaultEmail,
    TriggerEmail,
    TriggerHttp,
    TriggerWebsocket,
    TriggerPostgres,
    TriggerKafka,
    TriggerNats,
    TriggerMqtt,
    TriggerSqs,
    TriggerGcp,
    TriggerAzure,
    TriggerPoll,
    TriggerCli,
    TriggerNextcloud,
    TriggerGoogle,
    TriggerGithub,
}

/// Query-string flag accepted by every "get by path" route that supports
/// the draft overlay. Compose into a route-specific query struct via
/// `#[serde(flatten)]` when the route already has other query fields.
#[derive(Debug, Deserialize, Default)]
pub struct WithDraftQuery {
    /// When true, attach the authed user's draft for this entity (if any)
    /// as a separate `draft` field on the response. Defaults to false so
    /// non-editor callers see the deployed shape unchanged.
    #[serde(default)]
    pub get_draft: bool,
}

/// Response wrapper that sends the deployed entity untouched and attaches
/// the authed user's draft (if any) as a sibling `draft` field — the
/// frontend pairs the two to diff/restore/discard.
///
/// Wire shape is `<deployed fields...> + is_draft + draft_saved_at? +
/// no_deployed? + draft?` — non-editor callers ignore the overlay fields
/// and keep getting the deployed shape they used to. The deployed and
/// the draft are NEVER merged on the server; the editor's saved shape
/// can diverge from the deployed shape arbitrarily, so any per-kind
/// translation lives in the frontend loader where the types are known.
///
/// `inner` is held as `serde_json::Value` so the caller only needs
/// `Serialize` on its response type — most read-only response shapes
/// (e.g. `ScriptWithStarred`) only derive `Serialize`, and requiring
/// `DeserializeOwned` would force derive cascades through many crates.
/// One row of `other_drafts_users` — represents a draft on the same path
/// owned by someone other than the authed user. `username` is `None` for
/// the legacy NULL-email row (workspace-scoped pre-migration draft), which
/// the frontend surfaces as a "Legacy draft" entry with an info tooltip.
#[derive(Debug, Serialize)]
pub struct OtherDraftUser {
    /// `None` represents a legacy workspace-level draft (no owner).
    pub username: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct WithDraftOverlay {
    #[serde(flatten)]
    pub inner: serde_json::Value,
    pub is_draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_saved_at: Option<DateTime<Utc>>,
    /// True when no deployed row exists at this path — the response
    /// body is a best-effort stand-in synthesized from the draft, and
    /// only `draft` is canonical. Frontend uses this to disable "diff
    /// vs deployed" / "reset to deployed" actions and to skip its
    /// own deployed-shape parsing of `inner`. Omitted when false.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub no_deployed: bool,
    /// The user's saved draft payload (whatever shape the editor wrote).
    /// Present when `get_draft=true` and a draft exists. Pair with the
    /// deployed (the rest of the response) for diff/restore UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<serde_json::Value>,
    /// Other users with a draft on the same path (excludes the authed
    /// user). Frontend surfaces this list in a banner so the user can
    /// view another's JSON or fork it. Empty list is omitted to keep
    /// the common-case response lean.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other_drafts_users: Vec<OtherDraftUser>,
}

/// List every other user (and the legacy NULL-email row, if any) that
/// has a draft at `(workspace, kind, path)`. Returns usernames only —
/// emails never leave the server. LEFT JOIN against `usr` so an
/// orphaned draft (user removed from the workspace) still surfaces, with
/// its `username` falling back to `None` rather than dropping the row.
/// The authed user is excluded via `email <> authed_email`; the legacy
/// row matches because `email IS NULL` fails that comparison.
async fn fetch_other_drafts_users(
    db: &DB,
    w_id: &str,
    authed_email: &str,
    kind: UserDraftItemKind,
    path: &str,
) -> Result<Vec<OtherDraftUser>> {
    let rows = sqlx::query_as!(
        OtherDraftUser,
        r#"SELECT u.username as "username?"
           FROM draft d
           LEFT JOIN usr u
                  ON u.workspace_id = d.workspace_id
                 AND u.email = d.email
           WHERE d.workspace_id = $1
             AND d.path = $2
             AND d.typ = $3
             AND (d.email IS NULL OR d.email <> $4)
           ORDER BY d.email NULLS LAST"#,
        w_id,
        path,
        kind as UserDraftItemKind,
        authed_email,
    )
    .fetch_all(db)
    .await?;
    Ok(rows)
}

/// If `get_draft` is true AND the authed user has a draft saved for
/// `(workspace, kind, path)`, attach it as `draft` on the response.
/// The deployed payload (`deployed`) is always serialized into `inner`
/// untouched — the wire response is `<deployed fields...> + is_draft +
/// draft? + draft_saved_at? + other_drafts_users?` regardless of whether
/// the authed user has a draft.
pub async fn maybe_overlay_draft<T>(
    db: &DB,
    w_id: &str,
    email: &str,
    kind: UserDraftItemKind,
    path: &str,
    get_draft: bool,
    deployed: T,
) -> Result<WithDraftOverlay>
where
    T: serde::Serialize,
{
    let inner = serde_json::to_value(&deployed)?;

    // `other_drafts_users` is independent of the authed user's own draft
    // overlay — drop-from-draft callers (reset-to-deployed reloads, etc.)
    // still need to know who else is editing this path so the home-page /
    // editor surfaces stay coherent across reloads.
    let other_drafts_users = fetch_other_drafts_users(db, w_id, email, kind, path).await?;

    if !get_draft {
        return Ok(WithDraftOverlay {
            inner,
            is_draft: false,
            draft_saved_at: None,
            no_deployed: false,
            draft: None,
            other_drafts_users,
        });
    }

    let row = sqlx::query!(
        r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>",
                  created_at
           FROM draft
           WHERE workspace_id = $1
             AND email = $2
             AND path = $3
             AND typ = $4"#,
        w_id,
        email,
        path,
        kind as UserDraftItemKind,
    )
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(WithDraftOverlay {
            inner,
            is_draft: false,
            draft_saved_at: None,
            no_deployed: false,
            draft: None,
            other_drafts_users,
        });
    };

    let draft_json: serde_json::Value = serde_json::from_str(row.value.0.get())?;

    Ok(WithDraftOverlay {
        inner,
        is_draft: true,
        draft_saved_at: Some(row.created_at),
        no_deployed: false,
        draft: Some(draft_json),
        other_drafts_users,
    })
}

/// Delete the authed user's draft for `(workspace, kind, path)`.
/// Idempotent — returns Ok even when no row exists. Scoped to a single
/// email so other users' drafts at the same path are untouched.
///
/// Called from item delete handlers (`delete_script_by_path`,
/// `delete_flow_by_path`, etc.) so the user can't be left with a stale
/// per-user draft after the underlying item is gone.
pub async fn delete_user_draft(
    db: &DB,
    w_id: &str,
    email: &str,
    kind: UserDraftItemKind,
    path: &str,
) -> Result<()> {
    sqlx::query!(
        r#"DELETE FROM draft
           WHERE workspace_id = $1
             AND email = $2
             AND path = $3
             AND typ = $4"#,
        w_id,
        email,
        path,
        kind as UserDraftItemKind,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Fetch the authed user's draft as a standalone payload, used by
/// "get by path" routes when no deployed row exists at the path but a
/// draft might. Returns the draft as `WithDraftOverlay` with both
/// `inner` (best-effort stand-in for the missing deployed) and `draft`
/// populated to the same JSON, and `no_deployed = true` so the frontend
/// knows there's no real deployed to compare against.
///
/// Callers must already have established that no deployed row exists.
/// Returns `Ok(None)` when there's also no draft — caller should 404.
///
/// The draft JSON is expected to be a JSON object (every editor writes
/// drafts as object-shaped editable state, so `serde(flatten)` works on
/// the inner value). A non-object draft would render with no fields
/// flattened — defensive but degraded.
pub async fn fetch_draft_only(
    db: &DB,
    w_id: &str,
    email: &str,
    kind: UserDraftItemKind,
    path: &str,
) -> Result<Option<WithDraftOverlay>> {
    let row = sqlx::query!(
        r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>",
                  created_at
           FROM draft
           WHERE workspace_id = $1
             AND email = $2
             AND path = $3
             AND typ = $4"#,
        w_id,
        email,
        path,
        kind as UserDraftItemKind,
    )
    .fetch_optional(db)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };

    let draft_json: serde_json::Value = serde_json::from_str(row.value.0.get())?;
    let other_drafts_users = fetch_other_drafts_users(db, w_id, email, kind, path).await?;
    Ok(Some(WithDraftOverlay {
        // Best-effort stand-in for the missing deployed — same JSON as
        // `draft`. Frontend should read `.draft` for the editor state
        // and skip "diff vs deployed" UI when `no_deployed` is set.
        inner: draft_json.clone(),
        is_draft: true,
        draft_saved_at: Some(row.created_at),
        no_deployed: true,
        draft: Some(draft_json),
        other_drafts_users,
    }))
}
