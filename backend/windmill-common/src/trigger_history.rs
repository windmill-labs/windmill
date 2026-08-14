/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Append-only history of schedule and trigger mutations (`trigger_history`).
//!
//! Every field of a row is derived by the server at write time: the caller
//! passes what it is doing, never who it claims to be or where it claims to
//! come from. A history row therefore says three things a caller cannot forge:
//! **who** (the authed username, or nobody for a server-initiated change),
//! **what** (a field-level diff computed from the row before and after the
//! write), and **from what kind of client** ([`TriggerSource`]).

use serde::{Deserialize, Serialize};
use sqlx::PgConnection;

use crate::error::Result;

/// Header a first-party client sets to name itself. Only the values in
/// [`TriggerSource::from_client_header`] mean anything; anything else, header
/// absent included, falls back to what the credentials say.
pub const CLIENT_HEADER: &str = "x-windmill-client";

/// `trigger_kind` a schedule is recorded under. Triggers use their own
/// `TriggerCrud::TRIGGER_TYPE`.
pub const SCHEDULE_TRIGGER_KIND: &str = "schedule";

/// The kind of client a trigger mutation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerSource {
    /// A browser session in the Windmill app.
    Ui,
    /// The `wmill` CLI (including the git-sync pull that shells out to it).
    Cli,
    /// A direct API call with a token: user scripts, CI, third-party clients.
    Api,
    /// No request at all: a worker or a trigger listener disabling something
    /// after a failure.
    Worker,
}

impl TriggerSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerSource::Ui => "ui",
            TriggerSource::Cli => "cli",
            TriggerSource::Api => "api",
            TriggerSource::Worker => "worker",
        }
    }

    fn from_client_header(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cli" => Some(TriggerSource::Cli),
            "ui" => Some(TriggerSource::Ui),
            "api" => Some(TriggerSource::Api),
            _ => None,
        }
    }

    /// The source of the request currently being served.
    ///
    /// The declared client wins when it is one we know; otherwise the token
    /// decides, and only the session token minted at browser login attributes
    /// to the UI. Both inputs are attribution, never authority — nothing reads
    /// a history row to make an access decision, so a caller lying about either
    /// only mislabels its own row.
    pub fn of_request(is_session_token: bool) -> Self {
        match REQUEST_CLIENT.try_with(|client| *client) {
            Ok(Some(source)) => source,
            Ok(None) if is_session_token => TriggerSource::Ui,
            Ok(None) => TriggerSource::Api,
            // Outside a request there is no caller to attribute to. The
            // server-initiated paths pass `Worker` themselves; this is what
            // keeps a stray call from inventing one.
            Err(_) => TriggerSource::Worker,
        }
    }
}

/// What a mutation did to the trigger it is recorded against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerOperation {
    Create,
    Update,
    Delete,
    Enable,
    Disable,
    Suspend,
}

impl TriggerOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            TriggerOperation::Create => "create",
            TriggerOperation::Update => "update",
            TriggerOperation::Delete => "delete",
            TriggerOperation::Enable => "enable",
            TriggerOperation::Disable => "disable",
            TriggerOperation::Suspend => "suspend",
        }
    }
}

tokio::task_local! {
    static REQUEST_CLIENT: Option<TriggerSource>;
}

/// Run `f` with `client` as the declared client of every trigger mutation it
/// causes. Entered for every request, unmarked ones included, so that having no
/// scope at all means "not serving a request" — which is what
/// [`TriggerSource::Worker`] records.
pub async fn scope_client<F: std::future::Future>(
    client: Option<TriggerSource>,
    f: F,
) -> F::Output {
    REQUEST_CLIENT.scope(client, f).await
}

/// Parse the declared client of the request being served, if any.
pub fn client_from_header(value: &str) -> Option<TriggerSource> {
    TriggerSource::from_client_header(value)
}

/// Row fields that say nothing about the change itself: bookkeeping the history
/// row already carries, and listener runtime state that moves on its own.
const IGNORED_FIELDS: &[&str] = &[
    "workspace_id",
    "edited_at",
    "edited_by",
    "extra_perms",
    "last_server_ping",
    "server_id",
    // Written from the requester on every schedule mutation, purely for workers
    // that predate `permissioned_as`; it tracks the editor, not the schedule.
    "email",
];

/// A `changes` payload bigger than this is replaced by the list of field names
/// it would have held. A schedule's `args` is caller-supplied and unbounded,
/// and a history row is not worth a multi-megabyte write.
const MAX_CHANGES_BYTES: usize = 32 * 1024;

/// The row at `path` as JSON, or `None` when there is none.
///
/// `table` is interpolated: pass a compile-time constant, never anything a
/// caller can reach.
pub async fn snapshot_row(
    conn: &mut PgConnection,
    table: &'static str,
    workspace_id: &str,
    path: &str,
) -> Result<Option<serde_json::Value>> {
    // SAFETY: `table` is a compile-time constant.
    let snapshot: Option<serde_json::Value> = sqlx::query_scalar(&format!(
        "SELECT to_jsonb(t) FROM {table} t WHERE workspace_id = $1 AND path = $2"
    ))
    .bind(workspace_id)
    .bind(path)
    .fetch_optional(&mut *conn)
    .await?;
    Ok(snapshot)
}

/// A field-level diff of two row snapshots, as `{field: {"old": …, "new": …}}`.
///
/// A create (`before` absent) keeps only the fields that were actually set, so
/// the row records the trigger's initial shape rather than every column
/// default. Returns `None` when nothing meaningful changed.
pub fn summarize_changes(
    before: Option<&serde_json::Value>,
    after: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let empty = serde_json::Map::new();
    let before = before.and_then(|v| v.as_object()).unwrap_or(&empty);
    let after = after.and_then(|v| v.as_object())?;

    let mut changes = serde_json::Map::new();
    for (field, new_value) in after {
        if IGNORED_FIELDS.contains(&field.as_str()) {
            continue;
        }
        let old_value = before.get(field);
        match old_value {
            Some(old_value) if old_value == new_value => continue,
            None if new_value.is_null() => continue,
            _ => {}
        }
        let mut entry = serde_json::Map::new();
        if let Some(old_value) = old_value {
            entry.insert("old".to_string(), old_value.clone());
        }
        entry.insert("new".to_string(), new_value.clone());
        changes.insert(field.clone(), serde_json::Value::Object(entry));
    }

    if changes.is_empty() {
        return None;
    }
    let value = serde_json::Value::Object(changes);
    Some(truncate_changes(value))
}

fn truncate_changes(changes: serde_json::Value) -> serde_json::Value {
    if serde_json::to_string(&changes).map_or(0, |s| s.len()) <= MAX_CHANGES_BYTES {
        return changes;
    }
    let fields = changes
        .as_object()
        .map(|o| o.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    serde_json::json!({ "truncated_fields": fields })
}

/// One trigger mutation, as it is about to be recorded.
pub struct TriggerHistoryEvent<'a> {
    pub workspace_id: &'a str,
    /// `"schedule"`, or the trigger's `TRIGGER_TYPE` (`"http"`, `"kafka"`, …).
    pub trigger_kind: &'a str,
    pub path: &'a str,
    pub operation: TriggerOperation,
    pub source: TriggerSource,
    /// `None` when the server acted on its own.
    pub username: Option<&'a str>,
    pub changes: Option<serde_json::Value>,
}

/// Append `event` to the history.
///
/// Pass the same connection as the mutation for the two to commit together.
/// The worker paths deliberately do not: see [`record_best_effort`].
pub async fn record(conn: &mut PgConnection, event: TriggerHistoryEvent<'_>) -> Result<()> {
    sqlx::query!(
        "INSERT INTO trigger_history
            (workspace_id, trigger_kind, path, operation, source, username, changes)
         VALUES ($1, $2, $3, $4, $5, $6, $7)",
        event.workspace_id,
        event.trigger_kind,
        event.path,
        event.operation.as_str(),
        event.source.as_str(),
        event.username,
        event.changes,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Append one row per path, all describing the same change.
///
/// For the workspace-wide operations that rewrite every schedule at once, where
/// a per-path diff would cost a snapshot per row and say the same thing each
/// time.
pub async fn record_bulk(
    conn: &mut PgConnection,
    workspace_id: &str,
    trigger_kind: &str,
    paths: &[String],
    operation: TriggerOperation,
    source: TriggerSource,
    username: Option<&str>,
    changes: Option<serde_json::Value>,
) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    sqlx::query!(
        "INSERT INTO trigger_history
            (workspace_id, trigger_kind, path, operation, source, username, changes)
         SELECT $1, $2, p, $3, $4, $5, $6 FROM unnest($7::text[]) p",
        workspace_id,
        trigger_kind,
        operation.as_str(),
        source.as_str(),
        username,
        changes,
        paths,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Append `event` on its own connection, logging rather than propagating a
/// failure.
///
/// For the server-initiated disables, which run inside the transaction that
/// completes a job: a failed statement there would poison that transaction and
/// take the job down with it, and a history row is not worth that. The cost is
/// that a history row can outlive a rolled-back disable.
pub async fn record_best_effort(db: &crate::DB, event: TriggerHistoryEvent<'_>) {
    let mut conn = match db.acquire().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("could not record trigger history for {}: {e:#}", event.path);
            return;
        }
    };
    if let Err(e) = record(&mut conn, event).await {
        tracing::error!("could not record trigger history: {e:#}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// The whole worker side of the attribution rests on this: a mutation made
    /// outside a request records `worker` without each call site saying so.
    #[tokio::test]
    async fn client_is_absent_outside_a_request() {
        assert_eq!(TriggerSource::of_request(false), TriggerSource::Worker);
        assert_eq!(
            scope_client(None, async { TriggerSource::of_request(true) }).await,
            TriggerSource::Ui
        );
        assert_eq!(
            scope_client(None, async { TriggerSource::of_request(false) }).await,
            TriggerSource::Api
        );
        assert_eq!(
            scope_client(Some(TriggerSource::Cli), async {
                TriggerSource::of_request(true)
            })
            .await,
            TriggerSource::Cli
        );
    }

    #[test]
    fn diff_keeps_only_what_changed() {
        let before = json!({"schedule": "0 0 * * *", "enabled": true, "edited_at": "a"});
        let after = json!({"schedule": "0 1 * * *", "enabled": true, "edited_at": "b"});
        assert_eq!(
            summarize_changes(Some(&before), Some(&after)),
            Some(json!({"schedule": {"old": "0 0 * * *", "new": "0 1 * * *"}}))
        );
        assert_eq!(summarize_changes(Some(&before), Some(&before)), None);
    }

    #[test]
    fn create_records_only_the_fields_that_were_set() {
        let after = json!({"schedule": "0 0 * * *", "summary": null, "workspace_id": "w"});
        assert_eq!(
            summarize_changes(None, Some(&after)),
            Some(json!({"schedule": {"new": "0 0 * * *"}}))
        );
    }

    #[test]
    fn oversized_changes_keep_the_field_names() {
        let after = json!({ "args": "x".repeat(MAX_CHANGES_BYTES + 1) });
        assert_eq!(
            summarize_changes(None, Some(&after)),
            Some(json!({"truncated_fields": ["args"]}))
        );
    }
}
