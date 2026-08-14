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
//!
//! # Authorization contract
//!
//! None of the helpers here authorize anything: they take a connection and
//! write what they are given, exactly like `audit_log`. A caller must already
//! have authorized the mutation *and* performed it, and must derive `username`
//! from the request's `ApiAuthed` and `source` from
//! [`TriggerSource::of_request`] — never from anything the request body
//! carries. Reads are gated separately, by the RLS policies on the table and by
//! the token scopes the listing route checks.

use sqlx::PgConnection;

use crate::error::Result;

/// Header a first-party client sets to name itself. Only `cli`, `ui` and `api`
/// mean anything; any other value, and the header being absent, falls back to
/// what the credentials say.
pub const CLIENT_HEADER: &str = "x-windmill-client";

/// `trigger_kind` a schedule is recorded under. Triggers use their own
/// `TriggerCrud::TRIGGER_TYPE`.
pub const SCHEDULE_TRIGGER_KIND: &str = "schedule";

/// The kind of client a trigger mutation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    // Listener runtime state like the two above: every trigger update clears it,
    // so keeping it here would tag an ordinary edit with the failure it had
    // before. The server-initiated disables put the error in `changes`
    // themselves, so nothing is lost.
    "error",
    // Written from the requester on every schedule mutation, purely for workers
    // that predate `permissioned_as`; it tracks the editor, not the schedule.
    "email",
];

/// A `changes` payload bigger than this is replaced by the list of field names
/// it would have held. A schedule's `args` is caller-supplied and bounded only
/// by the API's request-size limit, and a history row is not worth a
/// multi-megabyte write.
const MAX_CHANGES_BYTES: usize = 32 * 1024;

/// The row at `path` as JSON, or `None` when there is none — which, on an RLS
/// connection, also covers a row the caller cannot see.
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

/// A field-level diff of two row snapshots, as `{field: {"old": …, "new": …}}`,
/// with `"old"` omitted where there is none to report.
///
/// A create (`before` absent) keeps every non-null column of the new row, which
/// is its initial shape including whatever the column defaults supplied —
/// `to_jsonb` cannot tell a caller-set column from a defaulted one. Returns
/// `None` when nothing meaningful changed.
pub fn summarize_changes(
    before: Option<&serde_json::Value>,
    after: Option<&serde_json::Value>,
) -> Option<serde_json::Value> {
    let empty = serde_json::Map::new();
    let before = before.and_then(|v| v.as_object()).unwrap_or(&empty);
    let after = after.and_then(|v| v.as_object())?;

    // Names of the changed fields, and the running size of what has been cloned
    // so far. Measured as it goes rather than by serializing the finished map:
    // a caller-sized `args` would otherwise be cloned in full and then copied
    // again just to learn it was too big.
    let mut fields = Vec::new();
    let mut changes = serde_json::Map::new();
    let mut bytes = 0usize;
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
        fields.push(field.clone());
        if bytes <= MAX_CHANGES_BYTES {
            bytes += json_len(new_value) + old_value.map_or(0, json_len) + field.len();
        }
        if bytes > MAX_CHANGES_BYTES {
            continue;
        }
        let mut entry = serde_json::Map::new();
        if let Some(old_value) = old_value {
            entry.insert("old".to_string(), old_value.clone());
        }
        entry.insert("new".to_string(), new_value.clone());
        changes.insert(field.clone(), serde_json::Value::Object(entry));
    }

    if fields.is_empty() {
        return None;
    }
    if bytes > MAX_CHANGES_BYTES {
        return Some(serde_json::json!({ "truncated_fields": fields }));
    }
    Some(serde_json::Value::Object(changes))
}

/// Serialized size of `value` without building the string for it.
fn json_len(value: &serde_json::Value) -> usize {
    struct Counter(usize);
    impl std::io::Write for Counter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0 += buf.len();
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    let _ = serde_json::to_writer(&mut counter, value);
    counter.0
}

/// The last word on payload size, applied at the write itself so a hand-built
/// `changes` (the server-initiated disables carry an error string of unknown
/// length) is bounded too, not just a computed diff.
fn cap_changes(changes: Option<serde_json::Value>) -> Option<serde_json::Value> {
    let changes = changes?;
    if json_len(&changes) <= MAX_CHANGES_BYTES {
        return Some(changes);
    }
    let fields = changes
        .as_object()
        .map(|o| o.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    Some(serde_json::json!({ "truncated_fields": fields }))
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

impl<'a> TriggerHistoryEvent<'a> {
    /// The event for a trigger the server disabled on its own after a failure.
    ///
    /// `forced_state` is the column the disable wrote, in the same
    /// `{field: {old, new}}` shape as a diff — the two disable paths write
    /// different columns (`enabled` for a schedule, `mode` for a trigger).
    pub fn server_disable(
        workspace_id: &'a str,
        trigger_kind: &'a str,
        path: &'a str,
        mut forced_state: serde_json::Value,
        error: &str,
    ) -> Self {
        if let Some(obj) = forced_state.as_object_mut() {
            obj.insert("error".to_string(), serde_json::json!({ "new": error }));
        }
        Self {
            workspace_id,
            trigger_kind,
            path,
            operation: TriggerOperation::Disable,
            source: TriggerSource::Worker,
            username: None,
            changes: Some(forced_state),
        }
    }
}

/// Append `event` to the history.
///
/// Pass the same connection as the mutation for the two to commit together.
/// Does not authorize — see the module docs.
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
        cap_changes(event.changes) as _,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Append one row per path, all describing the same change.
///
/// For the workspace-wide operations that rewrite every schedule at once, where
/// a per-path diff would cost a snapshot per row and say the same thing each
/// time. Does not authorize — see the module docs.
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
        cap_changes(changes) as _,
        paths,
    )
    .execute(&mut *conn)
    .await?;
    Ok(())
}

/// Append `event` on its own connection, logging rather than propagating a
/// failure.
///
/// For the server-initiated disables that hold no transaction of their own: a
/// history row is not worth failing the caller over. **Only** for those — taking
/// a second pooled connection while a transaction is held is how a
/// pool-exhaustion deadlock starts, so a caller inside one records through a
/// savepoint on its own transaction instead. Does not authorize — see the
/// module docs.
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

    /// `error` and `edited_at` stand in for the whole ignore list: every trigger
    /// update clears `error`, so without it an ordinary edit would carry the
    /// failure the trigger had before it.
    #[test]
    fn diff_keeps_only_what_changed() {
        let before = json!({"schedule": "0 0 * * *", "enabled": true, "edited_at": "a", "error": "boom"});
        let after = json!({"schedule": "0 1 * * *", "enabled": true, "edited_at": "b", "error": null});
        assert_eq!(
            summarize_changes(Some(&before), Some(&after)),
            Some(json!({"schedule": {"old": "0 0 * * *", "new": "0 1 * * *"}}))
        );
        assert_eq!(summarize_changes(Some(&before), Some(&before)), None);
    }

    #[test]
    fn create_drops_null_columns_and_bookkeeping() {
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
