/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! Dispatches the workspace variable expiration handler for variables coming due.

use std::collections::HashSet;

use serde_json::value::RawValue;
use sqlx::types::Json;
use windmill_common::utils::report_critical_error;
use windmill_common::DB;

use crate::jobs::{push_variable_expiration_handler, ExpiringVariable};

/// Lead time when the handler setting has no `notify_before_secs`.
const VARIABLE_EXPIRATION_DEFAULT_NOTIFY_BEFORE_SECS: i64 = 24 * 3600;

/// Subtracted from `remind_every_secs` so a reminder whose interval is a multiple of the
/// sweep's period is not skipped by tick jitter.
const VARIABLE_EXPIRATION_REMINDER_SLACK_SECS: i64 = 30;

/// How long a workspace whose handler failed to start waits before it is tried again.
const VARIABLE_EXPIRATION_RETRY_AFTER_SECS: i64 = 3600;

/// Cap per pass, so a backlog drains over several passes instead of enqueuing at once.
const VARIABLE_EXPIRATION_MAX_PER_PASS: i64 = 100;

/// Advisory lock id ensuring only one server replica sweeps at a time. Must not reuse one of
/// the other monitor pass locks (737_483_920..=737_483_925): both sides try-lock and skip.
const VARIABLE_EXPIRATION_LOCK_ID: i64 = 737_483_926;

/// System caller only: dispatches across every workspace. A handler job that fails is not
/// re-dispatched before its next reminder, as re-running it would repeat its side effects
/// against the third party it rotates; retrying is the handler's job.
pub async fn dispatch_expiring_variables(db: &DB) {
    // Transaction-scoped, not session-scoped: a session lock would go back to the pool still
    // held if this future were dropped mid-pass, wedging the sweep on every replica.
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
        return;
    }

    dispatch_expiring_variables_inner(db).await;

    if let Err(e) = lock_tx.rollback().await {
        tracing::error!("variable expiration: releasing lock failed: {e:#}");
    }
}

async fn dispatch_expiring_variables_inner(db: &DB) {
    // No lower bound on `value_valid_until`, so a downed instance catches up. Rows are taken
    // round-robin across workspaces (`rn`) so a large backlog cannot fill the cap and starve the
    // other workspaces. First notices go before every reminder, which would otherwise refill the
    // cap on every pass.
    let due = sqlx::query!(
        r#"SELECT workspace_id AS "workspace_id!", path AS "path!", description AS "description!",
                  value_valid_until AS "value_valid_until!", is_secret AS "is_secret!",
                  expiration_dispatched_for, expiration_notified_at,
                  handler_path AS "handler_path!",
                  extra_args AS "extra_args: Json<Box<RawValue>>"
           FROM (
               SELECT v.workspace_id, v.path, v.description, v.value_valid_until, v.is_secret,
                      v.expiration_dispatched_for, v.expiration_notified_at,
                      ws.variable_expiration_handler->>'path' AS handler_path,
                      (ws.variable_expiration_handler->'extra_args')::text::json AS extra_args,
                      v.value_valid_until IS NOT DISTINCT FROM v.expiration_dispatched_for AS reminder,
                      row_number() OVER (PARTITION BY v.workspace_id
                          ORDER BY v.value_valid_until IS NOT DISTINCT FROM v.expiration_dispatched_for,
                                   v.value_valid_until) AS rn
               FROM variable v
               JOIN workspace_settings ws ON ws.workspace_id = v.workspace_id
               JOIN workspace w ON w.id = v.workspace_id
               WHERE NOT w.deleted
                 AND v.value_valid_until IS NOT NULL
                 AND (v.value_valid_until IS DISTINCT FROM v.expiration_dispatched_for
                      OR (v.value_valid_until > now()
                          AND v.expiration_notified_at <= now() - make_interval(secs =>
                              NULLIF((ws.variable_expiration_handler->>'remind_every_secs')::bigint, 0) - $3)))
                 AND v.value_valid_until <= now() + make_interval(secs =>
                     COALESCE((ws.variable_expiration_handler->>'notify_before_secs')::bigint, $1))
                 AND ws.variable_expiration_handler->>'path' IS NOT NULL
                 AND (ws.variable_expiration_failed_at IS NULL
                      OR ws.variable_expiration_failed_at <= now() - make_interval(secs => $4)
                      -- Redeploying the handler is how it gets fixed, so it ends the wait.
                      OR EXISTS (SELECT 1 FROM script s WHERE s.workspace_id = v.workspace_id
                                 AND s.path = substr(ws.variable_expiration_handler->>'path', 8)
                                 AND starts_with(ws.variable_expiration_handler->>'path', 'script/')
                                 AND s.created_at > ws.variable_expiration_failed_at)
                      OR EXISTS (SELECT 1 FROM flow f WHERE f.workspace_id = v.workspace_id
                                 AND f.path = substr(ws.variable_expiration_handler->>'path', 6)
                                 AND starts_with(ws.variable_expiration_handler->>'path', 'flow/')
                                 AND f.edited_at > ws.variable_expiration_failed_at))
                 AND (NOT COALESCE((ws.variable_expiration_handler->>'muted_on_user_path')::boolean, false)
                      OR v.path NOT LIKE 'u/%')
           ) due
           ORDER BY reminder, rn, value_valid_until
           LIMIT $2"#,
        VARIABLE_EXPIRATION_DEFAULT_NOTIFY_BEFORE_SECS,
        VARIABLE_EXPIRATION_MAX_PER_PASS,
        VARIABLE_EXPIRATION_REMINDER_SLACK_SECS,
        VARIABLE_EXPIRATION_RETRY_AFTER_SECS as f64,
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
    // A handler fails alike for every variable of its workspace: try it once per pass.
    let mut failed_this_pass: HashSet<String> = HashSet::new();

    for row in due {
        if failed_this_pass.contains(&row.workspace_id) {
            continue;
        }

        // Claim and push in one transaction. The claim matches the row as it was read, so it
        // loses both to a concurrent re-arm and to whichever replica got there first.
        let mut tx = match db.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                tracing::error!("Error opening tx for expiring variable {}: {e:#}", row.path);
                continue;
            }
        };

        let claimed = sqlx::query_scalar!(
            "UPDATE variable SET expiration_dispatched_for = $3, expiration_notified_at = now()
             WHERE workspace_id = $1 AND path = $2 AND value_valid_until = $3
               AND expiration_dispatched_for IS NOT DISTINCT FROM $4
               AND expiration_notified_at IS NOT DISTINCT FROM $5
             RETURNING 1",
            row.workspace_id,
            row.path,
            row.value_valid_until,
            row.expiration_dispatched_for,
            row.expiration_notified_at,
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
            value_valid_until: row.value_valid_until,
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
                // Dropping the tx un-claims the variable. The workspace then sits out: its rows
                // stay due, and would otherwise retake the same capped slots on every pass,
                // ahead of every other workspace.
                failed_this_pass.insert(row.workspace_id.clone());
                if let Err(e) = sqlx::query!(
                    // A handler saved since this pass read it gets its own first try.
                    "UPDATE workspace_settings SET variable_expiration_failed_at = now()
                     WHERE workspace_id = $1 AND variable_expiration_handler->>'path' = $2",
                    row.workspace_id,
                    row.handler_path,
                )
                .execute(db)
                .await
                {
                    tracing::error!(
                        "Error deferring variable expiration handler of {}: {e:#}",
                        row.workspace_id
                    );
                }
                report_critical_error(
                    format!(
                        "The variable expiration handler {} of workspace {} failed to start, so \
                         variables in this workspace are not being notified. They are tried again \
                         in an hour, or as soon as the handler is redeployed or its setting saved: \
                         {e:#}",
                        row.handler_path, row.workspace_id
                    ),
                    db.clone(),
                    Some(&row.workspace_id),
                    None,
                )
                .await;
                continue;
            }
        };

        if let Err(e) = tx.commit().await {
            tracing::error!("Error committing dispatch for {}: {e:#}", row.path);
            continue;
        }
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
