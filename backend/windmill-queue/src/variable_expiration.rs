/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Detection pass for variables whose value is about to expire.
//!
//! Owns the transaction that claims a variable and pushes its handler job, so it lives in
//! this crate beside `push_variable_expiration_handler` rather than in the server's `monitor`
//! module, which only drives it on a timer.

use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use serde_json::value::RawValue;
use sqlx::types::Json;
use windmill_common::utils::report_critical_error;
use windmill_common::DB;

use crate::jobs::{push_variable_expiration_handler, ExpiringVariable};

/// How far ahead of a variable's `value_expires_at` its handler runs.
const VARIABLE_EXPIRATION_LEAD_TIME: &str = "1 hour";

/// Cap on variables dispatched per pass, so a large first-pass backlog is drained over
/// several passes rather than enqueuing every handler job at once.
const VARIABLE_EXPIRATION_MAX_PER_PASS: i64 = 100;

/// Advisory lock id ensuring only one server replica sweeps at a time (adjacent to
/// `SCHEDULE_RECONCILE_LOCK_ID`).
const VARIABLE_EXPIRATION_LOCK_ID: i64 = 737_483_923;

/// Consecutive passes a workspace's handler must fail before the failure is surfaced once as
/// a critical alert. Most push failures are a transient blip; one that keeps failing has a
/// real cause — the handler script is gone, or its stored path does not resolve — and would
/// otherwise leave every expiry in that workspace silently undispatched.
const VARIABLE_EXPIRATION_ALERT_THRESHOLD: u32 = 3;

/// Cap on the exponential back-off (in passes) between retries of a workspace whose handler
/// keeps failing. The delay grows 2, 4, 8 and holds here, and resets as soon as one of its
/// pushes succeeds.
const VARIABLE_EXPIRATION_MAX_BACKOFF_PASSES: u32 = 8;

/// State for a workspace whose handler keeps failing to push: paces retries and drives the
/// one-time visibility so the loop is neither hot nor silent.
#[derive(Default)]
struct HandlerFailureState {
    /// The handler these failures were counted against. A workspace that points at a
    /// different one has not failed yet, so the state is discarded rather than aged out.
    handler_path: String,
    consecutive_failures: u32,
    /// Passes still to skip before the next attempt (exponential back-off).
    cooldown_passes: u32,
    /// Whether the persistent failure has already been surfaced.
    surfaced: bool,
}

lazy_static::lazy_static! {
    /// `workspace_id` -> back-off/visibility state. Keyed by workspace rather than by
    /// variable because the handler is a workspace setting: when it is unresolvable every
    /// variable in that workspace fails identically, and one alert names the cause where a
    /// hundred per-row errors would only repeat it.
    ///
    /// Per-replica, like `SCHEDULE_REARM_FAILURES`: the advisory lock serializes concurrent
    /// sweeps but does not pin successive ones to one process, so a broken handler is retried
    /// and alerted once per replica before all have backed off. Pacing only — no double
    /// dispatch rides on it, as the claim is persisted in `expiration_dispatched_for`.
    static ref VARIABLE_EXPIRATION_FAILURES: Mutex<HashMap<String, HandlerFailureState>> =
        Mutex::new(HashMap::new());
}

/// Enqueue the workspace variable expiration handler for every variable coming due.
///
/// Not an authorization boundary: it dispatches handlers across every workspace, so this is
/// a system caller (the monitor loop) only.
///
/// Detection rather than pre-enqueue: pushing a future-dated job would pin the handler's
/// script version months ahead and turn this into a reconciliation problem (see
/// `reconcile_unarmed_schedules`) instead of a detection one.
///
/// Dispatch-once, not run-to-success: the claim is committed with the push, so a handler job
/// that then fails is not re-dispatched for that date. Retrying is the handler's own job (a
/// retry policy, or the workspace error handler) — re-running it here would re-run whatever
/// side effects the failed attempt already had against the third party it rotates.
pub async fn dispatch_expiring_variables(db: &DB) {
    // Transaction-scoped advisory lock, not session-scoped: a session lock taken on a pooled
    // connection would ride that connection back into the pool still held if this future is
    // dropped mid-pass, wedging the sweep on every replica until restart. The tx is held open
    // only to own the lock.
    let mut lock_tx = match db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!("variable expiration: failed to begin lock tx: {e:#}");
            return;
        }
    };
    let locked: bool = match sqlx::query_scalar("SELECT pg_try_advisory_xact_lock($1)")
        .bind(VARIABLE_EXPIRATION_LOCK_ID)
        .fetch_one(&mut *lock_tx)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("variable expiration: advisory lock failed: {e:#}");
            return;
        }
    };
    if !locked {
        // Another replica is already sweeping this tick.
        return;
    }

    dispatch_expiring_variables_inner(db).await;

    // Ends the transaction and releases the xact lock; a plain drop would too.
    if let Err(e) = lock_tx.rollback().await {
        tracing::error!("variable expiration: releasing lock failed: {e:#}");
    }
}

async fn dispatch_expiring_variables_inner(db: &DB) {
    // Every workspace with a variable coming due, before any back-off is applied. A
    // deliberate superset of the dispatch predicate below — it skips the mute and the cap,
    // which can only make a workspace spend its back-off more eagerly, never less.
    let with_due = sqlx::query!(
        r#"SELECT DISTINCT v.workspace_id AS "workspace_id!",
                  ws.variable_expiration_handler->>'path' AS "handler_path!"
           FROM variable v
           JOIN workspace_settings ws ON ws.workspace_id = v.workspace_id
           JOIN workspace w ON w.id = v.workspace_id
           WHERE NOT w.deleted
             AND v.value_expires_at IS NOT NULL
             AND v.value_expires_at IS DISTINCT FROM v.expiration_dispatched_for
             AND v.value_expires_at <= now() + ($1::text)::interval
             AND ws.variable_expiration_handler->>'path' IS NOT NULL"#,
        VARIABLE_EXPIRATION_LEAD_TIME,
    )
    .fetch_all(db)
    .await;

    let with_due: HashMap<String, String> = match with_due {
        Ok(rows) => rows
            .into_iter()
            .map(|r| (r.workspace_id, r.handler_path))
            .collect(),
        Err(e) => {
            tracing::error!("Error listing workspaces with expiring variables: {e:#}");
            return;
        }
    };
    if with_due.is_empty() {
        return;
    }

    // Excluded in the query rather than filtered out of its result: the per-pass cap is
    // applied by `LIMIT`, so a workspace whose every push fails would otherwise fill those
    // slots on every pass and starve every other workspace on the instance. Filtering before
    // the cap is the ordering `reconcile_unarmed_schedules_inner` relies on for the same
    // reason.
    //
    // Only a workspace that has work this pass spends a pass of its back-off, so one that
    // goes quiet does not silently serve out its cooldown while nothing is expiring.
    let cooling_down: Vec<String> = {
        let mut failures = VARIABLE_EXPIRATION_FAILURES.lock().unwrap();
        // Back-off paces retries of one handler, so pointing the workspace at another one
        // ends it. Without this a repaired workspace stays out of the sweep for up to
        // `VARIABLE_EXPIRATION_MAX_BACKOFF_PASSES` passes — most of the lead time it has.
        failures.retain(|w_id, state| match with_due.get(w_id) {
            Some(current) => *current == state.handler_path,
            None => true,
        });
        failures
            .iter_mut()
            .filter_map(|(w_id, state)| {
                if !with_due.contains_key(w_id) || state.cooldown_passes == 0 {
                    return None;
                }
                state.cooldown_passes -= 1;
                Some(w_id.clone())
            })
            .collect()
    };

    // The due predicate has no lower bound on `value_expires_at`: an instance that was down
    // catches up rather than silently skipping the variables whose window elapsed meanwhile.
    let due = sqlx::query!(
        r#"SELECT v.workspace_id AS "workspace_id!", v.path AS "path!", v.description AS "description!",
                  v.value_expires_at AS "value_expires_at!", v.is_secret AS "is_secret!",
                  ws.variable_expiration_handler->>'path' AS "handler_path!",
                  (ws.variable_expiration_handler->'extra_args')::text::json AS "extra_args: Json<Box<RawValue>>"
           FROM variable v
           JOIN workspace_settings ws ON ws.workspace_id = v.workspace_id
           JOIN workspace w ON w.id = v.workspace_id
           WHERE NOT w.deleted
             AND v.workspace_id <> ALL($3)
             AND v.value_expires_at IS NOT NULL
             AND v.value_expires_at IS DISTINCT FROM v.expiration_dispatched_for
             AND v.value_expires_at <= now() + ($1::text)::interval
             AND ws.variable_expiration_handler->>'path' IS NOT NULL
             AND (NOT COALESCE((ws.variable_expiration_handler->>'muted_on_user_path')::boolean, false)
                  OR v.path NOT LIKE 'u/%')
           ORDER BY v.value_expires_at
           LIMIT $2"#,
        VARIABLE_EXPIRATION_LEAD_TIME,
        VARIABLE_EXPIRATION_MAX_PER_PASS,
        &cooling_down,
    )
    .fetch_all(db)
    .await;

    let due = match due {
        Ok(due) => due,
        Err(e) => {
            tracing::error!("Error listing expiring variables: {e:#}");
            return;
        }
    };

    let mut dispatched = 0usize;
    // A workspace's handler resolves the same way for every one of its variables, so once it
    // has failed this pass the rest of its rows would only repeat the same error.
    let mut failed_this_pass: HashSet<String> = HashSet::new();

    for row in due {
        if failed_this_pass.contains(&row.workspace_id) {
            continue;
        }

        // Claim and push in one transaction. Storing the date that was dispatched makes the
        // claim lose both to a concurrent re-arm (the `value_expires_at` guard) and to
        // whichever replica got there first.
        let mut tx = match db.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("Error opening tx for expiring variable {}: {e:#}", row.path);
                continue;
            }
        };

        let claimed = sqlx::query_scalar!(
            "UPDATE variable SET expiration_dispatched_for = $3
             WHERE workspace_id = $1 AND path = $2 AND value_expires_at = $3
               AND expiration_dispatched_for IS DISTINCT FROM $3
             RETURNING 1",
            row.workspace_id,
            row.path,
            row.value_expires_at,
        )
        .fetch_optional(&mut *tx)
        .await;

        match claimed {
            Ok(Some(_)) => {}
            Ok(None) => continue,
            Err(e) => {
                tracing::error!("Error claiming expiring variable {}: {e:#}", row.path);
                continue;
            }
        }

        let expiring = ExpiringVariable {
            workspace_id: row.workspace_id.clone(),
            variable_path: row.path.clone(),
            description: row.description,
            value_expires_at: row.value_expires_at,
            is_secret: row.is_secret,
        };

        let pushed = push_variable_expiration_handler(
            db,
            tx,
            &row.workspace_id,
            &row.handler_path,
            &expiring,
            row.extra_args,
        )
        .await;

        let tx = match pushed {
            Ok((_job_id, tx)) => tx,
            Err(e) => {
                // The claim rides in the same transaction, so dropping it un-claims the
                // variable and a later pass retries it once the back-off elapses.
                tracing::error!(
                    "Error pushing variable expiration handler for {}: {e:#}",
                    row.path
                );
                failed_this_pass.insert(row.workspace_id.clone());
                record_handler_failure(db, &row.workspace_id, &row.handler_path, e).await;
                continue;
            }
        };

        if let Err(e) = tx.commit().await {
            tracing::error!("Error committing dispatch for {}: {e:#}", row.path);
            continue;
        }
        clear_handler_failure(&row.workspace_id);
        windmill_common::feature_usage::log_feature_usage(
            "variable_expiration",
            "dispatched",
            if row.is_secret { "secret" } else { "plain" },
        );
        dispatched += 1;
    }

    if dispatched > 0 {
        tracing::info!("Dispatched {dispatched} variable expiration handler job(s)");
    }
}

/// Back off this workspace's handler, and surface the cause once it is clearly not transient.
async fn record_handler_failure(
    db: &DB,
    w_id: &str,
    handler_path: &str,
    e: windmill_common::error::Error,
) {
    let should_surface = {
        let mut failures = VARIABLE_EXPIRATION_FAILURES.lock().unwrap();
        let state = failures.entry(w_id.to_string()).or_default();
        if state.handler_path != handler_path {
            *state = HandlerFailureState::default();
            state.handler_path = handler_path.to_string();
        }
        state.consecutive_failures += 1;
        state.cooldown_passes =
            (1u32 << state.consecutive_failures.min(5)).min(VARIABLE_EXPIRATION_MAX_BACKOFF_PASSES);
        let surface =
            !state.surfaced && state.consecutive_failures >= VARIABLE_EXPIRATION_ALERT_THRESHOLD;
        state.surfaced |= surface;
        surface
    };

    if should_surface {
        report_critical_error(
            format!(
                "The variable expiration handler {handler_path} of workspace {w_id} has repeatedly \
                 failed to start. Variables in this workspace will not run their expiration \
                 handler until the cause is fixed: {e:#}"
            ),
            db.clone(),
            Some(w_id),
            None,
        )
        .await;
    }
}

/// Drop a workspace's back-off as soon as one of its pushes succeeds, so a handler fixed out
/// of band recovers on the next pass.
fn clear_handler_failure(w_id: &str) {
    VARIABLE_EXPIRATION_FAILURES.lock().unwrap().remove(w_id);
}
