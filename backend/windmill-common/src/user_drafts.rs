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
    /// When true, overlay the authed user's draft for this entity (if any)
    /// onto the deployed payload before serializing. Defaults to false so
    /// non-editor callers see the deployed shape unchanged.
    #[serde(default)]
    pub get_draft: bool,
}

/// Response wrapper that flattens its inner payload alongside the
/// `is_draft` / `draft_saved_at` overlay fields.
///
/// Wire shape is `<inner fields...> + is_draft + draft_saved_at?` — i.e.
/// callers that ignore the overlay fields keep getting the same response
/// they used to. `draft_saved_at` is omitted on `is_draft = false`.
///
/// `inner` is held as `serde_json::Value` so the caller only needs
/// `Serialize` on its response type — most read-only response shapes
/// (e.g. `ScriptWithStarred`) only derive `Serialize`, and requiring
/// `DeserializeOwned` would force derive cascades through many crates.
#[derive(Debug, Serialize)]
pub struct WithDraftOverlay {
    #[serde(flatten)]
    pub inner: serde_json::Value,
    pub is_draft: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub draft_saved_at: Option<DateTime<Utc>>,
    /// True when the response is the user's draft only — no deployed row
    /// exists at this path. Frontend uses this to hide "Reset to deployed"
    /// actions that have nothing to fall back to. Omitted from the wire
    /// when false to keep the no-overlay shape unchanged.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub no_deployed: bool,
}

/// If `get_draft` is true AND the authed user has a draft saved for
/// `(workspace, kind, path)`, deep-merge the draft JSON onto the
/// serialized form of `deployed` and return the result with
/// `is_draft = true`. Otherwise return the deployed payload unchanged.
///
/// **Merge semantics** are draft-wins-per-key with object recursion:
/// fields the user touched (editor fields like `content`, `summary`, ...)
/// come from the draft; fields the user didn't touch (hashes, ownership,
/// timestamps) fall through from the deployed row. Arrays are replaced
/// wholesale, not concatenated.
///
/// `deployed` is serialized once up-front, so the on-wire shape is
/// identical between the no-draft and the overlay branches.
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

    if !get_draft {
        return Ok(WithDraftOverlay {
            inner,
            is_draft: false,
            draft_saved_at: None,
            no_deployed: false,
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
        });
    };

    let mut merged = inner;
    let patch: serde_json::Value = serde_json::from_str(row.value.0.get())?;
    deep_merge(&mut merged, patch);

    Ok(WithDraftOverlay {
        inner: merged,
        is_draft: true,
        draft_saved_at: Some(row.created_at),
        no_deployed: false,
    })
}

/// Recursive object merge: `source` wins at every overlapping key,
/// scalars/arrays replace wholesale, missing keys from `source` leave
/// `target` untouched.
fn deep_merge(target: &mut serde_json::Value, source: serde_json::Value) {
    use serde_json::Value;
    match (target, source) {
        (Value::Object(t), Value::Object(s)) => {
            for (k, v) in s {
                deep_merge(t.entry(k).or_insert(Value::Null), v);
            }
        }
        (t, s) => *t = s,
    }
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
/// draft might. Returns the draft JSON wrapped as `WithDraftOverlay`
/// with `is_draft = true`, so the response shape matches the overlay
/// path the handler uses when a deployed row IS present.
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

    let inner: serde_json::Value = serde_json::from_str(row.value.0.get())?;
    Ok(Some(WithDraftOverlay {
        inner,
        is_draft: true,
        draft_saved_at: Some(row.created_at),
        no_deployed: true,
    }))
}
