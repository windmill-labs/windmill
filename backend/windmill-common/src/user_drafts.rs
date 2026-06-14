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

impl UserDraftItemKind {
    /// The snake_case wire/DB string — same encoding serde and sqlx use.
    /// For interpolating into dynamically-built SQL (`?::DRAFT_KIND`
    /// binds want a string); keep in lockstep with the variants above.
    pub fn as_str(&self) -> &'static str {
        match self {
            UserDraftItemKind::Script => "script",
            UserDraftItemKind::Flow => "flow",
            UserDraftItemKind::App => "app",
            UserDraftItemKind::RawApp => "raw_app",
            UserDraftItemKind::Resource => "resource",
            UserDraftItemKind::Variable => "variable",
            UserDraftItemKind::TriggerSchedule => "trigger_schedule",
            UserDraftItemKind::TriggerWebhook => "trigger_webhook",
            UserDraftItemKind::TriggerDefaultEmail => "trigger_default_email",
            UserDraftItemKind::TriggerEmail => "trigger_email",
            UserDraftItemKind::TriggerHttp => "trigger_http",
            UserDraftItemKind::TriggerWebsocket => "trigger_websocket",
            UserDraftItemKind::TriggerPostgres => "trigger_postgres",
            UserDraftItemKind::TriggerKafka => "trigger_kafka",
            UserDraftItemKind::TriggerNats => "trigger_nats",
            UserDraftItemKind::TriggerMqtt => "trigger_mqtt",
            UserDraftItemKind::TriggerSqs => "trigger_sqs",
            UserDraftItemKind::TriggerGcp => "trigger_gcp",
            UserDraftItemKind::TriggerAzure => "trigger_azure",
            UserDraftItemKind::TriggerPoll => "trigger_poll",
            UserDraftItemKind::TriggerCli => "trigger_cli",
            UserDraftItemKind::TriggerNextcloud => "trigger_nextcloud",
            UserDraftItemKind::TriggerGoogle => "trigger_google",
            UserDraftItemKind::TriggerGithub => "trigger_github",
        }
    }

    /// Every variant, for code that must enumerate kinds (e.g. generating
    /// the `draft_only` existence SQL). Keep in lockstep with the enum.
    pub const ALL: [UserDraftItemKind; 24] = [
        UserDraftItemKind::Script,
        UserDraftItemKind::Flow,
        UserDraftItemKind::App,
        UserDraftItemKind::RawApp,
        UserDraftItemKind::Resource,
        UserDraftItemKind::Variable,
        UserDraftItemKind::TriggerSchedule,
        UserDraftItemKind::TriggerWebhook,
        UserDraftItemKind::TriggerDefaultEmail,
        UserDraftItemKind::TriggerEmail,
        UserDraftItemKind::TriggerHttp,
        UserDraftItemKind::TriggerWebsocket,
        UserDraftItemKind::TriggerPostgres,
        UserDraftItemKind::TriggerKafka,
        UserDraftItemKind::TriggerNats,
        UserDraftItemKind::TriggerMqtt,
        UserDraftItemKind::TriggerSqs,
        UserDraftItemKind::TriggerGcp,
        UserDraftItemKind::TriggerAzure,
        UserDraftItemKind::TriggerPoll,
        UserDraftItemKind::TriggerCli,
        UserDraftItemKind::TriggerNextcloud,
        UserDraftItemKind::TriggerGoogle,
        UserDraftItemKind::TriggerGithub,
    ];

    /// The deployed table backing this kind, keyed by `(workspace_id,
    /// path)`. SINGLE SOURCE for both the draft access check (which table
    /// RLS resolves item-level `extra_perms` against) and the
    /// `draft_only` existence check on the list endpoint.
    ///
    /// `None` for kinds with no per-path backing table:
    ///   - `TriggerWebhook` is a property of a script/flow row, not its own row;
    ///   - the native triggers (`poll`/`cli`/`nextcloud`/`google`/`github`)
    ///     are keyed by `external_id`/`service_name`, not `path`, and are
    ///     not supported as draftable items.
    /// Callers treat `None` as "no deployed counterpart" → `draft_only =
    /// true` on the list, and a path-only access check on save/get.
    pub fn deployed_table(&self) -> Option<&'static str> {
        use UserDraftItemKind::*;
        match self {
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
            TriggerWebhook | TriggerPoll | TriggerCli | TriggerNextcloud | TriggerGoogle
            | TriggerGithub => None,
        }
    }

    /// Whether OTHER users' drafts at a path are visible to a viewer — the
    /// "others are editing" list, the draft-owner circles, and the
    /// `get_draft_for_user` (View JSON / Fork) endpoint.
    ///
    /// Enabled only for the full-page editor items (script / flow / app),
    /// which have the cross-user draft UI. Disabled for the drawer items
    /// (resource / variable / triggers): they have no such UI, and for
    /// secret variables, exposing another user's draft would hand out the
    /// `$encrypted:` ciphertext — which the deploy endpoints decrypt with
    /// the workspace key, so a viewer could launder it into plaintext.
    /// Keeping cross-user drafts private to their owner for these kinds
    /// closes that vector.
    pub fn shares_drafts_across_users(&self) -> bool {
        use UserDraftItemKind::*;
        matches!(self, Script | Flow | App | RawApp)
    }
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
/// `inner` is a boxed `erased_serde::Serialize` trait object so the
/// deployed payload — a possibly MB-scale flow/app — is serialized
/// straight to the response in ONE pass, instead of first being
/// materialized into a `serde_json::Value` tree and then serialized
/// again. The caller still only needs `Serialize` on its response type
/// (no `DeserializeOwned` cascades), and the struct stays non-generic so
/// every handler keeps `JsonResult<WithDraftOverlay>`.
/// One row of `other_drafts_users` — represents a draft on the same path
/// owned by someone other than the authed user. `username` is `None` for
/// the legacy NULL-email row (workspace-scoped pre-migration draft), which
/// the frontend surfaces as a "Legacy draft" entry with an info tooltip.
#[derive(Debug, Serialize)]
pub struct OtherDraftUser {
    /// `None` represents a legacy workspace-level draft (no owner).
    pub username: Option<String>,
}

#[derive(Serialize)]
pub struct WithDraftOverlay {
    /// Deployed payload, flattened to the top level. Boxed-erased so a
    /// large deployed entity is serialized once (see the type doc above).
    #[serde(flatten)]
    pub inner: Box<dyn erased_serde::Serialize + Send>,
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
    T: serde::Serialize + Send + 'static,
{
    // Non-editor callers (worker/CLI reads of possibly MB-scale flows &
    // apps) pass `get_draft = false` and never render the draft overlay or
    // the "others editing" surfaces — so skip the extra `usr` join entirely
    // for them. Only editor reads (`get_draft = true`) pay for it.
    if !get_draft {
        return Ok(WithDraftOverlay {
            inner: Box::new(deployed),
            is_draft: false,
            draft_saved_at: None,
            no_deployed: false,
            draft: None,
            other_drafts_users: Vec::new(),
        });
    }

    // `other_drafts_users` is independent of the authed user's OWN draft —
    // drop-from-draft editor reloads (reset-to-deployed) still need to know
    // who else is editing this path, and they pass `get_draft = true`.
    // Only the cross-user-visible kinds (script/flow/app) surface it; the
    // drawer kinds (resource/variable/triggers) keep drafts private to
    // their owner.
    let other_drafts_users = if kind.shares_drafts_across_users() {
        fetch_other_drafts_users(db, w_id, email, kind, path).await?
    } else {
        Vec::new()
    };

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
            inner: Box::new(deployed),
            is_draft: false,
            draft_saved_at: None,
            no_deployed: false,
            draft: None,
            other_drafts_users,
        });
    };

    let draft_json: serde_json::Value = serde_json::from_str(row.value.0.get())?;

    Ok(WithDraftOverlay {
        inner: Box::new(deployed),
        is_draft: true,
        draft_saved_at: Some(row.created_at),
        no_deployed: false,
        draft: Some(draft_json),
        other_drafts_users,
    })
}

/// One row of a "draft-only" list synthesis: the authed user has a draft
/// at `path` with no deployed counterpart. `value` is the editor's saved
/// draft JSON (each list handler maps it into its own `Listable*` shape).
#[derive(sqlx::FromRow)]
pub struct DraftOnlyListRow {
    pub path: String,
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub created_at: DateTime<Utc>,
}

/// Fetch the authed user's draft rows at paths with NO deployed
/// counterpart, for synthesizing draft-only entries into a list response.
/// The deployed table to check absence against comes from
/// `kind.deployed_table()` — the same single source the access check and
/// `draft_only` flag use — so the per-handler copies of this NOT EXISTS
/// query can't drift. Returns empty for kinds with no path-keyed table.
/// Callers keep their own gating (`include_draft_only`, page 0, no
/// filters) and per-type row mapping.
pub async fn fetch_draft_only_list_rows(
    db: &DB,
    w_id: &str,
    email: &str,
    kind: UserDraftItemKind,
) -> Result<Vec<DraftOnlyListRow>> {
    let Some(table) = kind.deployed_table() else {
        return Ok(Vec::new());
    };
    // `table` is from the closed `deployed_table()` enum mapping, never
    // user input. `typ` binds as text and casts to DRAFT_KIND.
    let sql = format!(
        "SELECT path, value, created_at FROM draft \
         WHERE workspace_id = $1 AND typ = $2::text::DRAFT_KIND AND email = $3 \
         AND NOT EXISTS (SELECT 1 FROM {table} t \
             WHERE t.workspace_id = draft.workspace_id AND t.path = draft.path)"
    );
    let rows = sqlx::query_as::<_, DraftOnlyListRow>(&sql)
        .bind(w_id)
        .bind(kind.as_str())
        .bind(email)
        .fetch_all(db)
        .await?;
    Ok(rows)
}

/// The get-by-path draft choreography, shared by every entity's
/// "get by path" route (scripts / flows / apps / variables / resources /
/// schedules / triggers). Given the deployed entity as an `Option` (the
/// caller maps its own "not found" to `None`):
///   - `Some(deployed)` → overlay the authed user's draft (if `get_draft`).
///   - `None` + `get_draft` → fall back to a draft-only response
///     (`no_deployed = true`) when a draft exists, else the caller's 404.
///   - `None` without `get_draft` → the caller's 404.
///
/// `not_found` is only invoked on the 404 paths, so each route keeps its
/// own message. Promoted from the per-handler copies, which had drifted
/// (different 404 text, some missing the draft-only fallback).
pub async fn overlay_or_draft_only<T: serde::Serialize + Send + 'static>(
    db: &DB,
    w_id: &str,
    email: &str,
    kind: UserDraftItemKind,
    path: &str,
    get_draft: bool,
    deployed: Option<T>,
    not_found: impl FnOnce() -> crate::error::Error,
) -> Result<WithDraftOverlay> {
    match deployed {
        Some(deployed) => {
            maybe_overlay_draft(db, w_id, email, kind, path, get_draft, deployed).await
        }
        None if get_draft => fetch_draft_only(db, w_id, email, kind, path)
            .await?
            .ok_or_else(not_found),
        None => Err(not_found()),
    }
}

/// Delete EVERY user's draft (and the legacy NULL-email row) at a
/// path+kind. Use when the underlying item is being DELETED outright:
/// the item is gone for everyone, so leaving teammates' drafts behind
/// would orphan them forever — they'd keep surfacing through
/// `fetch_other_drafts_users` with no item to deploy onto. Discarding
/// one's OWN draft while the item lives on goes through the
/// `save_draft` route with `value: null` instead (caller-scoped).
/// Idempotent on the no-draft case.
pub async fn delete_all_drafts_for_path(
    db: &DB,
    w_id: &str,
    kind: UserDraftItemKind,
    path: &str,
) -> Result<()> {
    sqlx::query!(
        r#"DELETE FROM draft
           WHERE workspace_id = $1
             AND path = $2
             AND typ = $3"#,
        w_id,
        path,
        kind as UserDraftItemKind,
    )
    .execute(db)
    .await?;
    Ok(())
}

/// Discard the deploying user's OWN draft (plus the legacy NULL-email
/// workspace draft) for a path+kind, leaving teammates' drafts intact.
/// Use on RENAME: the item moved to a new path, so the per-user draft
/// left at the old path is orphaned and there's no SQL FK to cascade it.
/// Other users' drafts are independent — they keep theirs and get the
/// StaleDraftModal on their next reload, mirroring the script/flow/app
/// rename path. Idempotent on the no-draft case.
pub async fn delete_own_draft_for_path(
    db: &DB,
    w_id: &str,
    kind: UserDraftItemKind,
    path: &str,
    email: &str,
) -> Result<()> {
    sqlx::query!(
        r#"DELETE FROM draft
           WHERE workspace_id = $1
             AND path = $2
             AND typ = $3
             AND (email = $4 OR email IS NULL)"#,
        w_id,
        path,
        kind as UserDraftItemKind,
        email,
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
    // Same kind gate as the deployed-overlay path above: only the
    // cross-user-visible kinds (script/flow/app) surface who else is
    // editing; the drawer kinds (resource/variable/triggers) keep drafts
    // private to their owner.
    let other_drafts_users = if kind.shares_drafts_across_users() {
        fetch_other_drafts_users(db, w_id, email, kind, path).await?
    } else {
        Vec::new()
    };
    Ok(Some(WithDraftOverlay {
        // Best-effort stand-in for the missing deployed — same JSON as
        // `draft`. Frontend should read `.draft` for the editor state
        // and skip "diff vs deployed" UI when `no_deployed` is set.
        inner: Box::new(draft_json.clone()),
        is_draft: true,
        draft_saved_at: Some(row.created_at),
        no_deployed: true,
        draft: Some(draft_json),
        other_drafts_users,
    }))
}

/// Marker prefix for draft secret values encrypted at rest with the
/// workspace crypt key (`build_crypt`, no key suffix — distinct from the
/// per-root-job `$encrypted:` ciphertexts workers resolve in job args,
/// which never share a table with drafts). Written by `save_draft` for
/// secret variables; resolved back to plaintext by the variable deploy
/// endpoints.
pub const ENCRYPTED_DRAFT_PREFIX: &str = "$encrypted:";

fn draft_decrypt_error() -> crate::error::Error {
    crate::error::Error::BadRequest(
        "An encrypted draft secret could not be decrypted (the workspace encryption key may \
         have changed since the draft was saved). Reset the field and re-enter the secret."
            .to_string(),
    )
}

/// Decrypt a single `$encrypted:`-marked draft value back to plaintext
/// with the workspace crypt key. Fails with a user-facing 400 when the
/// payload doesn't decrypt (e.g. the workspace key was rotated after the
/// draft save).
pub async fn decrypt_draft_secret_value(db: &DB, w_id: &str, value: &str) -> Result<String> {
    let encrypted = value.strip_prefix(ENCRYPTED_DRAFT_PREFIX).unwrap_or(value);
    let mc = crate::variables::build_crypt(db, w_id).await?;
    crate::variables::decrypt(&mc, encrypted.to_string()).map_err(|_| draft_decrypt_error())
}
