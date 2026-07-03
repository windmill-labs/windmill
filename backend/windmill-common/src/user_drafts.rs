/*
 * Author: Diego Imbert
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Shared types and helpers for the per-user `draft` table. Lives in
//! `windmill-common` so entity crates can use it without depending on the
//! top-level `windmill-api` crate. Keep it free of HTTP/axum concerns.

// `DraftUserRef` lives in `windmill-types` (where the list-endpoint row
// structs `ListableScript`/`ListableFlow` declare `Vec<DraftUserRef>` and
// can't reach `windmill-common` without a cycle). Re-exported here so draft
// handlers keep a single import path.
pub use windmill_types::user_drafts::DraftUserRef;

use crate::db::DB;
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Item kinds a user can have an autosaved draft on. Must stay in lockstep
/// with the frontend `USER_DRAFT_ITEM_KINDS` and the Postgres `DRAFT_KIND`
/// enum (adding a kind also needs an `ALTER TYPE ... ADD VALUE` migration).
/// `snake_case` is the shared wire/DB encoding (HTTP params, JSON, `draft.typ`).
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
    /// All unsaved scripts of one data pipeline, bundled into a single draft
    /// keyed at the pipeline's folder path. Not a runnable: it has no deployed
    /// backing table and is private to its owner.
    DataPipeline,
}

impl UserDraftItemKind {
    /// The snake_case wire/DB string, for interpolating into dynamically-built
    /// SQL (`?::DRAFT_KIND` binds want a string).
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
            UserDraftItemKind::DataPipeline => "data_pipeline",
        }
    }

    /// Every variant, for code that must enumerate kinds (e.g. generating
    /// the `draft_only` existence SQL).
    pub const ALL: [UserDraftItemKind; 25] = [
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
        UserDraftItemKind::DataPipeline,
    ];

    /// The deployed table backing this kind, keyed by `(workspace_id, path)`.
    /// SINGLE SOURCE for both the draft access check (which table RLS resolves
    /// item-level `extra_perms` against) and the `draft_only` existence check.
    /// `None` for kinds with no per-path backing table (webhook is a property
    /// of a script/flow row; native triggers are keyed by external_id, not
    /// path) — callers treat that as "no deployed counterpart": `draft_only =
    /// true` and a path-only access check.
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
            // Keyed at a folder path, not a runnable; access falls back to the
            // path-only (folder write) check.
            DataPipeline => None,
        }
    }

    /// Whether OTHER users' drafts at a path are visible to a viewer (the
    /// "others are editing" list, owner circles, and the `get_draft_for_user`
    /// View JSON / Fork endpoint). Enabled only for the full-page editor items
    /// which have the cross-user draft UI. Drawer items keep drafts private to
    /// their owner: they have no such UI, and exposing a secret variable draft
    /// would hand out the `$encrypted:` ciphertext, which a viewer could
    /// launder into plaintext via a deploy.
    pub fn shares_drafts_across_users(&self) -> bool {
        use UserDraftItemKind::*;
        matches!(self, Script | Flow | App | RawApp)
    }
}

/// Query-string flag accepted by every "get by path" route that supports
/// the draft overlay. `#[serde(flatten)]` into a route-specific query struct
/// when the route has other query fields.
#[derive(Debug, Deserialize, Default)]
pub struct WithDraftQuery {
    /// When true, attach the authed user's draft (if any) as a separate
    /// `draft` field. Defaults to false so non-editor callers see the
    /// deployed shape unchanged.
    #[serde(default)]
    pub get_draft: bool,
}

/// One row of `other_drafts_users`: a draft on the same path owned by
/// someone other than the authed user. `username` is `None` for the legacy
/// NULL-email row, surfaced in the frontend as a "Legacy draft" entry.
#[derive(Debug, Serialize)]
pub struct OtherDraftUser {
    /// `None` represents a legacy workspace-level draft (no owner).
    pub username: Option<String>,
    /// When this user's draft was last saved (the `draft.created_at` upsert
    /// timestamp), surfaced in the fork modal as "Last updated".
    pub draft_saved_at: DateTime<Utc>,
}

/// Response wrapper: the deployed entity untouched plus the authed user's
/// draft (if any) as a sibling `draft` field, which the frontend pairs to
/// diff/restore/discard. The deployed and the draft are NEVER merged on the
/// server — the editor's saved shape can diverge arbitrarily, so any per-kind
/// translation lives in the frontend loader. `inner` is boxed-erased so a
/// possibly MB-scale deployed payload serializes in ONE pass (no
/// `serde_json::Value` round-trip) while keeping the struct non-generic.
#[derive(Serialize)]
pub struct WithDraftOverlay {
    /// Deployed payload, flattened to the top level.
    #[serde(flatten)]
    pub inner: Box<dyn erased_serde::Serialize + Send>,
    pub is_draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_saved_at: Option<DateTime<Utc>>,
    /// True when no deployed row exists at this path: `inner` is only a
    /// best-effort stand-in synthesized from the draft and only `draft` is
    /// canonical. Frontend uses this to disable "diff/reset vs deployed" and
    /// skip its deployed-shape parsing of `inner`. Omitted when false.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub no_deployed: bool,
    /// The user's saved draft payload (whatever shape the editor wrote).
    /// Present when `get_draft=true` and a draft exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft: Option<serde_json::Value>,
    /// Other users with a draft on the same path (excludes the authed user).
    /// Empty list is omitted to keep the common-case response lean.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub other_drafts_users: Vec<OtherDraftUser>,
}

/// List every other user (and the legacy NULL-email row, if any) with a
/// draft at `(workspace, kind, path)`. Returns usernames only — emails never
/// leave the server. LEFT JOIN against `usr` so an orphaned draft (user
/// removed from the workspace) still surfaces with `username = None`. The
/// authed user is excluded via `email <> authed_email`; the legacy row
/// matches because `email IS NULL` fails that comparison.
async fn fetch_other_drafts_users(
    db: &DB,
    w_id: &str,
    authed_email: &str,
    kind: UserDraftItemKind,
    path: &str,
) -> Result<Vec<OtherDraftUser>> {
    // A superadmin authoring in a workspace they are not a member of has no `usr`
    // row: fall back to their instance-derived username (`password.username`), or
    // their email when derivation is disabled. Else a real teammate's draft renders
    // as a phantom "Legacy draft". The genuine NULL-email legacy row keeps
    // `username = None` (no `usr`/`password` match and `d.email` is NULL).
    let rows = sqlx::query_as!(
        OtherDraftUser,
        r#"SELECT COALESCE(u.username, p.username, CASE WHEN p.email IS NOT NULL THEN d.email END) as "username?",
                  d.created_at as "draft_saved_at!"
           FROM draft d
           LEFT JOIN usr u
                  ON u.workspace_id = d.workspace_id
                 AND u.email = d.email
           LEFT JOIN password p
                  ON p.email = d.email
                 AND p.super_admin = true
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

/// If `get_draft` is true AND the authed user has a draft for
/// `(workspace, kind, path)`, attach it as `draft`. `deployed` is always
/// serialized into `inner` untouched.
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
    // Non-editor callers (worker/CLI reads of possibly MB-scale flows/apps)
    // pass `get_draft = false` and render no overlay, so skip the `usr` join.
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

    // Independent of the authed user's OWN draft: reset-to-deployed reloads
    // still need to know who else is editing this path. Only the cross-user
    // kinds surface it (see `shares_drafts_across_users`).
    let other_drafts_users = if kind.shares_drafts_across_users() {
        fetch_other_drafts_users(db, w_id, email, kind, path).await?
    } else {
        Vec::new()
    };

    // Prefer the user's OWN per-user draft, falling back to the legacy
    // NULL-email workspace draft. `NULLS LAST` + `LIMIT 1` drops the legacy
    // row when an owned one exists.
    let row = sqlx::query!(
        r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>",
                  created_at
           FROM draft
           WHERE workspace_id = $1
             AND (email = $2 OR email IS NULL)
             AND path = $3
             AND typ = $4
           ORDER BY email NULLS LAST
           LIMIT 1"#,
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

/// One row of a "draft-only" list synthesis: a draft at `path` with no
/// deployed counterpart. `value` is the editor's saved JSON (each handler
/// maps it into its own `Listable*` shape).
#[derive(sqlx::FromRow)]
pub struct DraftOnlyListRow {
    pub path: String,
    pub value: sqlx::types::Json<Box<serde_json::value::RawValue>>,
    pub created_at: DateTime<Utc>,
}

/// Fetch the authed user's draft rows at paths with NO deployed counterpart,
/// for synthesizing draft-only entries into a list response. Absence is
/// checked against `kind.deployed_table()` (the shared single source).
/// Returns empty for kinds with no path-keyed table. Callers keep their own
/// gating (`include_draft_only`, page 0, no filters) and row mapping.
pub async fn fetch_draft_only_list_rows(
    db: &DB,
    w_id: &str,
    email: &str,
    kind: UserDraftItemKind,
) -> Result<Vec<DraftOnlyListRow>> {
    let Some(table) = kind.deployed_table() else {
        return Ok(Vec::new());
    };
    // `table` is from the closed `deployed_table()` enum, never user input.
    // `(email = $3 OR email IS NULL)` surfaces the user's own draft-only rows
    // AND the legacy NULL-email rows; `DISTINCT ON (path)` with `email IS NULL`
    // last collapses a path that has both to the owned row.
    let sql = format!(
        "SELECT DISTINCT ON (path) path, value, created_at FROM draft \
         WHERE workspace_id = $1 AND typ = $2::text::DRAFT_KIND \
           AND (email = $3 OR email IS NULL) \
           AND NOT EXISTS (SELECT 1 FROM {table} t \
             WHERE t.workspace_id = draft.workspace_id AND t.path = draft.path) \
         ORDER BY path, (email IS NULL)"
    );
    let rows = sqlx::query_as::<_, DraftOnlyListRow>(&sql)
        .bind(w_id)
        .bind(kind.as_str())
        .bind(email)
        .fetch_all(db)
        .await?;
    Ok(rows)
}

/// The get-by-path draft choreography, shared by every entity's "get by path"
/// route. Given the deployed entity as an `Option` (caller maps its own "not
/// found" to `None`):
///   - `Some(deployed)` → overlay the authed user's draft (if `get_draft`).
///   - `None` + `get_draft` → draft-only response (`no_deployed = true`) when
///     a draft exists, else the caller's 404 via `not_found`.
///   - `None` without `get_draft` → the caller's 404.
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

/// Delete EVERY user's draft (and the legacy NULL-email row) at a path+kind.
/// Use when the item is DELETED outright: it's gone for everyone, so leaving
/// teammates' drafts behind would orphan them forever. Discarding one's OWN
/// draft while the item lives on goes through `update_draft` with `value: null`.
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

/// Discard the deploying user's OWN draft (plus the legacy NULL-email row)
/// for a path+kind, leaving teammates' drafts intact. Use on RENAME: the
/// item moved, so the draft at the old path is orphaned (no FK to cascade).
/// Teammates keep theirs and get the StaleDraftModal on their next reload.
/// Idempotent on the no-draft case.
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

/// Fetch the authed user's draft as a standalone payload, for "get by path"
/// routes when no deployed row exists but a draft might. Returns it as a
/// `WithDraftOverlay` with `inner` and `draft` both set to the same JSON and
/// `no_deployed = true`. Callers must have established no deployed row exists;
/// `Ok(None)` when there's also no draft (caller should 404).
///
/// The draft JSON is expected to be an object (so `serde(flatten)` on `inner`
/// works); a non-object draft renders with no fields flattened.
pub async fn fetch_draft_only(
    db: &DB,
    w_id: &str,
    email: &str,
    kind: UserDraftItemKind,
    path: &str,
) -> Result<Option<WithDraftOverlay>> {
    // Own draft first, legacy NULL-email row as fallback (see `maybe_overlay_draft`).
    let row = sqlx::query!(
        r#"SELECT value as "value!: sqlx::types::Json<Box<serde_json::value::RawValue>>",
                  created_at
           FROM draft
           WHERE workspace_id = $1
             AND (email = $2 OR email IS NULL)
             AND path = $3
             AND typ = $4
           ORDER BY email NULLS LAST
           LIMIT 1"#,
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
    let other_drafts_users = if kind.shares_drafts_across_users() {
        fetch_other_drafts_users(db, w_id, email, kind, path).await?
    } else {
        Vec::new()
    };
    Ok(Some(WithDraftOverlay {
        // Best-effort stand-in for the missing deployed — same JSON as `draft`.
        inner: Box::new(draft_json.clone()),
        is_draft: true,
        draft_saved_at: Some(row.created_at),
        no_deployed: true,
        draft: Some(draft_json),
        other_drafts_users,
    }))
}

/// Marker prefix for draft secret values encrypted at rest with the workspace
/// crypt key (`build_crypt`). Written by `update_draft` for secret variables;
/// resolved back to plaintext by the variable deploy endpoints.
pub const ENCRYPTED_DRAFT_PREFIX: &str = "$encrypted:";

fn draft_decrypt_error() -> crate::error::Error {
    crate::error::Error::BadRequest(
        "An encrypted draft secret could not be decrypted (the workspace encryption key may \
         have changed since the draft was saved). Reset the field and re-enter the secret."
            .to_string(),
    )
}

/// Decrypt a `$encrypted:`-marked draft value back to plaintext with the
/// workspace crypt key. Fails with a user-facing 400 when it doesn't decrypt
/// (e.g. the workspace key was rotated after the draft save).
pub async fn decrypt_draft_secret_value(db: &DB, w_id: &str, value: &str) -> Result<String> {
    let encrypted = value.strip_prefix(ENCRYPTED_DRAFT_PREFIX).unwrap_or(value);
    let mc = crate::variables::build_crypt(db, w_id).await?;
    crate::variables::decrypt(&mc, encrypted.to_string()).map_err(|_| draft_decrypt_error())
}
